use anyhow::{Context, Result, bail};
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::{ffi::OsStrExt, fs::FileTypeExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
    process::Command,
};

pub const SOURCE_NAME: &str = "AirPodsHiRes";
pub const SOURCE_DESCRIPTION: &str = "AirPods Hi-Res Mic";
pub const FIFO_NAME: &str = "airpods-hires-mic.fifo";

pub struct VirtualMic {
    fifo_path: PathBuf,
    fifo: File,
    module_id: Option<u32>,
}

impl VirtualMic {
    pub fn create(sample_rate: u32, channels: u8) -> Result<Self> {
        if channels != 1 {
            bail!(
                "AirPods virtual microphone requires mono PCM, decoder reported {channels} channels"
            );
        }
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .context("XDG_RUNTIME_DIR is not set; run inside the desktop user session")?;
        let fifo_path = PathBuf::from(runtime_dir).join(FIFO_NAME);
        unload_named_source(&fifo_path);
        remove_owned_fifo(&fifo_path)?;

        let path = CString::new(fifo_path.as_os_str().as_bytes())
            .context("runtime FIFO path contains a NUL byte")?;
        // SAFETY: path is a valid C string; mode grants access only to the current user.
        if unsafe { libc::mkfifo(path.as_ptr(), 0o600) } != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to create microphone FIFO");
        }

        let fifo = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo_path)
        {
            Ok(file) => file,
            Err(error) => {
                let _ = fs::remove_file(&fifo_path);
                return Err(error).context("failed to open microphone FIFO");
            }
        };

        let args = module_args(&fifo_path, sample_rate);
        let output = Command::new("pactl")
            .args(&args)
            .output()
            .context("failed to run pactl load-module")?;
        if !output.status.success() {
            let _ = fs::remove_file(&fifo_path);
            bail!(
                "module-pipe-source failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let module_id = match String::from_utf8(output.stdout)
            .context("pactl returned a non-UTF-8 module id")
            .and_then(|value| {
                value
                    .trim()
                    .parse::<u32>()
                    .context("pactl returned an invalid module id")
            }) {
            Ok(module_id) => module_id,
            Err(error) => {
                unload_named_source(&fifo_path);
                let _ = fs::remove_file(&fifo_path);
                return Err(error);
            }
        };

        log::info!("[pw] virtual microphone created: {SOURCE_NAME} (module {module_id})");
        Ok(Self {
            fifo_path,
            fifo,
            module_id: Some(module_id),
        })
    }

    pub fn write(&mut self, samples: &[i16]) -> Result<bool> {
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        match self.fifo.write_all(&bytes) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(false),
            Err(error) => Err(error).context("virtual microphone FIFO write failed"),
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        let mut first_error = None;
        if let Some(module_id) = self.module_id.take() {
            match Command::new("pactl")
                .args(["unload-module", &module_id.to_string()])
                .status()
            {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    first_error = Some(anyhow::anyhow!("pactl unload-module exited with {status}"))
                }
                Err(error) => first_error = Some(error.into()),
            }
        }
        if let Err(error) = remove_owned_fifo(&self.fifo_path) {
            first_error.get_or_insert(error);
        }
        if let Some(error) = first_error {
            Err(error.context("virtual microphone cleanup failed"))
        } else {
            Ok(())
        }
    }
}

impl Drop for VirtualMic {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            log::warn!("[pw] {error:#}");
        }
    }
}

fn module_args(fifo_path: &Path, sample_rate: u32) -> Vec<String> {
    vec![
        "load-module".into(),
        "module-pipe-source".into(),
        format!("source_name={SOURCE_NAME}"),
        format!("file={}", fifo_path.display()),
        "format=s16le".into(),
        format!("rate={sample_rate}"),
        "channels=1".into(),
        "channel_map=mono".into(),
        format!("source_properties=device.description=\"{SOURCE_DESCRIPTION}\""),
    ]
}

fn remove_owned_fifo(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_fifo() => {
            fs::remove_file(path).context("failed to remove microphone FIFO")
        }
        Ok(_) => bail!("refusing to remove non-FIFO path: {}", path.display()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("failed to inspect microphone FIFO"),
    }
}

fn unload_named_source(fifo_path: &Path) {
    let Ok(output) = Command::new("pactl")
        .args(["list", "short", "modules"])
        .output()
    else {
        return;
    };
    let expected_file = format!("file={}", fifo_path.display());
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.splitn(4, '\t');
        let (Some(module_id), Some(module_name), Some(module_args)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let owns_source = module_args
            .split_whitespace()
            .any(|argument| argument == format!("source_name={SOURCE_NAME}"));
        let owns_fifo = module_args
            .split_whitespace()
            .any(|argument| argument == expected_file);
        if module_name == "module-pipe-source" && owns_source && owns_fifo {
            let _ = Command::new("pactl")
                .args(["unload-module", module_id])
                .status();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_arguments_do_not_modify_bluetooth_profiles() {
        let args = module_args(Path::new("/run/user/1000/test.fifo"), 64_000);
        let joined = args.join(" ");
        assert!(joined.contains("module-pipe-source"));
        assert!(joined.contains("rate=64000"));
        assert!(!joined.contains("set-card-profile"));
        assert!(!joined.contains("a2dp"));
        assert!(!joined.contains("headset"));
    }
}
