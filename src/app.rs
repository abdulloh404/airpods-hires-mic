use crate::{
    aacp::AacpSession,
    cli::Cli,
    decoder::EldDecoder,
    dsp::MicProcessor,
    framing::{demux_audio_sdu, is_audio_sdu},
    virtual_mic::VirtualMic,
};
use anyhow::{Context, Result, bail};
use log::{debug, info, warn};
use std::{
    fs::{File, OpenOptions},
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

const RECEIVE_BUFFER_SIZE: usize = 4096;
const FIRST_AUDIO_TIMEOUT: Duration = Duration::from_secs(3);
const STALL_WARNING: Duration = Duration::from_secs(2);
const AUDIO_QUEUE_CAPACITY: usize = 64;
const LOCK_NAME: &str = "airpods-hires-mic.lock";

pub async fn run(cli: Cli) -> Result<()> {
    let _instance_lock = InstanceLock::acquire()?;
    preflight(cli.device.to_string(), !cli.transport_only)?;

    let mut session = AacpSession::connect(cli.device).await?;
    session.initialize().await?;
    session.start_audio().await?;

    let result = if cli.transport_only {
        receive_transport(&session).await
    } else {
        receive_and_decode(&mut session, cli.mic_gain_db, cli.mic_limiter_dbfs).await
    };
    if let Err(error) = session.stop_audio().await {
        warn!("[aacp] cleanup warning: {error:#}");
    }
    result
}

struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    fn acquire() -> Result<Self> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .context("XDG_RUNTIME_DIR is not set; run inside the desktop user session")?;
        let path = PathBuf::from(runtime_dir).join(LOCK_NAME);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(&path)
            .with_context(|| format!("failed to open instance lock: {}", path.display()))?;
        // SAFETY: flock only operates on this live file descriptor.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                bail!("another airpods-hires-mic instance is already running");
            }
            return Err(error).context("failed to lock airpods-hires-mic instance");
        }
        Ok(Self { _file: file })
    }
}

fn preflight(address: String, require_pulse: bool) -> Result<()> {
    require_command("bluetoothctl")?;
    if require_pulse {
        require_command("pactl")?;
        let pulse = Command::new("pactl")
            .arg("info")
            .output()
            .context("failed to run pactl")?;
        if !pulse.status.success() {
            bail!(
                "PipeWire/Pulse server is unreachable: {}",
                String::from_utf8_lossy(&pulse.stderr).trim()
            );
        }
    }

    let device = Command::new("bluetoothctl")
        .args(["info", &address])
        .output()
        .context("failed to run bluetoothctl")?;
    let device_info = String::from_utf8_lossy(&device.stdout);
    if !device.status.success()
        || !device_info
            .lines()
            .any(|line| line.trim() == "Connected: yes")
    {
        bail!("AirPods {address} are not connected through BlueZ");
    }

    if command_exists("pactl") {
        log_active_profile(&address);
    }
    Ok(())
}

fn require_command(command: &str) -> Result<()> {
    if !command_exists(command) {
        bail!("required command is unavailable: {command}");
    }
    Ok(())
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null 2>&1", "sh", command])
        .status()
        .is_ok_and(|status| status.success())
}

fn log_active_profile(address: &str) {
    let card_name = format!("bluez_card.{}", address.replace(':', "_"));
    let Ok(output) = Command::new("pactl").args(["list", "cards"]).output() else {
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut in_card = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Name:") {
            in_card = trimmed.ends_with(&card_name);
        } else if in_card && trimmed.starts_with("Active Profile:") {
            info!("[pw] AirPods {}", trimmed.to_lowercase());
            return;
        }
    }
    warn!("[pw] could not read the AirPods active profile; no profile changes will be made");
}

async fn receive_transport(session: &AacpSession) -> Result<()> {
    let mut buffer = vec![0u8; RECEIVE_BUFFER_SIZE];
    let mut packets = 0u64;
    let mut bytes = 0u64;
    let start = Instant::now();
    let mut last_audio = start;
    let mut first_audio = false;
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.tick().await;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            signal = &mut shutdown => {
                signal?;
                info!("[app] shutdown signal received");
                break;
            }
            _ = ticker.tick() => {
                check_audio_deadline(start, last_audio, first_audio)?;
                if first_audio {
                    info!("[aacp] packets={packets} bytes={bytes}");
                }
            }
            received = session.recv(&mut buffer) => {
                let size = received?;
                if size == 0 {
                    bail!("Bluetooth disconnected");
                }
                if is_audio_sdu(&buffer[..size]) {
                    if !first_audio {
                        info!("[aacp] first 0x58 audio SDU: {size} bytes");
                    }
                    first_audio = true;
                    last_audio = Instant::now();
                    packets += 1;
                    bytes += size as u64;
                } else {
                    debug!("[aacp] ignored non-audio SDU: {size} bytes");
                }
            }
        }
    }
    Ok(())
}

