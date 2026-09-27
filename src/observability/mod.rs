use shairplay::AudioFormat;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;
use tracing::{info, warn};

#[derive(Clone)]
pub struct ActivityMonitor {
    interval: Duration,
    snapshot_path: Option<PathBuf>,
    started_sessions: Arc<AtomicU64>,
    active_sessions: Arc<AtomicU64>,
    audio_flushes: Arc<AtomicU64>,
    audio_callbacks: Arc<AtomicU64>,
    audio_samples_in: Arc<AtomicU64>,
    backend_samples_out: Arc<AtomicU64>,
    backend_write_errors: Arc<AtomicU64>,
    backend_recoveries: Arc<AtomicU64>,
    alsa_underruns: Arc<AtomicU64>,
    alsa_buffer_frames_last: Arc<AtomicU64>,
    alsa_buffer_frames_max: Arc<AtomicU64>,
    alsa_latency_us_last: Arc<AtomicU64>,
    alsa_latency_us_max: Arc<AtomicU64>,
    metadata_updates: Arc<AtomicU64>,
    connected_clients: Arc<Mutex<HashSet<String>>>,
}

impl ActivityMonitor {
    pub fn new(interval_secs: u64, snapshot_path: Option<String>) -> Self {
        let interval_secs = if interval_secs == 0 {
            warn!("activity interval is 0; defaulting to 30 seconds");
            30
        } else {
            interval_secs
        };

        Self {
            interval: Duration::from_secs(interval_secs),
            snapshot_path: snapshot_path.map(PathBuf::from),
            started_sessions: Arc::new(AtomicU64::new(0)),
            active_sessions: Arc::new(AtomicU64::new(0)),
            audio_flushes: Arc::new(AtomicU64::new(0)),
            audio_callbacks: Arc::new(AtomicU64::new(0)),
            audio_samples_in: Arc::new(AtomicU64::new(0)),
            backend_samples_out: Arc::new(AtomicU64::new(0)),
            backend_write_errors: Arc::new(AtomicU64::new(0)),
            backend_recoveries: Arc::new(AtomicU64::new(0)),
            alsa_underruns: Arc::new(AtomicU64::new(0)),
            alsa_buffer_frames_last: Arc::new(AtomicU64::new(0)),
            alsa_buffer_frames_max: Arc::new(AtomicU64::new(0)),
            alsa_latency_us_last: Arc::new(AtomicU64::new(0)),
            alsa_latency_us_max: Arc::new(AtomicU64::new(0)),
            metadata_updates: Arc::new(AtomicU64::new(0)),
            connected_clients: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn spawn_periodic_logger(&self) -> JoinHandle<()> {
        let monitor = self.clone();
        let interval = self.interval;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                monitor.log_snapshot("periodic");
            }
        })
    }

    pub fn log_snapshot(&self, reason: &str) {
        let connected_clients = match self.connected_clients.lock() {
            Ok(clients) => clients.len() as u64,
            Err(_) => 0,
        };

        info!(
            target: "activity_monitor",
            reason,
            started_sessions = self.started_sessions.load(Ordering::Relaxed),
            active_sessions = self.active_sessions.load(Ordering::Relaxed),
            connected_clients,
            audio_callbacks = self.audio_callbacks.load(Ordering::Relaxed),
            audio_samples_in = self.audio_samples_in.load(Ordering::Relaxed),
            audio_flushes = self.audio_flushes.load(Ordering::Relaxed),
            backend_samples_out = self.backend_samples_out.load(Ordering::Relaxed),
            backend_write_errors = self.backend_write_errors.load(Ordering::Relaxed),
            backend_recoveries = self.backend_recoveries.load(Ordering::Relaxed),
            alsa_underruns = self.alsa_underruns.load(Ordering::Relaxed),
            alsa_buffer_frames_last = self.alsa_buffer_frames_last.load(Ordering::Relaxed),
            alsa_buffer_frames_max = self.alsa_buffer_frames_max.load(Ordering::Relaxed),
            alsa_latency_us_last = self.alsa_latency_us_last.load(Ordering::Relaxed),
            alsa_latency_us_max = self.alsa_latency_us_max.load(Ordering::Relaxed),
            metadata_updates = self.metadata_updates.load(Ordering::Relaxed),
            "activity snapshot"
        );

        self.append_snapshot_jsonl(reason, connected_clients);
    }

    pub fn on_session_started(&self, format: AudioFormat, backend: &'static str) {
        self.started_sessions.fetch_add(1, Ordering::Relaxed);
        self.active_sessions.fetch_add(1, Ordering::Relaxed);

        info!(
            target: "activity_monitor",
            backend,
            codec = ?format.codec,
            bits = format.bits,
            channels = format.channels,
            sample_rate = format.sample_rate,
            started_sessions = self.started_sessions.load(Ordering::Relaxed),
            active_sessions = self.active_sessions.load(Ordering::Relaxed),
            "audio session started"
        );
    }

    pub fn on_audio_callback(&self, sample_count: usize) {
        self.audio_callbacks.fetch_add(1, Ordering::Relaxed);
        self.audio_samples_in
            .fetch_add(sample_count as u64, Ordering::Relaxed);
    }

    pub fn on_audio_flushed(&self) {
        self.audio_flushes.fetch_add(1, Ordering::Relaxed);
        let mut current = self.active_sessions.load(Ordering::Relaxed);
        while current > 0 {
            match self.active_sessions.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn on_backend_samples_written(&self, sample_count: usize) {
        self.backend_samples_out
            .fetch_add(sample_count as u64, Ordering::Relaxed);
    }

    pub fn on_backend_write_error(&self, backend: &'static str, error: &str) {
        self.backend_write_errors.fetch_add(1, Ordering::Relaxed);
        warn!(
            target: "activity_monitor",
            backend,
            error,
            backend_write_errors = self.backend_write_errors.load(Ordering::Relaxed),
            "backend write error"
        );
    }

    pub fn on_backend_recovery(&self, backend: &'static str) {
        self.backend_recoveries.fetch_add(1, Ordering::Relaxed);
        warn!(
            target: "activity_monitor",
            backend,
            backend_recoveries = self.backend_recoveries.load(Ordering::Relaxed),
            "backend recovered from write error"
        );
    }

    pub fn on_metadata_update(&self) {
        self.metadata_updates.fetch_add(1, Ordering::Relaxed);
    }

    pub fn on_alsa_underrun(&self) {
        self.alsa_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn on_alsa_buffer_depth_frames(&self, frames: u64) {
        self.alsa_buffer_frames_last
            .store(frames, Ordering::Relaxed);
        update_max(&self.alsa_buffer_frames_max, frames);
    }

    pub fn on_alsa_latency_us(&self, latency_us: u64) {
        self.alsa_latency_us_last
            .store(latency_us, Ordering::Relaxed);
        update_max(&self.alsa_latency_us_max, latency_us);
    }

    pub fn on_client_connected(&self, addr: &str) {
        if let Ok(mut clients) = self.connected_clients.lock() {
            clients.insert(addr.to_string());
            info!(
                target: "activity_monitor",
                client = addr,
                connected_clients = clients.len(),
                "client connected"
            );
        }
    }

    pub fn on_client_disconnected(&self, addr: &str) {
        if let Ok(mut clients) = self.connected_clients.lock() {
            clients.remove(addr);
            info!(
                target: "activity_monitor",
                client = addr,
                connected_clients = clients.len(),
                "client disconnected"
            );
        }
    }

    fn append_snapshot_jsonl(&self, reason: &str, connected_clients: u64) {
        let Some(path) = &self.snapshot_path else {
            return;
        };

        let ts_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(v) => v.as_millis() as u64,
            Err(_) => 0,
        };

        let payload = serde_json::json!({
            "timestamp_ms": ts_ms,
            "reason": reason,
            "started_sessions": self.started_sessions.load(Ordering::Relaxed),
            "active_sessions": self.active_sessions.load(Ordering::Relaxed),
            "connected_clients": connected_clients,
            "audio_callbacks": self.audio_callbacks.load(Ordering::Relaxed),
            "audio_samples_in": self.audio_samples_in.load(Ordering::Relaxed),
            "audio_flushes": self.audio_flushes.load(Ordering::Relaxed),
            "backend_samples_out": self.backend_samples_out.load(Ordering::Relaxed),
            "backend_write_errors": self.backend_write_errors.load(Ordering::Relaxed),
            "backend_recoveries": self.backend_recoveries.load(Ordering::Relaxed),
            "alsa_underruns": self.alsa_underruns.load(Ordering::Relaxed),
            "alsa_buffer_frames_last": self.alsa_buffer_frames_last.load(Ordering::Relaxed),
            "alsa_buffer_frames_max": self.alsa_buffer_frames_max.load(Ordering::Relaxed),
            "alsa_latency_us_last": self.alsa_latency_us_last.load(Ordering::Relaxed),
            "alsa_latency_us_max": self.alsa_latency_us_max.load(Ordering::Relaxed),
            "metadata_updates": self.metadata_updates.load(Ordering::Relaxed)
        });

        let mut file = match OpenOptions::new().create(true).append(true).open(path) {
            Ok(file) => file,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to open activity snapshot file");
                return;
            }
        };

        if let Err(e) = writeln!(file, "{}", payload) {
            warn!(path = %path.display(), error = %e, "failed to write activity snapshot file");
        }
    }
}

