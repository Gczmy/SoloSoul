//! VaultService 测试（P025 拆分迁移）。

use super::*;
use solosoul_crypto::kdf::{derive_key, generate_salt};
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::TempDir;

fn setup_service() -> (VaultService, TempDir) {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join(".solosoul");
    std::fs::create_dir_all(&base).unwrap();
    let svc = VaultService::with_base_path(base);
    (svc, dir)
}

#[test]
fn test_scan_orphan_accounts_recovers_missing_from_manifest() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join(".solosoul");
    std::fs::create_dir_all(&base).unwrap();

    // 创建一个正常账户（会写入 accounts.json + acc_xxx 目录）
    {
        let svc = VaultService::with_base_path(base.clone());
        let account = svc.create_account("Alice", "password123", None).unwrap();
        let _account_id = account["id"].as_str().unwrap().to_string();
        assert_eq!(svc.list_accounts().len(), 1);

        // 模拟「清单被覆盖」事故：手工写一个 accounts.json 仅含另一账户，
        // 并把 Alice 的账户目录留下（成为孤儿）。
        let orphan_manifest = serde_json::json!([{
            "id": "acc_other",
            "name": "Other",
            "created_at": "2026-01-01T00:00:00+00:00",
            "last_accessed": null
        }]);
        std::fs::write(
            svc.base_path().join("accounts.json"),
            serde_json::to_string_pretty(&orphan_manifest).unwrap(),
        )
        .unwrap();
    }

    // 全新 service 加载被覆盖的清单：仅 acc_other（Alice 不在清单中，但目录仍在）
    let svc = VaultService::with_base_path(base);
    svc.load_accounts();
    assert_eq!(svc.list_accounts().len(), 1);
    assert_eq!(svc.list_accounts()[0].name, "Other");

    // 孤儿扫描应恢复 Alice
    let recovered = svc.scan_orphan_accounts().unwrap();
    assert!(!recovered.is_empty());
    assert!(svc.list_accounts().iter().any(|a| a.name == "Alice"));
    assert_eq!(svc.list_accounts().len(), 2);

    // 幂等：再次扫描不重复添加
    let recovered_again = svc.scan_orphan_accounts().unwrap();
    assert!(recovered_again.is_empty());
    assert_eq!(svc.list_accounts().len(), 2);
}

#[test]
fn test_scan_orphan_accounts_skips_corrupt_or_mismatched_config() {
    let (svc, _dir) = setup_service();

    // 目录名与 config.account_id 不一致 → 跳过
    let mismatch_dir = svc.base_path().join("acc_mismatch");
    std::fs::create_dir_all(&mismatch_dir).unwrap();
    let mismatched_config = serde_json::json!({
        "account_id": "acc_actually_different",
        "name": "Mismatch",
        "salt": "AAAA",
        "verify_hash": "BBBB",
        "created_at": "2026-01-01T00:00:00+00:00",
        "crypto_version": 3,
        "password_hint": null,
        "last_login_at": null,
        "last_operation_at": null,
        "last_operation_desc": null,
        "biometricEnabled": false,
        "pinEnabled": false,
        "pinLength": 0,
        "pinFailedAttempts": 0,
        "pinLockedUntil": null,
        "passwordFailedAttempts": 0,
        "passwordLockedUntil": null
    });
    std::fs::write(
        mismatch_dir.join("config.json"),
        serde_json::to_string_pretty(&mismatched_config).unwrap(),
    )
    .unwrap();

    // config.json 损坏 → 跳过
    let corrupt_dir = svc.base_path().join("acc_corrupt");
    std::fs::create_dir_all(&corrupt_dir).unwrap();
    std::fs::write(corrupt_dir.join("config.json"), b"{not valid json").unwrap();

    // 无 config.json → 跳过
    let no_config_dir = svc.base_path().join("acc_no_config");
    std::fs::create_dir_all(&no_config_dir).unwrap();

    let recovered = svc.scan_orphan_accounts().unwrap();
    assert!(recovered.is_empty());
    assert!(!svc.has_account("acc_mismatch"));
    assert!(!svc.has_account("acc_corrupt"));
    assert!(!svc.has_account("acc_no_config"));
}

#[test]
fn test_create_account_success() {
    let (svc, _dir) = setup_service();
    let result = svc.create_account("Alice", "password123", None);
    assert!(result.is_ok());
    let account = result.unwrap();
    assert_eq!(account["name"], "Alice");
    assert!(svc.has_any_account());
}

#[test]
fn test_create_account_empty_name_fails() {
    let (svc, _dir) = setup_service();
    let result = svc.create_account("", "password123", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("required"));
}

#[test]
fn test_create_account_short_password_fails() {
    let (svc, _dir) = setup_service();
    let result = svc.create_account("Alice", "short", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("8 characters"));
}

