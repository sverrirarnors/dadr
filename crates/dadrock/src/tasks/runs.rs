use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    sync::Mutex,
};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    discovery::{TaskPackage, TaskSource},
    ids::validate_task_id,
};
use crate::{file_store, DadrError, RecordId};

const RUNS_GITIGNORE_CONTENT: &str = "/**\n!.gitignore\n";
const LOG_JSONL: &str = "log.jsonl";
const LEGACY_LOG_JSON: &str = "log.json";
static RUN_STATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedTaskRun {
    pub id: RecordId,
    pub task_id: String,
    pub task_source: TaskSource,
    pub task_version: u32,
    pub status: TaskRunStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub run_dir: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunLogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStatus {
    Created,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Serialize)]
struct TaskRunErrorMetadata {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_output_path: Option<&'static str>,
}

pub fn task_runs_dir(project_cwd: &Utf8Path) -> Utf8PathBuf {
    project_cwd.join("dadr/runs")
}

pub fn resolve_task_run_dir(
    project_cwd: &Utf8Path,
    run_id: &str,
) -> Result<Utf8PathBuf, DadrError> {
    validate_task_run_id_query(run_id)?;
    list_task_runs(project_cwd)?
        .into_iter()
        .find(|run| run.id.to_string() == run_id)
        .map(|run| run.run_dir)
        .ok_or_else(|| DadrError::InvalidInput(format!("task run `{run_id}` not found")))
}

fn validate_task_run_id_query(run_id: &str) -> Result<(), DadrError> {
    if run_id.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "task run id cannot be empty".to_owned(),
        ));
    }
    if run_id.trim() != run_id
        || run_id.contains('/')
        || run_id.contains('\\')
        || run_id.contains("..")
        || Utf8Path::new(run_id).is_absolute()
    {
        return Err(DadrError::InvalidInput(format!(
            "task run id `{run_id}` must be an exact run id, not a path"
        )));
    }
    Ok(())
}

pub fn create_task_run(
    project_cwd: &Utf8Path,
    task: &TaskPackage,
    input: Value,
) -> Result<PersistedTaskRun, DadrError> {
    let task_id = task.manifest.id.as_str();
    validate_task_id(task_id)?;
    ensure_runs_gitignore(project_cwd)?;

    let id = RecordId::new_v7();
    let run_dir = task_runs_dir(project_cwd)
        .join(task_id)
        .join(id.to_string());
    fs::create_dir_all(run_dir.join("artifacts").as_std_path())?;

    let now = Utc::now();
    let run = PersistedTaskRun {
        id,
        task_id: task_id.to_owned(),
        task_source: task.source.clone(),
        task_version: task.manifest.version,
        status: TaskRunStatus::Created,
        created_at: now,
        updated_at: now,
        run_dir,
    };

    write_json_pretty_new(&run.run_dir.join("run.json"), &run)?;
    write_json_pretty_new(&run.run_dir.join("input.json"), &input)?;

    Ok(run)
}

pub fn read_task_run(run_dir: &Utf8Path) -> Result<PersistedTaskRun, DadrError> {
    let content = fs::read_to_string(run_dir.join("run.json").as_std_path())?;
    serde_json::from_str(&content).map_err(DadrError::from)
}

pub fn mark_task_run_running(run_dir: &Utf8Path) -> Result<PersistedTaskRun, DadrError> {
    let _guard = run_state_guard()?;
    let mut run = read_task_run(run_dir)?;
    ensure_can_enter_non_terminal(&run, "run")?;
    run.status = TaskRunStatus::Running;
    run.updated_at = Utc::now();
    write_json_pretty_replace(&run_dir.join("run.json"), &run)?;
    Ok(run)
}

pub fn mark_task_run_cancelled(run_dir: &Utf8Path) -> Result<PersistedTaskRun, DadrError> {
    let _guard = run_state_guard()?;
    let mut run = read_task_run(run_dir)?;
    match run.status {
        TaskRunStatus::Created | TaskRunStatus::Running => {
            run.status = TaskRunStatus::Cancelled;
            run.updated_at = Utc::now();
            write_json_pretty_replace(&run_dir.join("run.json"), &run)?;
        }
        TaskRunStatus::Cancelled => {}
        TaskRunStatus::Succeeded | TaskRunStatus::Failed => {
            return Err(DadrError::InvalidInput(format!(
                "task run `{}` cannot be cancelled because status is {}",
                run.id,
                run.status.as_str()
            )));
        }
    }
    Ok(run)
}

