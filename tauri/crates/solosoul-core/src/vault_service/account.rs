//! VaultService 账户 CRUD 域（P025 拆分）。
//! 账户生命周期：列出/创建/删除/重命名/安全标志复位。
use super::*;
use solosoul_crypto::kdf::{derive_key, generate_salt, KdfConfig};
use solosoul_vault::{VaultConfig, VaultStore};
use zeroize::Zeroizing;

impl super::VaultService {
    pub fn load_accounts(&self) {
        let rel = self.accounts_file_rel();
        match self.fs.exists(rel) {
            Ok(false) | Err(_) => {
                tracing::warn!("Accounts file does not exist: {}", rel);
                return;
            }
            Ok(true) => {}
        }
        match self.fs.read_file(rel) {
            Ok(content) => match serde_json::from_slice::<Vec<AccountEntry>>(&content) {
                Ok(accounts) => {
                    if let Ok(mut cache) = self.accounts_cache.write() {
                        for a in accounts {
                            cache.insert(a.id.clone(), a);
                        }
                        tracing::debug!("Loaded {} account(s) from {}", cache.len(), rel);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse accounts file: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Failed to read accounts file: {}", e);
            }
        }
    }

    pub(crate) fn save_accounts(&self) -> Result<(), String> {
        let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
        let list: Vec<&AccountEntry> = cache.values().collect();
        let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        self.fs.create_dir_all("")?;
        self.ensure_private_dir("")?;
        let rel = self.accounts_file_rel();
        // P135: 账户清单同样关键——原子写避免截断后 load_accounts 静默清空。
        self.fs.write_file_atomic(rel, content.as_bytes())?;
        self.ensure_private_file(rel)?;
        Ok(())
    }

    pub fn has_any_account(&self) -> bool {
        self.accounts_cache
            .read()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    /// 判断指定 `account_id` 的账户是否存在。
    /// 仅查内存缓存（accounts_cache），不做任何文件 IO，
    /// 适合需要高频判断但无需账户详情（如恢复覆盖前的存在性检查）的场景。
    pub fn has_account(&self, account_id: &str) -> bool {
        self.accounts_cache
            .read()
            .map(|c| c.contains_key(account_id))
            .unwrap_or(false)
    }

    /// 扫描 Vault 根目录下存在账户数据目录（`acc_*`）但不在 accounts.json
    /// 清单中的账户，按目录中的 config.json 重建清单条目并写回。
    ///
    /// 用于恢复「清单被覆盖但账户目录仍在」的场景（如 SAF 目录切换时
    /// 本地新账户清单覆盖了远端 accounts.json，旧账户目录成为孤儿）。
    ///
    /// 返回恢复的账户 id 列表。
    pub fn scan_orphan_accounts(&self) -> Result<Vec<String>, String> {
        let names = self.fs.list_dir("").map_err(|e| e.to_string())?;
        let mut recovered = Vec::new();
        for name in names {
            if !name.starts_with("acc_") {
                continue;
            }
            // 已存在于清单中的账户跳过
            if self.has_account(&name) {
                continue;
            }
            let config_rel = self.config_path_rel(&name);
            let content = match self.fs.read_file(&config_rel) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let config: AccountConfig = match serde_json::from_slice(&content) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("[scan_orphan_accounts] 跳过损坏 config {}: {}", name, e);
                    continue;
                }
            };
            // 目录名与 config 内 account_id 不一致时跳过（防御性）
            if config.account_id != name {
                tracing::warn!(
                    "[scan_orphan_accounts] 跳过不一致账户 {} (config.account_id={})",
                    name,
                    config.account_id
                );
                continue;
            }
            let entry = AccountEntry {
                id: config.account_id.clone(),
                name: config.name.clone(),
                created_at: config.created_at.clone(),
                last_accessed: config.last_login_at.clone(),
            };
            self.accounts_cache
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config.account_id.clone(), entry);
            tracing::info!(
                "[scan_orphan_accounts] 恢复孤儿账户 {} ({})",
                name,
                config.name
            );
            recovered.push(name);
        }
        if !recovered.is_empty() {
            self.save_accounts()?;
        }
        Ok(recovered)
    }

    pub fn list_accounts(&self) -> Vec<AccountSummary> {
        let cache = self.accounts_cache.read().ok();
        let accounts = match cache {
            Some(ref c) => c.values().cloned().collect::<Vec<_>>(),
            None => return vec![],
        };
        let mut result = Vec::new();
        for entry in &accounts {
            let config_rel = self.config_path_rel(&entry.id);
            let (password_hint, created_at, has_biometric_history, has_pin_history) =
                match self.fs.read_file(&config_rel) {
                    Ok(content) => match serde_json::from_slice::<AccountConfig>(&content) {
                        Ok(cfg) => (
                            cfg.password_hint,
                            Some(cfg.created_at),
                            cfg.biometric_enabled,
                            cfg.pin_enabled,
                        ),
                        Err(_) => (None, None, false, false),
                    },
                    Err(_) => (None, None, false, false),
                };

            result.push(AccountSummary {
                id: entry.id.clone(),
                name: entry.name.clone(),
                password_hint,
                created_at,
                has_biometric_history,
                has_pin_history,
            });
        }
        result
    }

