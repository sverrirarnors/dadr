// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod menu;
mod projects;
mod recent;
mod server;

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use tauri::{AppHandle, RunEvent};

const USAGE: &str = "\
Dadrock: the DADR review UI in a native window.

Usage: dadrock-desktop [PROJECT]

Opens PROJECT (a folder inside a DADR project) directly, or shows the
welcome window when no folder is given. Each project window runs its own
`dadrock preview` server; the app finds `dadrock` in $DADROCK_BIN, next to
its own executable, or on PATH.

Options:
  -h, --help     Print this help
  -V, --version  Print the version
";

fn main() -> ExitCode {
    let mut project = None;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-V" | "--version") => {
                println!("dadrock-desktop {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            Some("-h" | "--help") => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            // Older macOS releases pass a process serial number to apps
            // started from Finder.
            Some(flag) if flag.starts_with("-psn_") => {}
            Some(flag) if flag.starts_with('-') => {
                eprint!("dadrock-desktop: unknown option `{flag}`\n\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => project = Some(PathBuf::from(arg)),
        }
    }
    run(project);
    ExitCode::SUCCESS
}

fn run(project: Option<PathBuf>) {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(projects::Projects::default())
        .menu(menu::build)
        .on_menu_event(menu::handle)
        .on_window_event(projects::on_window_event)
        .invoke_handler(tauri::generate_handler![
            recent_projects,
            forget_project,
            open_project
        ])
        .setup(move |app| {
            // Resolve the login shell's PATH now, so the first project does
            // not wait on it.
            std::thread::spawn(server::search_path);
            match project {
                Some(path) => projects::open_in_background(app.handle(), Some(path)),
                None => projects::show_welcome(app.handle())?,
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start the Dadrock app")
        .run(|app, event| match event {
            // macOS apps stay running with no windows open; the Dock icon
            // brings the welcome window back.
            #[cfg(target_os = "macos")]
            RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => {
                let _ = projects::show_welcome(app);
            }
            RunEvent::Exit => projects::stop_all(app),
            _ => {}
        });
}

#[tauri::command]
fn recent_projects(app: AppHandle) -> Vec<recent::RecentProject> {
    recent::list(&app)
}

#[tauri::command]
fn forget_project(app: AppHandle, path: String) {
    recent::forget(&app, Path::new(&path));
}

/// Opens `path`, or asks for a folder when it is absent. Resolves to `false`
/// when the folder picker was cancelled.
#[tauri::command]
async fn open_project(app: AppHandle, path: Option<String>) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || projects::open(&app, path.map(PathBuf::from)))
        .await
        .map_err(|err| err.to_string())?
}
