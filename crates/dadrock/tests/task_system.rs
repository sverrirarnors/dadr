use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use camino::Utf8PathBuf;
use dadrock::{
    agent::{
        acp::parse_acp_agent_json_response,
        prompts::{run_task_prompt, task_output_repair_prompt},
        types::{AgentIntent, AgentProjectSnapshot, AgentRequestEnvelope, AgentTaskRequest},
    },
    commands::{
        cancel_task_run_command, command_catalog, find_command_info, list_task_runs_command,
        list_tasks_command, run_task_command, show_task_command, show_task_result_command,
        submit_task_result_command, CommandContext,
    },
    error::DomainErrorKind,
    index::ProjectIndex,
    project::{init_project, DiscoverySource, ProjectContext},
    tasks::{
        append_or_extend_task_run_log, append_task_run_log, create_task_run,
        create_task_run_for_task, discover_tasks, list_task_runs, mark_task_run_cancelled,
        parse_task_document, read_task_run, read_task_run_log, task_runs_dir, validate_task_output,
        write_task_failure, write_task_result, TaskContext, TaskDiagnosticCode, TaskInputType,
        TaskNoteRequirement, TaskOutputKind, TaskOutputValue, TaskPathRequirement,
        TaskRecordRequirement, TaskRecordStatusRequirement, TaskRunCommandRequest, TaskRunStatus,
        TaskScopeRequirement, TaskSourceKind, TaskToolMode,
    },
};
use serde_json::json;

static TASK_ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set_path(key: &'static str, value: impl AsRef<Path>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value.as_ref());
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn task_document_parses_front_matter_manifest_and_prompt_body() {
    let document = parse_task_document(
        "builtin:find-conflicts",
        r#"---
id: find-conflicts
title: "Find conflicting decisions"
description: "Scan live decisions and candidates for contradictions."
version: 1
inputs:
  - name: status_scope
    label: "Records to scan"
    type: select
    default: "live"
    options: ["live", "candidates", "accepted"]
requirements:
  records: none
  record_status: all
  paths: none
  notes: optional
  scope: live
output:
  kind: conflict_set.v1
tools:
  - name: dadrock.list_records
    mode: read
---

# Find conflicts

Return conflicts as JSON.
"#,
    )
    .unwrap();
    let manifest = document.manifest;

    assert_eq!(manifest.id, "find-conflicts");
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.inputs[0].field_type, TaskInputType::Select);
    assert_eq!(manifest.inputs[0].default.as_deref(), Some("live"));
    assert_eq!(manifest.requirements.records, TaskRecordRequirement::None);
    assert_eq!(
        manifest.requirements.record_status,
        TaskRecordStatusRequirement::All
    );
    assert_eq!(manifest.requirements.paths, TaskPathRequirement::None);
    assert_eq!(manifest.requirements.notes, TaskNoteRequirement::Optional);
    assert_eq!(manifest.requirements.scope, TaskScopeRequirement::Live);
    assert_eq!(manifest.output.kind, TaskOutputKind::ConflictSetV1);
    assert_eq!(manifest.tools[0].mode, TaskToolMode::Read);
    assert_eq!(
        document.prompt,
        "# Find conflicts\n\nReturn conflicts as JSON."
    );
}

#[test]
fn task_document_reads_scalar_input_defaults() {
    let document = parse_task_document(
        "project:ledger-summary",
        r#"---
id: ledger-summary
title: Ledger summary
description: Summarizes the ledger.
version: 2
inputs:
  - name: limit
    label: Limit
    type: number
    default: 5
  - name: include_rejected
    label: Include rejected
    type: checkbox
    default: false
output:
  kind: markdown_report.v1
---
Summarize the ledger.
"#,
    )
    .unwrap();

    assert_eq!(document.manifest.inputs[0].default.as_deref(), Some("5"));
    assert_eq!(
        document.manifest.inputs[1].default.as_deref(),
        Some("false")
    );
    assert_eq!(document.prompt, "Summarize the ledger.");
    let serialized = serde_json::to_value(&document.manifest).unwrap();
    assert!(serialized.get("prompt").is_none());
}

#[test]
fn task_document_requires_front_matter() {
    let error = parse_task_document("project:no-front-matter", "# Just a prompt\n").unwrap_err();
    assert!(error.to_string().contains("YAML front matter"));

    let error = parse_task_document("project:unclosed", "---\nid: unclosed\ntitle: Unclosed\n")
        .unwrap_err();
    assert!(error.to_string().contains("closing `---`"));
}

#[test]
fn task_document_ignores_retired_mcp_access_key() {
    // Task files written before MCP access became fixed for every task run
    // still load; the old key grants nothing and is not echoed back.
    let document = parse_task_document(
        "project:older-task",
        r#"---
id: older-task
title: Older task
description: Declares the retired per-task MCP access.
version: 1
output:
  kind: markdown_report.v1
mcp_access: full
---
Prompt.
"#,
    )
    .unwrap();

    let serialized = serde_json::to_value(&document.manifest).unwrap();
    assert!(serialized.get("mcp_access").is_none());
    assert_eq!(document.prompt, "Prompt.");
}

#[test]
fn task_manifest_rejects_invalid_ids() {
    let error = parse_task_document(
        "project:bad",
        r#"---
id: "Bad Task"
title: Bad
description: Bad id
version: 1
output:
  kind: markdown_report.v1
---
Prompt.
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("task id"));
}

#[test]
fn vendor_output_kind_is_preserved_for_fallback_rendering() {
    let document = parse_task_document(
        "project:lab-audit",
        r#"---
id: lab-audit
title: Lab audit
description: Project-specific audit.
version: 1
output:
  kind: lab.audit.v1
---
Prompt.
"#,
    )
    .unwrap();

    assert_eq!(
        document.manifest.output.kind,
        TaskOutputKind::Vendor("lab.audit.v1".to_owned())
    );
}

#[test]
fn task_manifest_rejects_built_in_output_kind_typos() {
    let error = parse_task_document(
        "project:bad-output",
        r#"---
id: bad-output
title: Bad output
description: Uses a typo for a built-in output kind.
version: 1
output:
  kind: candidate-draft.v1
---
Prompt.
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("unknown built-in task output kind"));
}

