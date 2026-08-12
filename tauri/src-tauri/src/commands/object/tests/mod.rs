use solosoul_vault::{VaultConfig, VaultStore};
use tempfile::TempDir;

fn setup_vault() -> (VaultStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let config =
        VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
    let vault = VaultStore::open(config).unwrap();
    (vault, dir)
}

// ── 主题子模块（P047 拆分）──────────────────
mod crud;
mod misc;
mod snapshot;
mod template_sync;
mod trash;
