use shairplay::{AudioFormat, AudioSession};
use std::fs::{File, OpenOptions};
use std::sync::Arc;
use std::sync::Mutex;

use super::BackendFactory;
use super::pcm::write_samples;
use crate::config::OutputSampleFormat;
use crate::observability::ActivityMonitor;

pub struct PipeBackend {
    path: Option<String>,
    output_format: OutputSampleFormat,
}

impl PipeBackend {
    pub fn new(path: Option<String>, output_format: OutputSampleFormat) -> Self {
        Self {
            path,
            output_format,
        }
    }
}

impl BackendFactory for PipeBackend {
    fn create_session(
        &self,
        _format: AudioFormat,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Box<dyn AudioSession>, String> {
        let path = self
            .path
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("/tmp/shairport-sync-rs.pcm");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("failed to open pipe output path '{path}': {e}"))?;

        Ok(Box::new(PipeSession {
            writer: Mutex::new(file),
            output_format: self.output_format,
            monitor,
        }))
    }
}

struct PipeSession {
    writer: Mutex<File>,
    output_format: OutputSampleFormat,
    monitor: Arc<ActivityMonitor>,
}

impl AudioSession for PipeSession {
    fn audio_process(&mut self, samples: &[f32]) {
        if let Ok(mut out) = self.writer.lock() {
            match write_samples(&mut *out, samples, self.output_format) {
                Ok(()) => self.monitor.on_backend_samples_written(samples.len()),
                Err(e) => self.monitor.on_backend_write_error("pipe", &e.to_string()),
            }
        } else {
            self.monitor
                .on_backend_write_error("pipe", "pipe writer mutex poisoned");
        }
    }
}