#[test]
fn test_create_account_duplicate_name_fails() {
    let (svc, _dir) = setup_service();
    svc.create_account("Alice", "password123", None).unwrap();
    let result = svc.create_account("alice", "password456", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already taken"));
}

#[test]
fn test_create_account_with_id_same_name_different_id_succeeds() {
    let (svc, _dir) = setup_service();
    // 正常创建账户 A（account_id 随机生成）
    let account_a = svc.create_account("Zzc", "password123", None).unwrap();
    let account_a_id = account_a["id"].as_str().unwrap().to_string();

    // 恢复场景：同名（大小写不同）但 account_id 不同 → 允许（身份是 account_id）
    let result = svc.create_account_with_id("acc_restore_same_name", "zzc", "password456", None);
    assert!(
        result.is_ok(),
        "同名校验不应对恢复场景生效: {:?}",
        result.err()
    );
    let account_b = result.unwrap();
    assert_eq!(account_b["name"], "zzc");
    assert_eq!(account_b["id"], "acc_restore_same_name");

    // 两个同名账户均可列出（登录页按 account_id 区分）
    let accounts = svc.list_accounts();
    assert_eq!(accounts.len(), 2);
    assert!(accounts.iter().any(|a| a.id == account_a_id));
    assert!(accounts.iter().any(|a| a.id == "acc_restore_same_name"));
}

#[test]
fn test_create_account_with_id_duplicate_id_fails() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Zzc", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();

    // 相同 account_id → 拒绝，错误字符串稳定供前端识别冲突
    let result = svc.create_account_with_id(&account_id, "Zzc", "password456", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Account ID already exists"));
}

#[test]
fn test_unlock_and_lock() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Bob", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // create_account leaves vault unlocked
    assert!(svc.is_unlocked());
    svc.lock();
    assert!(!svc.is_unlocked());

    svc.unlock(account_id, "password123").unwrap();
    assert!(svc.is_unlocked());

    svc.lock();
    assert!(!svc.is_unlocked());
}

#[test]
fn test_unlock_wrong_password_fails() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Carol", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    let result = svc.unlock(account_id, "wrongpassword");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid password"));
}

#[test]
fn test_password_lockout_seconds() {
    assert_eq!(password_lockout_seconds(0), 0);
    assert_eq!(password_lockout_seconds(4), 0);
    assert_eq!(password_lockout_seconds(5), 30);
    assert_eq!(password_lockout_seconds(9), 30);
    assert_eq!(password_lockout_seconds(10), 300);
    assert_eq!(password_lockout_seconds(11), 600);
    assert_eq!(password_lockout_seconds(15), 1800);
}

#[test]
fn test_unlock_rate_limit_locks_after_5_failures() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Dave", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();

    // 前 4 次失败：不锁定，返回 Invalid password
    for _ in 0..4 {
        let err = svc.unlock(&account_id, "wrong").unwrap_err();
        assert!(err.contains("Invalid password"));
    }
    // 第 5 次失败：触发 30s 阶梯锁定
    let err = svc.unlock(&account_id, "wrong").unwrap_err();
    assert!(err.contains("Invalid password"));
    // 锁定期间即使密码正确也被拒绝，且文案稳定（不递增计数）
    let err = svc.unlock(&account_id, "password123").unwrap_err();
    assert!(err.contains("Too many failed attempts"));
    let err = svc.unlock(&account_id, "wrong").unwrap_err();
    assert!(err.contains("Too many failed attempts"));
}

#[test]
fn test_verify_password_with_lockout_rate_limits() {
    // P012: verify_password_with_lockout 与 unlock 同款阶梯锁定——
    // 失败递增计数、第 5 次触发锁定、锁定期间拒绝、成功归零。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Grace", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();

    // 正确密码 → true（不计数）
    assert!(svc
        .verify_password_with_lockout(&account_id, "password123")
        .unwrap());
    // 前 4 次错误密码 → false（不抛异常，计数递增但未锁定）
    for _ in 0..4 {
        assert!(!svc
            .verify_password_with_lockout(&account_id, "wrong")
            .unwrap());
    }
    // 第 5 次失败触发 30s 阶梯锁定；锁定期间即使正确密码也被拒绝（文案稳定）
    assert!(!svc
        .verify_password_with_lockout(&account_id, "wrong")
        .unwrap());
    let err = svc
        .verify_password_with_lockout(&account_id, "password123")
        .unwrap_err();
    assert!(err.contains("Too many failed attempts"));

    // 成功路径归零：直接操作 config 清除锁定后，验证成功计数归零
    // （通过 unlock 成功触发 clear_password_failures——与 unlock 共享同一归零逻辑）
    let mut config = svc.read_account_config(&account_id).unwrap();
    config.password_failed_attempts = 0;
    config.password_locked_until = None;
    svc.write_config_atomic(
        &account_id,
        serde_json::to_string_pretty(&config).unwrap().as_bytes(),
    )
    .unwrap();
    assert!(svc
        .verify_password_with_lockout(&account_id, "password123")
        .unwrap());
    // 计数已归零：再失败 5 次 → 第 5 次触发锁定
    for _ in 0..5 {
        assert!(!svc
            .verify_password_with_lockout(&account_id, "wrong")
            .unwrap());
    }
    let err = svc
        .verify_password_with_lockout(&account_id, "password123")
        .unwrap_err();
    assert!(err.contains("Too many failed attempts"));
}

#[test]
fn test_unlock_rate_limit_resets_on_success() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Eve", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();

    // 4 次失败（未触发锁定）
    for _ in 0..4 {
        assert!(svc.unlock(&account_id, "wrong").is_err());
    }
    // 成功解锁 → 计数归零
    assert!(svc.unlock(&account_id, "password123").is_ok());
    // 计数已从 0 重新累计：再失败 5 次 → 第 5 次触发锁定
    for _ in 0..5 {
        let err = svc.unlock(&account_id, "wrong").unwrap_err();
        assert!(err.contains("Invalid password"));
    }
    let err = svc.unlock(&account_id, "password123").unwrap_err();
    assert!(err.contains("Too many failed attempts"));
}

