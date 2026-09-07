use anyhow::{Context, Result, anyhow, bail};
use libspa::{
    pod::{Object, Property, PropertyFlags, Value, serialize::PodSerializer},
    utils::Id,
};
use libspa_sys as spa_sys;
use pipewire as pw;
use pw::prelude::*;
use std::{
    collections::VecDeque,
    io::Cursor,
    mem::size_of,
    sync::{Arc, Mutex, TryLockError, mpsc::SyncSender},
    thread::{self, JoinHandle},
};

pub const SOURCE_NAME: &str = "Microphone_Virtual_Abdullohs_AirPods_Pro";
pub const SOURCE_DESCRIPTION: &str = "Microphone virtual - Abdulloh's AirPods Pro";

const MAX_QUEUED_AUDIO_MILLISECONDS: usize = 250;
const BYTES_PER_SAMPLE: usize = size_of::<i16>();

type StartupResult = std::result::Result<(), String>;
type AudioQueue = Arc<Mutex<VecDeque<i16>>>;

pub struct VirtualMic {
    samples: AudioQueue,
    max_queued_samples: usize,
    shutdown_sender: Option<pw::channel::Sender<()>>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl VirtualMic {
    pub fn create(sample_rate: u32, channels: u8) -> Result<Self> {
        if channels != 1 {
            bail!(
                "AirPods virtual microphone requires mono PCM, decoder reported {channels} channels"
            );
        }
        if sample_rate == 0 {
            bail!("AirPods virtual microphone requires a non-zero sample rate");
        }

        let max_queued_samples =
            (sample_rate as usize).saturating_mul(MAX_QUEUED_AUDIO_MILLISECONDS) / 1_000;
        let samples = Arc::new(Mutex::new(VecDeque::with_capacity(max_queued_samples)));
        let process_samples = Arc::clone(&samples);
        let (shutdown_sender, shutdown_receiver) = pw::channel::channel();
        let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("airpods-pipewire-source".into())
            .spawn(move || {
                let result = run_pipewire_source(
                    sample_rate,
                    process_samples,
                    shutdown_receiver,
                    &ready_sender,
                );
                if let Err(error) = &result {
                    let _ = ready_sender.try_send(Err(format!("{error:#}")));
                }
                result
            })
            .context("failed to start PipeWire virtual microphone thread")?;

        let ready = match ready_receiver.recv() {
            Ok(ready) => ready,
            Err(error) => {
                let _ = thread.join();
                return Err(error).context("PipeWire virtual microphone stopped during startup");
            }
        };
        if let Err(error) = ready {
            let _ = thread.join();
            bail!("failed to create PipeWire virtual microphone: {error}");
        }

        log::info!("[pw] virtual microphone created: {SOURCE_DESCRIPTION}");
        Ok(Self {
            samples,
            max_queued_samples,
            shutdown_sender: Some(shutdown_sender),
            thread: Some(thread),
        })
    }

    pub fn write(&mut self, samples: &[i16]) -> Result<bool> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            bail!("PipeWire virtual microphone thread is not running");
        }

