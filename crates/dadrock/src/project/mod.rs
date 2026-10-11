use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::DadrError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    Directory,
    Fallback,
}

/// Folder for candidates, the records waiting for a human decision.
pub const INBOX_DIR: &str = "inbox";
/// Folder for decisions in force: accepted records without successors.
pub const LEDGER_DIR: &str = "ledger";
/// Folder for records kept but not in force: rejected and superseded records.
pub const ARCHIVE_DIR: &str = "archive";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectContext {
    pub cwd: Utf8PathBuf,
    /// The `dadr/` directory.
    pub project_dir: Utf8PathBuf,
    pub discovery_source: DiscoverySource,
    pub initialized: bool,
}

impl ProjectContext {
    pub fn workspace_dir(&self) -> Utf8PathBuf {
        self.project_dir
            .parent()
            .map(Utf8Path::to_path_buf)
            .unwrap_or_else(|| self.cwd.clone())
    }

    pub fn inbox_dir(&self) -> Utf8PathBuf {
        self.project_dir.join(INBOX_DIR)
    }

    pub fn ledger_dir(&self) -> Utf8PathBuf {
        self.project_dir.join(LEDGER_DIR)
    }

    pub fn archive_dir(&self) -> Utf8PathBuf {
        self.project_dir.join(ARCHIVE_DIR)
    }

    /// The folders that hold records, in scan order.
    pub fn record_dirs(&self) -> [Utf8PathBuf; 3] {
        [self.ledger_dir(), self.inbox_dir(), self.archive_dir()]
    }

    pub fn templates_dir(&self) -> Utf8PathBuf {
        self.project_dir.join("templates")
    }

    pub fn tasks_dir(&self) -> Utf8PathBuf {
        self.project_dir.join("tasks")
    }

    pub fn runs_dir(&self) -> Utf8PathBuf {
        self.project_dir.join("runs")
    }

    pub fn require_initialized(&self) -> Result<(), DadrError> {
        if self.initialized {
            Ok(())
        } else {
            Err(DadrError::ProjectNotInitialized {
                project_dir: self.project_dir.clone(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitResult {
    pub project_dir: Utf8PathBuf,
    pub created: Vec<Utf8PathBuf>,
    pub existing: Vec<Utf8PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLayout {
    pub project_dir: Utf8PathBuf,
    pub inbox_dir: Utf8PathBuf,
    pub ledger_dir: Utf8PathBuf,
    pub archive_dir: Utf8PathBuf,
    pub templates_dir: Utf8PathBuf,
    pub tasks_dir: Utf8PathBuf,
    pub runs_dir: Utf8PathBuf,
}

pub fn project_layout(project: &ProjectContext) -> ProjectLayout {
    ProjectLayout {
        project_dir: project.project_dir.clone(),
        inbox_dir: project.inbox_dir(),
        ledger_dir: project.ledger_dir(),
        archive_dir: project.archive_dir(),
        templates_dir: project.templates_dir(),
        tasks_dir: project.tasks_dir(),
        runs_dir: project.runs_dir(),
    }
}

pub fn discover_from(start: impl AsRef<Path>) -> Result<ProjectContext, DadrError> {
    let cwd = absolutize(start.as_ref())?;
    let cwd_utf8 = path_to_utf8(cwd.clone())?;
    let search_root = nearest_git_root(&cwd);

    for ancestor in cwd.ancestors() {
        let dadr = if ancestor.file_name().is_some_and(|name| name == "dadr") {
            ancestor.to_path_buf()
        } else {
            ancestor.join("dadr")
        };
        if dadr_layout_exists(&dadr) {
            return Ok(ProjectContext {
                cwd: cwd_utf8,
                project_dir: path_to_utf8(dadr)?,
                discovery_source: DiscoverySource::Directory,
                initialized: true,
            });
        }
        if search_root.as_deref() == Some(ancestor) {
            break;
        }
    }

    Ok(ProjectContext {
        cwd: cwd_utf8.clone(),
        project_dir: cwd_utf8.join("dadr"),
        discovery_source: DiscoverySource::Fallback,
        initialized: false,
    })
}

fn nearest_git_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

fn dadr_layout_exists(dadr: &Path) -> bool {
    dadr.is_dir()
        && [INBOX_DIR, LEDGER_DIR, ARCHIVE_DIR]
            .iter()
            .all(|folder| dadr.join(folder).is_dir())
}

pub fn init_project(target: impl AsRef<Path>) -> Result<InitResult, DadrError> {
    let target = absolutize(target.as_ref())?;
    let project_dir = if target.file_name().is_some_and(|name| name == "dadr") {
        target
    } else {
        target.join("dadr")
    };
    let inbox_dir = project_dir.join(INBOX_DIR);
    let ledger_dir = project_dir.join(LEDGER_DIR);
    let archive_dir = project_dir.join(ARCHIVE_DIR);
    let templates_dir = project_dir.join("templates");
    let tasks_dir = project_dir.join("tasks");
    let runs_dir = project_dir.join("runs");
    let runs_gitignore = runs_dir.join(".gitignore");

    let mut result = InitResult {
        project_dir: path_to_utf8(project_dir.clone())?,
        created: Vec::new(),
        existing: Vec::new(),
    };

    create_dir_idempotent(&project_dir, &mut result)?;
    create_dir_idempotent(&inbox_dir, &mut result)?;
    create_dir_idempotent(&ledger_dir, &mut result)?;
    create_dir_idempotent(&archive_dir, &mut result)?;
    create_dir_idempotent(&templates_dir, &mut result)?;
    create_dir_idempotent(&tasks_dir, &mut result)?;
    create_dir_idempotent(&runs_dir, &mut result)?;
    write_file_idempotent(&runs_gitignore, "/**\n!.gitignore\n", &mut result)?;

    Ok(result)
}

fn create_dir_idempotent(path: &Path, result: &mut InitResult) -> Result<(), DadrError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(DadrError::ProjectInvariant(format!(
                "`{}` exists but is not a directory",
                path.display()
            )));
        }
        result.existing.push(path_to_utf8(path.to_path_buf())?);
        return Ok(());
    }
    fs::create_dir_all(path)?;
    result.created.push(path_to_utf8(path.to_path_buf())?);
    Ok(())
}

fn write_file_idempotent(
    path: &Path,
    content: &str,
    result: &mut InitResult,
) -> Result<(), DadrError> {
    if path.exists() {
        if !path.is_file() {
            return Err(DadrError::ProjectInvariant(format!(
                "`{}` exists but is not a file",
                path.display()
            )));
        }
        result.existing.push(path_to_utf8(path.to_path_buf())?);
        return Ok(());
    }
    fs::write(path, content)?;
    result.created.push(path_to_utf8(path.to_path_buf())?);
    Ok(())
}

fn absolutize(path: &Path) -> Result<PathBuf, DadrError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize_lexical_path(absolute))
}

fn normalize_lexical_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn path_to_utf8(path: PathBuf) -> Result<Utf8PathBuf, DadrError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| DadrError::PathNotUtf8 {
        path: path.display().to_string(),
    })
}

pub(crate) fn utf8_to_path(path: &Utf8Path) -> &Path {
    path.as_std_path()
}
