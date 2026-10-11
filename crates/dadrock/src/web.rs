use std::{
    any::Any,
    collections::BTreeMap,
    env, fs,
    net::{TcpListener, TcpStream},
    panic::{self, AssertUnwindSafe},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use dadrock::{
    agent::{
        acp::AcpPermissionPolicy,
        providers::{
            configured_local_acp_agent, detect_local_acp_agent, known_acp_providers,
            ACP_AGENT_ARGS_ENV, ACP_AGENT_ENV,
        },
    },
    commands::{
        self as core_commands, AgentDraftCommandRequest, ApplyCandidateRevisionRequest,
        CommandContext, CommandEnvelope, NewSuccessorRequest, SaveRecordSourceRequest,
        SaveRecordSourceResult, SupersedeRecordsRequest,
    },
    index::ProjectIndex,
    links::{check_project_links, LinkIssue},
    project::{DiscoverySource, ProjectContext},
    records::{status_record, PendingSupersedes, ScannedRecord, ScannedRecordSummary},
    tags::{build_tag_report, TagIssue, TagReport},
    tasks::{
        clear_task_runs as core_clear_task_runs, create_task_run_for_task, read_task_run_log,
        resolve_task_run_dir as resolve_project_task_run_dir, run_created_local_task,
        run_created_task_with_acp, write_task_failure, AcpTaskRunOptions, TaskContext,
        TaskExecutionMode,
    },
    templates::{candidate_template_body, resolve_template, TemplateName, TemplateSource},
    validation::{validate_project_index, ProjectValidationIssue},
    DadrError, Diagnostic, IssueCounts, RecordId, Severity,
};
use serde::{Deserialize, Serialize};

mod history;
mod http;
mod routing;
mod static_assets;

#[cfg(test)]
use std::{borrow::Cow, time::Instant};

#[cfg(test)]
use self::http::{
    read_http_request_with_deadline, reason_phrase, static_response, HttpHeader, MAX_BODY_BYTES,
    MAX_HEADER_BYTES,
};
#[cfg(test)]
use self::routing::{
    record_lifecycle_query, record_revision_query, task_cancel_query, task_log_query, task_query,
};
use self::{
    http::{
        api_error_response, json_body, json_response, read_http_request, write_http_response,
        ApiFailure, HttpRequest, HttpResponse,
    },
    routing::route_request,
};

/// Set by a parent that runs `dadrock preview` as a child process (the desktop
/// app) and holds the write end of its stdin. The server exits when stdin
/// closes, which the OS does whenever the parent goes away, crashes included,
/// so a server never outlives the window it was started for.
const EXIT_ON_STDIN_EOF_ENV: &str = "DADROCK_PREVIEW_EXIT_ON_STDIN_EOF";

#[derive(Debug, Clone)]
pub(crate) struct PreviewOptions {
    pub(crate) directory: Option<Utf8PathBuf>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

pub(crate) fn serve(options: PreviewOptions) -> Result<()> {
    validate_bind_host(&options.host).map_err(|err| anyhow::anyhow!(err))?;
    if env::var_os(EXIT_ON_STDIN_EOF_ENV).is_some() {
        thread::spawn(|| {
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
            std::process::exit(0);
        });
    }
    let cwd = preview_cwd(options.directory)?;
    let listener = TcpListener::bind((options.host.as_str(), options.port)).with_context(|| {
        format!(
            "failed to bind preview server on {}:{}",
            options.host, options.port
        )
    })?;
    let address = listener
        .local_addr()
        .context("failed to inspect preview server address")?;

    println!("dadrock preview serving {}", cwd);
    println!("open http://{address}");
    eprintln!("dadrock-preview: listening on http://{address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let cwd = cwd.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, cwd) {
                        eprintln!("dadrock preview request failed: {err:#}");
                    }
                });
            }
            Err(err) => eprintln!("dadrock preview connection failed: {err}"),
        }
    }

    Ok(())
}

fn validate_bind_host(host: &str) -> Result<(), DadrError> {
    let host = host.trim();
    let allowed = host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]";
    if allowed {
        Ok(())
    } else {
        Err(DadrError::InvalidInput(format!(
            "preview host `{host}` is not allowed; use localhost or 127.0.0.1"
        )))
    }
}

fn preview_cwd(directory: Option<Utf8PathBuf>) -> Result<Utf8PathBuf> {
    let current = CommandContext::current_dir()?.cwd;
    Ok(match directory {
        Some(directory) if directory.is_absolute() => directory,
        Some(directory) => current.join(directory),
        None => current,
    })
}

fn handle_connection(mut stream: TcpStream, cwd: Utf8PathBuf) -> Result<()> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .context("failed to set preview request timeout")?;
    let response = match read_http_request(&mut stream) {
        Ok(request) => match route_request(&request, &cwd) {
            Ok(response) => response,
            Err(error) => api_error_response(error.status, error.message),
        },
        Err(error) => api_error_response(error.status, error.message),
    };
    write_http_response(&mut stream, response)
}

fn initialize_project(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::initialize_project(&context, None)?;
    mutation_response(
        cwd,
        format!(
            "created {} paths, reused {} paths",
            envelope.data.created.len(),
            envelope.data.existing.len()
        ),
        envelope.changed_paths,
    )
}

fn create_record(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: CreateRecordRequest = json_body(request)?;
    let context = context_for(cwd)?;
    let supersedes = body
        .supersedes
        .map(SupersedeTargets::into_vec)
        .unwrap_or_default();
    let envelope = if !supersedes.is_empty() {
        // Dadrock writes the `## Supersedes` section: one link per
        // predecessor, then the reason.
        core_commands::new_successor_record(
            &context,
            NewSuccessorRequest {
                title: body.title.trim().to_owned(),
                tags: body.tags,
                body_markdown: body.body_markdown,
                supersedes,
                reason: body.supersedes_reason,
            },
        )?
    } else if let Some(body_markdown) = body.body_markdown.as_deref() {
        core_commands::new_record_with_body(&context, body.title.trim(), &body.tags, body_markdown)?
    } else {
        core_commands::new_record(&context, body.title.trim(), &body.tags)?
    };
    mutation_response_with_focus(
        cwd,
        format!("created candidate {}", envelope.data.id),
        envelope.changed_paths,
        Some(ApiMutationFocus {
            selected_id: Some(envelope.data.id.to_string()),
            status_filter: Some("candidate".to_owned()),
        }),
    )
}

fn list_tasks(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::list_tasks_command(&context_for(cwd)?);
    json_response(200, &envelope)
}

fn show_task(cwd: &Utf8PathBuf, task_id: &str) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::show_task_command(&context_for(cwd)?, task_id)?;
    json_response(200, &envelope)
}

fn list_task_runs(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::list_task_runs_command(&context_for(cwd)?)?;
    json_response(200, &envelope)
}

fn clear_task_history(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let project = context.discover_project()?;
    let project_root = task_project_root(&project);
    let cleared = core_clear_task_runs(&project_root)?;
    let changed_paths = cleared.into_iter().collect::<Vec<_>>();
    mutation_response(cwd, "Agent runs cleared.".to_owned(), changed_paths)
}