        match self.samples.try_lock() {
            Ok(mut queued) => {
                if samples.len() > self.max_queued_samples.saturating_sub(queued.len()) {
                    return Ok(false);
                }
                queued.extend(samples.iter().copied());
                Ok(true)
            }
            Err(TryLockError::WouldBlock) => Ok(false),
            Err(TryLockError::Poisoned(_)) => {
                bail!("PipeWire virtual microphone audio queue is unavailable")
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }
        if let Some(thread) = self.thread.take() {
            match thread.join() {
                Ok(result) => result.context("PipeWire virtual microphone stopped with an error"),
                Err(_) => bail!("PipeWire virtual microphone thread panicked"),
            }
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

fn run_pipewire_source(
    sample_rate: u32,
    samples: AudioQueue,
    shutdown_receiver: pw::channel::Receiver<()>,
    ready_sender: &SyncSender<StartupResult>,
) -> Result<()> {
    let mainloop = pw::MainLoop::new().context("failed to create the PipeWire main loop")?;
    let _shutdown_receiver = shutdown_receiver.attach(&mainloop, {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });
    let stream_error = Arc::new(Mutex::new(None));
    let listener_error = Arc::clone(&stream_error);
    let error_mainloop = mainloop.clone();
    let stream = pw::stream::Stream::with_user_data(
        &mainloop,
        SOURCE_NAME,
        pw::properties! {
            *pw::keys::NODE_NAME => SOURCE_NAME,
            *pw::keys::NODE_NICK => SOURCE_DESCRIPTION,
            *pw::keys::NODE_DESCRIPTION => SOURCE_DESCRIPTION,
            *pw::keys::DEVICE_DESCRIPTION => SOURCE_DESCRIPTION,
            *pw::keys::MEDIA_CLASS => "Audio/Source",
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::NODE_VIRTUAL => "true",
            *pw::keys::NODE_AUTOCONNECT => "false",
            *pw::keys::NODE_ALWAYS_PROCESS => "true",
            *pw::keys::NODE_PAUSE_ON_IDLE => "false",
        },
        samples,
    )
    .state_changed(move |_, state| {
        if let pw::stream::StreamState::Error(error) = state {
            log::error!("[pw] virtual microphone stream error: {error}");
            if let Ok(mut stream_error) = listener_error.lock() {
                *stream_error = Some(error);
            }
            error_mainloop.quit();
        }
    })
    .process(process_stream_buffer)
    .create()
    .context("failed to create the PipeWire source stream")?;

    let pipewire_sample_rate =
        i32::try_from(sample_rate).context("microphone sample rate is too large for PipeWire")?;
    let format_object = Object {
        type_: spa_sys::SPA_TYPE_OBJECT_Format,
        id: spa_sys::SPA_PARAM_EnumFormat,
        properties: vec![
            pod_property(
                spa_sys::SPA_FORMAT_mediaType,
                Value::Id(Id(spa_sys::SPA_MEDIA_TYPE_audio)),
            ),
            pod_property(
                spa_sys::SPA_FORMAT_mediaSubtype,
                Value::Id(Id(spa_sys::SPA_MEDIA_SUBTYPE_raw)),
            ),
            pod_property(
                spa_sys::SPA_FORMAT_AUDIO_format,
                Value::Id(Id(spa_sys::SPA_AUDIO_FORMAT_S16_LE)),
            ),
            pod_property(
                spa_sys::SPA_FORMAT_AUDIO_rate,
                Value::Int(pipewire_sample_rate),
            ),
            pod_property(spa_sys::SPA_FORMAT_AUDIO_channels, Value::Int(1)),
        ],
    };
    let format_bytes =
        PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Object(format_object))
            .map_err(|error| anyhow!("failed to serialize PipeWire audio format: {error:?}"))?
            .0
            .into_inner();
    let mut params = [format_bytes.as_ptr().cast::<spa_sys::spa_pod>()];

    stream
        .connect(
            pw::spa::Direction::Output,
            None,
            pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .context("failed to connect the PipeWire source stream")?;
    ready_sender
        .send(Ok(()))
        .map_err(|_| anyhow!("virtual microphone startup receiver disconnected"))?;

    mainloop.run();
    if let Some(error) = stream_error
        .lock()
        .map_err(|_| anyhow!("PipeWire stream error state is unavailable"))?
        .take()
    {
        bail!("PipeWire virtual microphone stream failed: {error}");
    }
    Ok(())
}

fn pod_property(key: u32, value: Value) -> Property {
    Property {
        key,
        flags: PropertyFlags::empty(),
        value,
    }
}

fn process_stream_buffer(stream: &pw::stream::Stream<AudioQueue>, samples: &mut AudioQueue) {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let Some(data) = buffer.datas_mut().first_mut() else {
        return;
    };
    let size = match data.data() {
        Some(output) => {
            output.fill(0);
            let size = output.len() - (output.len() % BYTES_PER_SAMPLE);
            if let Ok(mut queued) = samples.try_lock() {
                for bytes in output[..size].chunks_exact_mut(BYTES_PER_SAMPLE) {
                    let Some(sample) = queued.pop_front() else {
                        break;
                    };
                    bytes.copy_from_slice(&sample.to_le_bytes());
                }
            }
            size
        }
        None => 0,
    };
    let chunk = data.chunk_mut();
    *chunk.offset_mut() = 0;
    *chunk.stride_mut() = BYTES_PER_SAMPLE as i32;
    *chunk.size_mut() = size as u32;
}
