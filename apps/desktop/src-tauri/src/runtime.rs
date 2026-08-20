use std::{env, path::PathBuf};

#[cfg(debug_assertions)]
pub const SERVICE_NAME: &str = "airpods-hires-mic-dev.service";

#[cfg(not(debug_assertions))]
pub const SERVICE_NAME: &str = "airpods-hires-mic.service";

#[cfg(debug_assertions)]
const ENVIRONMENT_FILE: &str = "dev.environment";

#[cfg(not(debug_assertions))]
const ENVIRONMENT_FILE: &str = "environment";

pub fn environment_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config/airpods-hires-mic")
        .join(ENVIRONMENT_FILE))
}
