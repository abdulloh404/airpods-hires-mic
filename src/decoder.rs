use anyhow::{Context, Result, bail};
use std::{os::raw::c_int, ptr::NonNull};

const TT_MP4_RAW: c_int = 0;
const AAC_DEC_OK: c_int = 0;
const ELD_ASC: [u8; 4] = [0xF8, 0xE6, 0x30, 0x00];
const PCM_BUFFER_SAMPLES: usize = 8192;
const AIRPODS_ELD_PCM_RATE: u32 = 64_000;

#[repr(C)]
struct AacDecoderInstance {
    _private: [u8; 0],
}

type DecoderHandle = *mut AacDecoderInstance;

#[repr(C)]
struct StreamInfoPrefix {
    sample_rate: c_int,
    frame_size: c_int,
    num_channels: c_int,
    channel_type: *mut c_int,
    channel_indices: *mut u8,
    aac_sample_rate: c_int,
    profile: c_int,
    audio_object_type: c_int,
    channel_config: c_int,
    bit_rate: c_int,
    aac_samples_per_frame: c_int,
    aac_num_channels: c_int,
    extension_audio_object_type: c_int,
    extension_sample_rate: c_int,
}

unsafe extern "C" {
    fn aacDecoder_Open(transport_format: c_int, layers: u32) -> DecoderHandle;
    fn aacDecoder_ConfigRaw(
        decoder: DecoderHandle,
        config: *mut *mut u8,
        length: *const u32,
    ) -> c_int;
    fn aacDecoder_Fill(
        decoder: DecoderHandle,
        buffer: *mut *mut u8,
        buffer_size: *const u32,
        bytes_valid: *mut u32,
    ) -> c_int;
    fn aacDecoder_DecodeFrame(
        decoder: DecoderHandle,
        pcm: *mut i16,
        pcm_size: c_int,
        flags: u32,
    ) -> c_int;
    fn aacDecoder_GetStreamInfo(decoder: DecoderHandle) -> *mut StreamInfoPrefix;
    fn aacDecoder_Close(decoder: DecoderHandle);
}

#[derive(Debug)]
pub struct DecodedFrame {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub decoder_sample_rate: u32,
    pub channels: u8,
    pub frame_size: usize,
    pub audio_object_type: i32,
    pub bit_rate: i32,
}

pub struct EldDecoder {
    handle: NonNull<AacDecoderInstance>,
}

impl EldDecoder {
    pub fn new() -> Result<Self> {
        // SAFETY: FDK returns an opaque decoder owned by this wrapper and closed in Drop.
        let handle = unsafe { aacDecoder_Open(TT_MP4_RAW, 1) };
        let handle = NonNull::new(handle).context("FDK-AAC decoder allocation failed")?;
        let decoder = Self { handle };

        let mut config = ELD_ASC;
        let mut config_ptr = config.as_mut_ptr();
        let config_len = config.len() as u32;
        // SAFETY: pointers reference live buffers for this call and match FDK's one-layer API.
        let status =
            unsafe { aacDecoder_ConfigRaw(decoder.handle.as_ptr(), &mut config_ptr, &config_len) };
        if status != AAC_DEC_OK {
            bail!("FDK-AAC AAC-ELD configuration failed with status 0x{status:04X}");
        }
        Ok(decoder)
    }

    pub fn decode(&mut self, access_unit: &[u8]) -> Result<DecodedFrame> {
        if access_unit.is_empty() {
            bail!("cannot decode an empty AAC-ELD access unit");
        }

        let mut input = access_unit.to_vec();
        let mut input_ptr = input.as_mut_ptr();
        let input_len = input.len() as u32;
        let mut bytes_valid = input_len;
        // SAFETY: FDK only reads/copies from the input during this call.
        let fill_status = unsafe {
            aacDecoder_Fill(
                self.handle.as_ptr(),
                &mut input_ptr,
                &input_len,
                &mut bytes_valid,
            )
        };
        if fill_status != AAC_DEC_OK {
            bail!("FDK-AAC input failed with status 0x{fill_status:04X}");
        }
        if bytes_valid != 0 {
            bail!("FDK-AAC did not consume {bytes_valid} input bytes");
        }

        let mut samples = vec![0i16; PCM_BUFFER_SAMPLES];
        // SAFETY: output buffer holds PCM_BUFFER_SAMPLES i16 values as required by FDK.
        let decode_status = unsafe {
            aacDecoder_DecodeFrame(
                self.handle.as_ptr(),
                samples.as_mut_ptr(),
                samples.len() as c_int,
                0,
            )
        };
        if decode_status != AAC_DEC_OK {
            bail!("FDK-AAC decode failed with status 0x{decode_status:04X}");
        }

        // SAFETY: the stream-info pointer is owned by the live decoder and valid until its next call.
        let info = unsafe { aacDecoder_GetStreamInfo(self.handle.as_ptr()).as_ref() }
            .context("FDK-AAC returned no stream information")?;
        if info.sample_rate <= 0 || info.frame_size <= 0 || info.num_channels <= 0 {
            bail!("FDK-AAC returned an invalid PCM format");
        }
        let sample_count = info.frame_size as usize * info.num_channels as usize;
        if sample_count > samples.len() {
            bail!("decoded PCM frame is larger than the output buffer");
        }
        samples.truncate(sample_count);

        Ok(DecodedFrame {
            samples,
            // AirPods deliver 480 samples every 7.5 ms (64 kHz PCM clock).
            // FDK reports the 48 kHz AAC coding rate for this ELD configuration.
            sample_rate: AIRPODS_ELD_PCM_RATE,
            decoder_sample_rate: info.sample_rate as u32,
            channels: u8::try_from(info.num_channels).context("invalid channel count")?,
            frame_size: info.frame_size as usize,
            audio_object_type: info.audio_object_type,
            bit_rate: info.bit_rate,
        })
    }
}

impl Drop for EldDecoder {
    fn drop(&mut self) {
        // SAFETY: this is the unique live handle owned by this wrapper.
        unsafe { aacDecoder_Close(self.handle.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_with_airpods_eld_config() {
        EldDecoder::new().unwrap();
    }
}
