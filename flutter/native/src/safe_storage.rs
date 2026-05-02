//! Atomic file write with backup recovery.
//!
//! Prevents config corruption if the process crashes mid-write (power loss,
//! OOM killer, SIGKILL). Inspired by Anytype's `SafeStorage`.
//!
//! Strategy:
//!   1. Write data to `{path}.tmp`
//!   2. `fsync` the temp file to flush OS buffers to disk
//!   3. Back up the current file to `{path}.bak` (best-effort)
//!   4. Atomically rename `{path}.tmp` → `{path}`
//!
//! On startup, call `recover_or_load` to detect orphan `.tmp` files from a
//! previous crash and recover automatically.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Write `data` to `path` atomically.
///
/// If the target file already exists, it is backed up to `{path}.bak` before
/// being replaced. The temporary file `{path}.tmp` is used as the write
/// staging area and is `fsync`'d before the rename.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let bak_path = path.with_extension("bak");

    // Step 1: Write data to .tmp file
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    // Step 2: Back up existing file (best-effort)
    if path.exists() {
        // On macOS/Linux, rename is atomic on the same filesystem.
        // We use copy + remove as a safer fallback in case rename fails
        // across filesystem boundaries (unlikely but defensive).
        if let Err(e) = fs::copy(path, &bak_path) {
            // Non-fatal: log and continue — the tmp file is already synced
            eprintln!("[safe_storage] backup copy failed (non-fatal): {}", e);
        }
    }

    // Step 3: Atomic rename .tmp → target
    fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Read JSON content from `path`, recovering from orphan `.tmp` files.
///
/// Recovery priority:
///   1. If `.tmp` exists and is valid → use it (process crashed after sync
///      but before rename)
///   2. If main file exists and is valid → use it
///   3. If `.bak` exists and is valid → use it
///   4. Return `None` (caller decides default)
pub fn recover_or_load(path: &Path) -> Option<String> {
    let tmp_path = path.with_extension("tmp");
    let bak_path = path.with_extension("bak");

    // Check for orphan .tmp file (crash between sync and rename)
    if tmp_path.exists() {
        if let Ok(content) = fs::read_to_string(&tmp_path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                // .tmp has valid JSON — try to finalize the rename
                if fs::rename(&tmp_path, path).is_ok() {
                    return Some(content);
                }
                // Rename failed (permissions, cross-device) — return content
                // directly; caller can use it as-is.
                return Some(content);
            }
        }
        // .tmp exists but is invalid/corrupt — discard it
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
                // Restore backup to main file
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
        let data = b"{\"key\":\"value\"}";

        write_atomic(&path, data).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "{\"key\":\"value\"}");
    }

    #[test]
    fn test_write_atomic_creates_backup() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        // Write initial content
        fs::write(&path, b"{\"v\":1}").unwrap();

        // Overwrite atomically
        write_atomic(&path, b"{\"v\":2}").unwrap();

        // Main file should have new content
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\":2}");

        // Backup should have old content
        let bak = path.with_extension("bak");
        assert_eq!(fs::read_to_string(&bak).unwrap(), "{\"v\":1}");
    }

    #[test]
    fn test_recover_from_orphan_tmp() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let tmp = path.with_extension("tmp");

        // Simulate crash: .tmp exists with valid data, main file missing
        fs::write(&tmp, b"{\"recovered\":true}").unwrap();

        let result = recover_or_load(&path);
        assert!(result.is_some());
        assert!(result.unwrap().contains("recovered"));

        // Main file should now exist (recovered)
        assert!(path.exists());
    }

    #[test]
    fn test_recover_from_backup() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let bak = path.with_extension("bak");

        // Simulate: main file is corrupt, backup is valid
        fs::write(&path, b"NOT JSON").unwrap();
        fs::write(&bak, b"{\"from_backup\":true}").unwrap();

        let result = recover_or_load(&path);
        assert!(result.is_some());
        assert!(result.unwrap().contains("from_backup"));
    }

    #[test]
    fn test_recover_returns_none_when_nothing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        assert!(recover_or_load(&path).is_none());
    }

    #[test]
    fn test_write_atomic_cleans_tmp_on_success() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        write_atomic(&path, b"{\"ok\":true}").unwrap();

        let tmp = path.with_extension("tmp");
        assert!(!tmp.exists(), "tmp file should be removed after rename");
    }
}
