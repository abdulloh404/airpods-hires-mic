use airpods_hires_mic::dsp::{MIC_GAIN_DB, MIC_LIMIT_DBFS, validate_mic_settings};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

const GAIN_KEY: &str = "AIRPODS_MIC_GAIN_DB";
const LIMITER_KEY: &str = "AIRPODS_MIC_LIMITER_DBFS";

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicSettings {
    mic_gain_db: f32,
    limiter_dbfs: f32,
}

#[tauri::command]
pub fn get_mic_settings() -> Result<MicSettings, String> {
    let contents = read_environment()?;
    let settings = MicSettings {
        mic_gain_db: read_number(&contents, GAIN_KEY)?.unwrap_or(MIC_GAIN_DB),
        limiter_dbfs: read_number(&contents, LIMITER_KEY)?.unwrap_or(MIC_LIMIT_DBFS),
    };
    validate_mic_settings(settings.mic_gain_db, settings.limiter_dbfs)?;
    Ok(settings)
}

#[tauri::command]
pub fn save_mic_settings(settings: MicSettings) -> Result<MicSettings, String> {
    validate_mic_settings(settings.mic_gain_db, settings.limiter_dbfs)?;
    let path = environment_path()?;
    let contents = read_environment()?;
    let mut lines: Vec<&str> = contents
        .lines()
        .filter(|line| {
            !line.starts_with(&format!("{GAIN_KEY}="))
                && !line.starts_with(&format!("{LIMITER_KEY}="))
        })
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    let mut updated = lines.join("\n");
    if !updated.is_empty() {
        updated.push('\n');
    }
    updated.push_str(&format!("{GAIN_KEY}={}\n", settings.mic_gain_db));
    updated.push_str(&format!("{LIMITER_KEY}={}\n", settings.limiter_dbfs));
    write_atomic(&path, updated.as_bytes())?;
    Ok(settings)
}

fn environment_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/airpods-hires-mic/environment"))
}

fn read_environment() -> Result<String, String> {
    let path = environment_path()?;
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!(
            "failed to read microphone settings from {}: {error}",
            path.display()
        )),
    }
}

fn read_number(contents: &str, key: &str) -> Result<Option<f32>, String> {
    let Some(value) = contents
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
    else {
        return Ok(None);
    };
    value
        .trim()
        .parse::<f32>()
        .map(Some)
        .map_err(|_| format!("invalid {key} value in microphone settings"))
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "microphone settings path has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create microphone settings directory {}: {error}",
            parent.display()
        )
    })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let temporary = parent.join(format!(".environment.tmp-{}-{nonce}", process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| format!("failed to create temporary microphone settings: {error}"))?;
        file.write_all(contents)
            .map_err(|error| format!("failed to write microphone settings: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync microphone settings: {error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("failed to replace microphone settings: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