#[test]
fn test_unlock_rate_limit_expires_and_clears() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Frank", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();

    // 触发锁定
    for _ in 0..5 {
        assert!(svc.unlock(&account_id, "wrong").is_err());
    }
    assert!(svc.unlock(&account_id, "password123").is_err());

    // 手动把锁定截止时间改为过去（模拟锁定到期）
    let config_path = svc.base_path().join(&account_id).join("config.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    value["passwordLockedUntil"] =
        serde_json::Value::String((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339());
    std::fs::write(&config_path, serde_json::to_string_pretty(&value).unwrap()).unwrap();

    // 到期后正确密码解锁成功，且失败计数/锁定被归零
    assert!(svc.unlock(&account_id, "password123").is_ok());
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(value["passwordFailedAttempts"], 0);
    assert!(value["passwordLockedUntil"].as_str().is_none());
}

#[test]
fn test_list_accounts() {
    let (svc, _dir) = setup_service();
    svc.create_account("Alice", "password123", None).unwrap();
    svc.create_account("Bob", "password123", None).unwrap();
    let accounts = svc.list_accounts();
    assert_eq!(accounts.len(), 2);
}

#[test]
fn test_verify_password() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Dave", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    assert!(svc.verify_password(account_id, "password123").unwrap());
    assert!(!svc.verify_password(account_id, "wrong").unwrap());
}

#[test]
fn test_change_password() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Eve", "oldpassword", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // Simulate biometric unlock having been enabled
    let config_path = svc.base_path().join(account_id).join("config.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    raw["biometricEnabled"] = serde_json::Value::Bool(true);
    fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
    // 用旧版文件模拟已启用生物识别，验证 change_password 不会误删标记。
    let legacy_key_path = svc.base_path().join(account_id).join("biometric_key");
    let legacy_key_hex = "deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678";
    fs::write(&legacy_key_path, legacy_key_hex).unwrap();

    svc.unlock(account_id, "oldpassword").unwrap();
    svc.change_password(account_id, "oldpassword", "newpassword")
        .unwrap();

    // Old password should fail
    assert!(!svc.verify_password(account_id, "oldpassword").unwrap());
    // New password should succeed
    assert!(svc.verify_password(account_id, "newpassword").unwrap());

    let content = fs::read_to_string(&config_path).unwrap();
    let config: AccountConfig = serde_json::from_str(&content).unwrap();
    // 修改密码后生物识别启用标记应保持为 true；测试使用文件存储后端，实际密钥应同步更新。
    assert!(config.biometric_enabled);

    let bio_manager = make_biometric_manager(svc.base_path().clone());
    assert!(bio_manager.is_configured(account_id));
    let expected_hex = hex::encode(svc.get_session_key().unwrap().as_slice());
    let stored_hex = bio_manager
        .read_stored_key_hex(account_id, "verify after change")
        .unwrap();
    assert_eq!(stored_hex, expected_hex);
}

// ── R-4: 回滚失败必须并入上抛文案（而非「已尝试自动回滚」掩盖）──────────
//
// N-2 残余：rollback_reencrypt_and_config 失败时仅记日志，调用方文案仍写
// 「已尝试自动回滚」——磁盘满等共同根因下 config 可能被截断且用户无感知。
// 本测试用 toggleable mock fs：仅对 config.json 写入注入失败 → 改密的 config
// 写入失败触发回滚 → 回滚的 config 恢复同样失败 → 错误文案必须明示
// 「automatic rollback FAILED」并带底层原因（修复前失败仅记日志，文案无此信息）。
struct FailConfigWriteFs {
    inner: LocalVaultFileSystem,
    fail_config_writes: Arc<AtomicBool>,
}

impl VaultFileSystem for FailConfigWriteFs {
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String> {
        self.inner.read_file(relative_path)
    }
    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
        if self.fail_config_writes.load(Ordering::SeqCst) && relative_path.ends_with("config.json")
        {
            return Err("mock config write failure (injected)".to_string());
        }
        self.inner.write_file(relative_path, data)
    }
    // P135: config 写入已切换为原子写路径——mock 需同样注入才能触发回滚。
    fn write_file_atomic(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
        if self.fail_config_writes.load(Ordering::SeqCst) && relative_path.ends_with("config.json")
        {
            return Err("mock config write failure (injected)".to_string());
        }
        self.inner.write_file_atomic(relative_path, data)
    }
    fn remove_file(&self, relative_path: &str) -> Result<(), String> {
        self.inner.remove_file(relative_path)
    }
    fn exists(&self, relative_path: &str) -> Result<bool, String> {
        self.inner.exists(relative_path)
    }
    fn create_dir_all(&self, relative_path: &str) -> Result<(), String> {
        self.inner.create_dir_all(relative_path)
    }
    fn remove_dir_all(&self, relative_path: &str) -> Result<(), String> {
        self.inner.remove_dir_all(relative_path)
    }
    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String> {
        self.inner.list_dir(relative_path)
    }
    fn local_path(&self, relative_path: &str) -> Option<PathBuf> {
        self.inner.local_path(relative_path)
    }
}

