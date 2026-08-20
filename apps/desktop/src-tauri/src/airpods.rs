use crate::runtime::environment_path;
use airpods_core::dsp::{MIC_GAIN_DB, MIC_LIMIT_DBFS};
use bluer::{Address, Session};
use serde::Serialize;
use std::{fs, str::FromStr};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothDevice {
    address: String,
    name: String,
}

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

#[tauri::command]
pub async fn get_connected_bluetooth_devices() -> Result<Vec<BluetoothDevice>, String> {
    let session = Session::new()
        .await
        .map_err(|error| format!("failed to connect to BlueZ: {error}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|error| format!("failed to get the Bluetooth adapter: {error}"))?;
    let addresses = adapter
        .device_addresses()
        .await
        .map_err(|error| format!("failed to list Bluetooth devices: {error}"))?;
    let mut devices = Vec::new();

    for address in addresses {
        let Ok(device) = adapter.device(address) else {
            continue;
        };
        if !device.is_connected().await.unwrap_or(false) {
            continue;
        }
        devices.push(BluetoothDevice {
            address: address.to_string(),
            name: device
                .name()
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "Connected Bluetooth device".to_string()),
        });
    }
    devices.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(devices)
}

fn configured_device() -> Option<String> {
    let path = environment_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    contents.lines().find_map(|line| {
        line.strip_prefix("AIRPODS_DEVICE=")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}
