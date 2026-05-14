use camino::Utf8PathBuf;
use serde_json::Value;

use crate::{
    error::DadrError,
    project::ProjectContext,
    tasks::{
        append_task_run_log, default_user_tasks_dir, discover_tasks, list_task_runs,
        mark_task_run_cancelled, read_task_result, read_task_run, request_task_run_cancellation,
        resolve_task_run_dir as resolve_project_task_run_dir, run_task_with_acp,
        validate_task_output, write_task_result, PersistedTaskRun, TaskDiagnostic,
        TaskDiagnosticCode, TaskListing, TaskPackage, TaskRunCommandRequest, TaskRunStatus,
    },
};

use super::{CommandContext, CommandEnvelope};

pub fn list_tasks_command(context: &CommandContext) -> CommandEnvelope<TaskListing> {
    let user_tasks_dir = default_user_tasks_dir();
    let (project_root, mut diagnostics) = task_command_project_root(context);
    let mut listing = discover_tasks(&project_root, user_tasks_dir.as_deref());
    diagnostics.append(&mut listing.diagnostics);
    listing.diagnostics = diagnostics;
    CommandEnvelope::new("task list", listing)
}

pub fn show_task_command(
    context: &CommandContext,
    task_id: &str,
) -> Result<CommandEnvelope<TaskPackage>, DadrError> {
    let task = resolve_task_package(context, task_id)?;
    Ok(CommandEnvelope::new("task show", task))
}

pub fn run_task_command(
    context: &CommandContext,
    request: TaskRunCommandRequest,
) -> Result<CommandEnvelope<PersistedTaskRun>, DadrError> {
    let mut project = context.discover_project()?;
    project.cwd = task_project_root(&project);
    let run = run_task_with_acp(&project, request)?;
    let changed_paths = vec![run.run_dir.clone()];
    Ok(CommandEnvelope::new("task run", run).with_changed_paths(changed_paths))
}

pub fn list_task_runs_command(
    context: &CommandContext,
) -> Result<CommandEnvelope<Vec<PersistedTaskRun>>, DadrError> {
    let project = context.discover_project()?;
    let project_root = task_project_root(&project);
    Ok(CommandEnvelope::new(
        "task runs",
        list_task_runs(&project_root)?,
    ))
}

pub fn show_task_result_command(
    context: &CommandContext,
    run_id: &str,
) -> Result<CommandEnvelope<Value>, DadrError> {
    let run_dir = resolve_task_run_dir(context, run_id)?;
    let result = read_task_result(&run_dir)?;
    Ok(CommandEnvelope::new("task result", result))
}

pub fn cancel_task_run_command(
    context: &CommandContext,
    run_id: &str,
) -> Result<CommandEnvelope<PersistedTaskRun>, DadrError> {
    let run_dir = resolve_task_run_dir(context, run_id)?;
    let before = read_task_run(&run_dir)?;
    let cancellation_signal_sent = request_task_run_cancellation(&before.id);
    let run = mark_task_run_cancelled(&run_dir)?;
    if before.status != TaskRunStatus::Cancelled {
        if cancellation_signal_sent {
            append_task_run_log(&run_dir, "Cancellation requested.")?;
        } else {
            append_task_run_log(&run_dir, "Task cancelled.")?;
        }
    }
    Ok(CommandEnvelope::new("task cancel", run).with_changed_paths(vec![run_dir]))
}

pub fn submit_task_result_command(
    context: &CommandContext,
    run_id: &str,
    result: Value,
) -> Result<CommandEnvelope<PersistedTaskRun>, DadrError> {
    let run_dir = resolve_task_run_dir(context, run_id)?;
    let run = read_task_run(&run_dir)?;
    let task = resolve_task_package(context, &run.task_id)?;
    let result = validate_task_output(&task.manifest.output.kind, result)?;
    let updated = write_task_result(&run_dir, result)?;
    append_task_run_log(&run_dir, "Task result submitted through Dadrock MCP.")?;
    Ok(CommandEnvelope::new("task submit-result", updated).with_changed_paths(vec![run_dir]))
}

fn resolve_task_package(context: &CommandContext, task_id: &str) -> Result<TaskPackage, DadrError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(DadrError::InvalidInput(
            "task id cannot be empty".to_owned(),
        ));
    }

    let user_tasks_dir = default_user_tasks_dir();
    let project = context.discover_project()?;
    let project_root = task_project_root(&project);
    discover_tasks(&project_root, user_tasks_dir.as_deref())
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == task_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task `{task_id}` not found")))
}

fn task_command_project_root(context: &CommandContext) -> (Utf8PathBuf, Vec<TaskDiagnostic>) {
    match context.discover_project() {
        Ok(project) => (task_project_root(&project), Vec::new()),
        Err(err) => (
            context.cwd.clone(),
            vec![TaskDiagnostic::new(
                TaskDiagnosticCode::ProjectDiscoveryFailed,
                format!("failed to discover Dadrock project: {err}"),
            )
            .with_path(context.cwd.clone())],
        ),
    }
}

fn resolve_task_run_dir(context: &CommandContext, run_id: &str) -> Result<Utf8PathBuf, DadrError> {
    let project = context.discover_project()?;
    let project_root = task_project_root(&project);
    resolve_project_task_run_dir(&project_root, run_id)
}

fn task_project_root(project: &ProjectContext) -> Utf8PathBuf {
    project.workspace_dir()
}
