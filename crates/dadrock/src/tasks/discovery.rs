use std::{collections::BTreeMap, env, fs, io, path::PathBuf};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::{builtin, parse_task_manifest, TaskManifest};
use crate::paths::home_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskListing {
    pub tasks: Vec<TaskPackage>,
    pub diagnostics: Vec<TaskDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPackage {
    pub manifest: TaskManifest,
    pub source: TaskSource,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSource {
    pub kind: TaskSourceKind,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSourceKind {
    Project,
    User,
    Builtin,
}

impl TaskSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::Builtin => "builtin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDiagnostic {
    pub code: TaskDiagnosticCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

impl TaskDiagnostic {
    pub fn new(code: TaskDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source_id: None,
            path: None,
        }
    }

    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDiagnosticCode {
    InvalidManifest,
    MissingPrompt,
    ProjectDiscoveryFailed,
    ReadFailed,
}

impl TaskDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidManifest => "invalid_manifest",
            Self::MissingPrompt => "missing_prompt",
            Self::ProjectDiscoveryFailed => "project_discovery_failed",
            Self::ReadFailed => "read_failed",
        }
    }
}

pub fn discover_tasks(project_cwd: &Utf8Path, user_tasks_dir: Option<&Utf8Path>) -> TaskListing {
    let mut diagnostics = Vec::new();
    let mut tasks = BTreeMap::new();

    collect_builtin_tasks(&mut tasks, &mut diagnostics);

    let user_tasks_dir = match user_tasks_dir {
        Some(dir) => Some(dir.to_path_buf()),
        None => default_user_tasks_dir(),
    };
    if let Some(dir) = user_tasks_dir {
        collect_filesystem_tasks(TaskSourceKind::User, &dir, &mut tasks, &mut diagnostics);
    }

    collect_filesystem_tasks(
        TaskSourceKind::Project,
        &project_cwd.join("dadr/tasks"),
        &mut tasks,
        &mut diagnostics,
    );

    TaskListing {
        tasks: tasks.into_values().collect(),
        diagnostics,
    }
}

pub fn default_user_tasks_dir() -> Option<Utf8PathBuf> {
    if let Some(path) = env::var_os("DADROCK_TASKS_DIR") {
        if !path.is_empty() {
            return Utf8PathBuf::from_path_buf(PathBuf::from(path)).ok();
        }
    }

    home_dir().and_then(|home| Utf8PathBuf::from_path_buf(home.join(".dadrock/tasks")).ok())
}

fn collect_builtin_tasks(
    tasks: &mut BTreeMap<String, TaskPackage>,
    diagnostics: &mut Vec<TaskDiagnostic>,
) {
    for spec in builtin::starter_task_specs() {
        let declared_source_id = format!("builtin:{}", spec.id);
        let manifest = match parse_task_manifest(&declared_source_id, spec.manifest) {
            Ok(manifest) => manifest,
            Err(err) => {
                diagnostics.push(
                    TaskDiagnostic::new(
                        TaskDiagnosticCode::InvalidManifest,
                        format!("failed to load built-in task `{}`: {err}", spec.id),
                    )
                    .with_source_id(declared_source_id),
                );
                continue;
            }
        };

        if manifest.id != spec.id {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::InvalidManifest,
                    format!(
                        "built-in task `{}` manifest declares id `{}`",
                        spec.id, manifest.id
                    ),
                )
                .with_source_id(declared_source_id),
            );
            continue;
        }

        if spec.prompt.trim().is_empty() {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::MissingPrompt,
                    format!("built-in task `{}` has an empty prompt", manifest.id),
                )
                .with_source_id(format!("builtin:{}", manifest.id)),
            );
            continue;
        }

        tasks.insert(
            manifest.id.clone(),
            TaskPackage {
                source: TaskSource {
                    kind: TaskSourceKind::Builtin,
                    id: format!("builtin:{}", manifest.id),
                    path: None,
                },
                manifest,
                prompt: spec.prompt.to_owned(),
            },
        );
    }
}

