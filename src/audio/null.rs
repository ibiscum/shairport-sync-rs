use shairplay::{AudioFormat, AudioSession};

use super::BackendFactory;

pub struct NullBackend;

impl BackendFactory for NullBackend {
    fn create_session(&self, _format: AudioFormat) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(NullSession::default()))
    }
}

#[derive(Default)]
struct NullSession {
    sample_count: usize,
}

impl AudioSession for NullSession {
    fn audio_process(&mut self, samples: &[f32]) {
        self.sample_count += samples.len();
    }

    fn audio_flush(&mut self) {
        self.sample_count = 0;
    }
}
