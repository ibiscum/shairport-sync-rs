use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

pub struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    pub fn acquire(name: &str, port: u16) -> Result<Self, String> {
        let path = lock_path(name, port);
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("failed to open lock file {}: {e}", path.display()))?;

        file.try_lock_exclusive().map_err(|_| {
            format!(
                "another shairport-sync-rs instance is already running (lock file: {})",
                path.display()
            )
        })?;

        // Best-effort metadata for operators debugging stale processes.
        let _ = file.set_len(0);
        let _ = file.seek(SeekFrom::Start(0));
        let _ = writeln!(file, "pid={}", std::process::id());

        Ok(Self { _file: file })
    }
}

fn lock_path(name: &str, port: u16) -> PathBuf {
    let sanitized = sanitize_name(name);
    std::env::temp_dir().join(format!("shairport-sync-rs-{sanitized}-{port}.lock"))
}

fn sanitize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "instance".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_port_and_name_scoped_lock_file_path() {
        let path = lock_path("Living Room", 5000);
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
        assert!(name.contains("living-room"));
        assert!(name.contains("5000"));
    }

    #[test]
    fn sanitize_name_keeps_ascii_alnum() {
        assert_eq!(sanitize_name("AP_2 Receiver!"), "ap-2-receiver");
    }
}
