use alsa::pcm::{Access, Format, HwParams, PCM, State};
use alsa::{Direction, ValueOr};
use shairplay::{AudioFormat, AudioSession};
use std::sync::Arc;
use std::sync::Mutex;
use tracing::warn;

use super::BackendFactory;
use crate::observability::ActivityMonitor;

pub struct AlsaBackend {
    device: Option<String>,
    period_frames: Option<u32>,
    buffer_frames: Option<u32>,
}

impl AlsaBackend {
    pub fn new(device: Option<String>, period_frames: Option<u32>, buffer_frames: Option<u32>) -> Self {
        Self {
            device,
            period_frames,
            buffer_frames,
        }
    }
}

impl BackendFactory for AlsaBackend {
    fn create_session(
        &self,
        format: AudioFormat,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(AlsaSession::new(
            format,
            self.device.as_deref(),
            self.period_frames,
            self.buffer_frames,
            monitor,
        )?))
    }
}

struct AlsaSession {
    pcm: Mutex<PCM>,
    channels: usize,
    sample_rate: u32,
    monitor: Arc<ActivityMonitor>,
}

impl AlsaSession {
    fn new(
        format: AudioFormat,
        device: Option<&str>,
        period_frames: Option<u32>,
        buffer_frames: Option<u32>,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Self, String> {
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
            hwp.set_rate(format.sample_rate, ValueOr::Nearest)
                .map_err(|e| {
                    format!(
                        "failed to set ALSA sample rate to {}: {e}",
                        format.sample_rate
                    )
                })?;

            // Keep a responsive baseline unless overridden by config.
            let period = i64::from(period_frames.unwrap_or(1024).max(1));
            let buffer = i64::from(buffer_frames.unwrap_or(4096).max(1));
            let _ = hwp.set_period_size_near(period, ValueOr::Nearest);
            let _ = hwp.set_buffer_size_near(buffer);

            pcm.hw_params(&hwp)
                .map_err(|e| format!("failed to apply ALSA hw params: {e}"))?;

            // Keep playback responsive: start as soon as frames are available and wake frequently.
            let period_frames = hwp.get_period_size().unwrap_or(1024).max(1);
            let swp = pcm
                .sw_params_current()
                .map_err(|e| format!("failed to read ALSA sw params: {e}"))?;
            swp.set_start_threshold(1)
                .map_err(|e| format!("failed to set ALSA start threshold: {e}"))?;
            swp.set_avail_min(period_frames)
                .map_err(|e| format!("failed to set ALSA avail_min: {e}"))?;
            pcm.sw_params(&swp)
                .map_err(|e| format!("failed to apply ALSA sw params: {e}"))?;
        }

        pcm.prepare()
            .map_err(|e| format!("failed to prepare ALSA device: {e}"))?;

        Ok(Self {
            pcm: Mutex::new(pcm),
            channels: usize::from(format.channels),
            sample_rate: format.sample_rate,
            monitor,
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
                    self.monitor
                        .on_backend_write_error("alsa", "ALSA PCM mutex poisoned");
                    break;
                }
            };

            let io = match pcm.io_f32() {
                Ok(io) => io,
                Err(e) => {
                    warn!(error = %e, "failed to obtain ALSA f32 io handle");
                    self.monitor.on_backend_write_error("alsa", &e.to_string());
                    break;
                }
            };

            match io.writei(chunk) {
                Ok(written_frames) => {
                    if written_frames == 0 {
                        break;
                    }
                    self.monitor
                        .on_backend_samples_written(written_frames * self.channels);

                    if let Ok((_avail_frames, delay_frames)) = pcm.avail_delay()
                        && delay_frames > 0
                    {
                        let depth_frames = delay_frames as u64;
                        self.monitor.on_alsa_buffer_depth_frames(depth_frames);
                        let latency_us = depth_frames.saturating_mul(1_000_000)
                            / u64::from(self.sample_rate.max(1));
                        self.monitor.on_alsa_latency_us(latency_us);
                    }

                    offset += written_frames * self.channels;
                }
                Err(e) => {
                    warn!(error = %e, "ALSA write failed, attempting device prepare");
                    self.monitor.on_backend_write_error("alsa", &e.to_string());
                    self.monitor.on_alsa_underrun();
                    self.monitor.on_alsa_recovery_attempt();

                    if pcm.try_recover(e, true).is_ok() {
                        self.monitor.on_backend_recovery("alsa");
                        continue;
                    }

                    if let Err(prepare_err) = pcm.prepare() {
                        warn!(error = %prepare_err, "ALSA recover prepare failed");
                        self.monitor
                            .on_backend_write_error("alsa", &prepare_err.to_string());
                        self.monitor
                            .on_alsa_recovery_failure(&prepare_err.to_string());
                        break;
                    }

                    if pcm.state() == State::Prepared {
                        let _ = pcm.start();
                    }

                    self.monitor.on_backend_recovery("alsa");
                }
            }
        }
    }

    fn audio_flush(&mut self) {
        if let Ok(pcm) = self.pcm.lock() {
            // AirPlay flush should drop queued audio immediately instead of draining stale frames.
            if let Err(e) = PCM::drop(&pcm) {
                warn!(error = %e, "ALSA drop failed during flush");
                self.monitor.on_backend_write_error("alsa", &e.to_string());
                return;
            }

            if let Err(e) = pcm.prepare() {
                warn!(error = %e, "ALSA prepare failed after flush drop");
                self.monitor.on_backend_write_error("alsa", &e.to_string());
            }
        } else {
            warn!("ALSA PCM mutex poisoned during flush");
            self.monitor
                .on_backend_write_error("alsa", "ALSA PCM mutex poisoned during flush");
        }
    }
}
