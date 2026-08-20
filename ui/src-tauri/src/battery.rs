use airpods_hires_mic::ble_battery;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AirPodsBattery {
    left: Option<u8>,
    right: Option<u8>,
    case: Option<u8>,
    left_charging: bool,
    right_charging: bool,
    case_charging: bool,
    model_id: String,
    ble_address: String,
    rssi: i16,
}

#[tauri::command]
pub async fn scan_airpods_battery() -> Result<AirPodsBattery, String> {
    ble_battery::scan_airpods_battery()
        .await
        .map(|battery| AirPodsBattery {
            left: battery.left,
            right: battery.right,
            case: battery.case,
            left_charging: battery.left_charging,
            right_charging: battery.right_charging,
            case_charging: battery.case_charging,
            model_id: format!("{:04X}", battery.model_id),
            ble_address: battery.ble_address,
            rssi: battery.rssi,
        })
}