#[test]
fn test_change_password_rollback_failure_surfaced() {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join(".solosoul");
    std::fs::create_dir_all(&base).unwrap();
    let fail_flag = Arc::new(AtomicBool::new(false));
    let mock = Arc::new(FailConfigWriteFs {
        inner: LocalVaultFileSystem::new(base.clone()),
        fail_config_writes: fail_flag.clone(),
    });
    let svc = VaultService::with_file_system(base, mock);

    let account = svc.create_account("R4", "oldpassword", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    svc.unlock(account_id, "oldpassword").unwrap();

    // 注入失败：此后对 config.json 的写（改密写入 + 回滚恢复）都失败。
    // change_password 内部 unlock 只写 accounts.json，不受影响。
    fail_flag.store(true, Ordering::SeqCst);

    let err = svc
        .change_password(account_id, "oldpassword", "newpassword")
        .unwrap_err();
    // R-4：错误文案必须明示回滚失败，而非「已尝试自动回滚」
    assert!(
        err.contains("automatic rollback FAILED"),
        "错误文案必须明示回滚失败，实际: {}",
        err
    );
    assert!(
        err.contains("mock config write failure"),
        "错误文案须带底层原因，实际: {}",
        err
    );
}

#[test]
fn test_update_password_hint() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Frank", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // Simulate biometric unlock having been enabled
    let config_path = svc.base_path().join(account_id).join("config.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    raw["biometricEnabled"] = serde_json::Value::Bool(true);
    fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    svc.update_password_hint(account_id, "My favorite color")
        .unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: AccountConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(config.password_hint, Some("My favorite color".to_string()));
    assert!(config.biometric_enabled);
}

#[test]
fn test_rename_account() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Old Name", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 改名后：config 与账户清单都同步更新，账户 ID 不变。
    svc.rename_account(account_id, "New Name").unwrap();
    let summaries = svc.list_accounts();
    let summary = summaries.iter().find(|a| a.id == account_id).unwrap();
    assert_eq!(summary.name, "New Name");
    let config_path = svc.base_path().join(account_id).join("config.json");
    let content = fs::read_to_string(&config_path).unwrap();
    let config: AccountConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(config.name, "New Name");

    // 空白名与重名（大小写不敏感）拒绝。
    assert!(svc.rename_account(account_id, "   ").is_err());
    svc.create_account("Other", "password123", None).unwrap();
    assert!(svc.rename_account(account_id, "other").is_err());

    // 不存在的账户拒绝。
    assert!(svc.rename_account("acc_missing", "X").is_err());
}

#[test]
fn test_delete_account() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Grace", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    svc.delete_account(account_id).unwrap();
    assert!(!svc.has_any_account());
    assert!(!svc.base_path().join(account_id).exists());
}

#[test]
fn test_has_account() {
    let (svc, _dir) = setup_service();
    // 无账户时返回 false
    assert!(!svc.has_account("acc_nonexistent"));

    // 创建账户后可按 account_id 命中（纯内存查询，无文件 IO）
    let account = svc.create_account("Helen", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap().to_string();
    assert!(svc.has_account(&account_id));
    assert!(!svc.has_account("acc_other"));

    // 删除后不再命中
    svc.delete_account(&account_id).unwrap();
    assert!(!svc.has_account(&account_id));
}

#[test]
fn test_unlock_with_session_key() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Hank", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    let session_key = [0u8; 32];
    svc.unlock_with_session_key(account_id, &session_key)
        .unwrap();
    assert!(svc.is_unlocked());
    assert!(svc.get_session_key().is_some());
}

/// P001：附件静态加密密钥由会话密钥确定性派生（同密码 → 同密钥），
/// 且与数据库密钥（session_key 本身）域分离——不同用途密钥互相独立。
#[test]
fn test_attachment_encryption_key_derived_deterministically() {
    let (svc, _dir) = setup_service();
    svc.create_account("Ivy", "password123", None).unwrap();

    let key1 = svc.attachment_encryption_key().unwrap();
    let key2 = svc.attachment_encryption_key().unwrap();
    assert_eq!(key1.len(), 32);
    assert_eq!(key1.as_slice(), key2.as_slice());

    // 域分离：附件密钥 ≠ 会话密钥（数据库密钥）。
    let session_key = svc.get_session_key().unwrap();
    assert_ne!(key1.as_slice(), session_key.as_slice());
}

/// P001：锁定（无会话密钥）时附件密钥派生必须失败，不泄露半初始化状态。
#[test]
fn test_attachment_encryption_key_requires_unlock() {
    let (svc, _dir) = setup_service();
    svc.create_account("Ivy", "password123", None).unwrap();
    svc.lock();
    assert!(svc.attachment_encryption_key().is_err());
}
/// P001：改密后附件目录自动重加密——旧附件密钥解不开，新附件密钥可解密读出。
#[test]
fn test_change_password_reencrypts_attachments() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Rex", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 解锁后写入一个附件（加密落盘）。
    let att_key_before = {
        let k = svc.attachment_encryption_key().unwrap();
        let arr: [u8; 32] = k.as_slice().try_into().unwrap();
        arr
    };
    let account_dir = svc.base_path().join(account_id);
    let att_dir = account_dir.join("attachments").join("obj_1").join("att_1");
    std::fs::create_dir_all(&att_dir).unwrap();
    let plain = b"secret attachment content".repeat(20);
    let plain_path = att_dir.join("doc.txt");
    std::fs::write(&plain_path, &plain).unwrap();
    let enc_tmp = att_dir.join("doc.txt.enc");
    crate::attachment_crypto::encrypt_file_stream(&att_key_before, &plain_path, &enc_tmp)
        .expect("encrypt attachment");
    std::fs::rename(&enc_tmp, &plain_path).unwrap();
    assert!(crate::attachment_crypto::is_encrypted_file(&plain_path));

    // 改密。
    svc.change_password(account_id, "password123", "newpassword123")
        .unwrap();

    // 改密后：旧附件密钥应无法解密，新附件密钥可解密且内容一致。
    let att_key_after = {
        let k = svc.attachment_encryption_key().unwrap();
        let arr: [u8; 32] = k.as_slice().try_into().unwrap();
        arr
    };
    assert_ne!(att_key_before, att_key_after);
    assert!(crate::attachment_crypto::is_encrypted_file(&plain_path));
    // 旧密钥解密失败
    let wrong =
        crate::attachment_crypto::read_file_decrypted(&att_key_before, &plain_path, 1_000_000);
    assert!(wrong.is_err());
    // 新密钥解密成功且内容一致
    let decrypted =
        crate::attachment_crypto::read_file_decrypted(&att_key_after, &plain_path, 1_000_000)
            .unwrap();
    assert_eq!(decrypted, plain);
}

