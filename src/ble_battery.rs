use bluer::{Adapter, AdapterEvent, DiscoveryFilter, DiscoveryTransport, Session};
use futures_util::{StreamExt, pin_mut};
use std::time::Duration;
use tokio::time::timeout;

const APPLE_COMPANY_ID: u16 = 0x004c;
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct AirPodsBattery {
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub case: Option<u8>,
    pub left_charging: bool,
    pub right_charging: bool,
    pub case_charging: bool,
    pub model_id: u16,
    pub ble_address: String,
    pub rssi: i16,
}

pub async fn scan_airpods_battery() -> Result<AirPodsBattery, String> {
    let session = Session::new()
        .await
        .map_err(|error| format!("failed to connect to BlueZ: {error}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|error| format!("failed to get the Bluetooth adapter: {error}"))?;
    let previous_filter = adapter.discovery_filter().await;
    adapter
        .set_discovery_filter(DiscoveryFilter {
            transport: DiscoveryTransport::Le,
            duplicate_data: true,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("failed to configure BLE scan: {error}"))?;

    let result = scan_once(&adapter).await;
    let _ = adapter.set_discovery_filter(previous_filter).await;
    result
}

async fn scan_once(adapter: &Adapter) -> Result<AirPodsBattery, String> {
    let events = adapter
        .discover_devices_with_changes()
        .await
        .map_err(|error| format!("failed to start BLE scan: {error}"))?;
    pin_mut!(events);

    timeout(SCAN_TIMEOUT, async {
        while let Some(event) = events.next().await {
            let AdapterEvent::DeviceAdded(address) = event else {
                continue;
            };
            let Ok(device) = adapter.device(address) else {
                continue;
            };
            let Ok(Some(rssi)) = device.rssi().await else {
                continue;
            };
            let Ok(Some(manufacturer_data)) = device.manufacturer_data().await else {
                continue;
            };
            let Some(data) = manufacturer_data.get(&APPLE_COMPANY_ID) else {
                continue;
            };
            if let Some(battery) = parse_advertisement(data, address.to_string(), rssi) {
                return Ok(battery);
            }
        }
        Err("BLE scan ended before an AirPods advertisement was received".to_string())
    })
    .await
    .map_err(|_| {
        "no AirPods battery advertisement found; open the case near this computer and try again"
            .to_string()
    })?
}

pub fn parse_advertisement(data: &[u8], ble_address: String, rssi: i16) -> Option<AirPodsBattery> {
    if data.len() < 11 || data[0] != 0x07 || data[1] != 0x19 || data[2] != 0x01 {
        return None;
    }

    let model_id = u16::from_be_bytes([data[3], data[4]]);
    if !is_airpods_model(model_id) {
        return None;
    }

    let values_flipped = data[5] & 0x20 == 0;
    let pods = data[6];
    let left_nibble = if values_flipped {
        pods >> 4
    } else {
        pods & 0x0f
    };
    let right_nibble = if values_flipped {
        pods & 0x0f
    } else {
        pods >> 4
    };
    let flags = data[7] >> 4;

    Some(AirPodsBattery {
        left: battery_percent(left_nibble),
        right: battery_percent(right_nibble),
        case: battery_percent(data[7] & 0x0f),
        left_charging: flags & if values_flipped { 0x02 } else { 0x01 } != 0,
        right_charging: flags & if values_flipped { 0x01 } else { 0x02 } != 0,
        case_charging: flags & 0x04 != 0,
        model_id,
        ble_address,
        rssi,
    })
}

pub fn model_name(model_id: u16) -> &'static str {
    match model_id {
        0x0220 => "AirPods 1",
        0x0f20 => "AirPods 2",
        0x1320 => "AirPods 3",
        0x0e20 => "AirPods Pro",
        0x1420 => "AirPods Pro 2 (Lightning)",
        0x2420 => "AirPods Pro 2 (USB-C)",
        0x0a20 => "AirPods Max (Lightning)",
        0x1f20 => "AirPods Max (USB-C)",
        0x1920 => "AirPods 4",
        0x1b20 => "AirPods 4 (ANC)",
        0x2720 => "AirPods Pro (model 2720)",
        _ => "Unknown AirPods",
    }
}

fn is_airpods_model(model_id: u16) -> bool {
    matches!(
        model_id,
        0x0220
            | 0x0f20
            | 0x1320
            | 0x0e20
            | 0x1420
            | 0x2420
            | 0x0a20
            | 0x1f20
            | 0x1920
            | 0x1b20
            | 0x2720
    )
}

fn battery_percent(nibble: u8) -> Option<u8> {
    match nibble {
        0x00..=0x0a => Some(nibble * 10),
        _ => None,
    }
}
