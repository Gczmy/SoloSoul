//! VaultService SAF 同步域（P025 拆分）。
//! 远端（SAF）存储同步与脏标记。
use super::*;

impl super::VaultService {
    pub fn set_ui_prefs_sync_enabled(&self, enabled: bool) {
        self.ui_prefs_sync_enabled.store(enabled, Ordering::SeqCst);
        if let Some(v) = self.get_vault_store() {
            v.set_ui_prefs_sync_enabled(enabled);
        }
    }

    /// 读取设备级「同步设置偏好」开关（默认 true）。
    pub fn ui_prefs_sync_enabled(&self) -> bool {
        self.ui_prefs_sync_enabled.load(Ordering::SeqCst)
    }

    /// P027: 取底层文件系统句柄（Arc 克隆）——调用方可在短暂持锁取到句柄后
    /// 释放锁再做长时间操作（如网络同步），避免 std RwLock 写者饥饿。
    pub fn file_system(&self) -> Arc<dyn VaultFileSystem> {
        self.fs.clone()
    }

    pub fn sync_to_remote(&self) -> Result<(), String> {
        self.fs.sync_to_remote()
    }

    /// 从远端存储（如 SAF）同步 Vault 数据到本地。
    /// 若当前文件系统为本地文件系统，则为空操作。
    pub fn sync_from_remote(&self) -> Result<(), String> {
        self.fs.sync_from_remote()
    }

    /// 如果底层文件系统支持脏标记，同步尚未同步到远端的脏数据。
    /// 适用于定期后台自动同步的调用场景。
    pub fn sync_if_dirty(&self) -> Result<(), String> {
        self.fs.sync_if_dirty()
    }

    /// 当前 Vault 是否有尚未同步到远端的脏数据。
    pub fn is_dirty(&self) -> bool {
        self.fs.is_dirty()
    }

    /// 当前 Vault 是否使用远端（SAF）存储。
    pub fn is_remote_storage(&self) -> bool {
        self.fs.is_remote()
    }
}
