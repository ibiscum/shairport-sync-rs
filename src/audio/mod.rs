mod null;
mod pcm;
mod pipe;
mod stdout;

#[cfg(target_os = "linux")]
mod alsa;

use shairplay::{AudioFormat, AudioHandler, AudioSession, TrackMetadata};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

use crate::config::{AirPlayModeConfig, AppConfig, AudioBackend, OutputSampleFormat};
use crate::observability::ActivityMonitor;

trait BackendFactory: Send + Sync {
    fn create_session(
        &self,
        format: AudioFormat,
        monitor: Arc<ActivityMonitor>,
    ) -> Result<Box<dyn AudioSession>, String>;
}

pub fn make_handler(cfg: &AppConfig) -> Arc<dyn AudioHandler> {
    make_handler_with_monitor(cfg, Arc::new(ActivityMonitor::new(30, None)))
}

pub fn make_handler_with_monitor(
    cfg: &AppConfig,
    monitor: Arc<ActivityMonitor>,
) -> Arc<dyn AudioHandler> {
    Arc::new(AppAudioHandler {
        backend: cfg.backend,
        airplay_mode: cfg.airplay_mode,
        output_format: cfg.output_format,
        alsa_device: cfg.alsa_device.clone(),
        alsa_period_frames: cfg.alsa_period_frames,
        alsa_buffer_frames: cfg.alsa_buffer_frames,
        pipe_path: cfg.pipe_path.clone(),
        gain_state: Arc::new(Mutex::new(VolumeState {
            gain: 1.0,
            mode: None,
        })),
        first_ap2_stream_logged: AtomicBool::new(false),
        activity_monitor: monitor,
    })
}

struct AppAudioHandler {
    backend: AudioBackend,
    airplay_mode: AirPlayModeConfig,
    output_format: OutputSampleFormat,
    alsa_device: Option<String>,
    alsa_period_frames: Option<u32>,
    alsa_buffer_frames: Option<u32>,
    pipe_path: Option<String>,
    gain_state: Arc<Mutex<VolumeState>>,
    first_ap2_stream_logged: AtomicBool,
    activity_monitor: Arc<ActivityMonitor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeMode {
    Db,
    Linear,
    Percent,
}

#[derive(Debug, Clone, Copy)]
struct VolumeState {
    gain: f32,
    mode: Option<VolumeMode>,
}

struct GainSession {
    inner: Box<dyn AudioSession>,
    gain_state: Arc<Mutex<VolumeState>>,
    activity_monitor: Arc<ActivityMonitor>,
    scratch: Vec<f32>,
}

impl GainSession {
    fn new(
        inner: Box<dyn AudioSession>,
        gain_state: Arc<Mutex<VolumeState>>,
        activity_monitor: Arc<ActivityMonitor>,
    ) -> Self {
        Self {
            inner,
            gain_state,
            activity_monitor,
            scratch: Vec::new(),
        }
    }
}

impl AudioSession for GainSession {
    fn audio_process(&mut self, samples: &[f32]) {
        self.activity_monitor.on_audio_callback(samples.len());

        let gain = match self.gain_state.lock() {
            Ok(v) => v.gain,
            Err(_) => 1.0,
        };

        if (gain - 1.0).abs() < f32::EPSILON {
            self.inner.audio_process(samples);
            return;
        }

        self.scratch.clear();
        self.scratch.reserve(samples.len());
        self.scratch.extend(
            samples
                .iter()
                .map(|sample| (sample * gain).clamp(-1.0, 1.0)),
        );
        self.inner.audio_process(&self.scratch);
    }

    fn audio_flush(&mut self) {
        self.activity_monitor.on_audio_flushed();
        self.inner.audio_flush();
    }
}

fn backend_label(backend: AudioBackend) -> &'static str {
    match backend {
        AudioBackend::Null => "null",
        AudioBackend::Stdout => "stdout",
        AudioBackend::Pipe => "pipe",
        #[cfg(target_os = "linux")]
        AudioBackend::Alsa => "alsa",
    }
}

fn volume_db_to_linear(volume_db: f32) -> f32 {
    if !volume_db.is_finite() {
        return 1.0;
    }

    // AirPlay volume uses 0.0 dB as full scale and -144.0 dB as mute.
    if volume_db <= -144.0 {
        return 0.0;
    }

    10f32.powf(volume_db / 20.0).clamp(0.0, 1.0)
}

