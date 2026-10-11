//! Windows. The welcome window is a local page bundled with the app; every
//! project window loads the UI from its own `dadrock preview` process and
//! owns that process for as long as the window is open.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, Url, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::{recent, server::PreviewServer};

pub const WELCOME: &str = "welcome";
const PROJECT_PREFIX: &str = "project-";

#[derive(Default)]
pub struct Projects {
    next_id: AtomicU64,
    open: Mutex<HashMap<String, OpenProject>>,
}

struct OpenProject {
    root: PathBuf,
    // Dropping it stops the server, which is why it lives here.
    _server: PreviewServer,
}

/// Opens the project at `path`, or asks for a folder when `path` is `None`,
/// and closes the welcome window once the project is showing. Returns
/// `Ok(false)` when the folder picker was cancelled. Blocks while the server
/// starts, so call it off the main thread.
pub fn open(app: &AppHandle, path: Option<PathBuf>) -> Result<bool, String> {
    let Some(path) = path.or_else(|| pick_folder(app)) else {
        return Ok(false);
    };
    let root = project_root(&path)?;
    let projects = app.state::<Projects>();

    let existing = projects
        .open
        .lock()
        .unwrap()
        .iter()
        .find(|(_, project)| project.root == root)
        .map(|(label, _)| label.clone());
    if let Some(window) = existing.and_then(|label| app.get_webview_window(&label)) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        recent::remember(app, &root);
        close_welcome(app);
        return Ok(true);
    }

    let label = format!(
        "{PROJECT_PREFIX}{}",
        projects.next_id.fetch_add(1, Ordering::Relaxed)
    );
    let name = recent::display_name(&root);
    let server = {
        let app = app.clone();
        let label = label.clone();
        let name = name.clone();
        PreviewServer::start(&root, move || server_stopped(&app, &label, &name))?
    };
    let url = server.url().clone();

    // Registered before the window exists, so a window closed the moment it
    // opens still finds its server to stop.
    projects.open.lock().unwrap().insert(
        label.clone(),
        OpenProject {
            root: root.clone(),
            _server: server,
        },
    );
    if let Err(err) = build_project_window(app, &label, &name, url) {
        projects.open.lock().unwrap().remove(&label);
        return Err(format!("Could not open a window for {name}: {err}"));
    }
    recent::remember(app, &root);
    close_welcome(app);
    Ok(true)
}

/// Opens a project from a background thread, reporting failure in a dialog.
pub fn open_in_background(app: &AppHandle, path: Option<PathBuf>) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(err) = open(&app, path) {
            if app.webview_windows().is_empty() {
                let _ = show_welcome(&app);
            }
            show_error(&app, &err);
        }
    });
}

pub fn show_welcome(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WELCOME) {
        window.unminimize()?;
        return window.set_focus();
    }
    WebviewWindowBuilder::new(app, WELCOME, WebviewUrl::App("index.html".into()))
        .title("Dadrock")
        .inner_size(560.0, 620.0)
        .min_inner_size(440.0, 480.0)
        .center()
        .build()?;
    Ok(())
}

fn close_welcome(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WELCOME) {
        let _ = window.close();
    }
}

pub fn show_error(app: &AppHandle, message: &str) {
    app.dialog()
        .message(message)
        .title("Dadrock")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::Destroyed = event {
        // Stops the window's server.
        window
            .app_handle()
            .state::<Projects>()
            .open
            .lock()
            .unwrap()
            .remove(window.label());
    }
}

/// Stops every server; the app is exiting.
pub fn stop_all(app: &AppHandle) {
    app.state::<Projects>().open.lock().unwrap().clear();
}

pub fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
}

fn build_project_window(app: &AppHandle, label: &str, name: &str, url: Url) -> tauri::Result<()> {
    let origin = url.origin();
    WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
        .title(format!("{name} — Dadrock"))
        .inner_size(1360.0, 900.0)
        .min_inner_size(720.0, 480.0)
        .zoom_hotkeys_enabled(true)
        // Leaves drops to the web UI rather than turning them into Tauri events.
        .disable_drag_drop_handler()
        // Only the project's own server is shown in the window; links to
        // anywhere else, including Markdown links that ask for a new tab, go
        // to the system browser.
        .on_navigation(move |target| {
            if target.origin() == origin || target.scheme() == "about" {
                return true;
            }
            open_in_browser(target);
            false
        })
        .on_new_window(|target, _features| {
            open_in_browser(&target);
            NewWindowResponse::Deny
        })
        .build()?;
    Ok(())
}

fn open_in_browser(url: &Url) {
    if matches!(url.scheme(), "http" | "https" | "mailto") {
        let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
    }
}

fn pick_folder(app: &AppHandle) -> Option<PathBuf> {
    let mut dialog = app.dialog().file().set_title("Open a DADR project");
    if let Some(window) = focused_window(app) {
        dialog = dialog.set_parent(&window);
    }
    dialog.blocking_pick_folder()?.into_path().ok()
}

fn server_stopped(app: &AppHandle, label: &str, name: &str) {
    let _ = show_welcome(app);
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.close();
    }
    show_error(
        app,
        &format!("The dadrock server for {name} stopped unexpectedly. Open the project again to restart it."),
    );
}

fn project_root(path: &Path) -> Result<PathBuf, String> {
    let root =
        fs::canonicalize(path).map_err(|err| format!("Cannot open {}: {err}", path.display()))?;
    if !root.is_dir() {
        return Err(format!("{} is not a folder.", root.display()));
    }
    Ok(without_verbatim_prefix(root))
}

/// `canonicalize` spells `C:\x` as `\\?\C:\x` on Windows, which is not how the
/// user or dadrock writes it.
#[cfg(windows)]
fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|path| path.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

#[cfg(not(windows))]
fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    path
}