/// P001-2：改密时附件重加密失败（一个损坏附件触发）→ 整体回滚到旧钥——
/// config/DB/附件全部保持旧钥一致（P001 复核打回项：原实现直接上抛，
/// 留下 config 新钥 + 附件混态的永久不可读数据丢失路径）。
#[test]
fn test_change_password_attachment_reencrypt_failure_rolls_back() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Rex2", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    let att_key_before = {
        let k = svc.attachment_encryption_key().unwrap();
        let arr: [u8; 32] = k.as_slice().try_into().unwrap();
        arr
    };
    let account_dir = svc.base_path().join(account_id);
    let att_dir = account_dir.join("attachments").join("obj_1").join("att_1");
    std::fs::create_dir_all(&att_dir).unwrap();
    // 正常附件（加密落盘）。
    let plain = b"good attachment content".repeat(20);
    let good_path = att_dir.join("good.txt");
    std::fs::write(&good_path, &plain).unwrap();
    let enc_tmp = att_dir.join("good.txt.enc");
    crate::attachment_crypto::encrypt_file_stream(&att_key_before, &good_path, &enc_tmp).unwrap();
    std::fs::rename(&enc_tmp, &good_path).unwrap();
    // 损坏附件（SOLC 头 + 垃圾内容 → 解密必失败）。
    let bad_path = att_dir.join("bad.bin");
    std::fs::write(&bad_path, b"SOLC\x00\x00\x00garbage-not-a-real-ciphertext").unwrap();

    // 改密应失败，且错误文案明示回滚。
    let err = svc
        .change_password(account_id, "password123", "newpassword123")
        .unwrap_err();
    assert!(
        err.contains("attachment re-encryption failed") && err.contains("rollback"),
        "错误文案须同时含附件失败与回滚信息，实际: {}",
        err
    );

    // 回滚后：旧密码仍可用、新密码不可用（config 未换钥）。
    assert!(svc.verify_password(account_id, "password123").unwrap());
    assert!(!svc.verify_password(account_id, "newpassword123").unwrap());

    // 附件保持旧钥可读（未被部分重加密）。
    let good_dec =
        crate::attachment_crypto::read_file_decrypted(&att_key_before, &good_path, 1_000_000)
            .unwrap();
    assert_eq!(good_dec, plain);

    // 无残留临时文件（.rekey.tmp/.rekey.new/.rekey.rb.tmp 均被清理）。
    let mut leftovers = Vec::new();
    for entry in std::fs::read_dir(&att_dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains(".rekey.") {
            leftovers.push(name);
        }
    }
    assert!(leftovers.is_empty(), "残留临时文件: {:?}", leftovers);
}

/// P001-2：KDF 升级同样改变会话密钥 → 附件密钥随之变化，必须重加密附件
/// （原实现遗漏此步，升级后附件全部无法解密）。
#[test]
fn test_kdf_upgrade_reencrypts_attachments() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("KdfRex", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 写入一个加密附件（旧附件密钥）。
    let att_key_before = {
        let k = svc.attachment_encryption_key().unwrap();
        let arr: [u8; 32] = k.as_slice().try_into().unwrap();
        arr
    };
    let account_dir = svc.base_path().join(account_id);
    let att_dir = account_dir.join("attachments").join("obj_1").join("att_1");
    std::fs::create_dir_all(&att_dir).unwrap();
    let plain = b"kdf attachment".repeat(20);
    let plain_path = att_dir.join("doc.txt");
    std::fs::write(&plain_path, &plain).unwrap();
    let enc_tmp = att_dir.join("doc.txt.enc");
    crate::attachment_crypto::encrypt_file_stream(&att_key_before, &plain_path, &enc_tmp).unwrap();
    std::fs::rename(&enc_tmp, &plain_path).unwrap();

    // 模拟旧账户：将 config 的 KDF 参数改为开发档（8 MiB / 2 iter）。
    let config_path = svc.base_path().join(account_id).join("config.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    raw["kdfMemoryKb"] = serde_json::Value::from(8 * 1024u32);
    raw["kdfIterations"] = serde_json::Value::from(2u32);
    raw["kdfParallelism"] = serde_json::Value::from(4u32);
    fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    let old_kdf = KdfConfig::development();
    let salt_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        raw["salt"].as_str().unwrap(),
    )
    .unwrap();
    let salt_arr: [u8; 16] = salt_bytes.as_slice().try_into().unwrap();
    let old_key = derive_key("password123", &salt_arr, &old_kdf).unwrap();

    // 锁定账户再触发 KDF 升级——模拟生产路径（unlock() 在主密码验证后、设置
    // unlocked_account 之前进入升级分支；此前测试未 lock，unlocked_account
    // 仍为 Some，掩盖了 reencrypt_attachments 依赖 get_current_account 的缺陷）。
    svc.lock();

    // 执行 KDF 升级。
    svc.unlock_with_kdf_upgrade(account_id, "password123", &old_key)
        .unwrap();

    // 升级后：旧附件密钥解不开，新附件密钥可解密且内容一致。
    let att_key_after = {
        let k = svc.attachment_encryption_key().unwrap();
        let arr: [u8; 32] = k.as_slice().try_into().unwrap();
        arr
    };
    assert_ne!(att_key_before, att_key_after);
    assert!(crate::attachment_crypto::is_encrypted_file(&plain_path));
    assert!(
        crate::attachment_crypto::read_file_decrypted(&att_key_before, &plain_path, 1_000_000)
            .is_err()
    );
    let decrypted =
        crate::attachment_crypto::read_file_decrypted(&att_key_after, &plain_path, 1_000_000)
            .unwrap();
    assert_eq!(decrypted, plain);
}

