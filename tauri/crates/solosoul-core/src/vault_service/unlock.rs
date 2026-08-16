//! VaultService 解锁/会话域（P025 拆分）。
//! 密钥派生、解锁/锁定、改密/重加密、会话密钥管理。
use super::*;
use crate::pin::PinManager;
use solosoul_crypto::kdf::{derive_key, generate_salt, KdfConfig};
use solosoul_crypto::secure::secure_compare;
use solosoul_vault::{VaultConfig, VaultStore};
use std::sync::Arc;
use zeroize::Zeroizing;

impl super::VaultService {
    fn load_config_and_derive_master_key(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<DerivedMasterKey, String> {
        // P135: 带孤儿 .tmp/.bak 恢复的读取（配合原子写）。
        let content = self
            .read_config_with_recovery(account_id)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        let config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;

        let kdf_config = config.kdf_config();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;

        let mk: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        Ok((config, salt_arr, mk, master_key))
    }

    /// P032：读取并解析账户 config（供锁定预检与失败计数 RMW 使用）。
    pub(crate) fn read_account_config(&self, account_id: &str) -> Result<AccountConfig, String> {
        let content = self
            .read_config_with_recovery(account_id)
            .map_err(|_| "Account not found".to_string())?;
        let content =
            String::from_utf8(content).map_err(|_| "Config encoding error".to_string())?;
        serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())
    }

    /// P032：主密码验证失败——递增失败计数，命中阶梯档位时写入锁定截止时间。
    /// 读-改-写经 `PASSWORD_ATTEMPT_LOCK` 原子化（镜像 PIN_OP_LOCK 模式），
    /// 防止并发解锁时计数丢更新。
    fn record_password_failure(&self, account_id: &str) -> Result<(), String> {
        let _guard = PASSWORD_ATTEMPT_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut config = self.read_account_config(account_id)?;
        let attempts = config.password_failed_attempts + 1;
        config.password_failed_attempts = attempts;
        let lockout_secs = password_lockout_seconds(attempts);
        config.password_locked_until = if lockout_secs > 0 {
            Some((chrono::Utc::now() + chrono::Duration::seconds(lockout_secs as i64)).to_rfc3339())
        } else {
            None
        };
        let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        self.write_config_atomic(account_id, json.as_bytes())
    }

