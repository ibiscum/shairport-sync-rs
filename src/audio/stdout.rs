use shairplay::{AudioFormat, AudioSession};
use std::io;
use std::sync::Mutex;

use super::BackendFactory;
use super::pcm::write_samples;
use crate::config::OutputSampleFormat;

pub struct StdoutBackend {
    output_format: OutputSampleFormat,
}

impl StdoutBackend {
    pub fn new(output_format: OutputSampleFormat) -> Self {
        Self { output_format }
    }
}

impl BackendFactory for StdoutBackend {
    fn create_session(&self, _format: AudioFormat) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(StdoutSession::new(self.output_format)))
    }
}

struct StdoutSession {
    writer: Mutex<io::Stdout>,
    output_format: OutputSampleFormat,
}

impl StdoutSession {
    fn new(output_format: OutputSampleFormat) -> Self {
        Self {
            writer: Mutex::new(io::stdout()),
            output_format,
        }
    }
}

impl AudioSession for StdoutSession {
    fn audio_process(&mut self, samples: &[f32]) {
        // M0 scaffold backend: raw f32le PCM frames written to stdout.
        if let Ok(mut out) = self.writer.lock() {
            write_samples(&mut *out, samples, self.output_format);
        }
    }
}