#[test]
fn test_get_vault_store_when_locked() {
    let (svc, _dir) = setup_service();
    svc.create_account("Ivy", "password123", None).unwrap();
    // create_account leaves vault unlocked; lock first
    svc.lock();
    assert!(svc.get_vault_store().is_none());
}

#[test]
fn test_get_vault_store_when_unlocked() {
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Jack", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    svc.unlock(account_id, "password123").unwrap();
    assert!(svc.get_vault_store().is_some());
}

/// P003：`unlock_with_kdf_upgrade` 应将存储的开发档 KDF 参数透明升级到生产档，
/// 重加密全部数据（写入一条审计日志后升级，仍可解密读出），并更新 config 中的
/// verify hash 与参数。
#[test]
fn test_unlock_with_kdf_upgrade_reencrypts_and_upgrades_params() {
    let (svc, _dir) = setup_service();
    let account = svc
        .create_account("KdfUpgrade", "password123", None)
        .unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 写入一条审计日志（加密数据），用于验证升级后仍可解密。
    {
        let vault = svc.get_vault_store().expect("vault open after create");
        vault
            .log_structured(
                "test_kdf_upgrade",
                "test",
                Some(account_id),
                None,
                "user",
                Some("before-upgrade"),
            )
            .unwrap();
    }

    // 模拟旧账户：将 config 的 KDF 参数改为开发档（8 MiB / 2 iter）。
    let config_path = svc.base_path().join(account_id).join("config.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    raw["kdfMemoryKb"] = serde_json::Value::from(8 * 1024u32);
    raw["kdfIterations"] = serde_json::Value::from(2u32);
    raw["kdfParallelism"] = serde_json::Value::from(4u32);
    fs::write(&config_path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();

    // 用旧（开发档）参数派生旧密钥并执行透明升级。
    let old_kdf = KdfConfig::development();
    let salt_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        raw["salt"].as_str().unwrap(),
    )
    .unwrap();
    let salt_arr: [u8; 16] = salt_bytes.as_slice().try_into().unwrap();
    let old_key = derive_key("password123", &salt_arr, &old_kdf).unwrap();

    // 锁定账户再触发升级——模拟生产路径（unlock() 验证密码后、设置
    // unlocked_account 之前进入升级分支）。此前未 lock 时 unlocked_account 仍为
    // Some，掩盖了 reencrypt_attachments 依赖 get_current_account 的缺陷。
    svc.lock();
    svc.unlock_with_kdf_upgrade(account_id, "password123", &old_key)
        .unwrap();

    // config 应已升级为生产参数。
    let content = fs::read_to_string(&config_path).unwrap();
    let config: AccountConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(config.kdf_config(), KdfConfig::production());

    // 旧参数派生的密钥应不再匹配 verify hash（新密钥来自生产参数）。
    let old_verify = derive_key("password123", &salt_arr, &old_kdf).unwrap();
    assert_ne!(config.verify_hash, hex::encode(old_verify.as_slice()));

    // 升级在锁定态完成，且会话已重建（unlocked_account / vault_store 均已设置）。
    assert!(svc.is_unlocked());
    assert!(svc.get_vault_store().is_some());

    // 用新会话密钥（生产参数）打开 Vault 后，升级前的加密数据仍可解密。
    let vault = svc.get_vault_store().expect("vault open after upgrade");
    let logs = vault.list_audit_log(10).unwrap();
    assert!(!logs.is_empty());
    assert!(logs
        .iter()
        .any(|l| l.details.as_deref() == Some("before-upgrade")));
}

// ── P135: 原子写 + 崩溃恢复端到端 ──────────────────────────

#[test]
fn test_unlock_recovers_orphan_config_tmp() {
    // R-4① 崩溃故事：reencrypt 已提交、config 写入中进程崩溃——
    // 孤儿 .tmp（新 config）存在、主 config.json 缺失。
    // 下次 unlock 经 read_config_with_recovery → recover_or_load 提升 .tmp。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Orphan", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 模拟崩溃：删主 config.json，仅留孤儿 .tmp（内容=原 config）。
    let config_path = svc.base_path().join(account_id).join("config.json");
    let content = fs::read(&config_path).unwrap();
    fs::write(config_path.with_extension("tmp"), &content).unwrap();
    fs::remove_file(&config_path).unwrap();

    // unlock 应经恢复路径成功（而非 "Account not found"）。
    svc.unlock(account_id, "password123").unwrap();
    assert!(config_path.exists(), "orphan .tmp 应被提升回主 config.json");
    assert!(
        !config_path.with_extension("tmp").exists(),
        "提升后孤儿 .tmp 应被清除"
    );
    assert!(svc.verify_password(account_id, "password123").unwrap());
}

#[test]
fn test_unlock_recovers_backup_bak() {
    // 主文件损坏（非 JSON）但 .bak 完好 → recover_or_load 回退 .bak。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("BakUser", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    let config_path = svc.base_path().join(account_id).join("config.json");
    let content = fs::read(&config_path).unwrap();
    fs::write(config_path.with_extension("bak"), &content).unwrap();
    fs::write(&config_path, b"{corrupted json").unwrap();

    svc.unlock(account_id, "password123").unwrap();
    assert!(svc.verify_password(account_id, "password123").unwrap());
}

// ── R-4① 方案 2：两阶段 config 交换（config.json.pending）崩溃恢复 ──────────
//
// 模拟工具：构造「reencrypt 已提交、active config 未更新」的崩溃后状态——
// 数据用新钥重加密 + pending 文件残留 + active config 仍为旧。

/// 派生并返回（new_salt, new_key_arr）。
fn make_new_key(password: &str) -> ([u8; 16], [u8; 32]) {
    let salt = generate_salt();
    let kdf = KdfConfig::from_env();
    let new_key = derive_key(password, &salt, &kdf).unwrap();
    let arr: [u8; 32] = new_key.as_slice().try_into().unwrap();
    (salt, arr)
}

/// 构造 pending config 内容：基于当前 active config，仅替换 salt/verify_hash 为新钥对应值。
fn build_pending_config_json(
    svc: &VaultService,
    account_id: &str,
    new_salt: &[u8; 16],
    new_key: &[u8; 32],
) -> String {
    let config_path = svc.base_path().join(account_id).join("config.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    raw["salt"] = serde_json::Value::String(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        new_salt.as_slice(),
    ));
    let verify_hash = hex::encode(
        solosoul_crypto::hkdf_ext::derive_hkdf_key(new_key, new_salt, b"SOLOSOUL_VAULT_VERIFY_v1")
            .unwrap(),
    );
    raw["verify_hash"] = serde_json::Value::String(verify_hash);
    serde_json::to_string_pretty(&raw).unwrap()
}