pub fn list_task_runs(project_cwd: &Utf8Path) -> Result<Vec<PersistedTaskRun>, DadrError> {
    let runs_dir = task_runs_dir(project_cwd);
    let task_entries = match fs::read_dir(runs_dir.as_std_path()) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(read_task_runs_dir_error(&runs_dir, err)),
    };

    let mut runs = Vec::new();
    for task_entry in task_entries {
        let task_entry = task_entry.map_err(|err| read_task_runs_dir_error(&runs_dir, err))?;
        let file_type = task_entry
            .file_type()
            .map_err(|err| inspect_task_run_path_error(&task_entry.path(), err))?;
        if !file_type.is_dir() {
            continue;
        }
        let task_dir = path_to_utf8(task_entry.path())?;
        let run_entries = fs::read_dir(task_dir.as_std_path())
            .map_err(|err| read_task_runs_dir_error(&task_dir, err))?;

        for run_entry in run_entries {
            let run_entry = run_entry.map_err(|err| read_task_runs_dir_error(&task_dir, err))?;
            let file_type = run_entry
                .file_type()
                .map_err(|err| inspect_task_run_path_error(&run_entry.path(), err))?;
            if !file_type.is_dir() {
                continue;
            }
            let run_dir = path_to_utf8(run_entry.path())?;
            let run = match read_task_run(&run_dir) {
                Ok(run) => run,
                Err(DadrError::Io(err)) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(task_run_metadata_error(&run_dir.join("run.json"), err));
                }
            };
            runs.push(run);
        }
    }

    runs.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.task_id.cmp(&right.task_id))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(runs)
}

pub fn clear_task_runs(project_cwd: &Utf8Path) -> Result<Option<Utf8PathBuf>, DadrError> {
    let runs_dir = task_runs_dir(project_cwd);
    match fs::remove_dir_all(runs_dir.as_std_path()) {
        Ok(()) => Ok(Some(runs_dir)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(DadrError::InvalidInput(format!(
            "failed to clear task runs directory `{runs_dir}`: {err}"
        ))),
    }
}

pub fn read_task_result(run_dir: &Utf8Path) -> Result<Value, DadrError> {
    let run = read_task_run(run_dir)?;
    let result_path = run_dir.join("result.json");
    let content = match fs::read_to_string(result_path.as_std_path()) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(DadrError::InvalidInput(format!(
                "task run `{}` has no result.json; status is {}",
                run.id,
                run.status.as_str()
            )));
        }
        Err(err) => return Err(err.into()),
    };
    serde_json::from_str(&content).map_err(DadrError::from)
}

pub fn read_task_run_log(run_dir: &Utf8Path) -> Result<Vec<TaskRunLogEntry>, DadrError> {
    let log_path = run_dir.join(LOG_JSONL);
    if log_path.exists() {
        let file = fs::File::open(log_path.as_std_path())?;
        let reader = io::BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            entries.push(serde_json::from_str(&line)?);
        }
        return Ok(merge_adjacent_log_chunks(entries));
    }

    let log_path = run_dir.join(LEGACY_LOG_JSON);
    let content = match fs::read_to_string(log_path.as_std_path()) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    serde_json::from_str(&content).map_err(DadrError::from)
}

pub fn append_task_run_log(
    run_dir: &Utf8Path,
    message: impl Into<String>,
) -> Result<(), DadrError> {
    append_task_run_log_entry(
        run_dir,
        &TaskRunLogEntry {
            timestamp: Utc::now(),
            message: message.into(),
        },
    )
}

pub fn append_or_extend_task_run_log(
    run_dir: &Utf8Path,
    prefix: &str,
    message: &str,
) -> Result<(), DadrError> {
    if message.trim().is_empty() {
        return Ok(());
    }

    append_task_run_log_entry(
        run_dir,
        &TaskRunLogEntry {
            timestamp: Utc::now(),
            message: format!("{prefix}{message}"),
        },
    )
}

pub fn write_task_result(run_dir: &Utf8Path, result: Value) -> Result<PersistedTaskRun, DadrError> {
    let _guard = run_state_guard()?;
    let mut run = read_task_run(run_dir)?;
    ensure_can_finish(&run)?;
    let result_path = run_dir.join("result.json");
    write_json_pretty_new(&result_path, &result)?;

    run.status = TaskRunStatus::Succeeded;
    run.updated_at = Utc::now();
    if let Err(err) = write_json_pretty_replace(&run_dir.join("run.json"), &run) {
        let _ = file_store::remove_file(&result_path);
        return Err(err);
    }

    Ok(run)
}

pub fn write_task_failure(
    run_dir: &Utf8Path,
    message: impl Into<String>,
    raw_output: Option<Value>,
) -> Result<PersistedTaskRun, DadrError> {
    let _guard = run_state_guard()?;
    let mut run = read_task_run(run_dir)?;
    ensure_can_finish(&run)?;
    let raw_output_path = raw_output.as_ref().map(|_| "artifacts/raw-output.json");
    let error = TaskRunErrorMetadata {
        message: message.into(),
        raw_output_path,
    };

    if let Some(raw_output) = raw_output {
        write_json_pretty_replace(&run_dir.join("artifacts/raw-output.json"), &raw_output)?;
    }
    write_json_pretty_replace(&run_dir.join("error.json"), &error)?;

    run.status = TaskRunStatus::Failed;
    run.updated_at = Utc::now();
    write_json_pretty_replace(&run_dir.join("run.json"), &run)?;

    Ok(run)
}

