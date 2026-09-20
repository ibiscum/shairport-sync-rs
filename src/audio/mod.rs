mod null;
mod pcm;
mod pipe;
mod stdout;

#[cfg(target_os = "linux")]
mod alsa;

use shairplay::{AudioFormat, AudioHandler, AudioSession, TrackMetadata};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::config::{AppConfig, AudioBackend, OutputSampleFormat};

trait BackendFactory: Send + Sync {
    fn create_session(&self, format: AudioFormat) -> Result<Box<dyn AudioSession>, String>;
}

pub fn make_handler(cfg: &AppConfig) -> Arc<dyn AudioHandler> {
    Arc::new(AppAudioHandler {
        backend: cfg.backend,
        output_format: cfg.output_format,
        alsa_device: cfg.alsa_device.clone(),
        pipe_path: cfg.pipe_path.clone(),
    })
}

struct AppAudioHandler {
    backend: AudioBackend,
    output_format: OutputSampleFormat,
    alsa_device: Option<String>,
    pipe_path: Option<String>,
}

impl AudioHandler for AppAudioHandler {
    fn audio_init(&self, format: AudioFormat) -> Box<dyn AudioSession> {
        info!(
            channels = format.channels,
            bits = format.bits,
            sample_rate = format.sample_rate,
            "audio stream initialized"
        );

        let factory: Box<dyn BackendFactory> = match self.backend {
            AudioBackend::Null => Box::new(null::NullBackend),
            AudioBackend::Stdout => Box::new(stdout::StdoutBackend::new(self.output_format)),
            AudioBackend::Pipe => Box::new(pipe::PipeBackend::new(
                self.pipe_path.clone(),
                self.output_format,
            )),
            #[cfg(target_os = "linux")]
            AudioBackend::Alsa => Box::new(alsa::AlsaBackend::new(self.alsa_device.clone())),
        };

        match factory.create_session(format) {
            Ok(session) => session,
            Err(e) => {
                error!(error = %e, "failed to initialize audio backend, falling back to null backend");
                null::NullBackend
                    .create_session(format)
                    .expect("null backend is infallible")
            }
        }
    }

    fn on_volume(&self, volume: f32) {
        debug!(volume_db = volume, "volume update");
    }

    fn on_metadata(&self, metadata: &TrackMetadata) {
        debug!(?metadata, "track metadata update");
    }

    fn on_client_connected(&self, addr: &str) {
        info!(client = addr, "client connected");
    }

    fn on_client_disconnected(&self, addr: &str) {
        info!(client = addr, "client disconnected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_format() -> AudioFormat {
        AudioFormat {
            codec: shairplay::AudioCodec::Pcm,
            bits: 32,
            channels: 2,
            sample_rate: 44_100,
        }
    }

    #[test]
    fn null_backend_init_process_flush() {
        let cfg = AppConfig {
            name: "test".to_string(),
            port: 5000,
            backend: AudioBackend::Null,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            pipe_path: Some("/tmp/shairport-sync-rs-test-null.pcm".to_string()),
            password: None,
            max_clients: 1,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: crate::config::AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
        };

        let handler = make_handler(&cfg);
        let mut session = handler.audio_init(test_format());
        session.audio_process(&[0.0, 0.1, -0.1, 0.0]);
        session.audio_flush();
    }

    #[test]
    fn stdout_backend_init_process_flush() {
        let cfg = AppConfig {
            name: "test".to_string(),
            port: 5000,
            backend: AudioBackend::Stdout,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            pipe_path: Some("/tmp/shairport-sync-rs-test-stdout.pcm".to_string()),
            password: None,
            max_clients: 1,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: crate::config::AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
        };

        let handler = make_handler(&cfg);
        let mut session = handler.audio_init(test_format());
        session.audio_process(&[]);
        session.audio_flush();
    }

    #[test]
    fn pipe_backend_init_process_flush() {
        let cfg = AppConfig {
            name: "test".to_string(),
            port: 5000,
            backend: AudioBackend::Pipe,
            output_format: OutputSampleFormat::F32Le,
            alsa_device: Some("default".to_string()),
            pipe_path: Some("/tmp/shairport-sync-rs-test-pipe.pcm".to_string()),
            password: None,
            max_clients: 1,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: crate::config::AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
        };

        let handler = make_handler(&cfg);
        let mut session = handler.audio_init(test_format());
        session.audio_process(&[0.0, 0.1, -0.1, 0.0]);
        session.audio_flush();
    }

    #[test]
    fn pipe_backend_writes_f32le_bytes_to_file() {
        pipe_backend_writes_expected_bytes(OutputSampleFormat::F32Le);
    }

    #[test]
    fn pipe_backend_writes_s16le_bytes_to_file() {
        pipe_backend_writes_expected_bytes(OutputSampleFormat::S16Le);
    }

    #[test]
    fn pipe_backend_writes_s24le_bytes_to_file() {
        pipe_backend_writes_expected_bytes(OutputSampleFormat::S24Le);
    }

    fn pipe_backend_writes_expected_bytes(format: OutputSampleFormat) {
        let path = unique_temp_file("pipe-bytes");
        let path_str = path.to_string_lossy().to_string();
        let samples = [-1.0_f32, -0.5_f32, 0.0_f32, 0.5_f32, 1.0_f32];

        let cfg = AppConfig {
            name: "test".to_string(),
            port: 5000,
            backend: AudioBackend::Pipe,
            output_format: format,
            alsa_device: Some("default".to_string()),
            pipe_path: Some(path_str.clone()),
            password: None,
            max_clients: 1,
            raop_output_sample_rate: None,
            raop_output_max_channels: None,
            ap1_codecs: None,
            ap1_encryption: None,
            airplay_mode: crate::config::AirPlayModeConfig::Ap2,
            ap2_pin: None,
            ap2_pairing_store_path: None,
        };

        let handler = make_handler(&cfg);
        let mut session = handler.audio_init(test_format());
        session.audio_process(&samples);
        session.audio_flush();

        let actual = fs::read(&path).expect("failed to read pipe output file");
        let expected = expected_bytes(&samples, format);
        assert_eq!(actual, expected);

        let _ = fs::remove_file(path);
    }

    fn unique_temp_file(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("shairport-sync-rs-{prefix}-{pid}-{ts}.pcm"))
    }

    fn expected_bytes(samples: &[f32], format: OutputSampleFormat) -> Vec<u8> {
        match format {
            OutputSampleFormat::F32Le => samples
                .iter()
                .flat_map(|s| s.to_le_bytes())
                .collect(),
            OutputSampleFormat::S16Le => samples
                .iter()
                .flat_map(|s| {
                    let scaled = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
                    scaled.to_le_bytes()
                })
                .collect(),
            OutputSampleFormat::S24Le => samples
                .iter()
                .flat_map(|s| {
                    let scaled = (s.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
                    let b = scaled.to_le_bytes();
                    [b[0], b[1], b[2]]
                })
                .collect(),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_backend_smoke_if_available() {
        let backend = alsa::AlsaBackend::new(Some("default".to_string()));
        let mut session = match backend.create_session(test_format()) {
            Ok(session) => session,
            Err(err) => {
                eprintln!("skipping ALSA smoke test: {err}");
                return;
            }
        };

        session.audio_process(&[]);
        session.audio_flush();
    }
}