#[test]
fn test_recover_pending_promotes_when_reencrypt_committed() {
    // 崩溃点：reencrypt 已提交（数据=新钥），active config 未更新（仍旧），
    // pending 文件残留。下次用新密码 unlock 应 promote：pending 升为 active。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Promote", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 写一条非空 profile，保证 probe 能区分新旧钥。
    {
        let vault = svc.get_vault_store().unwrap();
        vault
            .save_profile(&solosoul_vault::Profile::new_with_id(
                account_id,
                "p",
                b"sensitive".to_vec(),
            ))
            .unwrap();
    }

    // 模拟：旧钥解锁 → 数据重加密到新钥（reencrypt 提交）→ 写 pending（新 config）→
    // 不更新 active config（模拟崩溃残留）。
    svc.unlock(account_id, "password123").unwrap();
    let old_key_arr = *svc.get_session_key().unwrap();
    let old_key = solosoul_vault::DataEncryptionKey::new(old_key_arr);
    let (new_salt, new_key_arr) = make_new_key("newpassword123");
    let new_key = solosoul_vault::DataEncryptionKey::new(new_key_arr);
    let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
    svc.write_config_pending(account_id, pending_json.as_bytes())
        .unwrap();
    svc.get_vault_store()
        .unwrap()
        .reencrypt_all(&old_key, &new_key)
        .unwrap();
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(pending_path.exists(), "pending 应存在");
    svc.lock();

    // 用新密码解锁 → 恢复应 promote（pending 升为 active），解锁成功。
    svc.unlock(account_id, "newpassword123").unwrap();
    assert!(!pending_path.exists(), "promote 后 pending 应被删除");
    assert!(svc.verify_password(account_id, "newpassword123").unwrap());
    assert!(!svc.verify_password(account_id, "password123").unwrap());
    // 数据以新钥可解密。
    let logs_vault = svc.get_vault_store().unwrap();
    let profile = logs_vault.load_profile(account_id).unwrap();
    assert!(profile.is_some(), "promote 后数据应以新钥可读");
}

#[test]
fn test_recover_pending_discards_when_reencrypt_not_committed() {
    // 崩溃点：pending 已写但 reencrypt 未提交（数据仍=旧钥）。
    // 下次用旧密码 unlock 应 discard：删 pending，保持旧钥。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Discard", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    {
        let vault = svc.get_vault_store().unwrap();
        vault
            .save_profile(&solosoul_vault::Profile::new_with_id(
                account_id,
                "p",
                b"sensitive".to_vec(),
            ))
            .unwrap();
    }

    svc.unlock(account_id, "password123").unwrap();
    let (new_salt, new_key_arr) = make_new_key("newpassword123");
    let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
    svc.write_config_pending(account_id, pending_json.as_bytes())
        .unwrap();
    // 注意：不执行 reencrypt_all——数据保持旧钥。
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(pending_path.exists());
    svc.lock();

    // 用旧密码解锁 → 恢复应 discard（pending 删除），解锁成功、数据仍旧钥可读。
    svc.unlock(account_id, "password123").unwrap();
    assert!(!pending_path.exists(), "discard 后 pending 应被删除");
    assert!(svc.verify_password(account_id, "password123").unwrap());
    assert!(!svc.verify_password(account_id, "newpassword123").unwrap());
    let vault = svc.get_vault_store().unwrap();
    let profile = vault.load_profile(account_id).unwrap();
    assert!(profile.is_some(), "discard 后数据仍以旧钥可读");
}

