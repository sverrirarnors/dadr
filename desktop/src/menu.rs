//! The menu bar. On macOS it is the full native set: Edit has to be there for
//! copy, paste, and undo shortcuts to reach the web UI at all. On Linux and
//! Windows the menu bar sits inside every window, above the UI's own header,
//! and the web view already handles editing keys, so it is a single File menu
//! that carries the shortcuts.

use tauri::{
    menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Runtime,
};

use crate::projects;

const OPEN_PROJECT: &str = "open-project";
const SHOW_WELCOME: &str = "show-welcome";
const RELOAD: &str = "reload";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let open = MenuItemBuilder::with_id(OPEN_PROJECT, "Open Project…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let recent = MenuItemBuilder::with_id(SHOW_WELCOME, "Recent Projects").build(app)?;
    let reload = MenuItemBuilder::with_id(RELOAD, "Reload")
        .accelerator("CmdOrCtrl+R")
        .build(app)?;

    #[cfg(target_os = "macos")]
    let menu = MenuBuilder::new(app)
        .item(
            &SubmenuBuilder::new(app, "Dadrock")
                .about(None)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?,
        )
        .item(
            &SubmenuBuilder::new(app, "File")
                .item(&open)
                .item(&recent)
                .separator()
                .close_window()
                .build()?,
        )
        .item(
            &SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?,
        )
        .item(
            &SubmenuBuilder::new(app, "View")
                .item(&reload)
                .separator()
                .fullscreen()
                .build()?,
        )
        .item(
            &SubmenuBuilder::new(app, "Window")
                .minimize()
                .maximize()
                .separator()
                .close_window()
                .build()?,
        );

    #[cfg(not(target_os = "macos"))]
    let menu = MenuBuilder::new(app).item(
        &SubmenuBuilder::new(app, "File")
            .item(&open)
            .item(&recent)
            .item(&reload)
            .separator()
            .close_window()
            .quit()
            .build()?,
    );

    menu.build()
}

pub fn handle(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        OPEN_PROJECT => projects::open_in_background(app, None),
        SHOW_WELCOME => {
            let _ = projects::show_welcome(app);
        }
        RELOAD => {
            if let Some(window) = projects::focused_window(app) {
                let _ = window.reload();
            }
        }
        _ => {}
    }
}
