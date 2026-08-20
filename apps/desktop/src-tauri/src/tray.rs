use tauri::{
    App, AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open-settings", "Open Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::with_id("airpods-hires-mic")
        .icon(
            app.default_window_icon()
                .ok_or("application icon is unavailable")?
                .clone(),
        )
        .tooltip("AirPods Hi-Res Mic")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open-settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
