//! RF-001：短时会话提交门闩。锁顺序：session_gate → vault_store → session_key
//! → unlocked_account → VaultStore 内部锁。持门闩期间禁止网络、KDF、压缩或推理，
//! 也不能重入会话捕获、发布、锁定或提交 API。
use super::*;

const STALE_SESSION: &str = "Vault session is no longer current";

/// 请求开始时捕获的会话身份；不复制会话密钥，不向 IPC 序列化。
#[derive(Clone)]
pub struct VaultSession {
    account_id: String,
    generation: u64,
    vault: Arc<VaultStore>,
}

impl VaultSession {
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// 原会话的读取入口；写入与对外发布须通过 VaultService::with_session。
    /// 等待外部任务时只保留令牌，不持有门闩或数据库锁。
    pub fn vault(&self) -> &VaultStore {
        &self.vault
    }
}

impl VaultService {
    /// 固定原会话的附件密钥；门闩内只复制会话密钥，HKDF 在门闩外运行。
    pub fn attachment_key_for_session(
        &self,
        session: &VaultSession,
    ) -> Result<Zeroizing<[u8; 32]>, String> {
        let key = self.with_session(session, |_| {
            self.get_session_key()
                .ok_or_else(|| STALE_SESSION.to_string())
        })?;
        crate::attachment_crypto::derive_attachment_key(&key).map(Zeroizing::new)
    }

    pub fn capture_session(&self, expected_account: &str) -> Result<VaultSession, String> {
        let generation = self.session_gate.lock().map_err(|_| STALE_SESSION)?;
        let vault = self.get_vault_store().ok_or(STALE_SESSION)?;
        let account_id = self.get_current_account().ok_or(STALE_SESSION)?;
        if account_id != expected_account {
            return Err(STALE_SESSION.to_string());
        }
        Ok(VaultSession {
            account_id,
            generation: *generation,
            vault,
        })
    }

    /// 在同一短临界区内校验并执行同步提交，避免“检查后、写入前”被锁定切换。
    /// 回调只操作传入的原 Vault，不能重新获取当前账户、等待异步工作或重入此 API。
    /// 回调错误原样传播；需要多条写入原子性时由用例自身开启数据库事务。
    pub fn with_session<T>(
        &self,
        session: &VaultSession,
        commit: impl FnOnce(&VaultStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let generation = self.session_gate.lock().map_err(|_| STALE_SESSION)?;
        let current = self.get_vault_store().ok_or(STALE_SESSION)?;
        if *generation != session.generation
            || self.get_current_account().as_deref() != Some(session.account_id())
            || !Arc::ptr_eq(&current, &session.vault)
        {
            return Err(STALE_SESSION.to_string());
        }
        commit(&session.vault)
    }

    /// 准备阶段只捕获代次；昂贵操作结束后发布时再次核对，锁定不等待准备阶段。
    pub(super) fn session_generation(&self) -> Result<u64, String> {
        self.session_gate
            .lock()
            .map(|g| *g)
            .map_err(|_| STALE_SESSION.to_string())
    }

    /// 换密钥已完成后先撤销原会话；重开失败时保持锁定，不能保留旧钥句柄。
    pub(super) fn invalidate_session(&self, expected_generation: u64) -> Result<u64, String> {
        let mut generation = self.session_gate.lock().map_err(|_| STALE_SESSION)?;
        if *generation != expected_generation {
            return Err(STALE_SESSION.to_string());
        }
        *generation = generation.wrapping_add(1);
        self.clear_session_state();
        Ok(*generation)
    }

    /// 调用者必须持有 session_gate。
    pub(super) fn clear_session_state(&self) {
        let mut store = self.vault_store.write().unwrap_or_else(|e| e.into_inner());
        let mut session_key = self.session_key.write().unwrap_or_else(|e| e.into_inner());
        let mut account = self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(vault) = store.take() {
            vault.lock();
        }
        if let Some(mut key) = session_key.take() {
            key.zeroize();
        }
        account.take();
    }

    pub(super) fn publish_session(
        &self,
        account_id: &str,
        key: [u8; 32],
        vault: Arc<VaultStore>,
        expected_generation: u64,
    ) -> Result<(), String> {
        // SQLite 读取和新句柄准备均在门闩外完成，失败不会留下部分会话状态。
        let prefs = vault.device_sync_preferences()?.unwrap_or_default();
        vault.set_ui_prefs_sync_enabled(prefs.ui_prefs_sync_enabled);
        let mut generation = self.session_gate.lock().map_err(|_| STALE_SESSION)?;
        if *generation != expected_generation {
            vault.lock();
            return Err(STALE_SESSION.to_string());
        }
        let mut store = self.vault_store.write().unwrap_or_else(|e| e.into_inner());
        let mut session_key = self.session_key.write().unwrap_or_else(|e| e.into_inner());
        let mut account = self
            .unlocked_account
            .write()
            .unwrap_or_else(|e| e.into_inner());
        // 旧 Arc 即使被后台请求持有也必须擦除密钥并关闭连接。
        if let Some(old) = store.take() {
            old.lock();
        }
        *session_key = Some(Zeroizing::new(key));
        *account = Some(account_id.to_owned());
        *store = Some(vault);
        self.ui_prefs_sync_enabled
            .store(prefs.ui_prefs_sync_enabled, Ordering::SeqCst);
        // 即使代次绕回，令牌持有原 Arc，指针身份校验也禁止旧会话复活。
        *generation = generation.wrapping_add(1);
        Ok(())
    }
}