fn show_task_result(cwd: &Utf8PathBuf, run_id: &str) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::show_task_result_command(&context_for(cwd)?, run_id)?;
    json_response(200, &envelope)
}

fn show_task_log(cwd: &Utf8PathBuf, run_id: &str) -> Result<HttpResponse, ApiFailure> {
    let run_dir = resolve_task_run_dir(cwd, run_id)?;
    let envelope = CommandEnvelope::new("task log", read_task_run_log(&run_dir)?);
    json_response(200, &envelope)
}

fn cancel_task_run(cwd: &Utf8PathBuf, run_id: &str) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::cancel_task_run_command(&context, run_id)?;
    json_response(200, &envelope)
}

fn run_task(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: RunTaskRequest = json_body(request)?;
    let context = context_for(cwd)?;
    let mut project = context.discover_project()?;
    project.cwd = task_project_root(&project);
    let input = body.input.unwrap_or_else(|| serde_json::json!({}));
    let task_context = body.context.unwrap_or_default();
    let (task, run) =
        create_task_run_for_task(&project, &body.task_id, input.clone(), &task_context)?;
    if task.manifest.execution == TaskExecutionMode::Local {
        let run = run_created_local_task(&project, &task, input, task_context, &run)?;
        let envelope =
            CommandEnvelope::new("task run", run.clone()).with_changed_paths(vec![run.run_dir]);
        return json_response(200, &envelope);
    }

    project.require_initialized()?;
    let local_agent = configured_local_acp_agent()?;
    let run_for_response = run.clone();
    let project_for_thread = project.clone();
    thread::spawn(move || {
        run_agent_task_thread(
            project_for_thread,
            task,
            input,
            task_context,
            local_agent,
            run,
        );
    });
    let envelope = CommandEnvelope::new("task run", run_for_response.clone())
        .with_changed_paths(vec![run_for_response.run_dir.clone()]);
    json_response(200, &envelope)
}

fn run_agent_task_thread(
    project: ProjectContext,
    task: dadrock::tasks::TaskPackage,
    input: serde_json::Value,
    task_context: TaskContext,
    local_agent: dadrock::agent::providers::LocalAcpAgentCommand,
    run: dadrock::tasks::PersistedTaskRun,
) {
    let run_id = run.id;
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        run_created_task_with_acp(
            &project,
            &task,
            input,
            task_context,
            &run,
            AcpTaskRunOptions {
                local_agent: &local_agent,
                permission_policy: AcpPermissionPolicy::AllowOnce,
                enable_dadrock_mcp: true,
                events: None,
            },
        )
    }));
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => eprintln!("dadrock task run {run_id} failed: {err}"),
        Err(payload) => {
            persist_background_task_panic(&run, payload);
        }
    }
}

fn persist_background_task_panic(
    run: &dadrock::tasks::PersistedTaskRun,
    payload: Box<dyn Any + Send>,
) {
    let message = format!(
        "background task panicked: {}",
        panic_payload_message(payload.as_ref())
    );
    let _ = write_task_failure(&run.run_dir, message.clone(), None);
    eprintln!("dadrock task run {} panicked: {message}", run.id);
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic payload".to_owned())
}

fn candidate_template(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let project = context.discover_project()?;
    project.require_initialized()?;
    let template = candidate_template_body(&project)?;
    let source = match template.resolution.source {
        TemplateSource::Project => "project",
        TemplateSource::BuiltIn => "built_in",
    };

    json_response(
        200,
        &ApiTemplateContent {
            name: template.resolution.name.to_string(),
            source: source.to_owned(),
            path: template.resolution.path.map(|path| path.to_string()),
            body: template.body,
        },
    )
}

fn accept_candidate(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
    query: &str,
) -> Result<HttpResponse, ApiFailure> {
    // An optional body names more candidates to accept in the same operation,
    // such as the other halves of a split.
    let body: AcceptCandidateBody = if request.body.iter().all(u8::is_ascii_whitespace) {
        AcceptCandidateBody::default()
    } else {
        json_body(request)?
    };
    let mut queries = vec![query.to_owned()];
    queries.extend(body.with);
    let context = context_for(cwd)?;
    let envelope = core_commands::accept_records(&context, &queries)?;
    let accepted = &envelope.data.accepted;
    let message = match accepted.as_slice() {
        [one] => format!("accepted candidate {}", one.id),
        many => format!("accepted {} candidates", many.len()),
    };
    let focus = accepted
        .first()
        .map(|first| record_focus(first.id, Some("accepted".to_owned())));
    mutation_response_with_focus(cwd, message, envelope.changed_paths, focus)
}

fn reject_candidate(cwd: &Utf8PathBuf, query: &str) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::reject_record(&context, query)?;
    mutation_response_with_focus(
        cwd,
        format!("rejected candidate {}", envelope.data.id),
        envelope.changed_paths,
        Some(record_focus(envelope.data.id, None)),
    )
}

fn supersede_records(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: SupersedeRequest = json_body(request)?;
    let context = context_for(cwd)?;
    let envelope = core_commands::supersede_records(
        &context,
        SupersedeRecordsRequest {
            from: vec![body.from],
            to: body.to.into_vec(),
        },
    )?;
    mutation_response(cwd, "record superseded".to_owned(), envelope.changed_paths)
}

fn record_source(cwd: &Utf8PathBuf, query: &str) -> Result<HttpResponse, ApiFailure> {
    let (_, index) = initialized_index_for(cwd)?;
    let found = index.resolve_one(query)?;
    let content = fs::read_to_string(found.record.path.as_std_path()).map_err(|err| {
        ApiFailure::bad_request(format!("failed to read {}: {err}", found.record.path))
    })?;

    json_response(
        200,
        &ApiRecordSource {
            id: found.record.id.to_string(),
            path: found.record.path.to_string(),
            content,
        },
    )
}

fn record_lifecycle(cwd: &Utf8PathBuf, query: &str) -> Result<HttpResponse, ApiFailure> {
    let (_, index) = initialized_index_for(cwd)?;
    let tag_report = build_tag_report(&index);
    let tag_map = record_tag_map(&tag_report);
    let result = status_record(&index, query)?;
    let mut diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .map(api_diagnostic_from_record_diagnostic)
        .collect();
    let pending = match dadrock::records::pending_supersedes(&index, query) {
        Ok(pending) => pending,
        Err(error) => {
            diagnostics.push(ApiDiagnostic::error(
                "invalid_supersedes",
                error.to_string(),
            ));
            PendingSupersedes::default()
        }
    };

    json_response(
        200,
        &ApiRecordLifecycle {
            pending_supersedes: pending
                .predecessors
                .iter()
                .map(|record| api_record(&index, record, &tag_map))
                .collect(),
            stale_supersedes: pending
                .stale
                .iter()
                .map(|stale| ApiStaleSupersedes {
                    predecessor: api_record(&index, &stale.predecessor, &tag_map),
                    current_successors: stale
                        .current_successors
                        .iter()
                        .map(|record| api_record(&index, record, &tag_map))
                        .collect(),
                })
                .collect(),
            record: api_record(&index, &result.record, &tag_map),
            predecessors: result
                .predecessors
                .iter()
                .map(|record| api_record(&index, record, &tag_map))
                .collect(),
            successors: result
                .successors
                .iter()
                .map(|record| api_record(&index, record, &tag_map))
                .collect(),
            diagnostics,
        },
    )
}

