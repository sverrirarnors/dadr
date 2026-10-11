use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    agent::{
        acp::{
            parse_acp_agent_json_response, run_acp_agent_json_with_events_and_cancellation,
            run_acp_agent_text_with_events_and_cancellation, run_acp_prompt_json_with_events,
            AcpCancellationToken, AcpPermissionPolicy, AcpPromptEvent, AcpPromptEventSink,
            AcpPromptOptions,
        },
        prompts::task_output_repair_prompt,
        providers::{configured_local_acp_agent, LocalAcpAgentCommand},
        types::{AgentProjectSnapshot, AgentRequestEnvelope, AgentTaskRequest},
    },
    error::DadrError,
    index::ProjectIndex,
    project::ProjectContext,
    tasks::{
        append_or_extend_task_run_log, append_task_run_log, create_task_run,
        default_user_tasks_dir, discover_tasks, mark_task_run_running, read_task_run,
        validate_task_output, write_task_failure, write_task_result, PersistedTaskRun,
        TaskExecutionMode, TaskInputType, TaskNoteRequirement, TaskPackage, TaskPathRequirement,
        TaskRecordRequirement, TaskRecordStatusRequirement, TaskRequirements, TaskRunStatus,
    },
    RecordId, RecordStatus,
};

static TASK_CANCELLATIONS: OnceLock<Mutex<HashMap<String, AcpCancellationToken>>> = OnceLock::new();

pub fn request_task_run_cancellation(run_id: &RecordId) -> bool {
    TASK_CANCELLATIONS
        .get()
        .and_then(|registry| registry.lock().ok())
        .and_then(|registry| registry.get(&run_id.to_string()).cloned())
        .is_some_and(|token| {
            token.cancel();
            true
        })
}

struct TaskCancellationRegistration {
    run_id: String,
}

impl TaskCancellationRegistration {
    fn register(run_id: RecordId, token: AcpCancellationToken) -> Result<Self, DadrError> {
        let run_id = run_id.to_string();
        TASK_CANCELLATIONS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| DadrError::Acp("task cancellation registry is unavailable".to_owned()))?
            .insert(run_id.clone(), token);
        Ok(Self { run_id })
    }
}

