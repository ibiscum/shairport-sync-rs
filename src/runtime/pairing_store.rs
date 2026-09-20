use serde::{Deserialize, Serialize};
use shairplay::PairingStore;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PairingFileState {
    identity_seed_hex: Option<String>,
    paired_devices_hex: BTreeMap<String, String>,
}

pub struct FilePairingStore {
    path: PathBuf,
    state: Mutex<PairingFileState>,
}

impl FilePairingStore {
    pub fn load_or_create(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let state = if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| {
                format!("failed to read AP2 pairing store file {}: {e}", path.display())
            })?;
            toml::from_str::<PairingFileState>(&content).map_err(|e| {
                format!(
                    "failed to parse AP2 pairing store file {}: {e}",
                    path.display()
                )
            })?
        } else {
            PairingFileState::default()
        };

        let store = Self {
            path,
            state: Mutex::new(state),
        };

        if !store.path.exists() {
            store.persist_snapshot()?;
        }

        Ok(store)
    }

    fn persist_snapshot(&self) -> Result<(), String> {
        let snapshot = self
            .state
            .lock()
            .map_err(|_| "failed to lock AP2 pairing store state".to_string())?
            .clone();
        self.persist_state(&snapshot)
    }

    fn persist_state(&self, state: &PairingFileState) -> Result<(), String> {
        let serialized = toml::to_string(state)
            .map_err(|e| format!("failed to serialize AP2 pairing store: {e}"))?;

        ensure_parent_dir(&self.path)?;
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, serialized).map_err(|e| {
            format!(
                "failed to write AP2 pairing store temp file {}: {e}",
                tmp_path.display()
            )
        })?;
        fs::rename(&tmp_path, &self.path).map_err(|e| {
            format!(
                "failed to atomically replace AP2 pairing store file {}: {e}",
                self.path.display()
            )
        })
    }
}

impl PairingStore for FilePairingStore {
    fn get(&self, device_id: &str) -> Option<[u8; 32]> {
        let state = self.state.lock().ok()?;
        let hex = state.paired_devices_hex.get(device_id)?;
        decode_hex_32(hex)
    }

    fn put(&self, device_id: &str, public_key: [u8; 32]) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => {
                warn!("failed to lock AP2 pairing store for put");
                return;
            }
        };
        state
            .paired_devices_hex
            .insert(device_id.to_string(), encode_hex_32(public_key));
        if let Err(e) = self.persist_state(&state) {
            warn!(error = %e, "failed to persist AP2 pairing store after put");
        }
    }

    fn remove(&self, device_id: &str) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => {
                warn!("failed to lock AP2 pairing store for remove");
                return;
            }
        };
        state.paired_devices_hex.remove(device_id);
        if let Err(e) = self.persist_state(&state) {
            warn!(error = %e, "failed to persist AP2 pairing store after remove");
        }
    }

    fn has_any_pairing(&self) -> bool {
        self.state
            .lock()
            .map(|s| !s.paired_devices_hex.is_empty())
            .unwrap_or(false)
    }

    fn load_identity(&self) -> Option<[u8; 32]> {
        let state = self.state.lock().ok()?;
        decode_hex_32(state.identity_seed_hex.as_deref()?)
    }

    fn save_identity(&self, seed: [u8; 32]) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => {
                warn!("failed to lock AP2 pairing store for save_identity");
                return;
            }
        };
        state.identity_seed_hex = Some(encode_hex_32(seed));
        if let Err(e) = self.persist_state(&state) {
            warn!(error = %e, "failed to persist AP2 pairing store after save_identity");
        }
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create AP2 pairing store directory {}: {e}", parent.display()))?;
    }
    Ok(())
}

fn encode_hex_32(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(nibble_to_hex((b >> 4) & 0x0F));
        out.push(nibble_to_hex(b & 0x0F));
    }
    out
}

fn decode_hex_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }

    let mut out = [0u8; 32];
    let bytes = s.as_bytes();
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = hex_to_nibble(bytes[i * 2])?;
        let lo = hex_to_nibble(bytes[i * 2 + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

fn nibble_to_hex(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        _ => (b'a' + (v - 10)) as char,
    }
}

fn hex_to_nibble(v: u8) -> Option<u8> {
    match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(v - b'a' + 10),
        b'A'..=b'F' => Some(v - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_file(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("shairport-sync-rs-{prefix}-{pid}-{ts}.toml"))
    }

    #[test]
    fn persists_pairing_and_identity_across_reloads() {
        let path = unique_temp_file("ap2-pairing-store");

        let store = FilePairingStore::load_or_create(path.clone()).expect("create store");
        let key = [3u8; 32];
        let seed = [9u8; 32];
        store.put("dev-1", key);
        store.save_identity(seed);

        let reloaded = FilePairingStore::load_or_create(path.clone()).expect("reload store");
        assert_eq!(reloaded.get("dev-1"), Some(key));
        assert_eq!(reloaded.load_identity(), Some(seed));
        assert!(reloaded.has_any_pairing());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn remove_deletes_pairing() {
        let path = unique_temp_file("ap2-pairing-remove");

        let store = FilePairingStore::load_or_create(path.clone()).expect("create store");
        store.put("dev-1", [7u8; 32]);
        assert!(store.has_any_pairing());

        store.remove("dev-1");
        assert_eq!(store.get("dev-1"), None);
        assert!(!store.has_any_pairing());

        let _ = fs::remove_file(path);
    }
}