    /// P032：主密码验证成功——归零失败计数与锁定。仅当有残留时才写 config，
    /// 避免每次成功登录都多一次磁盘写入。
    fn clear_password_failures(&self, account_id: &str, config: &AccountConfig) {
        if config.password_failed_attempts == 0 && config.password_locked_until.is_none() {
            return;
        }
        let _guard = PASSWORD_ATTEMPT_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut config = match self.read_account_config(account_id) {
            Ok(c) => c,
            Err(_) => return,
        };
        if config.password_failed_attempts == 0 && config.password_locked_until.is_none() {
            return;
        }
        config.password_failed_attempts = 0;
        config.password_locked_until = None;
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = self.write_config_atomic(account_id, json.as_bytes());
        }
    }

    /// P225: 计算 verify hash（crypto_version<3 旧 Argon2id 路径，否则 HKDF）。
    fn compute_verify_hash(
        config: &AccountConfig,
        mk: &[u8; 32],
        salt_arr: &[u8; 16],
        master_key: &[u8],
    ) -> Result<String, String> {
        if config.crypto_version < 3 {
            let verify_key = derive_key(
                &hex::encode(master_key),
                b"SOLOSOUL_VAULT_VERIFY_v1",
                &KdfConfig {
                    memory_kb: 8192,
                    iterations: 1,
                    parallelism: 1,
                },
            )
            .map_err(|_| "Verify failed".to_string())?;
            Ok(hex::encode(verify_key.as_slice()))
        } else {
            Ok(hex::encode(
                solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    mk,
                    salt_arr,
                    b"SOLOSOUL_VAULT_VERIFY_v1",
                )
                .map_err(|_| "Verify HKDF failed".to_string())?,
            ))
        }
    }

    /// R-4① 方案 2：恢复未完成的 reencrypt→config 两阶段交换。
    ///
    /// 崩溃窗口 = reencrypt 事务已提交、active config 未更新。此时
    /// `config.json.pending`（reencrypt 前写入的新 config 内容）残留：
    /// 用密码派生旧钥（active config）与新钥（pending config）各探测一次数据
    /// （`VaultStore::probe_data_key`），判定 reencrypt 是否已提交：
    /// - 新钥可解密 → reencrypt 已提交 → **promote**：pending 内容原子写为
    ///   active config，删除 pending，账户数据密钥切换到新钥；
    /// - 旧钥可解密 → reencrypt 未提交（事务回滚或从未执行）→ **discard**：
    ///   删除 pending，数据保持旧钥；
    /// - 两者都不可解 → 密码错误（或数据损坏）→ 不删 pending，上抛。
    ///
    /// 常态（无 pending 文件）零开销。probe 为只读，不触发迁移类副作用。
    fn recover_pending_reencrypt(&self, account_id: &str, password: &str) -> Result<(), String> {
        let pending_rel = self.config_pending_path_rel(account_id);
        if !self.fs.exists(&pending_rel).map_err(|e| e.to_string())? {
            return Ok(()); // 常态零开销
        }
        tracing::warn!(
            "R-4①: pending config found for {}, recovering interrupted reencrypt",
            account_id
        );

        // 读取并解析 pending（新）config。
        let pending_bytes = self
            .fs
            .read_file(&pending_rel)
            .map_err(|_| "Pending config read failed".to_string())?;
        let pending_content = String::from_utf8(pending_bytes.clone())
            .map_err(|_| "Pending config encoding error".to_string())?;
        let pending_config: AccountConfig = serde_json::from_str(&pending_content)
            .map_err(|_| "Pending config parse error".to_string())?;

        // 新钥探测：用密码 + pending config 派生新钥，尝试解密数据。
        if let Ok(new_key_arr) = self.derive_key_from_config(&pending_config, password) {
            if self.probe_data_key(account_id, &new_key_arr)? {
                // reencrypt 已提交 → promote：pending 内容写为 active config。
                tracing::info!("R-4①: promote pending config for {}", account_id);
                self.write_config_atomic(account_id, &pending_bytes)?;
                self.remove_config_pending(account_id);
                // 评审补强：同步更新生物识别凭证（promote 后数据=新钥，旧凭证陈旧会
                // 导致下一次生物识别解锁“打开成功但解密全失败”）与清除 PIN 凭证，
                // 镜像 change_password/unlock_with_kdf_upgrade 成功路径的尾部。
                self.refresh_credentials_after_promote(account_id, &new_key_arr);
                return Ok(());
            }
        }

        // 旧钥探测：用密码 + active config 派生旧钥，尝试解密数据。
        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        let _ = (config, salt_arr, master_key);
        if self.probe_data_key(account_id, &mk)? {
            // reencrypt 未提交 → discard：删除 pending，数据保持旧钥。
            tracing::info!("R-4①: discard pending config for {}", account_id);
            self.remove_config_pending(account_id);
            return Ok(());
        }

        // 密码错误（或数据损坏）：保留 pending 供下次重试/人工恢复。
        // 附带 pending 存在提示，帮助用户在「改密后崩溃、新旧密码不确定」场景下判断
        // 应尝试哪一侧密码（数据侧密钥为准）。
        Err(
            "Invalid password (interrupted key rotation pending; try the other password)"
                .to_string(),
        )
    }

    /// R-4① 方案 2：用给定密钥探测 vault.db 数据可解性。
    /// 纯只读独立连接（solosoul_vault::probe_data_key），不触发 open 的迁移/
    /// 回填副作用，可安全用错误密钥调用。
    fn probe_data_key(&self, account_id: &str, key: &[u8; 32]) -> Result<bool, String> {
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let db_path = account_dir_path.join("vault.db");
        solosoul_vault::probe_data_key(&db_path, &solosoul_vault::DataEncryptionKey::new(*key))
    }

    /// R-4① 方案 2：promote 后同步凭证（best-effort，失败仅记日志）。
    /// 镜像 change_password / unlock_with_kdf_upgrade 成功路径尾部：
    /// 生物识别凭证更新为新钥；PIN 凭证（旧钥）清除后由用户重新设置。
    fn refresh_credentials_after_promote(&self, account_id: &str, new_key: &[u8; 32]) {
        {
            let bio_manager = make_biometric_manager(self.base_path().clone());
            let new_key_hex = hex::encode(new_key.as_slice());
            if let Err(e) = bio_manager.update_credential(account_id, &new_key_hex) {
                tracing::warn!(
                    "Failed to update biometric credential after R-4① promote for {}: {}",
                    account_id,
                    e
                );
            }
        }
        {
            let pin_manager = PinManager::new(self.base_path().clone());
            if let Err(e) = pin_manager.clear_credential(account_id) {
                tracing::warn!(
                    "Failed to clear PIN credential after R-4① promote for {}: {}",
                    account_id,
                    e
                );
            }
        }
    }

    /// R-4① 方案 2：从给定的 AccountConfig 派生主密钥（[u8; 32]）。
    fn derive_key_from_config(
        &self,
        config: &AccountConfig,
        password: &str,
    ) -> Result<[u8; 32], String> {
        let salt_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
                .map_err(|_| "Invalid salt".to_string())?;
        let salt_arr: [u8; 16] = salt_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid salt length".to_string())?;
        let kdf_config = config.kdf_config();
        let master_key = derive_key(password, &salt_arr, &kdf_config)
            .map_err(|_| "Key derivation failed".to_string())?;
        master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())
    }

    pub fn unlock_secure(
        &self,
        account_id: &str,
        password: &Zeroizing<String>,
    ) -> Result<(), String> {
        self.unlock(account_id, password.as_ref())
    }

    pub fn unlock(&self, account_id: &str, password: &str) -> Result<(), String> {
        // R-4① 方案 2：解锁入口先恢复未完成的 reencrypt→config 交换。
        // 常态（无 pending 文件）零开销；有 pending 时 promote/discard 后
        // 再走正常解锁路径（config 已恢复一致，verify 与数据密钥对齐）。
        self.recover_pending_reencrypt(account_id, password)?;

        // P032：主密码失败限流——锁定预检放在昂贵 KDF 之前。
        let pre_config = self.read_account_config(account_id)?;
        if let Some(ref until) = pre_config.password_locked_until {
            if let Ok(until_time) = chrono::DateTime::parse_from_rfc3339(until) {
                if chrono::Utc::now() < until_time {
                    return Err(MASTER_PASSWORD_LOCKED_ERR.to_string());
                }
            }
        }

        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash =
            Self::compute_verify_hash(&config, &mk, &salt_arr, master_key.as_slice())?;

        if !secure_compare(computed_hash.as_bytes(), config.verify_hash.as_bytes()) {
            // P032：失败递增计数并持久化（阶梯锁定），随后返回原错误文案。
            self.record_password_failure(account_id)?;
            return Err("Invalid password".to_string());
        }

        // P032：验证成功归零失败计数/锁定（有残留时才写，避免每次登录多余 IO）。
        self.clear_password_failures(account_id, &config);

        // Update last accessed
        let _now = chrono::Utc::now().to_rfc3339();
        if let Ok(mut cache) = self.accounts_cache.write() {
            if let Some(entry) = cache.get_mut(account_id) {
                entry.last_accessed = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        self.save_accounts().ok();

        // P003: 已有账户若使用低于生产档的 KDF 参数（开发档 8MiB/2iter 或平衡档
        // 16MiB/3iter），在 release 构建下解锁成功后透明升级到生产参数并重加密
        // 整个 Vault。debug 构建保持开发档以加速本地开发/测试。
        if !cfg!(debug_assertions) && config.kdf_config() != KdfConfig::production() {
            return self.unlock_with_kdf_upgrade(account_id, password, &master_key);
        }

        // Store session key
        let master_key_arr: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Argon2id output must be 32 bytes".to_string())?;
        // P001：锁中毒按不可恢复处理——`into_inner()` 强制取回写锁（与
        // create_account_common / lock() 同款），杜绝 unlock 成功后会话状态
        // 部分缺失（密钥已设、unlocked_account/vault_store 未设）的不一致。
        *self.session_key.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Zeroizing::new(master_key_arr));
        *self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(account_id.to_string());

        // Open vault with data key
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(master_key_arr);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
        *self.vault_store.write().unwrap_or_else(|e| e.into_inner()) = Some(vault_arc);

        // 用户已通过主密码验证身份，重置 PIN 锁定状态。
        let pin_manager = PinManager::new(self.base_path().clone());
        if let Err(e) = pin_manager.reset_attempts(account_id) {
            tracing::warn!("Failed to reset PIN attempts after unlock: {}", e);
        }

        Ok(())
    }

    /// N-2：reencrypt→config 两阶段失败回滚。reencrypt_all 成功后若 config 写入失败，
    /// 账户会处于“数据已换新钥、config 仍记旧参数”的不可用态。本方法恢复旧 config，
    /// 并在调用方持有的同一 VaultStore 上切换内存密钥为新钥读回数据、再重加密回旧钥，
    /// 保持账户一致可用。
    ///
    /// R-4：返回 `Result<(), String>`——回滚自身失败（磁盘满等共同根因）必须上抛，
    /// 由调用方并入错误文案，不再以「已尝试自动回滚」掩盖回滚未生效的事实。
    fn rollback_reencrypt_and_config(
        &self,
        account_id: &str,
        vault: &VaultStore,
        old_config_content: &[u8],
        old_key: &solosoul_vault::DataEncryptionKey,
        new_key: &solosoul_vault::DataEncryptionKey,
    ) -> Result<(), String> {
        // 1) 恢复旧 config（盐/参数/verify_hash 与旧密钥一致）——原子写，
        //    避免回滚自身再次写坏 config。
        self.fs
            .write_file_atomic(&self.config_path_rel(account_id), old_config_content)
            .map_err(|e| format!("rollback: failed to restore config: {}", e))?;
        // 2) 数据重加密回旧密钥：同一 store 先切内存密钥为新钥读回，再以旧钥写回
        vault.set_data_key(new_key.clone());
        if let Err(e) = vault.reencrypt_all(new_key, old_key) {
            // 先恢复内存密钥再上抛，避免调用方在回滚失败后仍持有错误的内存密钥
            vault.set_data_key(old_key.clone());
            return Err(format!(
                "rollback: re-encrypt back to old key failed: {}",
                e
            ));
        }
        vault.set_data_key(old_key.clone());
        Ok(())
    }

    /// P003：将账户 KDF 参数透明升级到生产档并重加密整个 Vault。
    ///
    /// 仅在 `unlock` 成功验证密码后调用（release 构建、存储参数低于生产档时）。
    /// 流程与 `change_password` 一致：用旧密钥打开 Vault → `reencrypt_all` 重加密
    /// 全部数据 → 更新 config（新 salt / 新 verify hash / 生产参数）→ 用新密钥重开
    /// Vault → 同步更新生物识别凭证、清除 PIN 凭证（其保存的旧密钥已失效）。
    pub(crate) fn unlock_with_kdf_upgrade(
        &self,
        account_id: &str,
        password: &str,
        old_master_key: &Zeroizing<Vec<u8>>,
    ) -> Result<(), String> {
        // 旧密钥（已验证通过）。
        let old_key_arr: [u8; 32] = old_master_key
            .as_slice()
            .try_into()
            .map_err(|_| "Master key must be 32 bytes".to_string())?;
        let old_key = solosoul_vault::DataEncryptionKey::new(old_key_arr);

        // 新 salt + 生产参数派生新主密钥。
        let salt = generate_salt();
        let new_kdf_config = KdfConfig::production();
        let new_key = derive_key(password, &salt, &new_kdf_config)
            .map_err(|e| format!("New key derivation failed: {}", e))?;
        let new_key_arr: [u8; 32] = new_key
            .as_slice()
            .try_into()
            .map_err(|_| "Key derivation output must be 32 bytes".to_string())?;
        let new_key_enc = solosoul_vault::DataEncryptionKey::new(new_key_arr);

        // N-2：备份旧 config——reencrypt 成功后若 config 写入失败，恢复旧 config 并
        // 把数据重加密回旧密钥，避免“数据已换新钥、config 仍记旧参数”的账户不可用态。
        let config_rel = self.config_path_rel(account_id);
        let old_config_content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;

        // 更新 config：新 salt、新 verify hash、生产参数。
        let content = String::from_utf8(old_config_content.clone())
            .map_err(|_| "Config encoding error".to_string())?;
        let mut config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;
        config.crypto_version = 3; // P2-010: HKDF-based verify hash
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        let mk: [u8; 32] = new_key_arr;
        config.verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );
        config.kdf_memory_kb = Some(new_kdf_config.memory_kb);
        config.kdf_iterations = Some(new_kdf_config.iterations);
        config.kdf_parallelism = Some(new_kdf_config.parallelism);
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

        // R-4① 方案 2：reencrypt 前先把新 config 原子写入 pending 载体。
        self.write_config_pending(account_id, config_json.as_bytes())?;

        // 用旧密钥打开 Vault 并重加密全部数据。
        // N-2：reencrypt_all 事务内全有或全无（任一行失败整体回滚，数据保持旧密钥）。
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(old_key_arr);
        let vault = VaultStore::open(vault_config)
            .map_err(|e| format!("Failed to open vault for KDF upgrade: {}", e))?;
        if let Err(e) = vault.reencrypt_all(&old_key, &new_key_enc) {
            // reencrypt 失败（事务回滚，数据仍为旧钥）：pending 无意义，清除后上抛。
            self.remove_config_pending(account_id);
            return Err(format!("KDF upgrade re-encryption failed: {}", e));
        }

        // P135: 原子写——config 更新为关键写入路径。
        if let Err(e) = self.write_config_atomic(account_id, config_json.as_bytes()) {
            // N-2：config 写入失败 → 回滚（恢复旧 config + 数据重加密回旧密钥），
            // 保持账户一致可用，并把失败原因上抛。
            // R-4：回滚自身失败（磁盘满等共同根因）必须并入上抛文案。
            let rollback_note = match self.rollback_reencrypt_and_config(
                account_id,
                &vault,
                &old_config_content,
                &old_key,
                &new_key_enc,
            ) {
                Ok(_) => {
                    // 回滚成功：数据已重加密回旧钥、config 已恢复 → pending 同步清除。
                    self.remove_config_pending(account_id);
                    "an automatic rollback to the previous key was attempted.".to_string()
                }
                Err(rb) => {
                    // 回滚失败：保留 pending——数据可能仍为新钥，下次解锁经
                    // recover_pending_reencrypt promote 可完成交换（恢复线索）。
                    format!("automatic rollback FAILED: {}", rb)
                }
            };
            return Err(format!(
                "KDF upgrade failed to update config: {}; {}",
                e, rollback_note
            ));
        }
        // R-4① 方案 2：config 写入成功 → 交换完成，清除 pending。
        self.remove_config_pending(account_id);
        // 释放临时 Vault 连接，随后以新密钥重开（避免同一 DB 双连接）
        drop(vault);

        // 更新会话密钥并重开 Vault（新密钥）。
        // P001：锁中毒按不可恢复处理（同 unlock 侧统一），杜绝改密/KDF 升级后
        // unlocked_account 未设置的会话不一致。
        *self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(account_id.to_string());
        self.reopen_vault_with_new_key(
            account_id,
            new_key_arr,
            "KDF upgrade succeeded but vault reopen failed",
        )?;

        // 此路径不再单独调用 pin_manager.reset_attempts：clear_credential 已
        // 将 pin_failed_attempts 归零并清除 pin_locked_until，与 unlock 尾部的
        // reset_attempts 效果等价。

        // 生物识别凭证保存的是旧主密钥，需同步更新。
        self.update_credentials_after_rekey(account_id, new_key_arr, "KDF upgrade");

        // PIN 凭证保存的是旧主密钥，且重加密需要 PIN 输入（不可用），清除后由用户重新设置。
        self.clear_pin_credential_after_rekey(account_id, "KDF upgrade");

        // SAF 远端存储：本地 vault.db 已用新密钥重加密，同步到远端避免
        // 下次 sync_from_remote 用旧副本覆盖。
        self.sync_remote_after_rekey(
            account_id,
            "KDF upgrade",
            "KDF upgrade failed to sync encrypted data to remote storage",
            "",
        )?;

        tracing::info!("KDF params upgraded to production for {}", account_id);
        Ok(())
    }

    /// P001：RwLock 中毒（持锁线程 panic）按不可恢复处理——`into_inner()` 强制取回写锁
    /// 继续执行。「锁定即擦除会话密钥」是核心安全不变量：静默跳过清零会让派生密钥
    /// 在锁定后仍驻留内存（fail-open）。即使写锁内状态因 panic 部分更新，取回后
    /// 执行 `take()`/`zeroize()` 仍保证各状态收敛（密钥清零、会话与 vault 句柄移除）。
    pub fn lock(&self) {
        let mut store = self.vault_store.write().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut v) = *store {
            v.lock();
        }
        store.take();
        if let Some(mut k) = self
            .session_key
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            k.zeroize();
        }
        self.unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }

    pub fn is_unlocked(&self) -> bool {
        let key = self.session_key.read().ok();
        let ua = self.unlocked_account.read().ok();
        key.map(|k| k.is_some()).unwrap_or(false) && ua.map(|u| u.is_some()).unwrap_or(false)
    }

    /// Verify whether the given password matches the account's master password.
    /// Does NOT unlock (no session key storage).
    /// R-4① 方案 2：入口会先恢复未完成的 reencrypt→config 交换（promote 时原子改写
    /// config.json / 删除 pending）——这是崩溃恢复所需的修复性副作用，非解锁状态变更。
    /// Verify hash is derived from the Argon2id master key using HKDF-SHA256 (P2-010).
    pub fn verify_password(&self, account_id: &str, password: &str) -> Result<bool, String> {
        // R-4① 方案 2：verify 入口同样先恢复 pending（reencrypt→config 交换），
        // 保证密码校验基于一致的 config；常态零开销。
        self.recover_pending_reencrypt(account_id, password)?;
        let (config, salt_arr, mk, master_key) =
            self.load_config_and_derive_master_key(account_id, password)?;
        // Backward compat: crypto_version < 3 uses old Argon2id verify hash
        let computed_hash =
            Self::compute_verify_hash(&config, &mk, &salt_arr, master_key.as_slice())?;

        Ok(secure_compare(
            computed_hash.as_bytes(),
            config.verify_hash.as_bytes(),
        ))
    }

    /// Unlock vault with a pre-derived session key (used by biometric unlock).
    /// The session key must match the account's encryption key.
    pub fn unlock_with_session_key(
        &self,
        account_id: &str,
        session_key: &[u8; 32],
    ) -> Result<(), String> {
        // R-4① 方案 2：存在未完成的 reencrypt→config 交换时，会话密钥（生物识别/
        // PIN）可能是旧钥而数据已是新钥——需密码派生密钥才能恢复，这里显式拒绝
        // 并引导走密码解锁（recover_pending_reencrypt 会完成交换）。
        let pending_rel = self.config_pending_path_rel(account_id);
        if self.fs.exists(&pending_rel).map_err(|e| e.to_string())? {
            return Err(
                "Pending key rotation detected; please unlock with your password".to_string(),
            );
        }

        // Set session key
        // P001：锁中毒按不可恢复处理——`into_inner()` 强制取回写锁（与
        // create_account_common / lock() 同款），杜绝会话密钥解锁成功后
        // unlocked_account/vault_store 部分缺失的不一致。
        *self.session_key.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Zeroizing::new(*session_key));
        *self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(account_id.to_string());

        // Open vault with data key
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(*session_key);
        let vault =
            VaultStore::open(vault_config).map_err(|e| format!("Failed to open vault: {}", e))?;
        let vault_arc = Arc::new(vault);
        // 设备级偏好同步开关：unlock 新建 VaultStore 后应用期望值（默认 true）。
        vault_arc.set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
        *self.vault_store.write().unwrap_or_else(|e| e.into_inner()) = Some(vault_arc);

        // 用户已通过更强因子（生物识别 / PIN 本身）验证身份，重置 PIN 锁定状态。
        let pin_manager = PinManager::new(self.base_path().clone());
        if let Err(e) = pin_manager.reset_attempts(account_id) {
            tracing::warn!("Failed to reset PIN attempts after unlock: {}", e);
        }

        Ok(())
    }

    /// P014 共享尾部：用新数据密钥重开 Vault 并刷新会话密钥（change_password /
    /// unlock_with_kdf_upgrade 共用）。err_prefix 拼入重开失败的错误消息。
    fn reopen_vault_with_new_key(
        &self,
        account_id: &str,
        new_key_arr: [u8; 32],
        err_prefix: &str,
    ) -> Result<(), String> {
        // P001：锁中毒按不可恢复处理——`into_inner()` 强制取回写锁（与
        // create_account_common / lock() 同款）。改密/KDF 升级的关键路径上
        // 静默跳过会话密钥/句柄更新会导致「新钥已生效但会话状态未切换」。
        *self.session_key.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Zeroizing::new(new_key_arr));
        *self.vault_store.write().unwrap_or_else(|e| e.into_inner()) = None;
        let account_dir_path = self
            .fs
            .local_path(&self.account_dir_rel(account_id))
            .ok_or("无法解析账户本地目录")?;
        let vault_config =
            VaultConfig::new(account_id, account_dir_path).with_data_key(new_key_arr);
        match VaultStore::open(vault_config) {
            Ok(vault) => {
                let vault_arc = Arc::new(vault);
                // 设备级偏好同步开关：新建 VaultStore 后应用期望值（默认 true）。
                vault_arc
                    .set_ui_prefs_sync_enabled(self.ui_prefs_sync_enabled.load(Ordering::SeqCst));
                *self.vault_store.write().unwrap_or_else(|e| e.into_inner()) = Some(vault_arc);
            }
            Err(e) => {
                return Err(format!("{}: {}", err_prefix, e));
            }
        }
        Ok(())
    }

    /// P014 共享尾部：改密/KDF 升级后同步更新生物识别凭证中保存的主密钥。
    fn update_credentials_after_rekey(
        &self,
        account_id: &str,
        new_key_arr: [u8; 32],
        context: &str,
    ) {
        let bio_manager = make_biometric_manager(self.base_path().clone());
        let new_key_hex = hex::encode(new_key_arr.as_slice());
        if let Err(e) = bio_manager.update_credential(account_id, &new_key_hex) {
            tracing::warn!(
                "Failed to update biometric credential after {} for {}: {}",
                context,
                account_id,
                e
            );
        }
    }

    /// P014 共享尾部：改密/KDF 升级后清除 PIN 凭证（重加密需 PIN 输入不可用），由用户重新设置。
    fn clear_pin_credential_after_rekey(&self, account_id: &str, context: &str) {
        let pin_manager = PinManager::new(self.base_path().clone());
        if let Err(e) = pin_manager.clear_credential(account_id) {
            tracing::warn!(
                "Failed to clear PIN credential after {} for {}: {}",
                context,
                account_id,
                e
            );
        }
    }

    /// P014 共享尾部：将重加密的 vault.db 同步到 SAF 远端，避免下次 sync_from_remote
    /// 用旧密钥副本覆盖本地。err_detail 仅在非空时拼入用户可见错误（change_password 提供）。
    fn sync_remote_after_rekey(
        &self,
        account_id: &str,
        context: &str,
        err_prefix: &str,
        err_detail: &str,
    ) -> Result<(), String> {
        if !self.is_remote_storage() {
            return Ok(());
        }
        if let Err(e) = self.sync_to_remote() {
            tracing::error!(
                "Failed to sync re-encrypted vault.db to SAF after {} for {}: {}",
                context,
                account_id,
                e
            );
            return Err(if err_detail.is_empty() {
                format!("{}: {}", err_prefix, e)
            } else {
                format!("{}: {}. {}", err_prefix, e, err_detail)
            });
        }
        tracing::info!(
            "Successfully synced re-encrypted vault.db to SAF after {} for {}",
            context,
            account_id
        );
        Ok(())
    }

    pub fn change_password(
        &self,
        account_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        // Verify old password first; this opens the vault with the old data key.
        self.unlock(account_id, old_password)?;

        // Capture old data key.
        let old_key_arr = self
            .get_session_key()
            .ok_or("Session key not available after unlock")?;
        let old_key = solosoul_vault::DataEncryptionKey::new(*old_key_arr);

        // Generate new salt and derive new key.
        let salt = generate_salt();
        let new_kdf_config = KdfConfig::from_env();
        let new_key = derive_key(new_password, &salt, &new_kdf_config)
            .map_err(|e| format!("New key derivation failed: {}", e))?;
        let new_key_arr: [u8; 32] = new_key
            .as_slice()
            .try_into()
            .map_err(|_| "Key derivation output must be 32 bytes".to_string())?;
        let new_key_enc = solosoul_vault::DataEncryptionKey::new(new_key_arr);

        // N-2：在 reencrypt 之前读取并解析旧 config（备份 + 校验）。任何读取/解析失败
        // 都发生在数据改动之前——若失败直接返回，杜绝"数据已换新钥、config 仍记旧参数"
        // 的混态（旧实现把读取放在 reencrypt 之后，读取失败会留下混态）。
        let config_rel = self.config_path_rel(account_id);
        let old_config_content = self
            .fs
            .read_file(&config_rel)
            .map_err(|_| "Account not found".to_string())?;
        let content = String::from_utf8(old_config_content.clone())
            .map_err(|_| "Config encoding error".to_string())?;
        let old_config: AccountConfig =
            serde_json::from_str(&content).map_err(|_| "Config parse error".to_string())?;

        // Derive new verify hash via HKDF (P2-010).
        let mk: [u8; 32] = new_key_arr; // already 32 bytes from try_into above
        let verify_hash = hex::encode(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&mk, &salt, b"SOLOSOUL_VAULT_VERIFY_v1")
                .map_err(|e| format!("Verify HKDF failed: {}", e))?,
        );

        // 基于 reencrypt 前解析的旧 config 派生新 config。
        let mut config = old_config;
        config.crypto_version = 3; // P2-010: HKDF-based verify hash
        config.salt =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt.as_slice());
        config.verify_hash = verify_hash;
        config.kdf_memory_kb = Some(new_kdf_config.memory_kb);
        config.kdf_iterations = Some(new_kdf_config.iterations);
        config.kdf_parallelism = Some(new_kdf_config.parallelism);
        let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

        // R-4① 方案 2：reencrypt 前先把新 config 原子写入 pending 载体——
        // 崩溃后残留的 pending 由 unlock/verify 入口 recover_pending_reencrypt
        // 用 probe 判定 reencrypt 是否已提交：已提交 → promote（pending 升为
        // active），未提交 → discard（删 pending）。
        self.write_config_pending(account_id, config_json.as_bytes())?;

        // Re-encrypt all sensitive data with the new key（reencrypt_all 事务内全有或全无）。
        {
            let vault_guard = self
                .get_vault_store()
                .ok_or("Vault not available for re-encryption")?;
            let vault = vault_guard.as_ref();
            if let Err(e) = vault.reencrypt_all(&old_key, &new_key_enc) {
                // reencrypt 失败（事务回滚，数据仍为旧钥）：pending 无意义，清除后上抛。
                self.remove_config_pending(account_id);
                return Err(format!("Re-encryption failed: {}", e));
            }
        }

        // P135: 原子写——config 更新为关键写入路径。
        if let Err(e) = self.write_config_atomic(account_id, config_json.as_bytes()) {
            // N-2：config 写入失败 → 回滚（恢复旧 config + 数据重加密回旧密钥），
            // 避免账户不可用；会话密钥尚未切换，回滚后当前会话仍以旧密钥工作。
            // R-4：回滚自身失败（磁盘满等共同根因）必须并入上抛文案。
            let rollback_note = if let Some(vault_guard) = self.get_vault_store() {
                match self.rollback_reencrypt_and_config(
                    account_id,
                    vault_guard.as_ref(),
                    &old_config_content,
                    &old_key,
                    &new_key_enc,
                ) {
                    Ok(_) => {
                        // 回滚成功：数据已重加密回旧钥、config 已恢复 → pending 同步清除。
                        self.remove_config_pending(account_id);
                        "an automatic rollback to the previous key was attempted.".to_string()
                    }
                    Err(rb) => {
                        // 回滚失败：保留 pending——数据可能仍为新钥，下次解锁经
                        // recover_pending_reencrypt promote 可完成交换（恢复线索）。
                        format!("automatic rollback FAILED: {}", rb)
                    }
                }
            } else {
                "automatic rollback skipped (vault unavailable)".to_string()
            };
            return Err(format!(
                "Password updated but config write failed: {}; {}",
                e, rollback_note
            ));
        }

        // R-4① 方案 2：config 写入成功 → 交换完成，清除 pending。
        self.remove_config_pending(account_id);

        // Update session key and reopen vault with new data key.
        self.reopen_vault_with_new_key(
            account_id,
            new_key_arr,
            "Password updated but vault reopen failed",
        )?;

        // 如果用户已启用生物识别，同步更新其中保存的主密钥，使改密后 Touch ID 仍可用。
        self.update_credentials_after_rekey(account_id, new_key_arr, "password change");

        // 如果用户已启用 PIN 解锁，同步更新 PIN 凭证。
        // 由于 PIN 派生 KEK 时需要 PIN 输入（不可用），此处清除凭证并标记为未配置，
        // 用户需要重新设置 PIN。
        self.clear_pin_credential_after_rekey(account_id, "password change");

        // 关键修复：reencrypt_all 已将 vault.db 用新密钥重新加密到本地临时目录，
        // 但在 Android SAF 模式下，本地临时目录与远端 SAF 存储是分离的。
        // 若不主动 sync_to_remote，重新登录时 sync_from_remote 会用旧的 SAF 副本
        // （仍用旧密钥加密）覆盖本地 vault.db，导致所有解密失败（object not found /
        // audit details decryption failed）。
        self.sync_remote_after_rekey(
            account_id,
            "password change",
            "Password updated but failed to sync encrypted data to remote storage",
            concat!(
                "The local database is correct but the remote copy is stale. ",
                "Please retry syncing from Settings — do NOT restart the app before syncing, ",
                "as that would overwrite the local data with the stale remote copy."
            ),
        )?;

        Ok(())
    }

    pub fn get_session_key(&self) -> Option<zeroize::Zeroizing<[u8; 32]>> {
        self.session_key.read().ok()?.clone()
    }

    pub fn get_vault_store(&self) -> Option<Arc<solosoul_vault::VaultStore>> {
        if !self.is_unlocked() {
            return None;
        }
        self.vault_store.read().ok().and_then(|g| g.clone())
    }

    pub fn get_current_account(&self) -> Option<String> {
        self.unlocked_account.read().ok()?.clone()
    }
}