impl Drop for TaskCancellationRegistration {
    fn drop(&mut self) {
        if let Some(mut registry) = TASK_CANCELLATIONS
            .get()
            .and_then(|registry| registry.lock().ok())
        {
            registry.remove(&self.run_id);
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl TaskContext {
    pub fn is_empty(&self) -> bool {
        self.system_prompt.as_deref().is_none_or(str::is_empty)
            && self.instructions.as_deref().is_none_or(str::is_empty)
            && self.paths.is_empty()
            && self.records.is_empty()
            && self.text.as_deref().is_none_or(str::is_empty)
    }
}

pub fn validate_task_context_requirements(
    project: &ProjectContext,
    task: &TaskPackage,
    input: &Value,
    requirements: &TaskRequirements,
    context: &TaskContext,
) -> Result<(), DadrError> {
    validate_required_inputs(task, input)?;
    if task.manifest.id == "code-to-record"
        && task.source.kind == crate::tasks::TaskSourceKind::Builtin
    {
        if let Some(limit) = input.get("limit") {
            if limit.as_u64().is_none_or(|n| n == 0) {
                return Err(DadrError::InvalidInput(
                    "limit must be a positive integer".to_owned(),
                ));
            }
        }
    }
    if matches!(requirements.paths, TaskPathRequirement::Required)
        && context.paths.iter().all(|p| p.trim().is_empty())
    {
        return Err(DadrError::InvalidInput(
            "this task requires at least one context file or directory".to_owned(),
        ));
    }
    if matches!(requirements.notes, TaskNoteRequirement::Required)
        && context
            .instructions
            .as_deref()
            .or(context.text.as_deref())
            .is_none_or(|notes| notes.trim().is_empty())
        && !task_has_note_input(task, input)
    {
        return Err(DadrError::InvalidInput(
            "this task requires notes or instructions".to_owned(),
        ));
    }
    validate_record_requirements(project, requirements, input, context)?;
    Ok(())
}

fn validate_required_inputs(task: &TaskPackage, input: &Value) -> Result<(), DadrError> {
    for field in &task.manifest.inputs {
        let Some(value) = input.get(&field.name) else {
            if field.required {
                return Err(DadrError::InvalidInput(format!(
                    "task requires required input `{}`",
                    field.name
                )));
            }
            continue;
        };
        if field.required && input_value_is_empty(value, field.field_type) {
            return Err(DadrError::InvalidInput(format!(
                "task requires required input `{}`",
                field.name
            )));
        }
        validate_input_options(task, &field.name, field.field_type, &field.options, value)?;
    }
    Ok(())
}

fn validate_input_options(
    task: &TaskPackage,
    name: &str,
    field_type: TaskInputType,
    options: &[String],
    value: &Value,
) -> Result<(), DadrError> {
    if !matches!(
        field_type,
        TaskInputType::Select | TaskInputType::Multiselect
    ) {
        return Ok(());
    }
    let values = input_string_values(value);
    for value in values {
        if !options.iter().any(|option| option == &value) {
            return Err(DadrError::InvalidInput(format!(
                "invalid value `{value}` for task `{}` input `{name}`",
                task.manifest.id
            )));
        }
    }
    Ok(())
}

fn input_string_values(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => vec![value.trim().to_owned()],
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn input_value_is_empty(value: &Value, field_type: TaskInputType) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        Value::Array(values) => {
            values.is_empty()
                || values
                    .iter()
                    .all(|value| value.as_str().is_none_or(|s| s.trim().is_empty()))
        }
        Value::Bool(value) if matches!(field_type, TaskInputType::Checkbox) => !*value,
        _ => false,
    }
}

fn task_has_note_input(task: &TaskPackage, input: &Value) -> bool {
    task.manifest.inputs.iter().any(|field| {
        matches!(
            field.field_type,
            TaskInputType::Text | TaskInputType::Textarea
        ) && input
            .get(&field.name)
            .is_some_and(|value| !input_value_is_empty(value, field.field_type))
    })
}

fn validate_record_requirements(
    project: &ProjectContext,
    requirements: &TaskRequirements,
    input: &Value,
    context: &TaskContext,
) -> Result<(), DadrError> {
    let record_queries = task_record_queries(input, context);
    match requirements.records {
        TaskRecordRequirement::None => return Ok(()),
        TaskRecordRequirement::One if record_queries.len() != 1 => {
            return Err(DadrError::InvalidInput(
                "this task requires exactly one record".to_owned(),
            ));
        }
        TaskRecordRequirement::Many if record_queries.len() < 2 => {
            return Err(DadrError::InvalidInput(
                "this task requires at least two records".to_owned(),
            ));
        }
        TaskRecordRequirement::One | TaskRecordRequirement::Many => {}
    }

    if matches!(
        requirements.record_status,
        TaskRecordStatusRequirement::Candidate | TaskRecordStatusRequirement::Accepted
    ) {
        let index = ProjectIndex::scan(project)?;
        let required_status = match requirements.record_status {
            TaskRecordStatusRequirement::Candidate => RecordStatus::Candidate,
            TaskRecordStatusRequirement::Accepted => RecordStatus::Accepted,
            TaskRecordStatusRequirement::All => return Ok(()),
        };
        for query in record_queries {
            let found = index.resolve_one(&query)?;
            if found.record.status != required_status {
                return Err(DadrError::InvalidInput(format!(
                    "this task requires {} record{}",
                    required_status,
                    if matches!(requirements.records, TaskRecordRequirement::One) {
                        ""
                    } else {
                        "s"
                    }
                )));
            }
        }
    }
    Ok(())
}

fn task_record_queries(input: &Value, context: &TaskContext) -> Vec<String> {
    let mut queries: Vec<String> = context
        .records
        .iter()
        .map(|record| record.trim())
        .filter(|record| !record.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if !queries.is_empty() {
        return queries;
    }

    for name in ["record_id", "record_ids"] {
        if let Some(value) = input.get(name) {
            collect_record_queries_from_value(value, &mut queries);
        }
    }
    queries
}

fn collect_record_queries_from_value(value: &Value, queries: &mut Vec<String>) {
    match value {
        Value::String(value) => {
            queries.extend(
                value
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
        Value::Array(values) => {
            for value in values {
                collect_record_queries_from_value(value, queries);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRunCommandRequest {
    pub task_id: String,
    pub input: Value,
    #[serde(default)]
    pub context: TaskContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_agent: Option<LocalAcpAgentCommand>,
    pub permission_policy: AcpPermissionPolicy,
    #[serde(default = "default_enable_dadrock_mcp")]
    pub enable_dadrock_mcp: bool,
}

pub fn run_task_with_acp(
    project: &ProjectContext,
    request: TaskRunCommandRequest,
) -> Result<PersistedTaskRun, DadrError> {
    let TaskRunCommandRequest {
        task_id,
        input,
        context,
        local_agent,
        permission_policy,
        enable_dadrock_mcp,
    } = request;
    let (task, run) = prepare_task_run(project, &task_id, input.clone(), &context, None)?;
    match task.manifest.execution {
        TaskExecutionMode::Agent => {
            let local_agent = local_agent
                .map(Ok)
                .unwrap_or_else(configured_local_acp_agent)?;
            run_created_task_with_acp(
                project,
                &task,
                input,
                context,
                &run,
                AcpTaskRunOptions {
                    local_agent: &local_agent,
                    permission_policy,
                    enable_dadrock_mcp,
                    events: None,
                },
            )
        }
        TaskExecutionMode::Local => run_created_local_task(project, &task, input, context, &run),
    }
}

struct TaskRunFailure {
    error: Box<DadrError>,
    raw_output: Option<Value>,
}

impl From<DadrError> for TaskRunFailure {
    fn from(error: DadrError) -> Self {
        Self {
            error: Box::new(error),
            raw_output: None,
        }
    }
}

#[derive(Clone)]
pub struct AcpTaskRunOptions<'a> {
    pub local_agent: &'a LocalAcpAgentCommand,
    pub permission_policy: AcpPermissionPolicy,
    pub enable_dadrock_mcp: bool,
    pub events: Option<AcpPromptEventSink>,
}

pub fn create_task_run_for_acp(
    project: &ProjectContext,
    task_id: &str,
    input: Value,
) -> Result<(TaskPackage, PersistedTaskRun), DadrError> {
    prepare_task_run(
        project,
        task_id,
        input,
        &TaskContext::default(),
        Some(TaskExecutionMode::Agent),
    )
}

pub fn create_task_run_for_task(
    project: &ProjectContext,
    task_id: &str,
    input: Value,
    context: &TaskContext,
) -> Result<(TaskPackage, PersistedTaskRun), DadrError> {
    prepare_task_run(project, task_id, input, context, None)
}

fn prepare_task_run(
    project: &ProjectContext,
    task_id: &str,
    input: Value,
    context: &TaskContext,
    required_execution: Option<TaskExecutionMode>,
) -> Result<(TaskPackage, PersistedTaskRun), DadrError> {
    let task = resolve_task(project, task_id)?;
    if let Some(required_execution) = required_execution {
        if task.manifest.execution != required_execution {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` is not an {} task",
                task.manifest.id,
                task_execution_label(required_execution)
            )));
        }
    }
    if task.manifest.execution == TaskExecutionMode::Agent {
        project.require_initialized()?;
    }
    validate_task_context_requirements(
        project,
        &task,
        &input,
        &task.manifest.requirements,
        context,
    )?;
    let run = create_task_run(&project.cwd, &task, input)?;
    Ok((task, run))
}

fn task_execution_label(execution: TaskExecutionMode) -> &'static str {
    match execution {
        TaskExecutionMode::Agent => "ACP",
        TaskExecutionMode::Local => "local",
    }
}

pub fn run_created_task_with_acp(
    project: &ProjectContext,
    task: &TaskPackage,
    input: Value,
    context: TaskContext,
    run: &PersistedTaskRun,
    options: AcpTaskRunOptions<'_>,
) -> Result<PersistedTaskRun, DadrError> {
    write_task_context(&run.run_dir, &context)?;
    let options = AcpTaskRunOptions {
        events: options
            .events
            .or_else(|| Some(task_run_log_sink(run.run_dir.clone()))),
        ..options
    };
    match execute_created_task_run(CreatedAcpTaskRun {
        project,
        task,
        input,
        context,
        run,
        options,
    }) {
        Ok(run) => Ok(run),
        Err(failure) => {
            let TaskRunFailure { error, raw_output } = failure;
            if let Ok(current_run) = read_task_run(&run.run_dir) {
                if current_run.status == TaskRunStatus::Cancelled {
                    let _ = append_task_run_log(&run.run_dir, "Task cancelled.");
                    return Ok(current_run);
                }
            }
            let message = error.to_string();
            let _ = append_task_run_log(&run.run_dir, format!("Task failed: {message}"));
            let _ = write_task_failure(&run.run_dir, message, raw_output);
            Err(*error)
        }
    }
}

pub fn run_created_local_task(
    project: &ProjectContext,
    task: &TaskPackage,
    input: Value,
    context: TaskContext,
    run: &PersistedTaskRun,
) -> Result<PersistedTaskRun, DadrError> {
    write_task_context(&run.run_dir, &context)?;
    match execute_created_local_task_run(project, task, input, run) {
        Ok(run) => Ok(run),
        Err(error) => {
            let message = error.to_string();
            let _ = append_task_run_log(&run.run_dir, format!("Task failed: {message}"));
            let _ = write_task_failure(&run.run_dir, message, None);
            Err(error)
        }
    }
}

fn execute_created_local_task_run(
    _project: &ProjectContext,
    task: &TaskPackage,
    _input: Value,
    run: &PersistedTaskRun,
) -> Result<PersistedTaskRun, DadrError> {
    mark_task_run_running(&run.run_dir)?;
    append_task_run_log(
        &run.run_dir,
        format!("Running local task `{}`.", task.manifest.id),
    )?;
    Err(DadrError::InvalidInput(format!(
        "local task `{}` is not implemented",
        task.manifest.id
    )))
}

struct CreatedAcpTaskRun<'a> {
    project: &'a ProjectContext,
    task: &'a TaskPackage,
    input: Value,
    context: TaskContext,
    run: &'a PersistedTaskRun,
    options: AcpTaskRunOptions<'a>,
}

fn execute_created_task_run(
    request: CreatedAcpTaskRun<'_>,
) -> Result<PersistedTaskRun, TaskRunFailure> {
    let CreatedAcpTaskRun {
        project,
        task,
        input,
        context,
        run,
        options,
    } = request;
    let AcpTaskRunOptions {
        local_agent,
        permission_policy,
        enable_dadrock_mcp,
        events,
    } = options;
    mark_task_run_running(&run.run_dir).map_err(TaskRunFailure::from)?;
    append_task_run_log(
        &run.run_dir,
        format!("Running task `{}`.", task.manifest.id),
    )
    .map_err(TaskRunFailure::from)?;
    let cancellation = AcpCancellationToken::default();
    let _cancellation_registration =
        TaskCancellationRegistration::register(run.id, cancellation.clone())
            .map_err(TaskRunFailure::from)?;
    preflight_dadr_skill_access(
        project,
        task,
        local_agent,
        permission_policy,
        enable_dadrock_mcp,
        run,
        events.clone(),
    )
    .map_err(TaskRunFailure::from)?;
    let current_run = read_task_run(&run.run_dir).map_err(TaskRunFailure::from)?;
    if current_run.status == TaskRunStatus::Cancelled {
        return Ok(current_run);
    }
    let index = ProjectIndex::scan(project)?;
    let records = index.list_records(None);
    let task_prompt = context
        .system_prompt
        .as_deref()
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or(&task.prompt)
        .to_owned();
    let agent_request = AgentRequestEnvelope::run_task(
        task_prompt.clone(),
        AgentProjectSnapshot::new(project, records),
        agent_task_request(task, input, context, run, task_prompt),
    )?;
    let text = run_acp_agent_text_with_events_and_cancellation(
        local_agent,
        &agent_request,
        AcpPromptOptions {
            permission_policy,
            enable_dadrock_mcp,
            dadrock_mcp_access: task_mcp_access(task),
            ..AcpPromptOptions::default()
        },
        events.clone(),
        Some(cancellation.clone()),
    )?;
    let current_run = read_task_run(&run.run_dir).map_err(TaskRunFailure::from)?;
    if current_run.status == TaskRunStatus::Cancelled {
        return Ok(current_run);
    }
    if current_run.status == TaskRunStatus::Succeeded {
        return Ok(current_run);
    }
    let output = parse_acp_agent_json_response(&text).map_err(TaskRunFailure::from)?;
    let raw_output = output.clone();
    let validated_output = match validate_task_output(&task.manifest.output.kind, output) {
        Ok(output) => output,
        Err(error) => {
            append_task_run_log(
                &run.run_dir,
                format!("Task output failed validation; requesting corrected JSON: {error}"),
            )
            .map_err(TaskRunFailure::from)?;
            let repair_prompt = task_output_repair_prompt(
                &task.manifest.output.kind,
                &error.to_string(),
                &raw_output,
            )
            .map_err(TaskRunFailure::from)?;
            let mut repair_request = agent_request;
            repair_request.prompt = repair_prompt;
            let repaired_output = run_acp_agent_json_with_events_and_cancellation(
                local_agent,
                &repair_request,
                AcpPromptOptions {
                    permission_policy,
                    enable_dadrock_mcp,
                    dadrock_mcp_access: task_mcp_access(task),
                    ..AcpPromptOptions::default()
                },
                events,
                Some(cancellation),
            )
            .map_err(TaskRunFailure::from)?;
            let current_run = read_task_run(&run.run_dir).map_err(TaskRunFailure::from)?;
            if current_run.status == TaskRunStatus::Cancelled {
                return Ok(current_run);
            }
            let raw_repaired_output = repaired_output.clone();
            validate_task_output(&task.manifest.output.kind, repaired_output).map_err(|error| {
                TaskRunFailure {
                    error: Box::new(error),
                    raw_output: Some(raw_repaired_output),
                }
            })?
        }
    };
    let run = write_task_result(&run.run_dir, validated_output).map_err(TaskRunFailure::from)?;
    append_task_run_log(&run.run_dir, "Task succeeded.").map_err(TaskRunFailure::from)?;
    Ok(run)
}

fn preflight_dadr_skill_access(
    project: &ProjectContext,
    task: &TaskPackage,
    local_agent: &LocalAcpAgentCommand,
    permission_policy: AcpPermissionPolicy,
    enable_dadrock_mcp: bool,
    run: &PersistedTaskRun,
    events: Option<AcpPromptEventSink>,
) -> Result<(), DadrError> {
    append_task_run_log(
        &run.run_dir,
        "Checking local ACP agent access to installed `dadr` skill.",
    )?;
    let output = run_acp_prompt_json_with_events(
        local_agent,
        project.cwd.clone().into_std_path_buf(),
        dadr_skill_preflight_prompt(&task.manifest.id),
        AcpPromptOptions {
            permission_policy,
            enable_dadrock_mcp,
            dadrock_mcp_access: task_mcp_access(task),
            ..AcpPromptOptions::default()
        },
        events,
    )?;
    validate_dadr_skill_preflight_output(&output, &task.manifest.id)?;
    let evidence = output
        .get("evidence")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("agent confirmed access");
    append_task_run_log(
        &run.run_dir,
        format!("Local ACP agent confirmed installed `dadr` skill access: {evidence}"),
    )?;
    Ok(())
}

fn dadr_skill_preflight_prompt(task_id: &str) -> String {
    format!(
        r#"dadrock task skill preflight.

Load your installed skill named `dadr`. Confirm that you can access the DADR task guide for task id `{task_id}`.

Return exactly one JSON object and no markdown fence or prose outside it:
{{
  "ok": true,
  "skill_name": "dadr",
  "task_guide": "tasks/{task_id}.md",
  "evidence": "short evidence from the loaded skill, such as the guide heading or routing entry",
  "message": "ready"
}}

If you cannot access an installed `dadr` skill, return:
{{
  "ok": false,
  "skill_name": null,
  "task_guide": null,
  "evidence": "",
  "message": "dadr skill is not installed or not available to this ACP agent"
}}
"#
    )
}

fn validate_dadr_skill_preflight_output(output: &Value, task_id: &str) -> Result<(), DadrError> {
    match output.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            let evidence = output
                .get("evidence")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if evidence.is_empty() {
                return Err(DadrError::Acp(format!(
                    "local ACP agent claimed access to the installed `dadr` skill for task `{task_id}` but returned no evidence"
                )));
            }
            Ok(())
        }
        Some(false) => {
            let message = output
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("dadr skill is not installed or not available to this ACP agent");
            Err(DadrError::Acp(format!(
                "local ACP agent cannot access the installed `dadr` skill required for task `{task_id}`: {message}"
            )))
        }
        None => Err(DadrError::Acp(format!(
            "local ACP agent did not return a valid `dadr` skill preflight result for task `{task_id}`"
        ))),
    }
}

fn resolve_task(project: &ProjectContext, task_id: &str) -> Result<TaskPackage, DadrError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(DadrError::InvalidInput(
            "task id cannot be empty".to_owned(),
        ));
    }

    let user_tasks_dir = default_user_tasks_dir();
    discover_tasks(&project.cwd, user_tasks_dir.as_deref())
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == task_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task `{task_id}` not found")))
}

fn agent_task_request(
    task: &TaskPackage,
    input: Value,
    context: TaskContext,
    run: &PersistedTaskRun,
    prompt: String,
) -> AgentTaskRequest {
    AgentTaskRequest {
        id: task.manifest.id.clone(),
        run_id: run.id.to_string(),
        title: task.manifest.title.clone(),
        description: task.manifest.description.clone(),
        output_kind: task.manifest.output.kind.clone(),
        input,
        context,
        prompt,
    }
}

fn default_enable_dadrock_mcp() -> bool {
    true
}

fn write_task_context(run_dir: &Utf8Path, context: &TaskContext) -> Result<(), DadrError> {
    if context.is_empty() {
        return Ok(());
    }
    let content = serde_json::to_string_pretty(context)?;
    std::fs::write(run_dir.join("context.json").as_std_path(), content)?;
    Ok(())
}

fn task_run_log_sink(run_dir: camino::Utf8PathBuf) -> AcpPromptEventSink {
    Arc::new(move |event| match event {
        AcpPromptEvent::AgentMessageChunk { text } => {
            let _ = append_or_extend_task_run_log(&run_dir, "agent: ", &text);
        }
        AcpPromptEvent::Status { message } => {
            if !message.trim().is_empty() {
                let _ = append_task_run_log(&run_dir, message);
            }
        }
        AcpPromptEvent::Permission { message } => {
            let message = format!("permission: {message}");
            if !message.trim().is_empty() {
                let _ = append_task_run_log(&run_dir, message);
            }
        }
    })
}

fn task_mcp_access(task: &TaskPackage) -> crate::mcp::DadrockMcpAccess {
    if task.manifest.id == "context-to-record" {
        crate::mcp::DadrockMcpAccess::ReadOnly
    } else {
        AcpPromptOptions::default().dadrock_mcp_access
    }
}
