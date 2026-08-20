use crate::runtime::SERVICE_NAME;
use serde::Serialize;
use std::{process::Output, time::Duration};
use tokio::{process::Command, time::timeout};

const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    active_state: String,
    sub_state: String,
    enabled: String,
    main_pid: Option<u32>,
}

#[tauri::command]
pub async fn get_service_status() -> Result<ServiceStatus, String> {
    let output = run_systemctl(&[
        "--user",
        "show",
        SERVICE_NAME,
        "--property=ActiveState,SubState,MainPID",
    ])
    .await?;
    let text = require_success(output, "read microphone service status")?;
    let mut active_state = "unknown".to_string();
    let mut sub_state = "unknown".to_string();
    let mut main_pid = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "ActiveState" => active_state = value.to_string(),
            "SubState" => sub_state = value.to_string(),
            "MainPID" => main_pid = value.parse::<u32>().ok().filter(|pid| *pid != 0),
            _ => {}
        }
    }

    let enabled = run_systemctl(&["--user", "is-enabled", SERVICE_NAME])
        .await
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(ServiceStatus {
        active_state,
        sub_state,
        enabled,
        main_pid,
    })
}

#[tauri::command]
pub async fn start_service() -> Result<(), String> {
    control_service("start").await
}

#[tauri::command]
pub async fn stop_service() -> Result<(), String> {
    control_service("stop").await
}

#[tauri::command]
pub async fn restart_service() -> Result<(), String> {
    control_service("restart").await
}

async fn control_service(action: &str) -> Result<(), String> {
    let output = run_systemctl(&["--user", action, SERVICE_NAME]).await?;
    require_success(output, &format!("{action} microphone service")).map(|_| ())
}

async fn run_systemctl(args: &[&str]) -> Result<Output, String> {
    let mut command = Command::new("systemctl");
    command.args(args);
    timeout(SYSTEMCTL_TIMEOUT, command.output())
        .await
        .map_err(|_| "systemctl timed out after 15 seconds".to_string())?
        .map_err(|error| format!("failed to run systemctl: {error}"))
}

fn require_success(output: Output, action: &str) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr);
        Err(format!("failed to {action}: {}", detail.trim()))
    }
}
