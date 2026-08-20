mod airpods;
mod battery;
mod runtime;
mod service;
mod settings;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(tray::setup)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            airpods::get_airpods_status,
            airpods::get_connected_bluetooth_devices,
            battery::scan_airpods_battery,
            service::get_service_status,
            service::start_service,
            service::stop_service,
            service::restart_service,
            settings::get_mic_settings,
            settings::save_mic_settings,
            settings::save_device_address,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AirPods Hi-Res Mic settings");
}