fn run_state_guard() -> Result<std::sync::MutexGuard<'static, ()>, DadrError> {
    RUN_STATE_LOCK
        .lock()
        .map_err(|_| DadrError::InvalidInput("task run state lock is poisoned".to_owned()))
}

fn ensure_can_enter_non_terminal(run: &PersistedTaskRun, verb: &str) -> Result<(), DadrError> {
    match run.status {
        TaskRunStatus::Created | TaskRunStatus::Running => Ok(()),
        TaskRunStatus::Cancelled => Err(DadrError::InvalidInput(format!(
            "task run `{}` was cancelled",
            run.id
        ))),
        TaskRunStatus::Succeeded => Err(DadrError::InvalidInput(format!(
            "task run `{}` already succeeded and cannot {verb}",
            run.id
        ))),
        TaskRunStatus::Failed => Err(DadrError::InvalidInput(format!(
            "task run `{}` already failed and cannot {verb}",
            run.id
        ))),
    }
}

fn ensure_can_finish(run: &PersistedTaskRun) -> Result<(), DadrError> {
    match run.status {
        TaskRunStatus::Created | TaskRunStatus::Running => Ok(()),
        TaskRunStatus::Cancelled => Err(DadrError::InvalidInput(format!(
            "task run `{}` was cancelled",
            run.id
        ))),
        TaskRunStatus::Succeeded => Err(DadrError::InvalidInput(format!(
            "task run `{}` already succeeded",
            run.id
        ))),
        TaskRunStatus::Failed => Err(DadrError::InvalidInput(format!(
            "task run `{}` already failed",
            run.id
        ))),
    }
}

fn append_task_run_log_entry(run_dir: &Utf8Path, entry: &TaskRunLogEntry) -> Result<(), DadrError> {
    fs::create_dir_all(run_dir.as_std_path())?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(run_dir.join(LOG_JSONL).as_std_path())?;
    serde_json::to_writer(&mut file, entry)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    file_store::sync_parent_dir(&run_dir.join(LOG_JSONL))?;
    Ok(())
}

fn merge_adjacent_log_chunks(entries: Vec<TaskRunLogEntry>) -> Vec<TaskRunLogEntry> {
    let mut merged: Vec<TaskRunLogEntry> = Vec::new();
    for entry in entries {
        if let Some(last) = merged.last_mut() {
            if let Some(prefix) = shared_log_chunk_prefix(&last.message, &entry.message) {
                last.message.push_str(&entry.message[prefix.len()..]);
                continue;
            }
        }
        merged.push(entry);
    }
    merged
}

fn shared_log_chunk_prefix<'a>(left: &'a str, right: &str) -> Option<&'a str> {
    let prefix_end = left.find(": ")? + 2;
    let prefix = &left[..prefix_end];
    right.starts_with(prefix).then_some(prefix)
}

fn path_to_utf8(path: std::path::PathBuf) -> Result<Utf8PathBuf, DadrError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| DadrError::PathNotUtf8 {
        path: path.display().to_string(),
    })
}

fn read_task_runs_dir_error(path: &Utf8Path, err: io::Error) -> DadrError {
    DadrError::InvalidInput(format!(
        "failed to read task runs directory `{path}`: {err}"
    ))
}

fn inspect_task_run_path_error(path: &Path, err: io::Error) -> DadrError {
    DadrError::InvalidInput(format!(
        "failed to inspect task run path `{}`: {err}",
        path.display()
    ))
}

fn task_run_metadata_error(path: &Utf8Path, err: DadrError) -> DadrError {
    DadrError::InvalidInput(format!("failed to read task run metadata `{path}`: {err}"))
}

fn ensure_runs_gitignore(project_cwd: &Utf8Path) -> Result<(), DadrError> {
    let runs_dir = task_runs_dir(project_cwd);
    fs::create_dir_all(runs_dir.as_std_path())?;

    let gitignore_path = runs_dir.join(".gitignore");
    match fs::read_to_string(gitignore_path.as_std_path()) {
        Ok(content) if content == RUNS_GITIGNORE_CONTENT => Ok(()),
        Ok(_) => file_store::write_replace(&gitignore_path, RUNS_GITIGNORE_CONTENT),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            file_store::write_new(&gitignore_path, RUNS_GITIGNORE_CONTENT)
        }
        Err(err) => Err(err.into()),
    }
}

fn write_json_pretty_new(path: &Utf8Path, value: &impl Serialize) -> Result<(), DadrError> {
    let mut content = serde_json::to_string_pretty(value)?;
    content.push('\n');
    file_store::write_new(path, &content)
}

fn write_json_pretty_replace(path: &Utf8Path, value: &impl Serialize) -> Result<(), DadrError> {
    let mut content = serde_json::to_string_pretty(value)?;
    content.push('\n');
    file_store::write_replace(path, &content)
}