fn collect_filesystem_tasks(
    kind: TaskSourceKind,
    root: &Utf8Path,
    tasks: &mut BTreeMap<String, TaskPackage>,
    diagnostics: &mut Vec<TaskDiagnostic>,
) {
    let entries = match fs::read_dir(root.as_std_path()) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return,
        Err(err) => {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::ReadFailed,
                    format!(
                        "failed to read {} task directory `{root}`: {err}",
                        kind.as_str()
                    ),
                )
                .with_path(root.to_path_buf()),
            );
            return;
        }
    };

    let mut task_dirs = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                diagnostics.push(TaskDiagnostic::new(
                    TaskDiagnosticCode::ReadFailed,
                    format!("failed to read an entry in `{root}`: {err}"),
                ));
                continue;
            }
        };
        let path = match Utf8PathBuf::from_path_buf(entry.path()) {
            Ok(path) => path,
            Err(path) => {
                diagnostics.push(TaskDiagnostic::new(
                    TaskDiagnosticCode::ReadFailed,
                    format!("task path is not valid UTF-8: {}", path.display()),
                ));
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                diagnostics.push(
                    TaskDiagnostic::new(
                        TaskDiagnosticCode::ReadFailed,
                        format!("failed to inspect task path `{path}`: {err}"),
                    )
                    .with_path(path),
                );
                continue;
            }
        };
        if file_type.is_dir() {
            task_dirs.push(path);
        }
    }
    task_dirs.sort_by(|left, right| left.as_str().cmp(right.as_str()));

    for task_dir in task_dirs {
        if let Some(task) = read_filesystem_task(kind, &task_dir, diagnostics) {
            tasks.insert(task.manifest.id.clone(), task);
        }
    }
}

fn read_filesystem_task(
    kind: TaskSourceKind,
    task_dir: &Utf8Path,
    diagnostics: &mut Vec<TaskDiagnostic>,
) -> Option<TaskPackage> {
    let fallback_source_id = filesystem_source_id(kind, task_dir);
    let manifest_path = task_dir.join("task.toml");
    let manifest_content = match fs::read_to_string(manifest_path.as_std_path()) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push(
                manifest_read_diagnostic(kind, &fallback_source_id, &manifest_path, err)
                    .with_path(manifest_path),
            );
            return None;
        }
    };

    let manifest = match parse_task_manifest(&fallback_source_id, &manifest_content) {
        Ok(manifest) => manifest,
        Err(err) => {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::InvalidManifest,
                    format!("failed to parse task manifest `{manifest_path}`: {err}"),
                )
                .with_source_id(fallback_source_id)
                .with_path(manifest_path),
            );
            return None;
        }
    };

    let source_id = format!("{}:{}", kind.as_str(), manifest.id);
    let prompt_path = match resolve_prompt_path(task_dir, &source_id, &manifest.prompt.system) {
        Ok(path) => path,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return None;
        }
    };
    let prompt = match fs::read_to_string(prompt_path.as_std_path()) {
        Ok(prompt) => prompt,
        Err(err) => {
            diagnostics.push(prompt_read_diagnostic(&source_id, &prompt_path, err));
            return None;
        }
    };
    if prompt.trim().is_empty() {
        diagnostics.push(
            TaskDiagnostic::new(
                TaskDiagnosticCode::MissingPrompt,
                format!("task `{}` prompt `{prompt_path}` is empty", manifest.id),
            )
            .with_source_id(source_id)
            .with_path(prompt_path),
        );
        return None;
    }

    Some(TaskPackage {
        manifest,
        source: TaskSource {
            kind,
            id: source_id,
            path: Some(task_dir.to_path_buf()),
        },
        prompt,
    })
}

fn manifest_read_diagnostic(
    kind: TaskSourceKind,
    source_id: &str,
    manifest_path: &Utf8Path,
    err: io::Error,
) -> TaskDiagnostic {
    let code = if err.kind() == io::ErrorKind::NotFound {
        TaskDiagnosticCode::InvalidManifest
    } else {
        TaskDiagnosticCode::ReadFailed
    };
    TaskDiagnostic::new(
        code,
        format!(
            "failed to read {} task manifest `{manifest_path}`: {err}",
            kind.as_str()
        ),
    )
    .with_source_id(source_id.to_owned())
}