fn update_max(counter: &AtomicU64, value: u64) {
    let mut current = counter.load(Ordering::Relaxed);
    while value > current {
        match counter.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(actual) => current = actual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shairplay::AudioCodec;
    use std::fs;
    use std::io::{BufRead, BufReader};

    fn unique_temp_file(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("shairport-sync-rs-{prefix}-{pid}-{ts}.jsonl"))
    }

    #[test]
    fn jsonl_snapshot_lines_are_valid_and_contain_required_fields() {
        let path = unique_temp_file("activity-snapshots");
        let monitor = ActivityMonitor::new(1, Some(path.to_string_lossy().to_string()));

        let format = AudioFormat {
            codec: AudioCodec::Pcm,
            bits: 32,
            channels: 2,
            sample_rate: 44_100,
        };

        monitor.on_session_started(format, "null");
        monitor.on_audio_callback(4);
        monitor.on_backend_samples_written(4);
        monitor.on_metadata_update();
        monitor.on_client_connected("127.0.0.1:12345");
        monitor.log_snapshot("test-first");

        monitor.on_client_disconnected("127.0.0.1:12345");
        monitor.on_audio_flushed();
        monitor.log_snapshot("test-second");

        let file = fs::File::open(&path).expect("snapshot file should exist");
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .expect("snapshot file should be readable");

        assert!(lines.len() >= 2, "expected at least 2 snapshot lines");

        for line in &lines {
            let value: serde_json::Value =
                serde_json::from_str(line).expect("each line must be valid JSON");

            for key in [
                "timestamp_ms",
                "reason",
                "started_sessions",
                "active_sessions",
                "connected_clients",
                "audio_callbacks",
                "audio_samples_in",
                "audio_flushes",
                "backend_samples_out",
                "backend_write_errors",
                "backend_recoveries",
                "alsa_underruns",
                "alsa_buffer_frames_last",
                "alsa_buffer_frames_max",
                "alsa_latency_us_last",
                "alsa_latency_us_max",
                "metadata_updates",
            ] {
                assert!(value.get(key).is_some(), "missing field: {key}");
            }

            assert!(
                value.get("timestamp_ms").and_then(|v| v.as_u64()).is_some(),
                "timestamp_ms should be a u64"
            );
            assert!(
                value.get("reason").and_then(|v| v.as_str()).is_some(),
                "reason should be a string"
            );
        }

        let _ = fs::remove_file(path);
    }
}
