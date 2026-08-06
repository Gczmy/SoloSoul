//! LLM Context Service — public_data_version 维护
//!
//! 原系统提示词构建（`build_context`，模式 B 后端构建）已无任何生产调用方
//! （整棵私有子树约 330 行仅被模块内测试引用），于 R2-11 连同其内存缓存层
//! （`PROMPT_CACHE`/`clear_cache`）一并移除。
//!
//! 本模块保留仍被活跃路径使用的 public_data_version 机制：
//! - `bump_public_data_version`：object_create / object_update 检测到 public
//!   级别变更时调用（`commands/object/mod.rs`）；
//! - 配套的 profile 读写辅助。

use super::profile_prefs::update_profile_prefs;
use solosoul_vault::VaultStore;

fn load_public_data_version(vault: &VaultStore, account_id: &str) -> Result<u64, String> {
    let profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        _ => return Ok(0),
    };

    let data: serde_json::Value = match serde_json::from_slice(&profile.data) {
        Ok(d) => d,
        Err(_) => return Ok(0),
    };

    let version = data
        .get("preferences")
        .and_then(|p| p.get("llmPublicDataVersion"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(version)
}

/// 递增 public_data_version。由 object_create / object_update 在检测到 public 级别变更时调用。
pub fn bump_public_data_version(vault: &VaultStore, account_id: &str) -> Result<u64, String> {
    let current = load_public_data_version(vault, account_id)?;
    let next = current + 1;
    save_public_data_version(vault, account_id, next)?;
    Ok(next)
}

fn save_public_data_version(
    vault: &VaultStore,
    account_id: &str,
    version: u64,
) -> Result<(), String> {
    update_profile_prefs(vault, account_id, |prefs| {
        prefs.insert(
            "llmPublicDataVersion".to_string(),
            serde_json::Value::Number(serde_json::Number::from(version)),
        );
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_load_public_data_version_no_profile() {
        let (vault, _dir) = setup_vault();
        let version = load_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_save_and_load_public_data_version() {
        let (vault, _dir) = setup_vault();
        let profile = Profile::new_with_id("test_account", "Test", Vec::new());
        vault.save_profile(&profile).unwrap();

        save_public_data_version(&vault, "test_account", 5).unwrap();
        let version = load_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn test_bump_public_data_version() {
        let (vault, _dir) = setup_vault();
        let profile = Profile::new_with_id("test_account", "Test", Vec::new());
        vault.save_profile(&profile).unwrap();

        let v1 = bump_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(v1, 1);

        let v2 = bump_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(v2, 2);
    }
}
