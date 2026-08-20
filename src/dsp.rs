pub const MIC_GAIN_DB: f32 = 18.0;
pub const MIC_LIMIT_DBFS: f32 = -3.0;
pub const MIN_MIC_GAIN_DB: f32 = 0.0;
pub const MAX_MIC_GAIN_DB: f32 = 30.0;
pub const MIN_MIC_LIMIT_DBFS: f32 = -12.0;
pub const MAX_MIC_LIMIT_DBFS: f32 = 0.0;

const PCM_SAMPLE_RATE: f32 = 64_000.0;
const LIMITER_RELEASE_SECONDS: f32 = 0.1;
const I16_PEAK: f32 = i16::MAX as f32;

pub fn validate_mic_settings(gain_db: f32, limiter_dbfs: f32) -> Result<(), String> {
    if !gain_db.is_finite() || !(MIN_MIC_GAIN_DB..=MAX_MIC_GAIN_DB).contains(&gain_db) {
        return Err(format!(
            "microphone gain must be between {MIN_MIC_GAIN_DB:.0} and {MAX_MIC_GAIN_DB:.0} dB"
        ));
    }
    if !limiter_dbfs.is_finite()
        || !(MIN_MIC_LIMIT_DBFS..=MAX_MIC_LIMIT_DBFS).contains(&limiter_dbfs)
    {
        return Err(format!(
            "limiter must be between {MIN_MIC_LIMIT_DBFS:.0} and {MAX_MIC_LIMIT_DBFS:.0} dBFS"
        ));
    }
    Ok(())
}

pub struct MicProcessor {
    gain_linear: f32,
    limit_sample: f32,
    limiter_release_coefficient: f32,
    limiter_gain: f32,
}

impl MicProcessor {
    pub fn new(gain_db: f32, limiter_dbfs: f32) -> Result<Self, String> {
        validate_mic_settings(gain_db, limiter_dbfs)?;
        let limiter_release_coefficient =
            1.0 - (-1.0 / (PCM_SAMPLE_RATE * LIMITER_RELEASE_SECONDS)).exp();
        Ok(Self {
            gain_linear: 10.0_f32.powf(gain_db / 20.0),
            limit_sample: I16_PEAK * 10.0_f32.powf(limiter_dbfs / 20.0),
            limiter_release_coefficient,
            limiter_gain: 1.0,
        })
    }

    pub fn process(&mut self, samples: &mut [i16]) {
        for sample in samples {
            let amplified = f32::from(*sample) * self.gain_linear;
            let required_gain = if amplified.abs() > self.limit_sample {
                self.limit_sample / amplified.abs()
            } else {
                1.0
            };
            if required_gain < self.limiter_gain {
                self.limiter_gain = required_gain;
            } else {
                self.limiter_gain += (1.0 - self.limiter_gain) * self.limiter_release_coefficient;
            }
            *sample = (amplified * self.limiter_gain).round() as i16;
        }
    }
}

impl Default for MicProcessor {
    fn default() -> Self {
        Self::new(MIC_GAIN_DB, MIC_LIMIT_DBFS).expect("default microphone settings are valid")
    }
}
