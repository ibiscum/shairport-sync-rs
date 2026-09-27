use shairplay::{AudioFormat, AudioSession};
use std::io;
use std::sync::Arc;
use std::sync::Mutex;

use super::BackendFactory;
use super::pcm::write_samples;
use crate::config::OutputSampleFormat;
use crate::observability::ActivityMonitor;

pub struct StdoutBackend {
    output_format: OutputSampleFormat,
}

impl StdoutBackend {
    pub fn new(output_format: OutputSampleFormat) -> Self {
        Self { output_format }
    }
}

impl BackendFactory for StdoutBackend {
    fn create_session(
        &self,
        _format: AudioFormat,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Box<dyn AudioSession>, String> {
        Ok(Box::new(StdoutSession::new(self.output_format, monitor)))
    }
}

struct StdoutSession {
    writer: Mutex<io::Stdout>,
    output_format: OutputSampleFormat,
    monitor: Arc<ActivityMonitor>,
}

impl StdoutSession {
    fn new(output_format: OutputSampleFormat, monitor: Arc<ActivityMonitor>) -> Self {
        Self {
            writer: Mutex::new(io::stdout()),
            output_format,
            monitor,
        }
    }
}

impl AudioSession for StdoutSession {
    fn audio_process(&mut self, samples: &[f32]) {
        // M0 scaffold backend: raw f32le PCM frames written to stdout.
        if let Ok(mut out) = self.writer.lock() {
            match write_samples(&mut *out, samples, self.output_format) {
                Ok(()) => self.monitor.on_backend_samples_written(samples.len()),
                Err(e) => self
                    .monitor
                    .on_backend_write_error("stdout", &e.to_string()),
            }
        } else {
            self.monitor
                .on_backend_write_error("stdout", "stdout writer mutex poisoned");
        }
    }
}
