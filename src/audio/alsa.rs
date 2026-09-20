use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};
use shairplay::{AudioFormat, AudioSession};
use std::sync::Mutex;
use tracing::warn;

use super::BackendFactory;

pub struct AlsaBackend {
    device: Option<String>,
}

impl AlsaBackend {
    pub fn new(device: Option<String>) -> Self {
        Self { device }
    }
}

impl BackendFactory for AlsaBackend {
    fn create_session(&self, format: AudioFormat) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(AlsaSession::new(format, self.device.as_deref())?))
    }
}

struct AlsaSession {
    pcm: Mutex<PCM>,
    channels: usize,
}

impl AlsaSession {
    fn new(format: AudioFormat, device: Option<&str>) -> Result<Self, String> {
        let device_name = device
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("default");

        let pcm = PCM::new(device_name, Direction::Playback, false)
            .map_err(|e| format!("failed to open ALSA device '{device_name}': {e}"))?;

        {
            let hwp =
                HwParams::any(&pcm).map_err(|e| format!("failed to alloc ALSA hw params: {e}"))?;
            hwp.set_access(Access::RWInterleaved)
                .map_err(|e| format!("failed to set ALSA access mode: {e}"))?;
            hwp.set_format(Format::float())
                .map_err(|e| format!("failed to set ALSA sample format to f32le: {e}"))?;
            hwp.set_channels(u32::from(format.channels))
                .map_err(|e| format!("failed to set ALSA channels to {}: {e}", format.channels))?;
            hwp.set_rate(format.sample_rate, ValueOr::Nearest).map_err(|e| {
                format!(
                    "failed to set ALSA sample rate to {}: {e}",
                    format.sample_rate
                )
            })?;
            pcm.hw_params(&hwp)
                .map_err(|e| format!("failed to apply ALSA hw params: {e}"))?;
        }

        pcm.prepare()
            .map_err(|e| format!("failed to prepare ALSA device: {e}"))?;

        Ok(Self {
            pcm: Mutex::new(pcm),
            channels: usize::from(format.channels),
        })
    }
}

impl AudioSession for AlsaSession {
    fn audio_process(&mut self, samples: &[f32]) {
        if self.channels == 0 {
            return;
        }

        let mut offset = 0usize;
        while offset < samples.len() {
            let remaining_samples = samples.len() - offset;
            let remaining_frames = remaining_samples / self.channels;
            if remaining_frames == 0 {
                break;
            }

            let to_write = remaining_frames * self.channels;
            let chunk = &samples[offset..offset + to_write];

            let pcm = match self.pcm.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    warn!("ALSA PCM mutex poisoned");
                    break;
                }
            };

            let io = match pcm.io_f32() {
                Ok(io) => io,
                Err(e) => {
                    warn!(error = %e, "failed to obtain ALSA f32 io handle");
                    break;
                }
            };

            match io.writei(chunk) {
                Ok(written_frames) => {
                    if written_frames == 0 {
                        break;
                    }
                    offset += written_frames * self.channels;
                }
                Err(e) => {
                    warn!(error = %e, "ALSA write failed, attempting device prepare");
                    if let Err(prepare_err) = pcm.prepare() {
                        warn!(error = %prepare_err, "ALSA recover prepare failed");
                        break;
                    }
                }
            }
        }
    }

    fn audio_flush(&mut self) {
        if let Ok(pcm) = self.pcm.lock() {
            if let Err(e) = pcm.drain() {
                warn!(error = %e, "ALSA drain failed during flush");
            }
        } else {
            warn!("ALSA PCM mutex poisoned during flush");
        }
    }
}