#[test]
fn test_recover_pending_wrong_password_preserves_pending() {
    // 密码错误时：两钥探测都失败 → 上抛且保留 pending（供下次重试/人工恢复）。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("WrongPw", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    {
        let vault = svc.get_vault_store().unwrap();
        vault
            .save_profile(&solosoul_vault::Profile::new_with_id(
                account_id,
                "p",
                b"sensitive".to_vec(),
            ))
            .unwrap();
    }

    svc.unlock(account_id, "password123").unwrap();
    let (new_salt, new_key_arr) = make_new_key("newpassword123");
    let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
    svc.write_config_pending(account_id, pending_json.as_bytes())
        .unwrap();
    svc.lock();

    let err = svc.unlock(account_id, "totally_wrong").unwrap_err();
    assert!(err.contains("Invalid password"), "实际: {}", err);
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(pending_path.exists(), "密码错误时 pending 必须保留");

    // 用正确密码重试 → 应恢复成功。
    svc.unlock(account_id, "password123").unwrap();
    assert!(!pending_path.exists());
}

#[test]
fn test_verify_password_with_lockout_recovers_pending() {
    // P012 复核打回项：verify_password_with_lockout 必须带 recover_pending_reencrypt 前导——
    // 改密/KDF 升级崩溃残留 pending 时，先完成 reencrypt→config 交换（promote：数据=新钥），
    // 校验基于一致 config，且不误判密码（promote 后新密码验证为 true）。
    let (svc, _dir) = setup_service();
    let account = svc
        .create_account("VerifyRecover", "password123", None)
        .unwrap();
    let account_id = account["id"].as_str().unwrap();
    {
        let vault = svc.get_vault_store().unwrap();
        vault
            .save_profile(&solosoul_vault::Profile::new_with_id(
                account_id,
                "p",
                b"sensitive".to_vec(),
            ))
            .unwrap();
    }

    // 模拟崩溃残留：数据重加密到新钥 + pending（新 config）残留 + active config 仍旧。
    svc.unlock(account_id, "password123").unwrap();
    let old_key = solosoul_vault::DataEncryptionKey::new(*svc.get_session_key().unwrap());
    let (new_salt, new_key_arr) = make_new_key("newpassword123");
    let new_key = solosoul_vault::DataEncryptionKey::new(new_key_arr);
    let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
    svc.write_config_pending(account_id, pending_json.as_bytes())
        .unwrap();
    svc.get_vault_store()
        .unwrap()
        .reencrypt_all(&old_key, &new_key)
        .unwrap();
    svc.lock();
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(pending_path.exists());

    // verify_password_with_lockout 走新密码：前导恢复 promote → 校验通过且 pending 清除。
    assert!(svc
        .verify_password_with_lockout(account_id, "newpassword123")
        .unwrap());
    assert!(!pending_path.exists(), "promote 后 pending 应被删除");
    // 恢复前导本身不得产生失败计数（否则首次成功校验被误计并错触发锁定）。
    let cfg = svc.read_account_config(account_id).unwrap();
    assert_eq!(cfg.password_failed_attempts, 0, "恢复前导不得误计失败");
    assert!(!svc
        .verify_password_with_lockout(account_id, "password123")
        .unwrap());
}

#[test]
fn test_change_password_success_removes_pending() {
    // 成功路径：change_password 完成后 pending 应被清除，无残留。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("Clean", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    svc.unlock(account_id, "password123").unwrap();
    svc.change_password(account_id, "password123", "newpassword123")
        .unwrap();
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(!pending_path.exists(), "成功改密后 pending 不应残留");
    assert!(svc.verify_password(account_id, "newpassword123").unwrap());
}

#[test]
fn test_unlock_with_session_key_rejects_pending() {
    // 生物识别/PIN 会话密钥解锁遇 pending 应显式拒绝，引导密码解锁。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("BioGuard", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();
    svc.unlock(account_id, "password123").unwrap();
    let session_key = *svc.get_session_key().unwrap();
    let (new_salt, new_key_arr) = make_new_key("newpassword123");
    let pending_json = build_pending_config_json(&svc, account_id, &new_salt, &new_key_arr);
    svc.write_config_pending(account_id, pending_json.as_bytes())
        .unwrap();
    svc.lock();

    let err = svc
        .unlock_with_session_key(account_id, &session_key)
        .unwrap_err();
    assert!(
        err.contains("Pending key rotation detected"),
        "实际: {}",
        err
    );
    let pending_path = svc.base_path().join(account_id).join("config.json.pending");
    assert!(pending_path.exists(), "拒绝解锁不应误删 pending");
}

#[test]
fn test_write_config_atomic_tightens_bak_permissions() {
    // 评审补强：write_atomic 的 .bak 经 fs::copy 生成，权限为 umask 默认——
    // write_config_atomic 应对 .bak 变体收紧到 0600（与主 config 一致）。
    let (svc, _dir) = setup_service();
    let account = svc.create_account("BakPerm", "password123", None).unwrap();
    let account_id = account["id"].as_str().unwrap();

    // 触发第二次 config 写（产生 .bak）。
    svc.update_password_hint(account_id, "hint").unwrap();

    let config_path = svc.base_path().join(account_id).join("config.json");
    let bak_path = config_path.with_extension("bak");
    if bak_path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&bak_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, ".bak 权限应收紧为 0600，实际 {mode:o}");
        }
    }
}