fn volume_linear_to_linear(volume_linear: f32) -> f32 {
    if !volume_linear.is_finite() {
        return 1.0;
    }

    volume_linear.clamp(0.0, 1.0)
}

fn volume_percent_to_linear(volume_percent: f32) -> f32 {
    if !volume_percent.is_finite() {
        return 1.0;
    }

    (volume_percent / 100.0).clamp(0.0, 1.0)
}

fn infer_volume_mode(raw_volume: f32, current_mode: Option<VolumeMode>) -> VolumeMode {
    if let Some(mode) = current_mode {
        return mode;
    }

    // Prefer dB by default because this is the documented shairplay callback format.
    if raw_volume < 0.0 {
        VolumeMode::Db
    } else if (0.0..=1.0).contains(&raw_volume) && raw_volume > 0.0 {
        VolumeMode::Linear
    } else if (1.0..=100.0).contains(&raw_volume) {
        VolumeMode::Percent
    } else {
        VolumeMode::Db
    }
}

fn volume_to_linear(raw_volume: f32, current_mode: Option<VolumeMode>) -> (f32, VolumeMode) {
    let mode = infer_volume_mode(raw_volume, current_mode);
    let gain = match mode {
        VolumeMode::Db => volume_db_to_linear(raw_volume),
        VolumeMode::Linear => volume_linear_to_linear(raw_volume),
        VolumeMode::Percent => volume_percent_to_linear(raw_volume),
    };

    (gain, mode)
}

impl AudioHandler for AppAudioHandler {
    fn audio_init(&self, format: AudioFormat) -> Box<dyn AudioSession> {
        if self.airplay_mode == AirPlayModeConfig::Ap2
            && self
                .first_ap2_stream_logged
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            info!(
                codec = ?format.codec,
                sample_rate = format.sample_rate,
                channels = format.channels,
                "first AP2 stream format observed"
            );
            info!(
                backend = backend_label(self.backend),
                alsa_device = self.alsa_device.as_deref().unwrap_or("n/a"),
                "first AP2 stream output route"
            );
        }

        info!(
            channels = format.channels,
            bits = format.bits,
            sample_rate = format.sample_rate,
            "audio stream initialized"
        );

        let backend = backend_label(self.backend);
        self.activity_monitor.on_session_started(format, backend);

        let factory: Box<dyn BackendFactory> = match self.backend {
            AudioBackend::Null => Box::new(null::NullBackend),
            AudioBackend::Stdout => Box::new(stdout::StdoutBackend::new(self.output_format)),
            AudioBackend::Pipe => Box::new(pipe::PipeBackend::new(
                self.pipe_path.clone(),
                self.output_format,
            )),
            #[cfg(target_os = "linux")]
            AudioBackend::Alsa => Box::new(alsa::AlsaBackend::new(
                self.alsa_device.clone(),
                self.alsa_period_frames,
                self.alsa_buffer_frames,
            )),
        };

