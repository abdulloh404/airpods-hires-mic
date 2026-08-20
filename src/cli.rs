use anyhow::{Context, Result};
use bluer::Address;
use clap::Parser;
use std::str::FromStr;

use crate::dsp::{MIC_GAIN_DB, MIC_LIMIT_DBFS, validate_mic_settings};

#[derive(Debug, Parser)]
#[command(
    name = "airpods-hires-mic",
    version,
    about = "Expose the AirPods proprietary high-resolution microphone as a virtual source"
)]
pub struct Cli {
    /// Bluetooth MAC address of connected AirPods
    #[arg(long, value_parser = parse_address)]
    pub device: Address,

    /// Print protocol diagnostics
    #[arg(long)]
    pub verbose: bool,

    /// Receive and count AACP audio packets without decoding or creating a virtual microphone
    #[arg(long)]
    pub transport_only: bool,

    /// Microphone gain in dB
    #[arg(
        long,
        env = "AIRPODS_MIC_GAIN_DB",
        default_value_t = MIC_GAIN_DB,
        value_parser = parse_gain_db
    )]
    pub mic_gain_db: f32,

    /// Limiter ceiling in dBFS
    #[arg(
        long,
        env = "AIRPODS_MIC_LIMITER_DBFS",
        default_value_t = MIC_LIMIT_DBFS,
        value_parser = parse_limiter_dbfs
    )]
    pub mic_limiter_dbfs: f32,
}

impl Cli {
    pub fn init_logging(&self) {
        let default_filter = if self.verbose { "debug" } else { "info" };
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter))
            .format_timestamp_secs()
            .init();
    }
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).with_context(|| format!("invalid Bluetooth MAC address: {value}"))
}

fn parse_gain_db(value: &str) -> Result<f32> {
    let gain_db = value
        .parse::<f32>()
        .with_context(|| format!("invalid microphone gain: {value}"))?;
    validate_mic_settings(gain_db, MIC_LIMIT_DBFS).map_err(anyhow::Error::msg)?;
    Ok(gain_db)
}

fn parse_limiter_dbfs(value: &str) -> Result<f32> {
    let limiter_dbfs = value
        .parse::<f32>()
        .with_context(|| format!("invalid limiter ceiling: {value}"))?;
    validate_mic_settings(MIC_GAIN_DB, limiter_dbfs).map_err(anyhow::Error::msg)?;
    Ok(limiter_dbfs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_mac() {
        assert!(parse_address("F8:1E:49:E9:51:34").is_ok());
    }

    #[test]
    fn rejects_invalid_mac() {
        assert!(parse_address("not-a-mac").is_err());
    }
}