fn save_record_source(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
    query: &str,
) -> Result<HttpResponse, ApiFailure> {
    let body: SaveRecordSourceBody = json_body(request)?;
    let context = context_for(cwd)?;
    let envelope = core_commands::save_record_source(
        &context,
        SaveRecordSourceRequest {
            query: query.to_owned(),
            content: body.content,
        },
    )?;

    mutation_response_with_focus(
        cwd,
        format!("saved {}", envelope.data.id),
        envelope.changed_paths,
        Some(record_focus(
            envelope.data.id,
            Some(saved_record_status_filter(&envelope.data)),
        )),
    )
}

fn apply_record_revision(
    request: &HttpRequest,
    cwd: &Utf8PathBuf,
    query: &str,
) -> Result<HttpResponse, ApiFailure> {
    let body: ApplyRecordRevisionBody = json_body(request)?;
    let context = context_for(cwd)?;
    let envelope = core_commands::apply_candidate_revision(
        &context,
        ApplyCandidateRevisionRequest {
            query: query.to_owned(),
            title: body.title,
            tags: body.tags,
            body_markdown: body.body_markdown,
        },
    )?;
    mutation_response_with_focus(
        cwd,
        format!("applied revision {}", envelope.data.candidate.id),
        envelope.changed_paths,
        Some(record_focus(
            envelope.data.candidate.id,
            Some("candidate".to_owned()),
        )),
    )
}

fn validate_project(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::validate_project(&context_for(cwd)?)?;
    mutation_response(
        cwd,
        format!("project validation: {}", envelope.data.counts.label()),
        envelope.changed_paths,
    )
}

fn validate_records(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::validate_records(&context_for(cwd)?, Vec::new())?;
    let counts = IssueCounts::from_severities(envelope.data.records.iter().flat_map(|record| {
        record
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.severity)
    }));
    mutation_response(
        cwd,
        format!("record validation: {}", counts.label()),
        envelope.changed_paths,
    )
}

fn validate_links(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::validate_links(&context_for(cwd)?)?;
    mutation_response(
        cwd,
        format!("link validation: {}", envelope.data.counts.label()),
        envelope.changed_paths,
    )
}

fn validate_tags(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let envelope = core_commands::validate_tags(&context_for(cwd)?)?;
    mutation_response(
        cwd,
        format!("tag validation: {}", envelope.data.counts.label()),
        envelope.changed_paths,
    )
}

fn agent_draft(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: AgentDraftRequest = json_body(request)?;
    let local_agent = configured_local_acp_agent().map_err(|err| {
        ApiFailure::bad_request(format!("agent draft configuration failed: {err}"))
    })?;
    let envelope = core_commands::draft_with_acp_agent(
        &context_for(cwd)?,
        AgentDraftCommandRequest {
            prompt: body.prompt,
            local_agent,
            permission_policy: AcpPermissionPolicy::Deny,
            prompt_packs: body.prompt_packs,
            enable_dadrock_mcp: true,
            write: body.write,
        },
    )
    .map_err(|err| ApiFailure::bad_request(format!("agent draft failed: {err}")))?;
    let message = envelope
        .data
        .candidate
        .as_ref()
        .map(|candidate| format!("drafted candidate {}", candidate.id))
        .unwrap_or_else(|| "agent draft generated".to_owned());
    let focus = envelope
        .data
        .candidate
        .as_ref()
        .map(|candidate| record_focus(candidate.id, Some("candidate".to_owned())));
    mutation_response_with_focus(cwd, message, envelope.changed_paths, focus)
}

fn mutation_response(
    cwd: &Utf8PathBuf,
    message: String,
    changed_paths: Vec<Utf8PathBuf>,
) -> Result<HttpResponse, ApiFailure> {
    mutation_response_with_focus(cwd, message, changed_paths, None)
}

fn mutation_response_with_focus(
    cwd: &Utf8PathBuf,
    message: String,
    changed_paths: Vec<Utf8PathBuf>,
    focus: Option<ApiMutationFocus>,
) -> Result<HttpResponse, ApiFailure> {
    json_response(
        200,
        &ApiMutationResponse {
            message,
            changed_paths: changed_paths.iter().map(ToString::to_string).collect(),
            focus,
            snapshot: build_snapshot(cwd)?,
        },
    )
}

fn record_focus(record_id: RecordId, status_filter: Option<String>) -> ApiMutationFocus {
    ApiMutationFocus {
        selected_id: Some(record_id.to_string()),
        status_filter,
    }
}

fn context_for(cwd: &Utf8PathBuf) -> Result<CommandContext, DadrError> {
    CommandContext::from_path(cwd.as_str())
}

fn initialized_index_for(cwd: &Utf8PathBuf) -> Result<(ProjectContext, ProjectIndex), ApiFailure> {
    Ok(context_for(cwd)?.initialized_index()?)
}

fn build_snapshot(cwd: &Utf8PathBuf) -> Result<ApiSnapshot, DadrError> {
    let context = context_for(cwd)?;
    let project = context.discover_project()?;
    let command_catalog = core_commands::command_catalog().data;
    if !project.initialized {
        return Ok(ApiSnapshot {
            project: api_project(&project),
            records: Vec::new(),
            counts: ApiRecordCounts::default(),
            tags: Vec::new(),
            project_validation: ApiDiagnosticPanel::default(),
            record_validation: ApiDiagnosticPanel::default(),
            link_health: ApiDiagnosticPanel::default(),
            settings: ApiSettings {
                agent: agent_settings_lines(),
                agent_available: agent_available(),
                ..ApiSettings::default()
            },
            command_catalog,
        });
    }

    let index = ProjectIndex::scan(&project)?;
    let tag_report = build_tag_report(&index);
    let records = api_records(&index, &tag_report);
    let counts = ApiRecordCounts::from_records(&records);

    let project_report = validate_project_index(&index);
    let project_diagnostics = project_report
        .issues
        .iter()
        .map(api_diagnostic_from_project_issue)
        .collect::<Vec<_>>();
    let record_diagnostics = record_validation_diagnostics(index.scanned_records());
    let record_counts =
        IssueCounts::from_severities(index.scanned_records().iter().flat_map(|record| {
            record
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.severity)
        }));
    let link_report = check_project_links(&index);
    let link_diagnostics = link_report
        .issues
        .iter()
        .map(api_diagnostic_from_link_issue)
        .collect::<Vec<_>>();
    let tags = api_tags(&tag_report);
    let settings = settings_panel(&context, &project, &tag_report);

    Ok(ApiSnapshot {
        project: api_project(&project),
        records,
        counts,
        tags,
        project_validation: ApiDiagnosticPanel {
            counts: project_report.counts,
            diagnostics: project_diagnostics,
        },
        record_validation: ApiDiagnosticPanel {
            counts: record_counts,
            diagnostics: record_diagnostics,
        },
        link_health: ApiDiagnosticPanel {
            counts: link_report.counts,
            diagnostics: link_diagnostics,
        },
        settings,
        command_catalog,
    })
}