        match factory.create_session(format, Arc::clone(&self.activity_monitor)) {
            Ok(session) => Box::new(GainSession::new(
                session,
                Arc::clone(&self.gain_state),
                Arc::clone(&self.activity_monitor),
            )),
            Err(e) => {
                error!(error = %e, "failed to initialize audio backend, falling back to null backend");
                self.activity_monitor.on_backend_write_error(backend, &e);
                let session = null::NullBackend
                    .create_session(format, Arc::clone(&self.activity_monitor))
                    .expect("null backend is infallible");
                Box::new(GainSession::new(
                    session,
                    Arc::clone(&self.gain_state),
                    Arc::clone(&self.activity_monitor),
                ))
            }
        }
    }

    fn on_volume(&self, volume: f32) {
        let mut mode = None;
        let mut gain = 1.0;

        if let Ok(mut current) = self.gain_state.lock() {
            let (new_gain, new_mode) = volume_to_linear(volume, current.mode);
            current.gain = new_gain;
            current.mode = Some(new_mode);
            gain = new_gain;
            mode = Some(new_mode);
        }
        debug!(raw_volume = volume, gain, ?mode, "volume update");

        if gain <= 0.0 {
            debug!(raw_volume = volume, ?mode, "effective gain is mute");
        }
    }

    fn on_metadata(&self, metadata: &TrackMetadata) {
        self.activity_monitor.on_metadata_update();
        debug!(?metadata, "track metadata update");
    }

    fn on_client_connected(&self, addr: &str) {
        self.activity_monitor.on_client_connected(addr);
        info!(client = addr, "client connected");
    }

    fn on_client_disconnected(&self, addr: &str) {
        self.activity_monitor.on_client_disconnected(addr);
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
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
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
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: crate::config::LogFormat::Text,
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
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
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
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: crate::config::LogFormat::Text,
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
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
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
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: crate::config::LogFormat::Text,
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
            alsa_period_frames: Some(1024),
            alsa_buffer_frames: Some(4096),
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
            activity_interval_secs: 30,
            activity_snapshot_path: None,
            log_format: crate::config::LogFormat::Text,
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
            OutputSampleFormat::F32Le => samples.iter().flat_map(|s| s.to_le_bytes()).collect(),
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

    #[test]
    fn volume_db_to_linear_maps_expected_values() {
        assert!((volume_db_to_linear(0.0) - 1.0).abs() < 1e-6);
        assert!((volume_db_to_linear(-6.0) - 0.501_187_2).abs() < 1e-6);
        assert!((volume_db_to_linear(-30.0) - 0.031_622_775).abs() < 1e-6);
        assert!((volume_db_to_linear(-60.0) - 0.001).abs() < 1e-6);
        assert!((volume_db_to_linear(-120.0) - 1.0e-6).abs() < 1e-9);
        assert_eq!(volume_db_to_linear(-144.0), 0.0);
        assert_eq!(volume_db_to_linear(-200.0), 0.0);
        assert_eq!(volume_db_to_linear(f32::INFINITY), 1.0);
    }

    #[test]
    fn volume_to_linear_supports_linear_and_percent_modes() {
        let (gain_linear, mode_linear) = volume_to_linear(0.5, None);
        assert_eq!(mode_linear, VolumeMode::Linear);
        assert!((gain_linear - 0.5).abs() < 1e-6);

        let (gain_percent, mode_percent) = volume_to_linear(50.0, None);
        assert_eq!(mode_percent, VolumeMode::Percent);
        assert!((gain_percent - 0.5).abs() < 1e-6);
    }

    #[test]
    fn volume_to_linear_keeps_selected_mode_for_ambiguous_zero() {
        let (gain_linear_zero, mode_linear) = volume_to_linear(0.0, Some(VolumeMode::Linear));
        assert_eq!(mode_linear, VolumeMode::Linear);
        assert_eq!(gain_linear_zero, 0.0);

        let (gain_db_zero, mode_db) = volume_to_linear(0.0, Some(VolumeMode::Db));
        assert_eq!(mode_db, VolumeMode::Db);
        assert_eq!(gain_db_zero, 1.0);
    }

    #[derive(Default)]
    struct RecordingSession {
        calls: usize,
        last: Vec<f32>,
    }

    impl AudioSession for RecordingSession {
        fn audio_process(&mut self, samples: &[f32]) {
            self.calls += 1;
            self.last.clear();
            self.last.extend_from_slice(samples);
        }
    }

    struct SharedRecordingSession {
        state: Arc<Mutex<RecordingSession>>,
    }

    impl AudioSession for SharedRecordingSession {
        fn audio_process(&mut self, samples: &[f32]) {
            if let Ok(mut state) = self.state.lock() {
                state.audio_process(samples);
            }
        }
    }

    #[test]
    fn gain_session_scales_samples_before_backend() {
        let shared = Arc::new(Mutex::new(RecordingSession::default()));
        let inner: Box<dyn AudioSession> = Box::new(SharedRecordingSession {
            state: Arc::clone(&shared),
        });
        let gain_state = Arc::new(Mutex::new(VolumeState {
            gain: 0.5_f32,
            mode: Some(VolumeMode::Linear),
        }));
        let mut session =
            GainSession::new(inner, gain_state, Arc::new(ActivityMonitor::new(30, None)));

        session.audio_process(&[-1.0, -0.2, 0.2, 1.0]);

        let recorded = shared.lock().expect("recording session mutex poisoned");
        assert_eq!(recorded.calls, 1);
        assert_eq!(recorded.last, vec![-0.5, -0.1, 0.1, 0.5]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_backend_smoke_if_available() {
        let backend = alsa::AlsaBackend::new(Some("default".to_string()), Some(1024), Some(4096));
        let mut session =
            match backend.create_session(test_format(), Arc::new(ActivityMonitor::new(30, None))) {
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
