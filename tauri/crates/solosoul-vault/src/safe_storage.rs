//! Atomic file write with backup recovery.
//! Prevents config corruption if the process crashes mid-write.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Write data to path atomically with .tmp + rename strategy
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    // Write to temp file
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    // Backup existing file (best-effort)
    if path.exists() {
        let bak_path = path.with_extension("bak");
        let _ = fs::copy(path, &bak_path);
    }
    // Atomic rename
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Read from path, recovering from orphan .tmp or .bak files
pub fn recover_or_load(path: &Path) -> Option<String> {
    let tmp_path = path.with_extension("tmp");
    let bak_path = path.with_extension("bak");

    // Check for orphan .tmp file (crash between sync and rename)
    if tmp_path.exists() {
        if let Ok(content) = fs::read_to_string(&tmp_path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                let _ = fs::rename(&tmp_path, path);
                return Some(content);
            }
        }
        let _ = fs::remove_file(&tmp_path);
    }

    // Try main file
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                return Some(content);
            }
        }
    }

    // Fall back to backup
    if bak_path.exists() {
        if let Ok(content) = fs::read_to_string(&bak_path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                let _ = fs::copy(&bak_path, path);
                return Some(content);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_atomic_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        write_atomic(&path, b"{\"key\":\"value\"}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"key\":\"value\"}");
    }

    #[test]
    fn test_recover_from_orphan_tmp() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, b"{\"recovered\":true}").unwrap();
        let result = recover_or_load(&path);
        assert!(result.is_some());
        assert!(path.exists());
    }

    #[test]
    fn test_recover_returns_none_when_nothing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        assert!(recover_or_load(&path).is_none());
    }
}