fn api_project(project: &ProjectContext) -> ApiProject {
    ApiProject {
        cwd: project.cwd.to_string(),
        decisions_dir: project.ledger_dir().to_string(),
        initialized: project.initialized,
        discovery_source: discovery_source_label(&project.discovery_source).to_owned(),
        status: if project.initialized {
            "Project discovered. Commands operate on the local decision records.".to_owned()
        } else {
            "No Dadrock project found here. Initialize to create dadr/ with inbox, ledger, archive, templates, tasks, and runs."
                .to_owned()
        },
    }
}

fn discovery_source_label(source: &DiscoverySource) -> &'static str {
    match source {
        DiscoverySource::Directory => "directory",
        DiscoverySource::Fallback => "fallback",
    }
}

fn api_records(index: &ProjectIndex, tags: &TagReport) -> Vec<ApiRecord> {
    let tag_map = record_tag_map(tags);
    index
        .summaries()
        .iter()
        .map(|record| api_record(index, record, &tag_map))
        .collect()
}

fn api_record(
    index: &ProjectIndex,
    record: &ScannedRecordSummary,
    tag_map: &BTreeMap<String, Vec<String>>,
) -> ApiRecord {
    let id = record.id.to_string();
    let simple_record_id = id
        .chars()
        .filter(|ch| *ch != '-')
        .collect::<String>()
        .to_ascii_lowercase();
    ApiRecord {
        id: id.clone(),
        id_prefix: simple_record_id.chars().take(12).collect(),
        simple_record_id,
        title: record.title.clone(),
        status: record.status.as_str().to_owned(),
        superseded: record.superseded,
        path: record.path.to_string(),
        preview: preview_for_summary(index, record),
        tags: tag_map.get(&id).cloned().unwrap_or_default(),
        timestamp_ms: record.timestamp_ms,
    }
}

fn record_tag_map(tags: &TagReport) -> BTreeMap<String, Vec<String>> {
    let mut by_record = BTreeMap::<String, Vec<String>>::new();
    for tag in &tags.tags {
        for record in &tag.records {
            by_record
                .entry(record.id.to_string())
                .or_default()
                .push(tag.tag.clone());
        }
    }
    by_record
}

fn preview_for_summary(index: &ProjectIndex, summary: &ScannedRecordSummary) -> String {
    let body = index
        .scanned_records()
        .iter()
        .find(|scanned| scanned.path == summary.path)
        .and_then(|scanned| scanned.record.as_ref())
        .map(|record| record.body.raw_markdown.trim())
        .unwrap_or("");
    if body.is_empty() {
        "No preview available.".to_owned()
    } else {
        body.to_owned()
    }
}

fn api_tags(report: &TagReport) -> Vec<ApiTag> {
    report
        .tags
        .iter()
        .map(|tag| ApiTag {
            tag: tag.tag.clone(),
            display: tag.display.clone(),
            record_count: tag.record_count,
            occurrence_count: tag.occurrence_count,
        })
        .collect()
}

fn record_validation_diagnostics(records: &[ScannedRecord]) -> Vec<ApiDiagnostic> {
    records
        .iter()
        .flat_map(|record| {
            record
                .diagnostics
                .iter()
                .map(api_diagnostic_from_record_diagnostic)
        })
        .collect()
}

fn api_diagnostic_from_record_diagnostic(diagnostic: &Diagnostic) -> ApiDiagnostic {
    ApiDiagnostic {
        severity: severity_label(diagnostic.severity).to_owned(),
        code: diagnostic.code.as_str().to_owned(),
        sub_code: None,
        message: diagnostic.message.clone(),
        path: diagnostic.path.as_ref().map(ToString::to_string),
        record_id: None,
        field: diagnostic.field.clone(),
        line: None,
        related_paths: Vec::new(),
    }
}

