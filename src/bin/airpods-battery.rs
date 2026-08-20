use airpods_hires_mic::ble_battery::{model_name, scan_airpods_battery};

#[tokio::main]
async fn main() {
    println!("Scanning for an AirPods BLE battery advertisement (10 second timeout)...");
    match scan_airpods_battery().await {
        Ok(battery) => {
            println!("Found: {}", model_name(battery.model_id));
            println!("Model ID: {:04X}", battery.model_id);
            println!(
                "BLE address: {} (RSSI {} dBm)",
                battery.ble_address, battery.rssi
            );
            print_battery("Left", battery.left, battery.left_charging);
            print_battery("Right", battery.right, battery.right_charging);
            print_battery("Case", battery.case, battery.case_charging);
        }
        Err(error) => {
            eprintln!("Battery scan failed: {error}");
            std::process::exit(1);
        }
    }
}

fn print_battery(label: &str, level: Option<u8>, charging: bool) {
    let level = level.map_or_else(|| "unavailable".to_string(), |value| format!("{value}%"));
    let charging = if charging { " (charging)" } else { "" };
    println!("{label}: {level}{charging}");
}
