use std::{collections::BTreeMap, env, fs, io, path::PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::{builtin, parse_task_document, TaskDocument, TaskManifest};
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
    /// The task document exactly as written: the embedded skill guide for a
    /// built-in task, the file's text for a user or project task. It is not
    /// serialized; JSON output carries the parsed manifest and prompt.
    #[serde(skip)]
    pub document: String,
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
        let source_id = format!("builtin:{}", spec.id);
        let document = match parse_task_document(&source_id, spec.document) {
            Ok(document) => document,
            Err(err) => {
                diagnostics.push(
                    TaskDiagnostic::new(
                        TaskDiagnosticCode::InvalidManifest,
                        format!("failed to load built-in task `{}`: {err}", spec.id),
                    )
                    .with_source_id(source_id),
                );
                continue;
            }
        };
        let TaskDocument { manifest, prompt } = document;

        if manifest.id != spec.id {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::InvalidManifest,
                    format!(
                        "built-in task `{}` manifest declares id `{}`",
                        spec.id, manifest.id
                    ),
                )
                .with_source_id(source_id),
            );
            continue;
        }

        if prompt.is_empty() {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::MissingPrompt,
                    format!("built-in task `{}` has an empty prompt", manifest.id),
                )
                .with_source_id(source_id),
            );
            continue;
        }

        tasks.insert(
            manifest.id.clone(),
            TaskPackage {
                source: TaskSource {
                    kind: TaskSourceKind::Builtin,
                    id: source_id,
                    path: None,
                },
                manifest,
                prompt,
                document: spec.document.to_owned(),
            },
        );
    }
}

/// Read `<root>/<id>.md` task documents. Other entries are ignored, except
/// directories in the old `<id>/task.toml` layout, which get a diagnostic.
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

    let mut task_files = Vec::new();
    let mut legacy_dirs = Vec::new();
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
        if path.file_name().is_some_and(|name| name.starts_with('.')) {
            continue;
        }
        if file_type.is_file() && path.extension() == Some("md") {
            task_files.push(path);
        } else if file_type.is_dir() && path.join("task.toml").is_file() {
            legacy_dirs.push(path);
        }
    }
    task_files.sort();
    legacy_dirs.sort();

    for legacy_dir in legacy_dirs {
        diagnostics.push(legacy_task_package_diagnostic(kind, root, &legacy_dir));
    }
    for task_file in task_files {
        if let Some(task) = read_filesystem_task(kind, &task_file, diagnostics) {
            tasks.insert(task.manifest.id.clone(), task);
        }
    }
}

fn read_filesystem_task(
    kind: TaskSourceKind,
    task_file: &Utf8Path,
    diagnostics: &mut Vec<TaskDiagnostic>,
) -> Option<TaskPackage> {
    let file_id = task_file.file_stem().unwrap_or_default();
    let fallback_source_id = format!("{}:{file_id}", kind.as_str());
    let content = match fs::read_to_string(task_file.as_std_path()) {
        Ok(content) => content,
        Err(err) => {
            diagnostics.push(
                TaskDiagnostic::new(
                    TaskDiagnosticCode::ReadFailed,
                    format!("failed to read {} task `{task_file}`: {err}", kind.as_str()),
                )
                .with_source_id(fallback_source_id)
                .with_path(task_file.to_path_buf()),
            );
            return None;
        }
    };

    let TaskDocument { manifest, prompt } = match parse_task_document(task_file.as_str(), &content)
    {
        Ok(document) => document,
        Err(err) => {
            diagnostics.push(
                TaskDiagnostic::new(TaskDiagnosticCode::InvalidManifest, err.to_string())
                    .with_source_id(fallback_source_id)
                    .with_path(task_file.to_path_buf()),
            );
            return None;
        }
    };

    if manifest.id != file_id {
        diagnostics.push(
            TaskDiagnostic::new(
                TaskDiagnosticCode::InvalidManifest,
                format!(
                    "task file `{task_file}` declares id `{}`; a task file must be named `<id>.md`, so rename it to `{}.md` or change its id to `{file_id}`",
                    manifest.id, manifest.id
                ),
            )
            .with_source_id(fallback_source_id)
            .with_path(task_file.to_path_buf()),
        );
        return None;
    }

    let source_id = format!("{}:{}", kind.as_str(), manifest.id);
    if prompt.is_empty() {
        diagnostics.push(
            TaskDiagnostic::new(
                TaskDiagnosticCode::MissingPrompt,
                format!(
                    "task `{}` in `{task_file}` has no prompt after its front matter",
                    manifest.id
                ),
            )
            .with_source_id(source_id)
            .with_path(task_file.to_path_buf()),
        );
        return None;
    }

    Some(TaskPackage {
        manifest,
        source: TaskSource {
            kind,
            id: source_id,
            path: Some(task_file.to_path_buf()),
        },
        prompt,
        document: content,
    })
}

fn legacy_task_package_diagnostic(
    kind: TaskSourceKind,
    root: &Utf8Path,
    task_dir: &Utf8Path,
) -> TaskDiagnostic {
    let name = task_dir.file_name().unwrap_or("unknown-task");
    TaskDiagnostic::new(
        TaskDiagnosticCode::InvalidManifest,
        format!(
            "`{task_dir}` uses the old task layout (`task.toml` plus a prompt file), which is no longer read; tasks are now `<dir>/<id>.md` files whose YAML front matter is the manifest and whose body is the prompt, so move this task to `{root}/{name}.md`"
        ),
    )
    .with_source_id(format!("{}:{name}", kind.as_str()))
    .with_path(task_dir.join("task.toml"))
}