    /// P015: create_account / create_account_with_id 的公共主体（各自入口校验通过后）。
    /// 派生密钥 → 写 config（原子）→ 写缓存 → 打开 Vault → 建立会话状态 → 返回摘要。
    /// 安全敏感代码（密钥派生/verify_hash/会话建立）收敛为单份，避免双份实现漂移。
    fn create_account_common(
        &self,
        account_id: &str,
        name: &str,
        password: &str,
        password_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let salt = generate_salt();
        let kdf_config = KdfConfig::from_env();
        let master_key = derive_key(password, &salt, &kdf_config)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

        let mk: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );

        let dir_rel = self.account_dir_rel(account_id);
        self.fs.create_dir_all(&dir_rel)?;
        self.ensure_private_dir(&dir_rel)?;

        let now = chrono::Utc::now().to_rfc3339();
        let config_data = AccountConfig {
            account_id: account_id.to_string(),
            name: name.to_string(),
            salt: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                salt.as_slice(),
            ),
            verify_hash,
            created_at: now.clone(),
            crypto_version: 3, // P2-010: HKDF-based verify hash
            biometric_enabled: false,
            pin_enabled: false,
            pin_length: 0,
            pin_failed_attempts: 0,
            pin_locked_until: None,
            password_failed_attempts: 0,
            password_locked_until: None,
            kdf_memory_kb: Some(kdf_config.memory_kb),
            kdf_iterations: Some(kdf_config.iterations),
            kdf_parallelism: Some(kdf_config.parallelism),
            password_hint: password_hint.map(|s| s.to_string()),
            last_login_at: Some(now.clone()),
            last_operation_at: None,
            last_operation_desc: None,
        };
        let config_json = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
        // P135: 原子写（.tmp + rename）——create 为关键写入路径。
        self.write_config_atomic(account_id, config_json.as_bytes())?;

        // Add to cache
        let entry = AccountEntry {
            id: account_id.to_string(),
            name: name.to_string(),
            created_at: now.clone(),
            last_accessed: Some(now),
        };
        // P001：RwLock 中毒按不可恢复处理——`into_inner()` 强制取回写锁，保证
        // 账户落盘后会话状态（accounts_cache / vault_store / session_key /
        // unlocked_account）一致建立，不再出现「账户已创建但会话状态部分缺失」
        // 的不一致（与 lock() 同款处理）。
        self.accounts_cache
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(account_id.to_string(), entry);
        self.save_accounts()?;

        // Open vault with data key
        let master_key_arr: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "HKDF output must be 32 bytes".to_string())?;
        let account_dir_path = self.fs.local_path(&dir_rel).ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(master_key_arr);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
        *self.vault_store.write().unwrap_or_else(|e| e.into_inner()) = Some(vault_arc);
        *self.session_key.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Zeroizing::new(master_key_arr));
        *self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(account_id.to_string());

        // P010: 返回值不再携带 salt/verifyHash——前端零消费（auth::bootstrap 仅读
        // id/name/passwordHint，CLI 仅读 id），暴露会扩大 WebView 攻击面（verifyHash
        // 可支持离线口令爆破）。两值仍写入磁盘 config（解锁/校验必需）。
        Ok(serde_json::json!({
            "id": account_id, "name": name,
            "passwordHint": config_data.password_hint,
        }))
    }

    pub fn create_account(
        &self,
        name: &str,
        password: &str,
        password_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }

        // R024: serialize account creation to eliminate the race between the
        // name-uniqueness check and writing the new account to disk/cache.
        let _create_guard = self.create_lock.lock().map_err(|e| e.to_string())?;

        let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
        if cache
            .values()
            .any(|a| a.name.to_lowercase() == name.to_lowercase())
        {
            return Err("Account name already taken".to_string());
        }
        drop(cache);

        let account_id = format!(
            "acc_{}",
            &uuid::Uuid::new_v4().to_string().replace("-", "")[..16]
        );
        self.create_account_common(&account_id, name, password, password_hint)
    }

    /// 使用指定的 account_id 创建账户（用于跨设备恢复等场景）。
    /// 账户身份以 `account_id` 为准：恢复场景允许同名账户（如大小写不同）共存，
    /// 因此不做账户名唯一性检查；仅当 `account_id` 在本机已存在时返回错误
    /// （"Account ID already exists"），由调用方决定是否覆盖恢复。
    pub fn create_account_with_id(
        &self,
        account_id: &str,
        name: &str,
        password: &str,
        password_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".to_string());
        }

        let _create_guard = self.create_lock.lock().map_err(|e| e.to_string())?;

        // 如果该 account_id 已经存在，直接拒绝，避免覆盖已有数据
        if self
            .accounts_cache
            .read()
            .map_err(|e| e.to_string())?
            .contains_key(account_id)
        {
            return Err("Account ID already exists".to_string());
        }

        self.create_account_common(account_id, name, password, password_hint)
    }

    /// 安全解锁：接受 Zeroizing<String> 主密码，避免调用侧额外明文拷贝。
    /// P225: 加载账户配置并派生主密钥（unlock / verify_password 共享前缀收敛）。
    /// 返回 (config, salt_arr, mk, master_key)。
    pub fn delete_account(&self, account_id: &str) -> Result<(), String> {
        self.lock();
        if let Ok(mut cache) = self.accounts_cache.write() {
            cache.remove(account_id);
        }
        self.save_accounts()?;
        let dir_rel = self.account_dir_rel(account_id);
        if self.fs.exists(&dir_rel).unwrap_or(false) {
            self.fs.remove_dir_all(&dir_rel)?;
        }
        Ok(())
    }

    pub fn reset_security_flags(&self, account_id: &str) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        config.biometric_enabled = false;
        config.pin_enabled = false;
        config.pin_length = 0;
        config.pin_failed_attempts = 0;
        config.pin_locked_until = None;

        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        // P135: 原子写——安全标志复位同样避免写坏 config。
        self.write_config_atomic(account_id, json.as_bytes())?;

        // 清理可能残留的凭证文件
        let dir_rel = self.account_dir_rel(account_id);
        // keystore_data.json（Android 双槽凭证）
        let keystore_path_rel = format!("{dir_rel}/keystore_data.json");
        if self.fs.exists(&keystore_path_rel).unwrap_or(false) {
            let _ = self.fs.remove_file(&keystore_path_rel);
        }
        // legacy biometric_key 文件
        let bio_key_rel = format!("{dir_rel}/biometric_key");
        if self.fs.exists(&bio_key_rel).unwrap_or(false) {
            let _ = self.fs.remove_file(&bio_key_rel);
        }
        // PIN 凭证文件：精确删除 pin_credential（无扩展名，PinManager 实际文件名），
        // 同时兼容历史遗留的 pin_*.cred 命名，通过本地路径枚举删除。
        if let Some(local_dir) = self.fs.local_path(&dir_rel) {
            if let Ok(entries) = std::fs::read_dir(&local_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str == "pin_credential"
                        || (name_str.starts_with("pin_") && name_str.ends_with(".cred"))
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
        // SAF/外部目录下 VaultFileSystem::remove_file 可能静默失败，
        // 再用本地 std::fs 路径兜底删除一次关键凭证文件。
        if let Some(local_dir) = self.fs.local_path(&dir_rel) {
            let keystore_local = local_dir.join("keystore_data.json");
            if keystore_local.exists() {
                if let Err(e) = std::fs::remove_file(&keystore_local) {
                    tracing::warn!(
                        "reset_security_flags: failed to remove keystore_data.json at {:?}: {}",
                        keystore_local,
                        e
                    );
                }
            }
            let bio_key_local = local_dir.join("biometric_key");
            if bio_key_local.exists() {
                if let Err(e) = std::fs::remove_file(&bio_key_local) {
                    tracing::warn!(
                        "reset_security_flags: failed to remove biometric_key at {:?}: {}",
                        bio_key_local,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "已重置账户 {} 的安全标志（biometric/pin 已关闭，残留凭证已清理）",
            account_id
        );
        Ok(())
    }

    pub fn update_password_hint(&self, account_id: &str, hint: &str) -> Result<(), String> {
        let config_rel = self.config_path_rel(account_id);
        let content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.password_hint = Some(hint.to_string());
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        // P135: 原子写——密码提示更新同样避免写坏 config。
        self.write_config_atomic(account_id, config_json.as_bytes())?;
        Ok(())
    }

    /// 修改账户名（账户 ID 不可变）：同步更新账户 config 与 accounts 清单。
    ///
    /// 名称唯一性检查与其他账户（大小写不敏感）冲突时报错，与 `create_account` 一致；
    /// 复用 `create_lock` 消除「检查唯一性 → 写入」之间的竞态（R024 同款）。
    pub fn rename_account(&self, account_id: &str, new_name: &str) -> Result<(), String> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err("Account name is required".to_string());
        }
        // R024: 与 create_account 共用锁，避免并发改名/创建导致重名。
        let _create_guard = self.create_lock.lock().map_err(|e| e.to_string())?;

        {
            let cache = self.accounts_cache.read().map_err(|e| e.to_string())?;
            if !cache.contains_key(account_id) {
                return Err("Account not found".to_string());
            }
            if cache
                .iter()
                .any(|(id, a)| id != account_id && a.name.to_lowercase() == name.to_lowercase())
            {
                return Err("Account name already taken".to_string());
            }
        }

        // 更新 config 中的名称（保留全部安全字段）。
        let mut config = self.read_account_config(account_id)?;
        config.name = name.to_string();
        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        // P135: 原子写——改名同样避免写坏 config。
        self.write_config_atomic(account_id, json.as_bytes())?;

        // 更新内存缓存与 accounts.json。
        if let Ok(mut cache) = self.accounts_cache.write() {
            if let Some(entry) = cache.get_mut(account_id) {
                entry.name = name.to_string();
            }
        }
        self.save_accounts()?;
        Ok(())
    }
}
