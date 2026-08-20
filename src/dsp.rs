pub const MIC_GAIN_DB: f32 = 18.0;
pub const MIC_LIMIT_DBFS: f32 = -3.0;

const MIC_GAIN_LINEAR: f32 = 7.943_282;
const MIC_LIMIT_SAMPLE: f32 = 23_197.0;
// Approximately 100 ms at the AirPods 64 kHz PCM clock.
const LIMITER_RELEASE_COEFFICIENT: f32 = 0.000_156;

pub struct MicProcessor {
    limiter_gain: f32,
}

impl MicProcessor {
    pub fn new() -> Self {
        Self { limiter_gain: 1.0 }
    }

    pub fn process(&mut self, samples: &mut [i16]) {
        for sample in samples {
            let amplified = f32::from(*sample) * MIC_GAIN_LINEAR;
            let required_gain = if amplified.abs() > MIC_LIMIT_SAMPLE {
                MIC_LIMIT_SAMPLE / amplified.abs()
            } else {
                1.0
            };
            if required_gain < self.limiter_gain {
                self.limiter_gain = required_gain;
            } else {
                self.limiter_gain += (1.0 - self.limiter_gain) * LIMITER_RELEASE_COEFFICIENT;
            }
            *sample = (amplified * self.limiter_gain).round() as i16;
        }
    }
}

impl Default for MicProcessor {
    fn default() -> Self {
        Self::new()
    }
}