fn api_diagnostic_from_project_issue(issue: &ProjectValidationIssue) -> ApiDiagnostic {
    ApiDiagnostic {
        severity: severity_label(issue.severity).to_owned(),
        code: issue.code.to_string(),
        sub_code: issue.sub_code.clone(),
        message: issue.message.clone(),
        path: issue.path.as_ref().map(ToString::to_string),
        record_id: issue.record_id.as_ref().map(ToString::to_string),
        field: issue.field.clone(),
        line: issue.line,
        related_paths: issue
            .related_paths
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn api_diagnostic_from_link_issue(issue: &LinkIssue) -> ApiDiagnostic {
    ApiDiagnostic {
        severity: severity_label(issue.severity).to_owned(),
        code: issue.code.to_string(),
        sub_code: None,
        message: issue.message.clone(),
        path: Some(issue.path.to_string()),
        record_id: issue.record_id.as_ref().map(ToString::to_string),
        field: Some(issue.target.clone()),
        line: Some(issue.line),
        related_paths: issue
            .related_paths
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn api_diagnostic_from_tag_issue(issue: &TagIssue) -> ApiDiagnostic {
    ApiDiagnostic {
        severity: severity_label(issue.severity).to_owned(),
        code: issue.code.to_string(),
        sub_code: None,
        message: issue.message.clone(),
        path: Some(issue.path.to_string()),
        record_id: issue.record_id.as_ref().map(ToString::to_string),
        field: issue.tag.clone(),
        line: issue.line,
        related_paths: Vec::new(),
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn settings_panel(
    context: &CommandContext,
    project: &ProjectContext,
    tag_report: &TagReport,
) -> ApiSettings {
    let mut settings = ApiSettings {
        tag_counts: tag_report.counts,
        agent: agent_settings_lines(),
        agent_available: agent_available(),
        tags: tag_lines(tag_report),
        diagnostics: tag_report
            .issues
            .iter()
            .map(api_diagnostic_from_tag_issue)
            .collect(),
        ..ApiSettings::default()
    };

    match core_commands::show_config(context) {
        Ok(envelope) => {
            let layout = envelope.data;
            settings.config = vec![
                format!("project dir: {}", layout.project_dir),
                format!("inbox dir: {}", layout.inbox_dir),
                format!("ledger dir: {}", layout.ledger_dir),
                format!("archive dir: {}", layout.archive_dir),
                format!("templates dir: {}", layout.templates_dir),
                format!("tasks dir: {}", layout.tasks_dir),
                format!("runs dir: {}", layout.runs_dir),
            ];
        }
        Err(err) => settings.diagnostics.push(ApiDiagnostic::error(
            "layout",
            format!("failed to resolve project layout: {err}"),
        )),
    }

    match resolve_template(project, TemplateName::Candidate) {
        Ok(candidate) => {
            let source = match candidate.source {
                TemplateSource::Project => "project",
                TemplateSource::BuiltIn => "built_in",
            };
            let path = candidate
                .path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "built-in".to_owned());
            settings
                .templates
                .push(format!("{}: {source} ({path})", candidate.name));
        }
        Err(err) => settings.diagnostics.push(ApiDiagnostic::error(
            "template",
            format!("failed to resolve template: {err}"),
        )),
    }

    settings
}

fn agent_available() -> bool {
    match env::var(ACP_AGENT_ENV) {
        Ok(agent) if !agent.trim().is_empty() => configured_local_acp_agent().is_ok(),
        _ => detect_local_acp_agent().is_some(),
    }
}

fn agent_settings_lines() -> Vec<String> {
    let args = env::var(ACP_AGENT_ARGS_ENV).unwrap_or_else(|_| "none".to_owned());
    let mut lines = vec!["mode: local ACP agent adaptor".to_owned()];
    match env::var(ACP_AGENT_ENV) {
        Ok(agent) if !agent.trim().is_empty() => match configured_local_acp_agent() {
            Ok(command) => lines.push(format!("configured: {}", command.command_line())),
            Err(err) => lines.push(format!("configured error: {err}")),
        },
        _ => {
            if let Some(detected) = detect_local_acp_agent() {
                lines.push(format!(
                    "auto-detected: {} ({})",
                    detected.provider.name,
                    detected.command.command_line()
                ));
            } else {
                lines.push("auto-detected: none".to_owned());
            }
        }
    }
    lines.push(format!("{ACP_AGENT_ARGS_ENV}: {args}"));
    lines.push(format!(
        "known providers: {}",
        known_acp_providers()
            .iter()
            .map(|provider| provider.command_line())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines
}

fn tag_lines(report: &TagReport) -> Vec<String> {
    if report.tags.is_empty() {
        return vec!["No tags found.".to_owned()];
    }
    report
        .tags
        .iter()
        .take(12)
        .map(|tag| {
            format!(
                "#{}: {} records, {} occurrences",
                tag.display, tag.record_count, tag.occurrence_count
            )
        })
        .collect()
}

fn resolve_task_run_dir(cwd: &Utf8PathBuf, run_id: &str) -> Result<Utf8PathBuf, DadrError> {
    let context = context_for(cwd)?;
    let project = context.discover_project()?;
    let project_root = task_project_root(&project);
    resolve_project_task_run_dir(&project_root, run_id)
}

fn task_project_root(project: &ProjectContext) -> Utf8PathBuf {
    project.workspace_dir()
}

fn saved_record_status_filter(record: &SaveRecordSourceResult) -> String {
    if record.superseded {
        "superseded".to_owned()
    } else {
        record.status.as_str().to_owned()
    }
}

#[derive(Serialize)]
struct ApiMutationResponse {
    message: String,
    changed_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<ApiMutationFocus>,
    snapshot: ApiSnapshot,
}

#[derive(Serialize)]
struct ApiMutationFocus {
    selected_id: Option<String>,
    status_filter: Option<String>,
}

#[derive(Serialize)]
struct ApiRecordSource {
    id: String,
    path: String,
    content: String,
}

#[derive(Serialize)]
struct ApiTemplateContent {
    name: String,
    source: String,
    path: Option<String>,
    body: String,
}

#[derive(Serialize)]
struct ApiSnapshot {
    project: ApiProject,
    records: Vec<ApiRecord>,
    counts: ApiRecordCounts,
    tags: Vec<ApiTag>,
    project_validation: ApiDiagnosticPanel,
    record_validation: ApiDiagnosticPanel,
    link_health: ApiDiagnosticPanel,
    settings: ApiSettings,
    command_catalog: core_commands::CommandCatalogResult,
}

#[derive(Serialize)]
struct ApiProject {
    cwd: String,
    /// The ledger, where the decisions in force live.
    decisions_dir: String,
    initialized: bool,
    discovery_source: String,
    status: String,
}

#[derive(Serialize)]
struct ApiRecord {
    id: String,
    id_prefix: String,
    simple_record_id: String,
    title: String,
    status: String,
    superseded: bool,
    path: String,
    preview: String,
    tags: Vec<String>,
    timestamp_ms: u64,
}

#[derive(Serialize)]
struct ApiRecordLifecycle {
    /// Every record a candidate's `## Supersedes` section names.
    pending_supersedes: Vec<ApiRecord>,
    /// The named records that have been superseded since, each with the
    /// decisions now in force in its place.
    stale_supersedes: Vec<ApiStaleSupersedes>,
    record: ApiRecord,
    predecessors: Vec<ApiRecord>,
    successors: Vec<ApiRecord>,
    diagnostics: Vec<ApiDiagnostic>,
}

#[derive(Serialize)]
struct ApiStaleSupersedes {
    predecessor: ApiRecord,
    current_successors: Vec<ApiRecord>,
}

#[derive(Default, Serialize)]
struct ApiRecordCounts {
    total: usize,
    candidates: usize,
    accepted: usize,
    rejected: usize,
    superseded: usize,
}

impl ApiRecordCounts {
    fn from_records(records: &[ApiRecord]) -> Self {
        let mut counts = Self {
            total: records.len(),
            ..Self::default()
        };
        for record in records {
            match record.status.as_str() {
                "candidate" => counts.candidates += 1,
                "accepted" if record.superseded => counts.superseded += 1,
                "accepted" => counts.accepted += 1,
                "rejected" => counts.rejected += 1,
                _ => {}
            }
        }
        counts
    }
}

#[derive(Serialize)]
struct ApiTag {
    tag: String,
    display: String,
    record_count: usize,
    occurrence_count: usize,
}

#[derive(Default, Serialize)]
struct ApiDiagnosticPanel {
    counts: IssueCounts,
    diagnostics: Vec<ApiDiagnostic>,
}

#[derive(Serialize)]
struct ApiDiagnostic {
    severity: String,
    code: String,
    sub_code: Option<String>,
    message: String,
    path: Option<String>,
    record_id: Option<String>,
    field: Option<String>,
    line: Option<usize>,
    related_paths: Vec<String>,
}

impl ApiDiagnostic {
    fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: "error".to_owned(),
            code: code.into(),
            sub_code: None,
            message: message.into(),
            path: None,
            record_id: None,
            field: None,
            line: None,
            related_paths: Vec::new(),
        }
    }
}

#[derive(Default, Serialize)]
struct ApiSettings {
    config: Vec<String>,
    templates: Vec<String>,
    agent: Vec<String>,
    /// True when a local ACP agent is configured or auto-detected on PATH.
    agent_available: bool,
    tags: Vec<String>,
    tag_counts: IssueCounts,
    diagnostics: Vec<ApiDiagnostic>,
}

#[derive(Deserialize)]
struct CreateRecordRequest {
    /// One record or several (a merge) that the new candidate supersedes.
    #[serde(default)]
    supersedes: Option<SupersedeTargets>,
    /// Why the decision changed, written in `## Supersedes` after the links.
    #[serde(default)]
    supersedes_reason: Option<String>,
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    body_markdown: Option<String>,
}

#[derive(Deserialize)]
struct SupersedeRequest {
    from: String,
    to: SupersedeTargets,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SupersedeTargets {
    One(String),
    Many(Vec<String>),
}

impl SupersedeTargets {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(record) => vec![record],
            Self::Many(records) => records,
        }
    }
}

#[derive(Default, Deserialize)]
struct AcceptCandidateBody {
    /// More candidates to accept in the same operation.
    #[serde(default)]
    with: Vec<String>,
}

#[derive(Deserialize)]
struct SaveRecordSourceBody {
    content: String,
}

#[derive(Deserialize)]
struct ApplyRecordRevisionBody {
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    body_markdown: String,
}

#[derive(Deserialize)]
struct AgentDraftRequest {
    prompt: String,
    #[serde(default)]
    prompt_packs: Vec<String>,
    #[serde(default = "default_true")]
    write: bool,
}

#[derive(Deserialize)]
struct RunTaskRequest {
    task_id: String,
    input: Option<serde_json::Value>,
    context: Option<TaskContext>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supersedes_api_persists_intent_and_links_on_acceptance() {
        let root = Utf8PathBuf::from_path_buf(
            std::env::temp_dir().join(format!("dadrock-web-supersedes-{}", RecordId::new_v7())),
        )
        .unwrap();
        fs::create_dir_all(&root).unwrap();
        initialize_project(&root).unwrap();
        let create = |title: &str, supersedes: Option<&str>| {
            let request = HttpRequest {
                method: "POST".into(),
                path: "/api/records".into(),
                headers: vec![],
                body: serde_json::to_vec(
                    &serde_json::json!({"title": title, "tags": [], "supersedes": supersedes, "body_markdown": "## Context\nPrevious needs.\n## Decision\nUse the selected method.\n## Alternatives\nRetain the old method.\n## Consequences\nRecompute results.\n"}),
                )
                .unwrap(),
            };
            let response = create_record(&request, &root).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            value["focus"]["selected_id"].as_str().unwrap().to_owned()
        };
        let old = create("Previous decision", None);
        accept_candidate(&post("", ""), &root, &old).unwrap();
        let next = create("Successor", Some(&old));
        // Each API call rediscovers the on-disk project; no browser state is needed.
        let pending: serde_json::Value =
            serde_json::from_slice(&record_lifecycle(&root, &next).unwrap().body).unwrap();
        assert_eq!(pending["pending_supersedes"][0]["id"], old);
        assert_eq!(pending["stale_supersedes"], serde_json::json!([]));
        let source: serde_json::Value =
            serde_json::from_slice(&record_source(&root, &next).unwrap().body).unwrap();
        let body = source["content"]
            .as_str()
            .unwrap()
            .splitn(3, "---")
            .nth(2)
            .unwrap();
        assert!(body.trim_start().starts_with(&format!(
            "## Supersedes\n\n[[{old}|Previous decision]]\n\n## Context\n"
        )));
        let response = accept_candidate(&post("", ""), &root, &next).unwrap();
        let mutation: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        // The successor and its predecessor each moved: inbox to ledger, ledger to archive.
        assert_eq!(mutation["changed_paths"].as_array().unwrap().len(), 4);
        let previous: serde_json::Value =
            serde_json::from_slice(&record_lifecycle(&root, &old).unwrap().body).unwrap();
        assert_eq!(previous["successors"][0]["id"], next);
        assert!(previous["record"]["path"]
            .as_str()
            .unwrap()
            .contains("/dadr/archive/"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_predecessor_stays_editable_without_accepting() {
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(format!(
            "dadrock-web-missing-predecessor-{}",
            RecordId::new_v7()
        )))
        .unwrap();
        fs::create_dir_all(&root).unwrap();
        initialize_project(&root).unwrap();
        let context = context_for(&root).unwrap();
        let body = format!("## Supersedes\n\n[[{}|Missing decision]]\n\n## Context\nContext\n## Decision\nDecision\n## Alternatives\nAlternatives\n## Consequences\nConsequences\n", RecordId::new_v7());
        let candidate = core_commands::new_record_with_body(&context, "Successor", &[], &body)
            .unwrap()
            .data;
        let before = fs::read_to_string(&candidate.path).unwrap();
        let id = candidate.id.to_string();
        let lifecycle: serde_json::Value =
            serde_json::from_slice(&record_lifecycle(&root, &id).unwrap().body).unwrap();
        assert_eq!(lifecycle["record"]["status"], "candidate");
        assert_eq!(lifecycle["diagnostics"][0]["code"], "invalid_supersedes");
        assert!(accept_candidate(&post("", ""), &root, &id).is_err());
        assert_eq!(fs::read_to_string(&candidate.path).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mutation_focus_serializes_for_lifecycle_selection() {
        let response = ApiMutationResponse {
            message: "accepted candidate".to_owned(),
            changed_paths: Vec::new(),
            focus: Some(ApiMutationFocus {
                selected_id: Some("01938b8e-4d70-7000-8000-000000000000".to_owned()),
                status_filter: Some("accepted".to_owned()),
            }),
            snapshot: ApiSnapshot {
                project: ApiProject {
                    cwd: "/tmp/project".to_owned(),
                    decisions_dir: "/tmp/project/dadr/ledger".to_owned(),
                    initialized: true,
                    discovery_source: "directory".to_owned(),
                    status: "Project discovered.".to_owned(),
                },
                records: Vec::new(),
                counts: ApiRecordCounts::default(),
                tags: Vec::new(),
                project_validation: ApiDiagnosticPanel::default(),
                record_validation: ApiDiagnosticPanel::default(),
                link_health: ApiDiagnosticPanel::default(),
                settings: ApiSettings::default(),
                command_catalog: core_commands::command_catalog().data,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""focus""#));
        assert!(json.contains(r#""status_filter":"accepted""#));
        assert!(json.contains(r#""selected_id":"01938b8e-4d70-7000-8000-000000000000""#));
    }

    #[test]
    fn http_request_parser_preserves_headers_and_body() {
        let mut request = std::io::Cursor::new(
            b"POST /api/records HTTP/1.1\r\nHost: localhost:4011\r\nContent-Length: 2\r\n\r\n{}",
        );

        let parsed =
            read_http_request_with_deadline(&mut request, Instant::now() + Duration::from_secs(1))
                .unwrap();

        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/api/records");
        assert_eq!(parsed.header("host"), Some("localhost:4011"));
        assert_eq!(parsed.body, b"{}");
    }

    #[test]
    fn http_request_parser_rejects_bad_length_encoding_size_and_deadline() {
        let cases = [
            (
                b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: abc\r\n\r\n".as_slice(),
                400,
            ),
            (
                b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n"
                    .as_slice(),
                501,
            ),
        ];

        for (raw, status) in cases {
            let mut request = std::io::Cursor::new(raw);
            let error = read_http_request_with_deadline(
                &mut request,
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap_err();
            assert_eq!(error.status, status);
        }

        let mut oversized_header = std::io::Cursor::new(format!(
            "GET / HTTP/1.1\r\nHost: localhost\r\nX-Fill: {}\r\n\r\n",
            "x".repeat(MAX_HEADER_BYTES)
        ));
        assert_eq!(
            read_http_request_with_deadline(
                &mut oversized_header,
                Instant::now() + Duration::from_secs(1)
            )
            .unwrap_err()
            .status,
            413
        );

        let mut oversized_body = std::io::Cursor::new(format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        ));
        assert_eq!(
            read_http_request_with_deadline(
                &mut oversized_body,
                Instant::now() + Duration::from_secs(1)
            )
            .unwrap_err()
            .status,
            413
        );

        let mut incomplete_body = std::io::Cursor::new(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\n\r\n{}",
        );
        let error = read_http_request_with_deadline(
            &mut incomplete_body,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap_err();
        assert_eq!(error.status, 400);
        assert!(error.message.contains("incomplete request body"));

        let mut expired = std::io::Cursor::new(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(
            read_http_request_with_deadline(&mut expired, Instant::now() - Duration::from_secs(1))
                .unwrap_err()
                .status,
            408
        );
    }

    #[test]
    fn route_rejects_non_localhost_host_header() {
        let mut request = get("/");
        request.headers[0].value = "evil.example.com".to_owned();

        let error = route_request(&request, &Utf8PathBuf::from("/tmp")).unwrap_err();

        assert_eq!(error.status, 400);
        assert!(error.message.contains("Host header"));
    }

    #[test]
    fn preview_bind_host_rejects_non_loopback_hosts() {
        assert!(validate_bind_host("127.0.0.1").is_ok());
        assert!(validate_bind_host("localhost").is_ok());

        let error = validate_bind_host("0.0.0.0").unwrap_err();
        assert!(error.to_string().contains("not allowed"));
    }

    #[test]
    fn unknown_reason_phrase_is_empty() {
        assert_eq!(reason_phrase(503), "");
    }

    #[test]
    fn static_response_accepts_borrowed_body() {
        let response = static_response(200, "text/plain", Cow::Borrowed(b"hello"));

        assert_eq!(response.body.as_ref(), b"hello");
    }

    #[test]
    fn background_task_panic_is_persisted_as_task_failure() {
        let root =
            std::env::temp_dir().join(format!("dadrock-web-panic-{}", dadrock::RecordId::new_v7()));
        let run_dir = Utf8PathBuf::from_path_buf(root.join("dadr/runs/custom/run")).unwrap();
        fs::create_dir_all(&run_dir).unwrap();
        let now = chrono::Utc::now();
        let run = dadrock::tasks::PersistedTaskRun {
            id: dadrock::RecordId::new_v7(),
            task_id: "custom".to_owned(),
            task_source: dadrock::tasks::TaskSource {
                kind: dadrock::tasks::TaskSourceKind::Builtin,
                id: "custom".to_owned(),
                path: None,
            },
            task_version: 1,
            status: dadrock::tasks::TaskRunStatus::Running,
            created_at: now,
            updated_at: now,
            run_dir: run_dir.clone(),
        };
        fs::write(
            run_dir.join("run.json").as_std_path(),
            serde_json::to_string_pretty(&run).unwrap(),
        )
        .unwrap();

        persist_background_task_panic(&run, Box::new("boom"));

        let updated = dadrock::tasks::read_task_run(&run_dir).unwrap();
        assert_eq!(updated.status, dadrock::tasks::TaskRunStatus::Failed);
        assert!(fs::read_to_string(run_dir.join("error.json").as_std_path())
            .unwrap()
            .contains("boom"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn task_query_matches_dynamic_task_paths() {
        assert_eq!(
            task_query("/api/tasks/context-to-record", "/api/tasks/"),
            Some("context-to-record")
        );
        assert_eq!(
            task_query(
                "/api/task-runs/01938b8e-4d70-7000-8000-000000000000",
                "/api/task-runs/"
            ),
            Some("01938b8e-4d70-7000-8000-000000000000")
        );
    }

    #[test]
    fn task_query_rejects_empty_and_nested_ids() {
        assert_eq!(task_query("/api/tasks/", "/api/tasks/"), None);
        assert_eq!(
            task_query("/api/tasks/find-conflicts/result", "/api/tasks/"),
            None
        );
        assert_eq!(
            task_query("/api/task-runs/task/run-id", "/api/task-runs/"),
            None
        );
    }

    #[test]
    fn task_log_query_matches_run_log_paths() {
        assert_eq!(
            task_log_query("/api/task-runs/01938b8e-4d70-7000-8000-000000000000/log"),
            Some("01938b8e-4d70-7000-8000-000000000000")
        );
        assert_eq!(task_log_query("/api/task-runs//log"), None);
        assert_eq!(task_log_query("/api/task-runs/task/run-id/log"), None);
    }

    #[test]
    fn task_cancel_query_matches_run_cancel_paths() {
        assert_eq!(
            task_cancel_query("/api/task-runs/01938b8e-4d70-7000-8000-000000000000/cancel"),
            Some("01938b8e-4d70-7000-8000-000000000000")
        );
        assert_eq!(task_cancel_query("/api/task-runs//cancel"), None);
        assert_eq!(task_cancel_query("/api/task-runs/task/run-id/cancel"), None);
    }

    #[test]
    fn supersede_request_accepts_multiple_successors() {
        let body: SupersedeRequest = serde_json::from_str(
            r#"{
                "from": "old-record",
                "to": ["new-record-a", "new-record-b"]
            }"#,
        )
        .unwrap();

        assert_eq!(body.from, "old-record");
        assert_eq!(body.to.into_vec(), vec!["new-record-a", "new-record-b"]);
    }

    #[test]
    fn record_lifecycle_query_matches_dynamic_status_paths() {
        assert_eq!(
            record_lifecycle_query("/api/records/01938b8e-4d70-7000-8000-000000000000/status"),
            Some("01938b8e-4d70-7000-8000-000000000000")
        );
        assert_eq!(record_lifecycle_query("/api/records//status"), None);
        assert_eq!(
            record_lifecycle_query("/api/records/01938b8e/status/extra"),
            None
        );
    }

    #[test]
    fn record_revision_query_matches_dynamic_revision_paths() {
        assert_eq!(
            record_revision_query("/api/records/01938b8e-4d70-7000-8000-000000000000/revision"),
            Some("01938b8e-4d70-7000-8000-000000000000")
        );
        assert_eq!(record_revision_query("/api/records//revision"), None);
        assert_eq!(
            record_revision_query("/api/records/01938b8e/revision/extra"),
            None
        );
        assert_eq!(record_revision_query("/api/records/01938b8e/source"), None);
    }

    #[cfg(not(dadrock_no_js_build))]
    #[test]
    fn root_serves_svelte_spa_after_cutover() {
        let response = route_request(&get("/"), &Utf8PathBuf::from("/tmp")).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(String::from_utf8_lossy(&response.body).contains("data-sveltekit"));
    }

    #[cfg(not(dadrock_no_js_build))]
    #[test]
    fn client_routes_fall_back_to_svelte_spa_after_cutover() {
        let response = route_request(
            &get("/records/0193fb1f-a3a0-7000-8000-000000000001"),
            &Utf8PathBuf::from("/tmp"),
        )
        .unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(String::from_utf8_lossy(&response.body).contains("data-sveltekit"));
    }

    #[cfg(not(dadrock_no_js_build))]
    #[test]
    fn embedded_svelte_assets_are_served_after_cutover() {
        let response = route_request(&get("/_app/env.js"), &Utf8PathBuf::from("/tmp")).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/javascript; charset=utf-8");
    }

    #[cfg(dadrock_no_js_build)]
    #[test]
    fn no_js_build_serves_preview_notice_instead_of_404() {
        let response = route_request(&get("/"), &Utf8PathBuf::from("/tmp")).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(String::from_utf8_lossy(&response.body).contains("without the web UI assets"));
    }

    #[test]
    fn legacy_asset_routes_are_removed_after_cutover() {
        let response = route_request(&get("/assets/app.js"), &Utf8PathBuf::from("/tmp")).unwrap();

        assert_eq!(response.status, 404);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
    }

    fn post(path: &str, body: &str) -> HttpRequest {
        HttpRequest {
            method: "POST".to_owned(),
            path: path.to_owned(),
            headers: vec![HttpHeader {
                name: "Host".to_owned(),
                value: "localhost".to_owned(),
            }],
            body: body.as_bytes().to_vec(),
        }
    }

    fn json(response: HttpResponse) -> serde_json::Value {
        serde_json::from_slice(&response.body).unwrap()
    }

    fn temp_root(label: &str) -> Utf8PathBuf {
        let root = Utf8PathBuf::from_path_buf(
            std::env::temp_dir().join(format!("dadrock-web-{label}-{}", RecordId::new_v7())),
        )
        .unwrap();
        fs::create_dir_all(&root).unwrap();
        initialize_project(&root).unwrap();
        root
    }

    const SECTIONS: &str = "## Context\nWhy.\n\n## Decision\nWhat.\n\n## Alternatives\nOther.\n\n## Consequences\nEffects.\n";

    fn create_candidate(root: &Utf8PathBuf, request: serde_json::Value) -> String {
        let response = create_record(&post("/api/records", &request.to_string()), root).unwrap();
        json(response)["focus"]["selected_id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn accepted_decision(root: &Utf8PathBuf, title: &str) -> String {
        let id = create_candidate(
            root,
            serde_json::json!({"title": title, "body_markdown": SECTIONS}),
        );
        accept_candidate(&post("", ""), root, &id).unwrap();
        id
    }

    fn body_of(root: &Utf8PathBuf, id: &str) -> String {
        let source = json(record_source(root, id).unwrap());
        source["content"]
            .as_str()
            .unwrap()
            .splitn(3, "---")
            .nth(2)
            .unwrap()
            .trim_start()
            .to_owned()
    }

    #[test]
    fn create_record_writes_supersedes_for_several_predecessors_with_reason() {
        let root = temp_root("merge-successor");
        let first = accepted_decision(&root, "Winsorize income");
        let second = accepted_decision(&root, "Drop implausible ages");
        let merged = create_candidate(
            &root,
            serde_json::json!({
                "title": "One outlier rule",
                "supersedes": [first, second],
                "supersedes_reason": "The two rules disagreed at the 99th percentile.",
                "body_markdown": SECTIONS,
            }),
        );
        assert_eq!(
            body_of(&root, &merged),
            format!(
                "## Supersedes\n\n[[{first}|Winsorize income]]\n[[{second}|Drop implausible ages]]\n\n\
                 The two rules disagreed at the 99th percentile.\n\n{SECTIONS}"
            )
        );
        let lifecycle = json(record_lifecycle(&root, &merged).unwrap());
        assert_eq!(lifecycle["pending_supersedes"][0]["id"], first);
        assert_eq!(lifecycle["pending_supersedes"][1]["id"], second);

        // A superseded record cannot get another successor.
        let response = accept_candidate(&post("", ""), &root, &merged).unwrap();
        assert_eq!(json(response)["changed_paths"].as_array().unwrap().len(), 6);
        let error = create_record(
            &post(
                "/api/records",
                &serde_json::json!({"title": "Late", "supersedes": first, "body_markdown": SECTIONS})
                    .to_string(),
            ),
            &root,
        )
        .unwrap_err();
        assert!(
            error.message.contains("already superseded"),
            "{}",
            error.message
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accept_endpoint_accepts_a_split_with_more_candidates() {
        let root = temp_root("split");
        let old = accepted_decision(&root, "Listwise deletion");
        let left = create_candidate(
            &root,
            serde_json::json!({"title": "Impute income", "supersedes": old, "body_markdown": SECTIONS}),
        );
        let right = create_candidate(
            &root,
            serde_json::json!({"title": "Complete cases for the outcome", "supersedes": [old], "body_markdown": SECTIONS}),
        );
        let body = serde_json::json!({"with": [right]}).to_string();
        let response = json(accept_candidate(&post("", &body), &root, &left).unwrap());
        assert_eq!(response["message"], "accepted 2 candidates");
        assert_eq!(response["focus"]["selected_id"], left);
        let previous = json(record_lifecycle(&root, &old).unwrap());
        let successors: Vec<_> = previous["successors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["id"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(successors, vec![left, right]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lifecycle_reports_stale_supersedes_with_current_successors() {
        let root = temp_root("stale");
        let old = accepted_decision(&root, "Trim income at p95");
        let first = create_candidate(
            &root,
            serde_json::json!({"title": "Winsorize at p99", "supersedes": old, "body_markdown": SECTIONS}),
        );
        let competing = create_candidate(
            &root,
            serde_json::json!({"title": "Winsorize at p97", "supersedes": old, "body_markdown": SECTIONS}),
        );
        accept_candidate(&post("", ""), &root, &first).unwrap();

        let lifecycle = json(record_lifecycle(&root, &competing).unwrap());
        assert_eq!(lifecycle["pending_supersedes"][0]["id"], old);
        assert_eq!(lifecycle["stale_supersedes"][0]["predecessor"]["id"], old);
        assert_eq!(
            lifecycle["stale_supersedes"][0]["current_successors"][0]["id"],
            first
        );
        assert_eq!(lifecycle["diagnostics"], serde_json::json!([]));
        let error = accept_candidate(&post("", ""), &root, &competing).unwrap_err();
        assert!(
            error.message.contains("Winsorize at p99"),
            "{}",
            error.message
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn get(path: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".to_owned(),
            path: path.to_owned(),
            headers: vec![HttpHeader {
                name: "Host".to_owned(),
                value: "localhost".to_owned(),
            }],
            body: Vec::new(),
        }
    }
}