fn prompt_read_diagnostic(
    source_id: &str,
    prompt_path: &Utf8Path,
    err: io::Error,
) -> TaskDiagnostic {
    let code = if err.kind() == io::ErrorKind::NotFound {
        TaskDiagnosticCode::MissingPrompt
    } else {
        TaskDiagnosticCode::ReadFailed
    };
    TaskDiagnostic::new(
        code,
        format!("failed to read task prompt `{prompt_path}`: {err}"),
    )
    .with_source_id(source_id.to_owned())
    .with_path(prompt_path.to_path_buf())
}

fn resolve_prompt_path(
    task_dir: &Utf8Path,
    source_id: &str,
    prompt: &str,
) -> Result<Utf8PathBuf, TaskDiagnostic> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(invalid_prompt_reference(
            source_id,
            task_dir.to_path_buf(),
            "task prompt path cannot be empty",
        ));
    }

    let prompt_path = Utf8Path::new(prompt);
    if prompt_path.is_absolute() {
        return Err(invalid_prompt_reference(
            source_id,
            prompt_path.to_path_buf(),
            format!("absolute prompt path `{prompt}` is not allowed"),
        ));
    }

    for component in prompt_path.components() {
        match component {
            Utf8Component::ParentDir => {
                return Err(invalid_prompt_reference(
                    source_id,
                    task_dir.join(prompt_path),
                    format!("prompt path `{prompt}` cannot include a parent directory component"),
                ));
            }
            Utf8Component::Prefix(_) | Utf8Component::RootDir => {
                return Err(invalid_prompt_reference(
                    source_id,
                    prompt_path.to_path_buf(),
                    format!("prompt path `{prompt}` must be relative to the task package"),
                ));
            }
            Utf8Component::CurDir | Utf8Component::Normal(_) => {}
        }
    }

    let prompt_path = task_dir.join(prompt_path);
    let canonical_task_dir = match canonical_utf8_path(task_dir) {
        Ok(path) => path,
        Err(err) => {
            return Err(TaskDiagnostic::new(
                TaskDiagnosticCode::ReadFailed,
                format!("failed to resolve task directory `{task_dir}`: {err}"),
            )
            .with_source_id(source_id.to_owned())
            .with_path(task_dir.to_path_buf()));
        }
    };
    let canonical_prompt_path = match canonical_utf8_path(&prompt_path) {
        Ok(path) => path,
        Err(err) => return Err(prompt_read_diagnostic(source_id, &prompt_path, err)),
    };

    if !canonical_prompt_path.starts_with(&canonical_task_dir) {
        return Err(invalid_prompt_reference(
            source_id,
            prompt_path,
            format!("prompt path `{prompt}` resolves outside task package `{task_dir}`"),
        ));
    }

    Ok(canonical_prompt_path)
}

fn canonical_utf8_path(path: &Utf8Path) -> Result<Utf8PathBuf, io::Error> {
    let canonical = fs::canonicalize(path.as_std_path())?;
    Utf8PathBuf::from_path_buf(canonical)
        .map_err(|path| io::Error::new(io::ErrorKind::InvalidData, path.display().to_string()))
}

fn invalid_prompt_reference(
    source_id: &str,
    path: Utf8PathBuf,
    message: impl Into<String>,
) -> TaskDiagnostic {
    TaskDiagnostic::new(TaskDiagnosticCode::InvalidManifest, message)
        .with_source_id(source_id.to_owned())
        .with_path(path)
}

fn filesystem_source_id(kind: TaskSourceKind, task_dir: &Utf8Path) -> String {
    format!(
        "{}:{}",
        kind.as_str(),
        task_dir.file_name().unwrap_or("unknown-task")
    )
}