async fn receive_and_decode(
    session: &mut AacpSession,
    mic_gain_db: f32,
    mic_limiter_dbfs: f32,
) -> Result<()> {
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>(AUDIO_QUEUE_CAPACITY);
    let (error_tx, mut error_rx) = mpsc::unbounded_channel::<String>();
    let decoder_task = tokio::task::spawn_blocking(move || {
        decoder_loop(audio_rx, error_tx, mic_gain_db, mic_limiter_dbfs)
    });

    let mut buffer = vec![0u8; RECEIVE_BUFFER_SIZE];
    let mut packets = 0u64;
    let mut dropped = 0u64;
    let start = Instant::now();
    let mut last_audio = start;
    let mut first_audio = false;
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.tick().await;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    let receive_result = loop {
        tokio::select! {
            signal = &mut shutdown => {
                match signal {
                    Ok(()) => {
                        info!("[app] shutdown signal received");
                        break Ok(());
                    }
                    Err(error) => break Err(error),
                }
            }
            Some(error) = error_rx.recv() => break Err(anyhow::anyhow!(error)),
            _ = ticker.tick() => {
                if let Err(error) = check_audio_deadline(start, last_audio, first_audio) {
                    break Err(error);
                }
                if first_audio {
                    info!("[audio] packets={packets} dropped={dropped}");
                }
            }
            received = session.recv(&mut buffer) => {
                let size = match received {
                    Ok(0) => break Err(anyhow::anyhow!("Bluetooth disconnected")),
                    Ok(size) => size,
                    Err(error) => break Err(error),
                };
                if is_audio_sdu(&buffer[..size]) {
                    if !first_audio {
                        info!("[aacp] first 0x58 audio SDU: {size} bytes");
                    }
                    first_audio = true;
                    last_audio = Instant::now();
                    packets += 1;
                    if audio_tx.try_send(buffer[..size].to_vec()).is_err() {
                        dropped += 1;
                    }
                } else {
                    debug!("[aacp] ignored non-audio SDU: {size} bytes");
                }
            }
        }
    };

    if let Err(error) = session.stop_audio().await {
        warn!("[aacp] cleanup warning: {error:#}");
    }
    drop(audio_tx);
    decoder_task.await.context("decoder task panicked")??;
    receive_result
}

fn decoder_loop(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    error_tx: mpsc::UnboundedSender<String>,
    mic_gain_db: f32,
    mic_limiter_dbfs: f32,
) -> Result<()> {
    let result = (|| {
        let mut decoder = EldDecoder::new().context("AAC-ELD decoder init failed")?;
        let mut mic_processor =
            MicProcessor::new(mic_gain_db, mic_limiter_dbfs).map_err(anyhow::Error::msg)?;
        let mut virtual_mic = None;
        let mut frames = 0u64;
        let mut decode_errors = 0u64;
        let mut fifo_drops = 0u64;

        while let Some(sdu) = audio_rx.blocking_recv() {
            let access_units = match demux_audio_sdu(&sdu) {
                Ok(units) => units,
                Err(error) => {
                    debug!("[audio] dropped malformed SDU: {error}");
                    continue;
                }
            };
            for access_unit in access_units {
                let mut frame = match decoder.decode(access_unit) {
                    Ok(frame) => frame,
                    Err(error) => {
                        decode_errors += 1;
                        debug!("[audio] decode error: {error}");
                        continue;
                    }
                };
                if virtual_mic.is_none() {
                    info!(
                        "[audio] format: {} channel(s) / {} Hz PCM clock / {} Hz FDK coding rate / {} samples / AOT {} / {} bps",
                        frame.channels,
                        frame.sample_rate,
                        frame.decoder_sample_rate,
                        frame.frame_size,
                        frame.audio_object_type,
                        frame.bit_rate
                    );
                    info!(
                        "[audio] processing: gain={mic_gain_db:+.1} dB / limiter={mic_limiter_dbfs:.1} dBFS"
                    );
                    virtual_mic = Some(VirtualMic::create(frame.sample_rate, frame.channels)?);
                }
                mic_processor.process(&mut frame.samples);
                if !virtual_mic.as_mut().unwrap().write(&frame.samples)? {
                    fifo_drops += 1;
                }
                frames += 1;
                if frames % 1000 == 0 {
                    info!(
                        "[audio] frames={frames} fifo_dropped={fifo_drops} errors={decode_errors}"
                    );
                }
            }
        }
        if let Some(mut mic) = virtual_mic {
            mic.shutdown()?;
        }
        info!("[audio] stopped: frames={frames} fifo_dropped={fifo_drops} errors={decode_errors}");
        Ok(())
    })();

    if let Err(error) = &result {
        let _ = error_tx.send(format!("{error:#}"));
    }
    result
}

fn check_audio_deadline(start: Instant, last_audio: Instant, first_audio: bool) -> Result<()> {
    if !first_audio && start.elapsed() >= FIRST_AUDIO_TIMEOUT {
        bail!("no AACP 0x58 audio packet received within 3 seconds");
    }
    if first_audio && last_audio.elapsed() >= STALL_WARNING {
        warn!(
            "[aacp] audio stream stalled for {} ms",
            last_audio.elapsed().as_millis()
        );
    }
    Ok(())
}

async fn shutdown_signal() -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate =
        signal(SignalKind::terminate()).context("failed to install SIGTERM handler")?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result.context("failed to install Ctrl+C handler")?,
        _ = terminate.recv() => {}
    }
    Ok(())
}
