use anyhow::{Context, Result};
use bluer::Address;
use clap::Parser;
use std::str::FromStr;

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
