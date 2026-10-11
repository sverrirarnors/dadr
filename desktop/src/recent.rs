//! The welcome window's list of recently opened projects, kept as a small JSON
//! file in the app's config directory.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const LIMIT: usize = 12;
const FILE: &str = "recent-projects.json";

#[derive(Debug, Clone, Serialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub missing: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    paths: Vec<PathBuf>,
}

pub fn list(app: &AppHandle) -> Vec<RecentProject> {
    load(app)
        .paths
        .into_iter()
        .map(|path| RecentProject {
            name: display_name(&path),
            missing: !path.is_dir(),
            path: path.to_string_lossy().into_owned(),
        })
        .collect()
}

pub fn remember(app: &AppHandle, path: &Path) {
    let mut stored = load(app);
    stored.paths.retain(|existing| existing != path);
    stored.paths.insert(0, path.to_path_buf());
    stored.paths.truncate(LIMIT);
    save(app, &stored);
}

pub fn forget(app: &AppHandle, path: &Path) {
    let mut stored = load(app);
    stored.paths.retain(|existing| existing != path);
    save(app, &stored);
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

fn load(app: &AppHandle) -> Stored {
    file(app)
        .and_then(|file| fs::read(file).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Best effort: losing the recent list is not worth an error dialog.
fn save(app: &AppHandle, stored: &Stored) {
    let Some(file) = file(app) else { return };
    if let Some(dir) = file.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(stored) {
        let _ = fs::write(file, bytes);
    }
}
