use airpods_hires_mic::dsp::{MIC_GAIN_DB, MIC_LIMIT_DBFS};
use bluer::{Address, Session};
use serde::Serialize;
use std::{env, fs, path::PathBuf, str::FromStr};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AirPodsStatus {
    configured: bool,
    address: Option<String>,
    name: Option<String>,
    connected: bool,
    battery_percentage: Option<u8>,
    mic_gain_db: f32,
    limiter_dbfs: f32,
}

#[tauri::command]
pub async fn get_airpods_status() -> Result<AirPodsStatus, String> {
    let Some(address_text) = configured_device() else {
        return Ok(AirPodsStatus {
            configured: false,
            address: None,
            name: None,
            connected: false,
            battery_percentage: None,
            mic_gain_db: MIC_GAIN_DB,
            limiter_dbfs: MIC_LIMIT_DBFS,
        });
    };

    let address = Address::from_str(&address_text)
        .map_err(|error| format!("invalid configured AirPods address: {error}"))?;
    let session = Session::new()
        .await
        .map_err(|error| format!("failed to connect to BlueZ: {error}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|error| format!("failed to get the Bluetooth adapter: {error}"))?;
    let device = adapter
        .device(address)
        .map_err(|error| format!("failed to open the AirPods device: {error}"))?;
    let connected = device
        .is_connected()
        .await
        .map_err(|error| format!("failed to read AirPods connection state: {error}"))?;
    let name = device.name().await.unwrap_or(None);
    let battery_percentage = device.battery_percentage().await.unwrap_or(None);

    Ok(AirPodsStatus {
        configured: true,
        address: Some(address_text),
        name,
        connected,
        battery_percentage,
        mic_gain_db: MIC_GAIN_DB,
        limiter_dbfs: MIC_LIMIT_DBFS,
    })
}

fn configured_device() -> Option<String> {
    let home = env::var_os("HOME")?;
    let path = PathBuf::from(home).join(".config/airpods-hires-mic/environment");
    let contents = fs::read_to_string(path).ok()?;
    contents.lines().find_map(|line| {
        line.strip_prefix("AIRPODS_DEVICE=")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}
