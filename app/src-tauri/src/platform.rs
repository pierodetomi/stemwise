//! What the shell does differently on each system (NFR-8): one module, the same interface everywhere.
//!
//! - The menu bar. macOS always has one for the frontmost app: it keeps only what macOS provides there (the app
//!   menu with About, Services, Hide and Quit; Edit for the text fields; Window). Windows and Linux have no such bar,
//!   and a menu inside the window would be a second app menu: there the window has none, and every command of
//!   Stemwise stays in the app menu of the interface (UI-20). The webview handles copy, paste and undo in the text
//!   fields by itself.

use tauri::{Builder, Wry};

/// The builder with the system's menu bar, where the system has one.
#[cfg(target_os = "macos")]
pub fn with_menu_bar(builder: Builder<Wry>) -> Builder<Wry> {
    builder.menu(|app| {
        use tauri::menu::{AboutMetadata, MenuBuilder, SubmenuBuilder};
        let about = AboutMetadata { name: Some("Stemwise".into()), ..Default::default() };
        let stemwise = SubmenuBuilder::new(app, "Stemwise")
            .about(Some(about))
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        let edit =
            SubmenuBuilder::new(app, "Edit").undo().redo().separator().cut().copy().paste().select_all().build()?;
        let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().fullscreen().build()?;
        MenuBuilder::new(app).items(&[&stemwise, &edit, &window]).build()
    })
}

/// Windows and Linux: no menu bar in the window.
#[cfg(not(target_os = "macos"))]
pub fn with_menu_bar(builder: Builder<Wry>) -> Builder<Wry> {
    builder
}
