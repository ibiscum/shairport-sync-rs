use shairplay::{AudioFormat, AudioSession};
use std::sync::Arc;

use super::BackendFactory;
use crate::observability::ActivityMonitor;

pub struct NullBackend;

impl BackendFactory for NullBackend {
    fn create_session(
        &self,
        _format: AudioFormat,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(NullSession {
            sample_count: 0,
            monitor,
        }))
    }
}

struct NullSession {
    sample_count: usize,
    monitor: Arc<ActivityMonitor>,
}

impl AudioSession for NullSession {
    fn audio_process(&mut self, samples: &[f32]) {
        self.sample_count += samples.len();
        self.monitor.on_backend_samples_written(samples.len());
    }

    fn audio_flush(&mut self) {
        self.sample_count = 0;
    }
}