#[test]
fn task_manifest_rejects_local_execution_until_executor_exists() {
    let error = parse_task_document(
        "project:local-task",
        r#"---
id: local-task
title: Local task
description: Attempts to use the future local task executor.
version: 1
execution: local
output:
  kind: markdown_report.v1
---
Prompt.
"#,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("local task execution is not implemented"));
}

#[test]
fn validate_task_output_accepts_native_output() {
    let output = json!({
        "kind": "markdown_report.v1",
        "title": "Audit",
        "body_markdown": "All clear."
    });

    let validated = validate_task_output(TaskOutputKind::MarkdownReportV1, output.clone()).unwrap();

    assert_eq!(serde_json::to_value(validated).unwrap(), output);
}

#[test]
fn validate_task_output_rejects_native_kind_mismatch() {
    let error = validate_task_output(
        TaskOutputKind::ConflictSetV1,
        json!({
            "kind": "markdown_report.v1",
            "body_markdown": "Wrong shape."
        }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("did not match expected kind"));
    assert!(error.to_string().contains("markdown_report.v1"));
    assert!(error.to_string().contains("conflict_set.v1"));
}

#[test]
fn validate_task_output_rejects_missing_kind() {
    let error = validate_task_output(
        TaskOutputKind::MarkdownReportV1,
        json!({
            "body_markdown": "Missing kind."
        }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("must include string `kind`"));
}

#[test]
fn validate_task_output_reports_malformed_json_with_context() {
    let error = validate_task_output(TaskOutputKind::MarkdownReportV1, "{not json").unwrap_err();

    assert!(error
        .to_string()
        .contains("failed to parse task output JSON"));

    let error =
        validate_task_output(TaskOutputKind::MarkdownReportV1, "{not json".to_owned()).unwrap_err();

    assert!(error
        .to_string()
        .contains("failed to parse task output JSON"));
}

#[test]
fn validate_task_output_preserves_vendor_wire_kind() {
    let output = json!({
        "kind": "lab.audit.v1",
        "summary": "Lab audit complete.",
        "score": 92
    });

    let validated = validate_task_output(
        TaskOutputKind::Vendor("lab.audit.v1".to_owned()),
        output.clone(),
    )
    .unwrap();

    assert_eq!(serde_json::to_value(validated).unwrap(), output);
}

#[test]
fn validate_task_output_deserializes_native_built_in_schema() {
    let output = json!({
        "kind": "conflict_set.v1",
        "summary": "Found 1 possible conflict.",
        "findings": [
            {
                "id": "finding-1",
                "title": "Treatment missingness handling",
                "left_record_id": "01960000000000000000000000",
                "right_record_id": "01950000000000000000000000",
                "relationship": "candidate_revises_accepted",
                "conflict_explanation": "One drops rows; one imputes.",
                "suggested_actions": ["open_records"]
            }
        ]
    });

    let validated = validate_task_output(TaskOutputKind::ConflictSetV1, output.clone()).unwrap();
    let typed: TaskOutputValue =
        serde_json::from_value(serde_json::to_value(validated).unwrap()).unwrap();

    let TaskOutputValue::ConflictSet(conflict_set) = typed else {
        panic!("expected conflict_set.v1 output");
    };
    assert_eq!(
        conflict_set.findings[0].left_record_id,
        "01960000000000000000000000"
    );
}

#[test]
fn validate_task_output_accepts_structured_diagnosis_summary() {
    let output = json!({
        "kind": "diagnosis_report.v1",
        "summary": {
            "headline": "Ledger has candidate backlog and broken links.",
            "total_records": 27,
            "issues_found": 2
        },
        "issues": [
            {
                "id": "issue-001",
                "severity": "high",
                "category": "stale_candidates",
                "title": "Large candidate backlog",
                "explanation": "Most records are candidates.",
                "suggested_actions": ["Triage the candidate batch."]
            }
        ]
    });

    let validated =
        validate_task_output(TaskOutputKind::DiagnosisReportV1, output.clone()).unwrap();

    assert_eq!(serde_json::to_value(validated).unwrap(), output);
}

#[test]
fn discovery_lists_built_in_starter_tasks() {
    let root = unique_temp_dir("dadrock-task-discovery-builtins");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics, Vec::new());
    let tasks = tasks_by_id(&listing.tasks);
    assert_eq!(
        tasks.keys().cloned().collect::<Vec<_>>(),
        vec![
            "code-to-record",
            "context-to-record",
            "custom",
            "decisions-to-spec",
            "edit-record",
            "find-conflicts",
            "merge-records",
            "supersede-record",
        ]
    );
    assert_eq!(
        tasks["custom"].manifest.output.kind,
        TaskOutputKind::MarkdownReportV1
    );
    assert_eq!(
        tasks["custom"].manifest.requirements.notes,
        TaskNoteRequirement::Required
    );
    assert_eq!(
        tasks["custom"].manifest.requirements.paths,
        TaskPathRequirement::Optional
    );
    assert_eq!(
        tasks["custom"].manifest.requirements.records,
        TaskRecordRequirement::None
    );
    assert_eq!(
        tasks["merge-records"].manifest.output.kind,
        TaskOutputKind::CandidateDraftV1
    );
    assert_eq!(tasks["merge-records"].manifest.title, "Merge records");
    assert_eq!(
        tasks["context-to-record"].manifest.output.kind,
        TaskOutputKind::CandidateDraftV1
    );
    assert_eq!(
        tasks["code-to-record"].manifest.output.kind,
        TaskOutputKind::CandidateBatchV1
    );
    assert_eq!(
        tasks["find-conflicts"].manifest.output.kind,
        TaskOutputKind::ConflictSetV1
    );
    assert_eq!(
        tasks["supersede-record"].manifest.output.kind,
        TaskOutputKind::SupersessionProposalV1
    );
    assert_eq!(
        tasks["edit-record"].manifest.output.kind,
        TaskOutputKind::RecordRevisionV1
    );
    assert_eq!(
        tasks["edit-record"].manifest.requirements.record_status,
        TaskRecordStatusRequirement::Candidate
    );
    assert_eq!(
        tasks["supersede-record"].manifest.requirements.records,
        TaskRecordRequirement::OneOrMore
    );
    assert_eq!(
        tasks["supersede-record"]
            .manifest
            .requirements
            .record_status,
        TaskRecordStatusRequirement::Accepted
    );
    assert!(tasks
        .values()
        .all(|task| task.source.kind == TaskSourceKind::Builtin));
    assert!(tasks
        .values()
        .all(|task| task.source.id == format!("builtin:{}", task.manifest.id)));
    assert_eq!(
        tasks["decisions-to-spec"].manifest.output.kind,
        TaskOutputKind::MarkdownReportV1
    );
    assert_eq!(
        tasks["decisions-to-spec"]
            .manifest
            .requirements
            .record_status,
        TaskRecordStatusRequirement::Accepted
    );
    for task in tasks.values() {
        // The skill task guide's body is the prompt; its front matter is not.
        assert!(
            !task.prompt.is_empty(),
            "{} has no prompt",
            task.manifest.id
        );
        assert!(
            !task.prompt.starts_with("---") && !task.prompt.contains("\nversion:"),
            "{} prompt still contains front matter",
            task.manifest.id
        );
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_task_shadows_builtin_task_by_id() {
    let root = unique_temp_dir("dadrock-task-project-shadow");
    let project_tasks = root.join("dadr/tasks");
    let user_tasks = root.join("user-tasks");
    write_task(
        &project_tasks,
        "find-conflicts",
        "Project conflict finder",
        "Project-specific conflict scan.",
        "markdown_report.v1",
        "Return project conflict results as JSON only.\n",
    );
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics, Vec::new());
    let tasks = tasks_by_id(&listing.tasks);
    let task = tasks["find-conflicts"];
    assert_eq!(task.source.kind, TaskSourceKind::Project);
    assert_eq!(task.source.id, "project:find-conflicts");
    assert!(task
        .source
        .path
        .as_ref()
        .is_some_and(|path| { path.ends_with("dadr/tasks/find-conflicts.md") }));
    assert_eq!(task.manifest.output.kind, TaskOutputKind::MarkdownReportV1);
    assert_eq!(task.prompt, "Return project conflict results as JSON only.");
    assert_eq!(tasks.len(), 8);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn invalid_project_task_is_reported_as_diagnostic() {
    let root = unique_temp_dir("dadrock-task-invalid-project");
    let tasks_dir = root.join("dadr/tasks");
    fs::create_dir_all(&tasks_dir).expect("create task dir");
    fs::write(
        tasks_dir.join("bad-project-task.md"),
        r#"---
id: "Bad Project Task"
title: Bad project task
description: Invalid id.
version: 1
output:
  kind: markdown_report.v1
---
Return JSON only.
"#,
    )
    .expect("write invalid task document");
    let user_tasks = root.join("user-tasks");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert!(listing.diagnostics.iter().any(|diagnostic| diagnostic.code
        == TaskDiagnosticCode::InvalidManifest
        && diagnostic.source_id.as_deref() == Some("project:bad-project-task")
        && diagnostic
            .path
            .as_ref()
            .is_some_and(|path| { path.ends_with("dadr/tasks/bad-project-task.md") })));
    assert!(!listing
        .tasks
        .iter()
        .any(|task| task.manifest.id == "Bad Project Task"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn user_task_shadows_builtin_task_by_id() {
    let root = unique_temp_dir("dadrock-task-user-shadow");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    write_task(
        &user_tasks,
        "find-conflicts",
        "User conflict finder",
        "User-specific conflict scan.",
        "markdown_report.v1",
        "Return user conflict results as JSON only.\n",
    );
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics, Vec::new());
    let tasks = tasks_by_id(&listing.tasks);
    let task = tasks["find-conflicts"];
    assert_eq!(task.source.kind, TaskSourceKind::User);
    assert_eq!(task.source.id, "user:find-conflicts");
    assert!(task
        .source
        .path
        .as_ref()
        .is_some_and(|path| path.ends_with("user-tasks/find-conflicts.md")));
    assert_eq!(task.manifest.output.kind, TaskOutputKind::MarkdownReportV1);
    assert_eq!(task.prompt, "Return user conflict results as JSON only.");
    assert_eq!(tasks.len(), 8);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_task_shadows_user_task_by_id() {
    let root = unique_temp_dir("dadrock-task-project-over-user");
    let project_tasks = root.join("dadr/tasks");
    let user_tasks = root.join("user-tasks");
    write_task(
        &user_tasks,
        "lab-audit",
        "User lab audit",
        "User-level audit.",
        "markdown_report.v1",
        "User prompt.\n",
    );
    write_task(
        &project_tasks,
        "lab-audit",
        "Project lab audit",
        "Project-level audit.",
        "markdown_report.v1",
        "Project prompt.\n",
    );
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics, Vec::new());
    let tasks = tasks_by_id(&listing.tasks);
    let task = tasks["lab-audit"];
    assert_eq!(task.source.kind, TaskSourceKind::Project);
    assert_eq!(task.manifest.title, "Project lab audit");
    assert_eq!(task.prompt, "Project prompt.");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_file_name_must_match_declared_id() {
    let root = unique_temp_dir("dadrock-task-stem-mismatch");
    let project_tasks = root.join("dadr/tasks");
    write_task(
        &project_tasks,
        "lab-audit",
        "Lab audit",
        "Project-specific lab audit.",
        "markdown_report.v1",
        "Return lab audit JSON only.\n",
    );
    fs::rename(
        project_tasks.join("lab-audit.md"),
        project_tasks.join("find-conflicts.md"),
    )
    .expect("rename task file");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert!(
        listing.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TaskDiagnosticCode::InvalidManifest
                && diagnostic.source_id.as_deref() == Some("project:find-conflicts")
                && diagnostic.message.contains("declares id `lab-audit`")
                && diagnostic
                    .path
                    .as_ref()
                    .is_some_and(|path| path.ends_with("dadr/tasks/find-conflicts.md"))
        }),
        "expected stem mismatch diagnostic; diagnostics: {:?}",
        listing.diagnostics
    );
    let tasks = tasks_by_id(&listing.tasks);
    assert!(!tasks.contains_key("lab-audit"));
    assert_eq!(tasks["find-conflicts"].source.kind, TaskSourceKind::Builtin);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn old_task_package_directory_is_reported_and_skipped() {
    let root = unique_temp_dir("dadrock-task-old-layout");
    let task_dir = root.join("dadr/tasks/find-conflicts");
    fs::create_dir_all(&task_dir).expect("create old task dir");
    fs::write(
        task_dir.join("task.toml"),
        r#"
id = "find-conflicts"
title = "Old conflict finder"
description = "Old layout."
version = 1

[output]
kind = "markdown_report.v1"

[prompt]
system = "prompt.md"
"#,
    )
    .expect("write old manifest");
    fs::write(task_dir.join("prompt.md"), "Old prompt.\n").expect("write old prompt");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics.len(), 1, "{:?}", listing.diagnostics);
    let diagnostic = &listing.diagnostics[0];
    assert_eq!(diagnostic.code, TaskDiagnosticCode::InvalidManifest);
    assert_eq!(
        diagnostic.source_id.as_deref(),
        Some("project:find-conflicts")
    );
    assert!(diagnostic.message.contains("<dir>/<id>.md"));
    assert!(diagnostic.message.contains("YAML front matter"));
    assert!(diagnostic
        .path
        .as_ref()
        .is_some_and(|path| path.ends_with("dadr/tasks/find-conflicts/task.toml")));
    let tasks = tasks_by_id(&listing.tasks);
    assert_eq!(tasks["find-conflicts"].source.kind, TaskSourceKind::Builtin);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_document_without_prompt_body_is_reported() {
    let root = unique_temp_dir("dadrock-task-empty-body");
    let user_tasks = root.join("user-tasks");
    write_task(
        &user_tasks,
        "empty-body",
        "Empty body",
        "Has front matter but no prompt.",
        "markdown_report.v1",
        "\n  \n",
    );
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert!(
        listing.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TaskDiagnosticCode::MissingPrompt
                && diagnostic.source_id.as_deref() == Some("user:empty-body")
        }),
        "expected missing prompt diagnostic; diagnostics: {:?}",
        listing.diagnostics
    );
    assert!(!listing
        .tasks
        .iter()
        .any(|task| task.manifest.id == "empty-body"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn non_task_entries_in_task_directories_are_ignored() {
    let root = unique_temp_dir("dadrock-task-ignored-entries");
    let project_tasks = root.join("dadr/tasks");
    fs::create_dir_all(project_tasks.join("notes")).expect("create unrelated dir");
    fs::write(project_tasks.join("notes/todo.md"), "Not a task.\n").expect("write nested file");
    fs::write(project_tasks.join("draft.txt"), "Not a task.\n").expect("write text file");
    fs::write(project_tasks.join(".hidden.md"), "Not a task.\n").expect("write hidden file");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");

    let listing = discover_tasks(&cwd, Some(&user_tasks));

    assert_eq!(listing.diagnostics, Vec::new());
    assert_eq!(listing.tasks.len(), 8);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_runs_dir_uses_dadr_runs() {
    let cwd = Utf8PathBuf::from("/tmp/example-project");

    assert_eq!(
        task_runs_dir(&cwd),
        Utf8PathBuf::from("/tmp/example-project/dadr/runs")
    );
}

#[test]
fn create_task_run_persists_gitignored_local_state() {
    let root = unique_temp_dir("dadrock-task-run-create");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");
    let input = json!({
        "scope": "live",
        "limit": 3
    });

    let run = create_task_run(&cwd, &task, input.clone()).unwrap();

    assert_eq!(run.task_id, "find-conflicts");
    assert_eq!(run.status, TaskRunStatus::Created);
    assert!(run.run_dir.starts_with(task_runs_dir(&cwd)));
    assert!(run.run_dir.ends_with(format!("find-conflicts/{}", run.id)));
    assert!(run.run_dir.join("run.json").is_file());
    assert!(run.run_dir.join("input.json").is_file());
    assert!(run.run_dir.join("artifacts").is_dir());
    assert_eq!(read_json(run.run_dir.join("input.json")), input);
    assert_eq!(read_task_run(&run.run_dir).unwrap(), run);

    let gitignore = fs::read_to_string(root.join("dadr/runs/.gitignore")).unwrap();
    assert_eq!(gitignore, "/**\n!.gitignore\n");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_task_run_persists_resolved_task_source_and_version() {
    let root = unique_temp_dir("dadrock-task-run-source-version");
    let project_tasks = root.join("dadr/tasks");
    let user_tasks = root.join("user-tasks");
    write_task_with_version(
        &project_tasks,
        "lab-audit",
        "Lab audit",
        "Project-specific lab audit.",
        7,
        "markdown_report.v1",
        "Return lab audit JSON only.\n",
    );
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");
    let listing = discover_tasks(&cwd, Some(&user_tasks));
    let tasks = tasks_by_id(&listing.tasks);
    let task = tasks["lab-audit"];

    let run = create_task_run(&cwd, task, json!({"scope": "project"})).unwrap();

    assert_eq!(run.task_id, "lab-audit");
    assert_eq!(run.task_source.id, "project:lab-audit");
    assert_eq!(run.task_source.kind, TaskSourceKind::Project);
    assert_eq!(run.task_version, 7);

    let run_json = read_json(run.run_dir.join("run.json"));
    assert_eq!(run_json["task_id"], json!("lab-audit"));
    assert_eq!(run_json["task_source"]["id"], json!("project:lab-audit"));
    assert_eq!(run_json["task_source"]["kind"], json!("project"));
    assert_eq!(run_json["task_version"], json!(7));

    let persisted = read_task_run(&run.run_dir).unwrap();
    assert_eq!(persisted.task_source, run.task_source);
    assert_eq!(persisted.task_version, 7);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn list_task_runs_skips_incomplete_run_directories_without_metadata() {
    let root = unique_temp_dir("dadrock-task-run-incomplete-dir");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "context-to-record");
    let run = create_task_run(&cwd, &task, json!({"context": "Use complete cases."})).unwrap();
    let stale_dir = task_runs_dir(&cwd)
        .join("code-to-record")
        .join("019e1f1b-c550-7f50-82ee-e5d644de6b54");
    fs::create_dir_all(stale_dir.join("artifacts").as_std_path()).expect("create stale run dir");

    let runs = list_task_runs(&cwd).unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, run.id);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_task_result_marks_run_succeeded_and_stores_result() {
    let root = unique_temp_dir("dadrock-task-run-result");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    let result = json!({
        "kind": "markdown_report.v1",
        "body_markdown": "All clear."
    });

    let updated = write_task_result(&run.run_dir, result.clone()).unwrap();

    assert_eq!(updated.id, run.id);
    assert_eq!(updated.task_id, run.task_id);
    assert_eq!(updated.status, TaskRunStatus::Succeeded);
    assert!(updated.updated_at >= run.updated_at);
    assert_eq!(updated.run_dir, run.run_dir);
    assert_eq!(read_json(updated.run_dir.join("result.json")), result);
    assert_eq!(read_task_run(&updated.run_dir).unwrap(), updated);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_task_failure_marks_run_failed_and_stores_error_metadata() {
    let root = unique_temp_dir("dadrock-task-run-failure");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    let raw_output = json!({
        "kind": "conflict_set.v1",
        "summary": "Wrong shape for custom"
    });

    let updated = write_task_failure(
        &run.run_dir,
        "task output did not match expected kind",
        Some(raw_output.clone()),
    )
    .unwrap();

    assert_eq!(updated.id, run.id);
    assert_eq!(updated.task_id, run.task_id);
    assert_eq!(updated.status, TaskRunStatus::Failed);
    assert!(updated.updated_at >= run.updated_at);
    assert_eq!(updated.run_dir, run.run_dir);
    assert_eq!(
        read_json(updated.run_dir.join("error.json"))["message"],
        json!("task output did not match expected kind")
    );
    assert_eq!(
        read_json(updated.run_dir.join("artifacts/raw-output.json")),
        raw_output
    );
    assert!(!updated.run_dir.join("result.json").exists());
    assert_eq!(read_task_run(&updated.run_dir).unwrap(), updated);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn mark_task_run_cancelled_persists_cancelled_status() {
    let root = unique_temp_dir("dadrock-task-run-cancelled");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 temp root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();

    let updated = mark_task_run_cancelled(&run.run_dir).unwrap();

    assert_eq!(updated.status, TaskRunStatus::Cancelled);
    assert_eq!(read_task_run(&updated.run_dir).unwrap(), updated);
}

#[test]
fn write_task_result_rejects_cancelled_runs() {
    let root = unique_temp_dir("dadrock-task-run-cancelled-result");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 temp root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    mark_task_run_cancelled(&run.run_dir).unwrap();

    let error =
        write_task_result(&run.run_dir, json!({"kind": "diagnosis_report.v1"})).unwrap_err();

    assert!(error.to_string().contains("was cancelled"));
    assert_eq!(
        read_task_run(&run.run_dir).unwrap().status,
        TaskRunStatus::Cancelled
    );
}

#[test]
fn task_cancel_command_marks_run_cancelled_and_logs_once() {
    let root = unique_temp_dir("dadrock-task-run-cancel-command");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    let context = CommandContext::from_path(&root).expect("context");

    let envelope = cancel_task_run_command(&context, &run.id.to_string()).unwrap();

    assert_eq!(envelope.command, "task cancel");
    assert_eq!(envelope.data.status, TaskRunStatus::Cancelled);
    assert_eq!(envelope.changed_paths, vec![run.run_dir.clone()]);
    let entries = read_task_run_log(&run.run_dir).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "Task cancelled.");

    let second = cancel_task_run_command(&context, &run.id.to_string()).unwrap();
    assert_eq!(second.data.status, TaskRunStatus::Cancelled);
    assert_eq!(read_task_run_log(&run.run_dir).unwrap().len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_log_appends_timestamped_messages() {
    let root = unique_temp_dir("dadrock-task-run-log");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();

    append_task_run_log(&run.run_dir, "Starting local ACP agent").unwrap();
    append_task_run_log(&run.run_dir, "Prompt sent; waiting for agent output").unwrap();

    assert!(run.run_dir.join("log.jsonl").is_file());
    assert!(!run.run_dir.join("log.json").exists());
    let entries = read_task_run_log(&run.run_dir).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message, "Starting local ACP agent");
    assert_eq!(entries[1].message, "Prompt sent; waiting for agent output");
    assert!(entries[1].timestamp >= entries[0].timestamp);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_log_extends_adjacent_agent_chunks_without_splitting_words() {
    let root = unique_temp_dir("dadrock-task-run-log-agent-chunks");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();

    append_task_run_log(&run.run_dir, "Prompt sent; waiting for agent output.").unwrap();
    append_or_extend_task_run_log(&run.run_dir, "agent: ", "Let me identify cons").unwrap();
    append_or_extend_task_run_log(&run.run_dir, "agent: ", "equential choices.").unwrap();
    append_task_run_log(&run.run_dir, "Task succeeded.").unwrap();

    let entries = read_task_run_log(&run.run_dir).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(
        entries[1].message,
        "agent: Let me identify consequential choices."
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_task_failure_rejects_terminal_runs_without_overwriting_error_artifact() {
    let root = unique_temp_dir("dadrock-task-run-failure-terminal");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    let first_raw = json!({"kind": "first"});
    let second_raw = json!({"kind": "second"});

    write_task_failure(&run.run_dir, "first failure", Some(first_raw.clone())).unwrap();
    let error = write_task_failure(&run.run_dir, "second failure", Some(second_raw)).unwrap_err();

    assert!(error.to_string().contains("already failed"));
    assert_eq!(
        read_json(run.run_dir.join("artifacts/raw-output.json")),
        first_raw
    );
    assert_eq!(
        read_task_run(&run.run_dir).unwrap().status,
        TaskRunStatus::Failed
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_task_result_rejects_failed_runs_before_creating_result() {
    let root = unique_temp_dir("dadrock-task-run-result-after-failure");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();

    write_task_failure(&run.run_dir, "failed first", None).unwrap();
    let error =
        write_task_result(&run.run_dir, json!({"kind": "diagnosis_report.v1"})).unwrap_err();

    assert!(error.to_string().contains("already failed"));
    assert!(!run.run_dir.join("result.json").exists());
    assert_eq!(
        read_task_run(&run.run_dir).unwrap().status,
        TaskRunStatus::Failed
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn write_task_result_rejects_existing_result_without_overwriting_it() {
    let root = unique_temp_dir("dadrock-task-run-result-existing");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "custom");
    let run = create_task_run(&cwd, &task, json!({"status": "all"})).unwrap();
    let first_result = json!({
        "kind": "markdown_report.v1",
        "body_markdown": "First result."
    });
    let second_result = json!({
        "kind": "markdown_report.v1",
        "body_markdown": "Second result."
    });

    write_task_result(&run.run_dir, first_result.clone()).unwrap();
    let error = write_task_result(&run.run_dir, second_result).unwrap_err();

    assert!(error.to_string().contains("already succeeded"));
    assert_eq!(read_json(run.run_dir.join("result.json")), first_result);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_task_run_preserves_existing_gitignore_content_and_appends_missing_rules() {
    let root = unique_temp_dir("dadrock-task-run-gitignore-preserve");
    let runs_dir = root.join("dadr/runs");
    fs::create_dir_all(&runs_dir).expect("create runs dir");
    fs::write(runs_dir.join(".gitignore"), "# local notes\n/cache/\n").expect("write gitignore");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "code-to-record");

    create_task_run(&cwd, &task, json!({})).unwrap();

    let gitignore = fs::read_to_string(runs_dir.join(".gitignore")).unwrap();
    assert_eq!(gitignore, "/**\n!.gitignore\n");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn create_task_run_rejects_invalid_or_path_traversing_task_ids() {
    let root = unique_temp_dir("dadrock-task-run-invalid-id");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let base_task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");

    for task_id in [
        "",
        "Find-Conflicts",
        "find_conflicts",
        "../escape",
        "escape/path",
    ] {
        let mut task = base_task.clone();
        task.manifest.id = task_id.to_owned();
        let error = create_task_run(&cwd, &task, json!({})).unwrap_err();
        assert!(
            error.to_string().contains("task id"),
            "unexpected error for {task_id:?}: {error}"
        );
    }

    assert!(!task_runs_dir(&cwd).exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_commands_use_discovered_project_root_from_subdirectory() {
    let root = unique_temp_dir("dadrock-task-run-command-project-root");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");
    let run = create_task_run(&cwd, &task, json!({"scope": "live"})).unwrap();
    let result = json!({
        "kind": "conflict_set.v1",
        "summary": "No conflicts found.",
        "findings": []
    });
    write_task_result(&run.run_dir, result.clone()).unwrap();
    let nested = root.join("some/subdir");
    fs::create_dir_all(&nested).expect("create nested cwd");
    let context = CommandContext::from_path(&nested).expect("context");

    let runs = list_task_runs_command(&context).unwrap();
    assert_eq!(runs.data.len(), 1);
    assert_eq!(runs.data[0].id, run.id);
    assert_eq!(runs.data[0].run_dir, run.run_dir);

    let envelope = show_task_result_command(&context, &run.id.to_string()).unwrap();
    assert_eq!(envelope.data, result);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn submit_task_result_command_validates_and_persists_by_run_id() {
    let root = unique_temp_dir("dadrock-task-run-submit-result");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");
    let run = create_task_run(&cwd, &task, json!({"scope": "live"})).unwrap();
    let context = CommandContext::from_path(&root).expect("context");
    let result = json!({
        "kind": "conflict_set.v1",
        "summary": "No conflicts found.",
        "findings": []
    });

    let envelope =
        submit_task_result_command(&context, &run.id.to_string(), result.clone()).unwrap();

    assert_eq!(envelope.command, "task submit-result");
    assert_eq!(envelope.data.id, run.id);
    assert_eq!(envelope.data.status, TaskRunStatus::Succeeded);
    assert_eq!(envelope.changed_paths, vec![run.run_dir.clone()]);
    assert_eq!(read_json(run.run_dir.join("result.json")), result);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn submit_task_result_command_rejects_result_kind_mismatch() {
    let root = unique_temp_dir("dadrock-task-run-submit-result-mismatch");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");
    let run = create_task_run(&cwd, &task, json!({"scope": "live"})).unwrap();
    let context = CommandContext::from_path(&root).expect("context");

    let error = submit_task_result_command(
        &context,
        &run.id.to_string(),
        json!({
            "kind": "markdown_report.v1",
            "body_markdown": "Wrong result kind."
        }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("did not match expected kind"));
    assert!(!run.run_dir.join("result.json").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn show_task_result_command_rejects_path_like_run_ids_even_when_files_exist() {
    let workspace = unique_temp_dir("dadrock-task-run-result-path-reject");
    let root = workspace.join("project");
    let outside = workspace.join("outside");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks =
        Utf8PathBuf::from_path_buf(workspace.join("user-tasks")).expect("utf8 user tasks");
    let task = discovered_task(&cwd, Some(&user_tasks), "find-conflicts");
    let run = create_task_run(&cwd, &task, json!({"scope": "live"})).unwrap();
    write_task_result(
        &run.run_dir,
        json!({
            "kind": "conflict_set.v1",
            "summary": "No conflicts found.",
            "findings": []
        }),
    )
    .unwrap();
    fs::create_dir_all(&outside).expect("create outside run dir");
    fs::copy(run.run_dir.join("run.json"), outside.join("run.json")).expect("copy run metadata");
    fs::copy(run.run_dir.join("result.json"), outside.join("result.json"))
        .expect("copy run result");
    let context = CommandContext::from_path(&root).expect("context");
    let absolute_outside = Utf8PathBuf::from_path_buf(outside.clone())
        .expect("utf8 outside")
        .to_string();

    for run_id in ["../outside", absolute_outside.as_str()] {
        let error = show_task_result_command(&context, run_id).unwrap_err();
        assert!(
            error.is_domain_kind(DomainErrorKind::InvalidInput),
            "unexpected error for {run_id:?}: {error}"
        );
        assert!(
            error.to_string().contains("task run id"),
            "missing run id context for {run_id:?}: {error}"
        );
    }

    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn list_task_runs_returns_error_for_malformed_run_metadata() {
    let root = unique_temp_dir("dadrock-task-run-malformed-metadata");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let run_json = task_runs_dir(&cwd)
        .join("find-conflicts")
        .join("malformed-run")
        .join("run.json");
    fs::create_dir_all(run_json.parent().unwrap()).expect("create run dir");
    fs::write(&run_json, "{not json").expect("write malformed run metadata");

    let error = list_task_runs(&cwd).unwrap_err();

    assert!(
        error.to_string().contains("run.json"),
        "missing metadata path in error: {error}"
    );
    assert!(
        error.to_string().contains("find-conflicts"),
        "missing task path context in error: {error}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn agent_request_run_task_serializes_task_payload_and_prompt() {
    let root = unique_temp_dir("dadrock-task-agent-request");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let project = agent_project_context(&cwd);
    let task = AgentTaskRequest {
        id: "custom".to_owned(),
        run_id: "01960000000000000000000000".to_owned(),
        title: "Diagnose decision ledger".to_owned(),
        description: "Inspect the decision ledger for drift.".to_owned(),
        output_kind: TaskOutputKind::MarkdownReportV1,
        input: json!({"status": "all"}),
        context: TaskContext {
            system_prompt: None,
            instructions: Some("Focus on stale candidates.".to_owned()),
            paths: vec!["crates/dadrock/src/tasks/runner.rs".to_owned()],
            records: vec!["01960000000000000000000000".to_owned()],
            text: Some("Prior review noted weak alternatives.".to_owned()),
        },
        prompt: "Read the ledger and return a report.".to_owned(),
    };

    let request = AgentRequestEnvelope::run_task(
        "Run the diagnosis.".to_owned(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        task.clone(),
    )
    .expect("run task request");

    assert_eq!(request.intent, AgentIntent::RunTask);
    assert_eq!(request.task.as_ref(), Some(&task));
    assert_eq!(request.candidate, None);

    let serialized = serde_json::to_value(&request).expect("serialize request");
    assert_eq!(serialized["intent"], json!("run_task"));
    assert_eq!(serialized["prompt"], json!("Run the diagnosis."));
    assert_eq!(serialized["task"]["id"], json!("custom"));
    assert_eq!(
        serialized["task"]["run_id"],
        json!("01960000000000000000000000")
    );
    assert_eq!(
        serialized["task"]["title"],
        json!("Diagnose decision ledger")
    );
    assert_eq!(
        serialized["task"]["output_kind"],
        json!("markdown_report.v1")
    );
    assert_eq!(serialized["task"]["input"], json!({"status": "all"}));
    assert_eq!(
        serialized["task"]["context"]["instructions"],
        json!("Focus on stale candidates.")
    );
    assert_eq!(
        serialized["task"]["context"]["paths"],
        json!(["crates/dadrock/src/tasks/runner.rs"])
    );
    assert_eq!(
        serialized["task"]["context"]["records"],
        json!(["01960000000000000000000000"])
    );
    assert_eq!(
        serialized["task"]["context"]["text"],
        json!("Prior review noted weak alternatives.")
    );
    assert_eq!(
        serialized["task"]["prompt"],
        json!("Read the ledger and return a report.")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_task_prompt_demands_declared_output_kind() {
    let root = unique_temp_dir("dadrock-task-agent-prompt");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let project = agent_project_context(&cwd);
    let request = AgentRequestEnvelope::run_task(
        "Run the markdown task.".to_owned(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        AgentTaskRequest {
            id: "custom".to_owned(),
            run_id: "01960000000000000000000000".to_owned(),
            title: "Diagnose decision ledger".to_owned(),
            description: "Inspect the decision ledger for drift.".to_owned(),
            output_kind: TaskOutputKind::MarkdownReportV1,
            input: json!({"status": "all"}),
            context: TaskContext {
                system_prompt: None,
                instructions: Some("Pay attention to lifecycle drift.".to_owned()),
                paths: vec!["crates/dadrock/src/mcp.rs".to_owned()],
                records: Vec::new(),
                text: None,
            },
            prompt: "Return a markdown report.".to_owned(),
        },
    )
    .expect("run task request");

    let prompt = run_task_prompt(&request).expect("render task prompt");

    assert!(prompt.contains("dadrock.submit_task_result"));
    assert!(prompt.contains("01960000000000000000000000"));
    assert!(prompt.contains("Pay attention to lifecycle drift."));
    assert!(prompt.contains("crates/dadrock/src/mcp.rs"));
    assert!(prompt.contains("Task mode rules:"));
    assert!(!prompt.contains("prefer calling it with the task `run_id`"));
    assert!(prompt.contains("\"kind\": \"markdown_report.v1\""));
    assert!(prompt.contains("Dadrock task request:"));
    assert!(prompt.contains("\"task\": {"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_task_prompt_includes_output_contract_for_declared_kind() {
    let root = unique_temp_dir("dadrock-task-agent-schema-prompt");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let project = agent_project_context(&cwd);
    let request = AgentRequestEnvelope::run_task(
        "Run the diagnosis task.".to_owned(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        AgentTaskRequest {
            id: "custom".to_owned(),
            run_id: "01960000000000000000000000".to_owned(),
            title: "Diagnose decision ledger".to_owned(),
            description: "Inspect the decision ledger for drift.".to_owned(),
            output_kind: TaskOutputKind::DiagnosisReportV1,
            input: json!({"include_candidates": true}),
            context: TaskContext::default(),
            prompt: "Return a diagnosis report.".to_owned(),
        },
    )
    .expect("run task request");

    let prompt = run_task_prompt(&request).expect("render task prompt");

    assert!(prompt.contains("The JSON object must match this output contract:"));
    assert!(prompt.contains("\"kind\": \"diagnosis_report.v1\""));
    assert!(prompt.contains("\"summary\": \"string or JSON object"));
    assert!(prompt.contains("\"issues\": ["));
    assert!(prompt.contains("\"suggested_actions\": [\"string\"]"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_task_prompt_includes_skill_record_and_tag_references() {
    let root = unique_temp_dir("dadrock-task-agent-skill-references");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let project = agent_project_context(&cwd);
    let request = AgentRequestEnvelope::run_task(
        "Create records from code.".to_owned(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        AgentTaskRequest {
            id: "code-to-record".to_owned(),
            run_id: "01960000000000000000000000".to_owned(),
            title: "Code to records".to_owned(),
            description: "Inspect project code and propose candidate DADR records.".to_owned(),
            output_kind: TaskOutputKind::CandidateBatchV1,
            input: json!({"path": "R/paperinfo"}),
            context: TaskContext::default(),
            prompt: "Return candidate proposals.".to_owned(),
        },
    )
    .expect("run task request");

    let prompt = run_task_prompt(&request).expect("render task prompt");

    assert!(prompt.contains("## Record anatomy"));
    assert!(prompt.contains("## Tags"));
    assert!(prompt.contains(include_str!("../../../skills/dadr/record.md").trim()));
    assert!(prompt.contains(include_str!("../../../skills/dadr/tags.md").trim()));
    assert!(prompt.contains("`../record.md` and `../tags.md` refer to"));
    assert!(!prompt.contains("DADR task guidance"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn skill_tag_reference_mirrors_docs_tag_vocabulary() {
    fn table<'a>(text: &'a str, header: &str) -> Vec<&'a str> {
        text.lines()
            .skip_while(|line| *line != header)
            .take_while(|line| line.starts_with('|'))
            .collect()
    }

    let docs = include_str!("../../../docs-src/tags.md");
    let skill = include_str!("../../../skills/dadr/tags.md");
    for header in [
        "| Tag | Decisions about |",
        "| Namespace | Use it for | Examples |",
    ] {
        let docs_table = table(docs, header);
        assert!(docs_table.len() > 2, "docs-src/tags.md lacks `{header}`");
        assert_eq!(
            docs_table,
            table(skill, header),
            "skills/dadr/tags.md must mirror the `{header}` table in docs-src/tags.md"
        );
    }
}

#[test]
fn run_task_prompt_embeds_builtin_task_guide() {
    let root = unique_temp_dir("dadrock-task-agent-builtin-guide");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = Utf8PathBuf::from_path_buf(root.join("user-tasks")).expect("utf8 user tasks");
    let project = agent_project_context(&cwd);
    let task = discovered_task(&cwd, Some(&user_tasks), "supersede-record");
    let request = AgentRequestEnvelope::run_task(
        task.prompt.clone(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        AgentTaskRequest {
            id: task.manifest.id.clone(),
            run_id: "01960000000000000000000000".to_owned(),
            title: task.manifest.title.clone(),
            description: task.manifest.description.clone(),
            output_kind: task.manifest.output.kind.clone(),
            input: json!({
                "record_ids": "01960000-0000-7000-8000-000000000000",
                "requested_changes": "new evidence"
            }),
            context: TaskContext::default(),
            prompt: task.prompt.clone(),
        },
    )
    .expect("run task request");

    let prompt = run_task_prompt(&request).expect("render task prompt");

    let encoded_guide = serde_json::to_string(&task.prompt).expect("encode task guide");
    assert!(prompt.contains(&encoded_guide));
    assert!(prompt.contains("Start from the predecessor's exact text"));
    assert!(prompt.contains("Do not narrate the supersession"));
    assert!(prompt.contains("superseded_record_ids"));
    assert!(!prompt.contains("dadrock status supersede"));
    assert!(!prompt.contains("[[<predecessor-uuid>]]"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_task_prompt_states_task_mode_rules() {
    let root = unique_temp_dir("dadrock-task-agent-task-mode");
    fs::create_dir_all(&root).expect("create temp dir");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let project = agent_project_context(&cwd);
    let request = AgentRequestEnvelope::run_task(
        "Create records from code.".to_owned(),
        AgentProjectSnapshot::new(&project, Vec::new()),
        AgentTaskRequest {
            id: "code-to-record".to_owned(),
            run_id: "01960000000000000000000000".to_owned(),
            title: "Code to records".to_owned(),
            description: "Inspect project code and propose candidate DADR records.".to_owned(),
            output_kind: TaskOutputKind::CandidateBatchV1,
            input: json!({"path": "R/paperinfo"}),
            context: TaskContext::default(),
            prompt: "Return candidate proposals.".to_owned(),
        },
    )
    .expect("run task request");

    let prompt = run_task_prompt(&request).expect("render task prompt");

    assert!(prompt.contains("task id `code-to-record`"));
    assert!(prompt.contains("dadrock decides how it is delivered"));
    assert!(prompt.contains(
        "Do not create, edit, move, accept, reject, or supersede records or other files."
    ));
    assert!(prompt.contains("proposals that a human reviews"));
    assert!(prompt.contains("put that content in the result instead"));
    assert!(!prompt.contains("installed `dadr` skill"));
    assert!(!prompt.contains("--- Prompt pack: dadr"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_output_repair_prompt_includes_error_raw_output_and_contract() {
    let raw_output = json!({
        "kind": "diagnosis_report.v1",
        "summary": {"headline": "Structured summary"},
        "issues": []
    });

    let prompt = task_output_repair_prompt(
        &TaskOutputKind::DiagnosisReportV1,
        "invalid type: map, expected a string",
        &raw_output,
    )
    .expect("repair prompt");

    assert!(prompt.contains("corrected JSON object"));
    assert!(prompt.contains("invalid type: map, expected a string"));
    assert!(prompt.contains("\"kind\": \"diagnosis_report.v1\""));
    assert!(prompt.contains("\"summary\": \"string or JSON object"));
    assert!(prompt.contains("Structured summary"));
}

#[test]
fn parse_acp_agent_json_response_requires_json_object() {
    let parsed =
        parse_acp_agent_json_response(r#"{"kind":"markdown_report.v1","body_markdown":"OK"}"#)
            .expect("object response");

    assert_eq!(parsed["kind"], json!("markdown_report.v1"));

    for raw in ["not json", "[]", "\"text\"", "null"] {
        let error = parse_acp_agent_json_response(raw).unwrap_err();
        assert!(error.is_domain_kind(DomainErrorKind::Acp));
        assert!(
            error.to_string().contains("JSON object"),
            "unexpected error for {raw:?}: {error}"
        );
    }
}

#[test]
fn parse_acp_agent_json_response_extracts_fenced_object() {
    let parsed = parse_acp_agent_json_response(
        r#"
I found several issues.

```json
{"kind":"markdown_report.v1","body_markdown":"OK"}
```
"#,
    )
    .expect("fenced object response");

    assert_eq!(parsed["kind"], json!("markdown_report.v1"));
    assert_eq!(parsed["body_markdown"], json!("OK"));
}

#[test]
fn parse_acp_agent_json_response_rejects_truncated_object() {
    let error = parse_acp_agent_json_response(r#"{"kind":"markdown_report.v1","body":"}"#)
        .expect_err("truncated object should fail");

    assert!(error.to_string().contains("invalid JSON object"));
}

#[test]
fn command_catalog_includes_task_surface() {
    let catalog = command_catalog();
    let expected = [
        (&["task"][..], true, false),
        (&["task", "list"][..], false, true),
        (&["task", "show"][..], false, true),
        (&["task", "run"][..], true, true),
        (&["task", "runs"][..], false, true),
        (&["task", "result"][..], false, true),
    ];

    for (path, mutates_project, supports_json) in expected {
        let info = find_command_info(path)
            .unwrap_or_else(|| panic!("missing command metadata for {path:?}"));
        assert_eq!(
            info.mutation.mutates_project(),
            mutates_project,
            "unexpected mutation flag for {path:?}"
        );
        assert_eq!(
            info.supports_json, supports_json,
            "unexpected JSON flag for {path:?}"
        );

        let name = path.join(" ");
        let record = catalog
            .data
            .commands
            .iter()
            .find(|command| command.name == name)
            .unwrap_or_else(|| panic!("missing catalog record for {path:?}"));
        assert_eq!(record.mutates_project, mutates_project);
        assert_eq!(record.supports_json, supports_json);
    }

    assert_eq!(
        find_command_info(&["task"]).unwrap().subcommands,
        &[
            "task list",
            "task show",
            "task run",
            "task runs",
            "task result",
        ]
    );
}

#[test]
fn list_tasks_command_returns_builtins() {
    let root = unique_temp_dir("dadrock-task-command-list");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&root).expect("context");

    let envelope = list_tasks_command(&context);

    assert!(envelope.ok);
    assert_eq!(envelope.command, "task list");
    assert_eq!(envelope.diagnostics, Vec::new());
    let tasks = tasks_by_id(&envelope.data.tasks);
    assert_eq!(
        tasks.keys().cloned().collect::<Vec<_>>(),
        vec![
            "code-to-record",
            "context-to-record",
            "custom",
            "decisions-to-spec",
            "edit-record",
            "find-conflicts",
            "merge-records",
            "supersede-record",
        ]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn code_to_record_rejects_run_when_no_context_paths_supplied() {
    let root = unique_temp_dir("dadrock-task-code-to-record-no-paths");
    init_project(&root).unwrap();
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let context = CommandContext::from_path(&root).expect("context");

    let err = run_task_command(
        &context,
        TaskRunCommandRequest {
            task_id: "code-to-record".to_owned(),
            input: json!({"path": "."}),
            context: TaskContext::default(),
            local_agent: None,
            permission_policy: Default::default(),
            enable_dadrock_mcp: true,
        },
    )
    .expect_err("code-to-record should reject empty context paths");

    let message = err
        .domain_message(DomainErrorKind::InvalidInput)
        .expect("expected InvalidInput");
    assert!(
        message.contains("context file or directory"),
        "unexpected error message: {message}"
    );

    let _ = fs::remove_dir_all(root);
}

/// Runs `dadrock task run` where no ACP agent can be found: nothing on
/// `PATH` and no `DADROCK_ACP_AGENT`.
fn dadrock_task_run_without_agent(root: &Path, args: &[&str]) -> std::process::Output {
    let empty_path = root.join("empty-path");
    fs::create_dir_all(&empty_path).expect("create empty PATH dir");
    std::process::Command::new(env!("CARGO_BIN_EXE_dadrock"))
        .args(["task", "run"])
        .args(args)
        .current_dir(root)
        .env("PATH", &empty_path)
        .env("DADROCK_TASKS_DIR", root.join("user-tasks"))
        .env_remove("DADROCK_ACP_AGENT")
        .env_remove("DADROCK_ACP_AGENT_ARGS")
        .output()
        .expect("run dadrock task run")
}

#[test]
fn task_run_without_agent_fails_clearly_and_creates_no_run() {
    let root = unique_temp_dir("dadrock-task-run-no-agent");
    init_project(&root).unwrap();

    let output = dadrock_task_run_without_agent(
        &root,
        &["context-to-record", "--context", "Explain the sample"],
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no local ACP agent found") && stderr.contains("DADROCK_ACP_AGENT"),
        "unexpected stderr: {stderr}"
    );
    let project_root = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    assert!(list_task_runs(&project_root).unwrap().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_validation_rejects_missing_required_input() {
    let root = unique_temp_dir("dadrock-task-required-input");
    init_project(&root).unwrap();
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let project = agent_project_context(&Utf8PathBuf::from_path_buf(root.clone()).unwrap());
    let user_tasks = Utf8PathBuf::from_path_buf(user_tasks).expect("utf8 user tasks");
    let task = discovered_task(&project.cwd, Some(&user_tasks), "context-to-record");

    let error = create_task_run_for_task(
        &project,
        &task.manifest.id,
        json!({}),
        &TaskContext::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("required input `context`"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_validation_rejects_invalid_select_input_values() {
    let root = unique_temp_dir("dadrock-task-invalid-select");
    init_project(&root).unwrap();
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let project = agent_project_context(&Utf8PathBuf::from_path_buf(root.clone()).unwrap());

    let error = create_task_run_for_task(
        &project,
        "find-conflicts",
        json!({"status_scope": "bogus"}),
        &TaskContext::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("invalid value `bogus`"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_run_validation_enforces_notes_records_and_status_requirements() {
    let root = unique_temp_dir("dadrock-task-requirements");
    init_project(&root).unwrap();
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let project = discover_from_pathbuf(&root);
    let agent_project =
        agent_project_context(&Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root"));
    let candidate = dadrock::records::create_candidate_with_body(
        &project,
        "Candidate A",
        &[],
        "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n\
         ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n",
    )
    .unwrap();
    let second_candidate =
        dadrock::records::create_candidate(&project, "Candidate B", &[]).unwrap();
    let accepted = dadrock::records::accept_candidate(
        &ProjectIndex::scan(&project).unwrap(),
        &candidate.id.to_string(),
    )
    .unwrap()
    .accepted
    .remove(0);

    let custom_error = create_task_run_for_task(
        &agent_project,
        "custom",
        json!({}),
        &TaskContext {
            records: vec![accepted.id.to_string(), second_candidate.id.to_string()],
            ..TaskContext::default()
        },
    )
    .unwrap_err();
    assert!(custom_error.to_string().contains("requires notes"));

    let merge_count_error = create_task_run_for_task(
        &agent_project,
        "merge-records",
        json!({"record_ids": [second_candidate.id.to_string()]}),
        &TaskContext {
            records: vec![second_candidate.id.to_string()],
            ..TaskContext::default()
        },
    )
    .unwrap_err();
    assert!(merge_count_error
        .to_string()
        .contains("at least two records"));

    let merge_status_error = create_task_run_for_task(
        &agent_project,
        "merge-records",
        json!({"record_ids": [accepted.id.to_string(), second_candidate.id.to_string()]}),
        &TaskContext {
            records: vec![accepted.id.to_string(), second_candidate.id.to_string()],
            text: Some("merge these".to_owned()),
            ..TaskContext::default()
        },
    )
    .unwrap_err();
    assert!(merge_status_error.to_string().contains("candidate records"));

    let supersede_status_error = create_task_run_for_task(
        &agent_project,
        "supersede-record",
        json!({"record_ids": second_candidate.id.to_string(), "requested_changes": "new evidence"}),
        &TaskContext {
            records: vec![second_candidate.id.to_string()],
            ..TaskContext::default()
        },
    )
    .unwrap_err();
    assert!(supersede_status_error
        .to_string()
        .contains("accepted record"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn list_tasks_command_uses_project_root_from_subdirectory() {
    let root = unique_temp_dir("dadrock-task-command-list-nested");
    init_project(&root).unwrap();
    write_task(
        &root.join("dadr/tasks"),
        "find-conflicts",
        "Project conflict finder",
        "Project-specific conflict scan.",
        "markdown_report.v1",
        "Return project conflict results as JSON only.\n",
    );
    let nested = root.join("analysis/subdir");
    fs::create_dir_all(&nested).expect("create nested cwd");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&nested).expect("context");

    let envelope = list_tasks_command(&context);

    let tasks = tasks_by_id(&envelope.data.tasks);
    let task = tasks["find-conflicts"];
    assert_eq!(task.source.kind, TaskSourceKind::Project);
    assert!(task
        .source
        .path
        .as_ref()
        .is_some_and(|path| path.starts_with(Utf8PathBuf::from_path_buf(root.clone()).unwrap())));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn show_task_command_resolves_one_task() {
    let root = unique_temp_dir("dadrock-task-command-show");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&root).expect("context");

    let envelope = show_task_command(&context, "find-conflicts").unwrap();

    assert!(envelope.ok);
    assert_eq!(envelope.command, "task show");
    assert_eq!(envelope.data.manifest.id, "find-conflicts");
    assert_eq!(envelope.data.source.kind, TaskSourceKind::Builtin);
    assert_eq!(envelope.data.source.id, "builtin:find-conflicts");
    assert!(!envelope.data.prompt.trim().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn show_task_command_uses_project_root_from_subdirectory() {
    let root = unique_temp_dir("dadrock-task-command-show-nested");
    init_project(&root).unwrap();
    write_task(
        &root.join("dadr/tasks"),
        "nested-project-task",
        "Nested project task",
        "Project task visible from nested directories.",
        "markdown_report.v1",
        "Return project task results as JSON only.\n",
    );
    let nested = root.join("analysis/subdir");
    fs::create_dir_all(&nested).expect("create nested cwd");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&nested).expect("context");

    let envelope = show_task_command(&context, "nested-project-task").unwrap();

    assert_eq!(envelope.data.manifest.id, "nested-project-task");
    assert_eq!(envelope.data.source.kind, TaskSourceKind::Project);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn show_task_command_rejects_empty_task_id() {
    let root = unique_temp_dir("dadrock-task-command-show-empty");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&root).expect("context");

    let error = show_task_command(&context, " ").unwrap_err();

    assert!(error.is_domain_kind(DomainErrorKind::InvalidInput));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn show_task_command_rejects_missing_task_id() {
    let root = unique_temp_dir("dadrock-task-command-show-missing");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&root).expect("context");

    let error = show_task_command(&context, "missing-task").unwrap_err();

    assert!(error.is_domain_kind(DomainErrorKind::InvalidInput));

    let _ = fs::remove_dir_all(root);
}

const CODE_TO_RECORD_GUIDE: &str = include_str!("../../../skills/dadr/tasks/code-to-record.md");

fn dadrock_task_show(cwd: &Path, user_tasks: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_dadrock"))
        .args(["task", "show"])
        .args(args)
        .current_dir(cwd)
        .env("DADROCK_TASKS_DIR", user_tasks)
        .output()
        .expect("run dadrock task show")
}

#[test]
fn show_task_command_keeps_the_document_out_of_json() {
    let root = unique_temp_dir("dadrock-task-command-show-document");
    fs::create_dir_all(&root).expect("create temp dir");
    let user_tasks = root.join("user-tasks");
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", &user_tasks);
    let context = CommandContext::from_path(&root).expect("context");

    let envelope = show_task_command(&context, "code-to-record").unwrap();

    assert_eq!(envelope.data.document, CODE_TO_RECORD_GUIDE);
    let json = serde_json::to_value(&envelope).unwrap();
    let keys = json["data"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(keys, ["manifest", "prompt", "source"].into());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_show_raw_prints_the_builtin_skill_guide_verbatim() {
    let root = unique_temp_dir("dadrock-task-show-raw-builtin");
    init_project(&root).unwrap();

    let output = dadrock_task_show(
        &root,
        &root.join("user-tasks"),
        &["code-to-record", "--raw"],
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        CODE_TO_RECORD_GUIDE
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn raw_builtin_task_copied_to_project_loads_as_the_same_task() {
    let root = unique_temp_dir("dadrock-task-show-raw-copy");
    init_project(&root).unwrap();
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    let user_tasks = cwd.join("user-tasks");
    let builtin = discovered_task(&cwd, Some(&user_tasks), "code-to-record");
    assert_eq!(builtin.source.kind, TaskSourceKind::Builtin);

    let output = dadrock_task_show(
        &root,
        user_tasks.as_std_path(),
        &["code-to-record", "--raw"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::create_dir_all(root.join("dadr/tasks")).expect("create project task dir");
    fs::write(root.join("dadr/tasks/code-to-record.md"), &output.stdout)
        .expect("write project task");

    let project = discovered_task(&cwd, Some(&user_tasks), "code-to-record");
    assert_eq!(project.source.kind, TaskSourceKind::Project);
    assert_eq!(project.manifest, builtin.manifest);
    assert_eq!(project.prompt, builtin.prompt);
    assert_eq!(project.document, builtin.document);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_show_raw_prints_a_project_task_file_verbatim() {
    let root = unique_temp_dir("dadrock-task-show-raw-project");
    init_project(&root).unwrap();
    let document = "---\n# Local tweak of a report task.\nid: local-report\ntitle:   \"Local report\"\ndescription: \"Report on the ledger.\"\nversion: 2\noutput:\n  kind: markdown_report.v1   # rendered as a report\n---\n\nReturn a Markdown report.\n\n<!-- keep this comment -->";
    fs::create_dir_all(root.join("dadr/tasks")).expect("create project task dir");
    fs::write(root.join("dadr/tasks/local-report.md"), document).expect("write project task");
    let nested = root.join("analysis");
    fs::create_dir_all(&nested).expect("create nested cwd");

    let output = dadrock_task_show(
        &nested,
        &root.join("user-tasks"),
        &["local-report", "--raw"],
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), document);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn task_show_rejects_raw_with_json() {
    let root = unique_temp_dir("dadrock-task-show-raw-json");
    init_project(&root).unwrap();

    let output = dadrock_task_show(
        &root,
        &root.join("user-tasks"),
        &["code-to-record", "--raw", "--json"],
    );

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"), "{stderr}");

    let _ = fs::remove_dir_all(root);
}

fn tasks_by_id(
    tasks: &[dadrock::tasks::TaskPackage],
) -> BTreeMap<&str, &dadrock::tasks::TaskPackage> {
    tasks
        .iter()
        .map(|task| (task.manifest.id.as_str(), task))
        .collect()
}

fn discovered_task(
    cwd: &Utf8PathBuf,
    user_tasks: Option<&Utf8PathBuf>,
    id: &str,
) -> dadrock::tasks::TaskPackage {
    let listing = discover_tasks(cwd, user_tasks.map(Utf8PathBuf::as_path));
    assert_eq!(listing.diagnostics, Vec::new());
    listing
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == id)
        .unwrap_or_else(|| panic!("expected task {id}"))
}

fn agent_project_context(cwd: &Utf8PathBuf) -> ProjectContext {
    ProjectContext {
        cwd: cwd.clone(),
        project_dir: cwd.join("dadr"),
        discovery_source: DiscoverySource::Fallback,
        initialized: true,
    }
}

fn discover_from_pathbuf(path: &PathBuf) -> ProjectContext {
    dadrock::project::discover_from(path).unwrap()
}

fn write_task(
    tasks_root: &std::path::Path,
    id: &str,
    title: &str,
    description: &str,
    output_kind: &str,
    prompt: &str,
) {
    write_task_with_version(tasks_root, id, title, description, 1, output_kind, prompt);
}

fn write_task_with_version(
    tasks_root: &std::path::Path,
    id: &str,
    title: &str,
    description: &str,
    version: u32,
    output_kind: &str,
    prompt: &str,
) {
    fs::create_dir_all(tasks_root).expect("create task dir");
    fs::write(
        tasks_root.join(format!("{id}.md")),
        format!(
            r#"---
id: {id}
title: "{title}"
description: "{description}"
version: {version}
output:
  kind: {output_kind}
---
{prompt}"#,
        ),
    )
    .expect("write task document");
}

fn unique_temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ))
}

fn read_json(path: Utf8PathBuf) -> serde_json::Value {
    let content = fs::read_to_string(path).expect("read json file");
    serde_json::from_str(&content).expect("parse json file")
}

#[test]
fn code_drafting_rejects_invalid_json_limits_before_starting_agent() {
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let root = unique_temp_dir("dadrock-invalid-limit");
    init_project(&root).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    for limit in [json!(0), json!(-1), json!(1.5), json!("5")] {
        let error = run_task_command(
            &context,
            TaskRunCommandRequest {
                task_id: "code-to-record".to_owned(),
                input: json!({"path":".", "limit":limit}),
                context: TaskContext {
                    paths: vec![".".to_owned()],
                    ..Default::default()
                },
                local_agent: None,
                permission_policy: dadrock::agent::acp::AcpPermissionPolicy::Deny,
                enable_dadrock_mcp: false,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("positive integer"));
    }
    fs::remove_dir_all(root).unwrap();
}

/// Fake ACP agent that records each `session/new` request and answers every
/// prompt with a `markdown_report.v1` result.
const RECORDING_ACP_AGENT: &str = r#"
import json, sys
log_path = sys.argv[1]
for line in sys.stdin:
    q = json.loads(line)
    method = q.get('method')
    if method == 'initialize':
        result = {'protocolVersion': 1, 'agentCapabilities': {}, 'authMethods': []}
    elif method == 'session/new':
        with open(log_path, 'a') as log:
            log.write(json.dumps(q.get('params', {})) + '\n')
        result = {'sessionId': 'test', 'configOptions': []}
    elif method == 'session/prompt':
        output = {'kind': 'markdown_report.v1', 'title': 'Spec', 'body_markdown': '## Spec\nDone.'}
        print(json.dumps({'jsonrpc': '2.0', 'method': 'session/update', 'params': {'sessionId': 'test', 'update': {'sessionUpdate': 'agent_message_chunk', 'content': {'type': 'text', 'text': json.dumps(output)}}}}), flush=True)
        result = {'stopReason': 'end_turn'}
    else:
        continue
    print(json.dumps({'jsonrpc': '2.0', 'id': q['id'], 'result': result}), flush=True)
"#;

fn run_task_with_recording_agent(root: &Path, task_id: &str) -> String {
    let script = root.join("recording-agent.py");
    let log = root.join(format!("{task_id}-sessions.jsonl"));
    fs::write(&script, RECORDING_ACP_AGENT).expect("write fake agent");
    let context = CommandContext::from_path(root).unwrap();
    let envelope = run_task_command(
        &context,
        TaskRunCommandRequest {
            task_id: task_id.to_owned(),
            input: json!({}),
            context: TaskContext::default(),
            local_agent: Some(dadrock::agent::providers::LocalAcpAgentCommand::new(
                "python3",
                vec![
                    script.to_string_lossy().into_owned(),
                    log.to_string_lossy().into_owned(),
                ],
            )),
            permission_policy: dadrock::agent::acp::AcpPermissionPolicy::Deny,
            enable_dadrock_mcp: true,
        },
    )
    .unwrap_or_else(|err| panic!("run {task_id}: {err}"));
    assert_eq!(envelope.data.status, TaskRunStatus::Succeeded);
    fs::read_to_string(log).expect("read recorded sessions")
}

/// The Dadrock MCP servers attached by each recorded `session/new` request.
fn attached_mcp_servers(sessions: &str) -> Vec<serde_json::Value> {
    let servers: Vec<_> = sessions
        .lines()
        .flat_map(|line| {
            let params: serde_json::Value = serde_json::from_str(line).expect("session params");
            params["mcpServers"]
                .as_array()
                .cloned()
                .expect("mcpServers array")
        })
        .collect();
    assert!(!servers.is_empty(), "no session attached an MCP server");
    servers
}

/// Fake ACP agent that records each prompt it receives and answers it with a
/// `markdown_report.v1` result.
const PROMPT_RECORDING_ACP_AGENT: &str = r#"
import json, sys
log_path = sys.argv[1]
for line in sys.stdin:
    q = json.loads(line)
    method = q.get('method')
    if method == 'initialize':
        result = {'protocolVersion': 1, 'agentCapabilities': {}, 'authMethods': []}
    elif method == 'session/new':
        result = {'sessionId': 'test', 'configOptions': []}
    elif method == 'session/prompt':
        with open(log_path, 'a') as log:
            for block in q['params']['prompt']:
                log.write(block.get('text', ''))
        output = {'kind': 'markdown_report.v1', 'title': 'Report', 'body_markdown': '## Report\nDone.'}
        print(json.dumps({'jsonrpc': '2.0', 'method': 'session/update', 'params': {'sessionId': 'test', 'update': {'sessionUpdate': 'agent_message_chunk', 'content': {'type': 'text', 'text': json.dumps(output)}}}}), flush=True)
        result = {'stopReason': 'end_turn'}
    else:
        continue
    print(json.dumps({'jsonrpc': '2.0', 'id': q['id'], 'result': result}), flush=True)
"#;

#[test]
fn custom_task_runs_from_the_cli_with_instructions_records_and_attachments() {
    let root = unique_temp_dir("dadrock-task-run-custom");
    init_project(&root).unwrap();
    let project = discover_from_pathbuf(&root);
    let record = dadrock::records::create_candidate(&project, "Income cutoff", &[]).unwrap();
    let short_id = record.id.to_string().replace('-', "")[..12].to_owned();
    let script = root.join("prompt-agent.py");
    let prompts = root.join("prompts.txt");
    fs::write(&script, PROMPT_RECORDING_ACP_AGENT).expect("write fake agent");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dadrock"))
        .args([
            "task",
            "run",
            "custom",
            "--instructions",
            "Summarize the sample restrictions",
            "--record",
            &short_id,
            "--attach",
            "analysis/",
            "--agent",
            "python3",
            "--agent-arg",
            script.to_str().unwrap(),
            "--agent-arg",
            prompts.to_str().unwrap(),
            "--no-mcp",
            "--json",
        ])
        .current_dir(&root)
        .env("DADROCK_TASKS_DIR", root.join("user-tasks"))
        .output()
        .expect("run dadrock task run custom");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["data"]["status"], json!("succeeded"));
    let run_dir = Utf8PathBuf::from(envelope["data"]["run_dir"].as_str().unwrap());
    assert_eq!(
        read_json(run_dir.join("context.json")),
        json!({
            "instructions": "Summarize the sample restrictions",
            "paths": ["analysis/"],
            "records": [record.id.to_string()],
        })
    );
    let prompt = fs::read_to_string(&prompts).expect("read recorded prompt");
    assert!(prompt.contains("Summarize the sample restrictions"));
    assert!(prompt.contains(&record.id.to_string()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn custom_task_without_instructions_names_the_flag_and_starts_nothing() {
    let root = unique_temp_dir("dadrock-task-run-custom-no-instructions");
    init_project(&root).unwrap();

    let output = dadrock_task_run_without_agent(&root, &["custom"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires notes or instructions"),
        "unexpected stderr: {stderr}"
    );
    let project_root = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");
    assert!(list_task_runs(&project_root).unwrap().is_empty());

    let output = dadrock_task_run_without_agent(
        &root,
        &["custom", "--instructions", "Count decisions by tag"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no local ACP agent found"),
        "unexpected stderr: {stderr}"
    );
    assert!(list_task_runs(&project_root).unwrap().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn every_task_run_attaches_task_run_dadrock_mcp() {
    let _env_lock = TASK_ENV_LOCK.lock().unwrap();
    let root = unique_temp_dir("dadrock-task-mcp-access");
    init_project(&root).unwrap();
    let _env = EnvVarGuard::set_path("DADROCK_TASKS_DIR", root.join("user-tasks"));
    let cwd = root.to_str().expect("utf8 root").to_owned();
    write_task(
        &root.join("dadr/tasks"),
        "lab-report",
        "Lab report",
        "Project task without any MCP declaration.",
        "markdown_report.v1",
        "Write a lab report.\n",
    );
    fs::write(
        root.join("dadr/tasks/older-report.md"),
        r#"---
id: older-report
title: Older report
description: Project task that still declares the retired full MCP access.
version: 1
output:
  kind: markdown_report.v1
mcp_access: full
---
Write a report.
"#,
    )
    .expect("write task with retired mcp_access");

    for task_id in ["decisions-to-spec", "lab-report", "older-report"] {
        let sessions = run_task_with_recording_agent(&root, task_id);
        for server in attached_mcp_servers(&sessions) {
            assert_eq!(server["name"], json!("dadrock"), "{task_id}: {server}");
            assert_eq!(
                server["args"],
                json!(["mcp", "serve", "--task-run", cwd]),
                "{task_id} must attach a task-run Dadrock MCP server"
            );
        }
    }

    let _ = fs::remove_dir_all(root);
}
