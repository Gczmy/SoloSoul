//! Vault 文件系统抽象层
//!
//! 将 Vault 所需的文件操作抽象为 trait，使上层业务代码（VaultService、
//! VaultStore 等）不再直接依赖 `std::fs`。桌面端/App-private 模式使用
//! `LocalVaultFileSystem`；Android 用户自定义目录模式后续通过 SAF-backed
//! 实现替换。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Vault 文件系统抽象。
///
/// 所有路径均为相对于 Vault 根目录的相对路径（例如 `"accounts.json"`、
/// `"acc_xxx/config.json"`）。实现者应拒绝任何可能导致逃逸出根目录的路径。
pub trait VaultFileSystem: Send + Sync {
    /// 读取文件内容。
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String>;

    /// 写入文件（覆盖）。父目录不存在时会自动创建。
    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String>;

    /// 删除文件。
    fn remove_file(&self, relative_path: &str) -> Result<(), String>;

    /// 判断文件/目录是否存在。
    fn exists(&self, relative_path: &str) -> Result<bool, String>;

    /// 递归创建目录。
    fn create_dir_all(&self, relative_path: &str) -> Result<(), String>;

    /// 递归删除目录。
    fn remove_dir_all(&self, relative_path: &str) -> Result<(), String>;

    /// 列出目录下的直接子项名称（仅文件名/目录名）。
    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String>;

    /// 若底层是本地文件系统，返回对应绝对路径；非本地实现返回 `None`。
    ///
    /// 该接口主要用于兼容仍需 `std::path::Path` 的场景（如 SQLite 数据库路径）。
    fn local_path(&self, relative_path: &str) -> Option<PathBuf>;

    /// 将本地数据同步到远端（SAF）存储。
    ///
    /// 默认实现为空操作，适用于本地文件系统。
    fn sync_to_remote(&self) -> Result<(), String> {
        Ok(())
    }

    /// 从远端（SAF）存储同步数据到本地。
    ///
    /// 默认实现为空操作，适用于本地文件系统。
    fn sync_from_remote(&self) -> Result<(), String> {
        Ok(())
    }

    /// 返回该文件系统是否基于远端（SAF）存储。
    ///
    /// 默认实现返回 false。
    fn is_remote(&self) -> bool {
        false
    }

    /// 将尚未同步到远端的脏数据同步到远端。
    ///
    /// 如果实现支持脏标记（dirty flag），每次写操作后应标记为脏，
    /// 然后可由后台任务定期调用此方法进行同步。
    /// 默认实现为空操作（适用于本地文件系统）。
    fn sync_if_dirty(&self) -> Result<(), String> {
        Ok(())
    }

    /// 返回是否有尚未同步到远端的脏数据。
    ///
    /// 默认实现返回 false（本地文件系统无脏标记）。
    fn is_dirty(&self) -> bool {
        false
    }
}

/// SAF-backed VaultFileSystem 的同步策略枚举。
///
/// 由于 SQLite 无法直接在 SAF `content://` URI 上工作，本实现将所有数据
/// 读写先落在本地临时目录，再根据策略与 SAF 目录同步。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafSyncStrategy {
    /// 仅在显式调用 sync_to_saf/sync_from_saf 时同步。
    Manual,
    /// 每次写操作后自动同步到 SAF（未完成，预留）。
    AutoOnWrite,
}

/// SAF 同步驱动 trait。
///
/// 由于 `solosoul-core` 不依赖 Tauri/移动端桥接，具体同步逻辑由上层（Tauri app）
/// 注入。实现者负责把 `local_dir` 中的内容与 SAF `tree_uri` 指向的目录同步。
pub trait SafSyncDriver: Send + Sync {
    fn sync_to_remote(&self, local_dir: &std::path::Path, tree_uri: &str) -> Result<(), String>;
    fn sync_from_remote(&self, local_dir: &std::path::Path, tree_uri: &str) -> Result<(), String>;
}

/// 用于测试或占位阶段的 no-op SAF 同步驱动。
pub struct NoOpSafSyncDriver;

impl SafSyncDriver for NoOpSafSyncDriver {
    fn sync_to_remote(&self, _local_dir: &std::path::Path, _tree_uri: &str) -> Result<(), String> {
        Ok(())
    }

    fn sync_from_remote(
        &self,
        _local_dir: &std::path::Path,
        _tree_uri: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}

/// SAF-backed 文件系统实现。
///
/// 当前 Phase 1 实现把所有文件操作代理到本地临时目录；
/// SAF 同步由调用方（AppState / commands）通过 `sync_to_remote`/`sync_from_remote` 触发，
/// 避免在 trait 同步方法中引入 async/IPC 复杂度。
pub struct SafVaultFileSystem {
    /// SAF tree URI 字符串（如 `content://com.android.externalstorage.documents/tree/primary%3ASoloSoul`）。
    tree_uri: String,
    /// 本地临时目录，SQLite 等需要真实 Path 的场景直接读写此处。
    local_temp_dir: PathBuf,
    /// 是否有尚未同步到 SAF 的脏数据。
    dirty: AtomicBool,
    sync_driver: Arc<dyn SafSyncDriver>,
}

impl SafVaultFileSystem {
    pub fn new(
        tree_uri: String,
        local_temp_dir: PathBuf,
        sync_driver: Arc<dyn SafSyncDriver>,
    ) -> Self {
        Self {
            tree_uri,
            local_temp_dir,
            dirty: AtomicBool::new(false),
            sync_driver,
        }
    }

    pub fn tree_uri(&self) -> &str {
        &self.tree_uri
    }

    pub fn local_temp_dir(&self) -> &Path {
        &self.local_temp_dir
    }

    fn resolve(&self, relative_path: &str) -> Result<PathBuf, String> {
        validate_relative_path(relative_path)?;
        if relative_path.is_empty() {
            Ok(self.local_temp_dir.clone())
        } else {
            Ok(self.local_temp_dir.join(relative_path))
        }
    }
}

impl VaultFileSystem for SafVaultFileSystem {
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String> {
        let path = self.resolve(relative_path)?;
        std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))
    }

    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
        }
        std::fs::write(&path, data).map_err(|e| format!("写入文件失败: {e}"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn remove_file(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn exists(&self, relative_path: &str) -> Result<bool, String> {
        let path = self.resolve(relative_path)?;
        Ok(path.exists())
    }

    fn create_dir_all(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::create_dir_all(&path).map_err(|e| format!("创建目录失败: {e}"))
    }

    fn remove_dir_all(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::remove_dir_all(&path).map_err(|e| format!("删除目录失败: {e}"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String> {
        let path = self.resolve(relative_path)?;
        let entries = std::fs::read_dir(&path).map_err(|e| format!("读取目录失败: {e}"))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    fn sync_if_dirty(&self) -> Result<(), String> {
        if self.dirty.load(Ordering::Acquire) {
            self.sync_to_remote()?;
        }
        Ok(())
    }

    fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    fn local_path(&self, relative_path: &str) -> Option<PathBuf> {
        self.resolve(relative_path).ok()
    }

    fn sync_to_remote(&self) -> Result<(), String> {
        let result = self
            .sync_driver
            .sync_to_remote(&self.local_temp_dir, &self.tree_uri);
        // 同步成功后才清除脏标记；失败时保留，下次继续同步。
        if result.is_ok() {
            self.dirty.store(false, Ordering::Release);
        }
        result
    }

    fn sync_from_remote(&self) -> Result<(), String> {
        self.sync_driver
            .sync_from_remote(&self.local_temp_dir, &self.tree_uri)
    }

    fn is_remote(&self) -> bool {
        true
    }
}

/// 本地文件系统实现，直接映射到 `std::fs`。
pub struct LocalVaultFileSystem {
    base: PathBuf,
}

impl LocalVaultFileSystem {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    fn resolve(&self, relative_path: &str) -> Result<PathBuf, String> {
        validate_relative_path(relative_path)?;
        if relative_path.is_empty() {
            Ok(self.base.clone())
        } else {
            Ok(self.base.join(relative_path))
        }
    }
}

impl VaultFileSystem for LocalVaultFileSystem {
    fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, String> {
        let path = self.resolve(relative_path)?;
        std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))
    }

    fn write_file(&self, relative_path: &str, data: &[u8]) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
        }
        std::fs::write(&path, data).map_err(|e| format!("写入文件失败: {e}"))
    }

    fn remove_file(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))
    }

    fn exists(&self, relative_path: &str) -> Result<bool, String> {
        let path = self.resolve(relative_path)?;
        Ok(path.exists())
    }

    fn create_dir_all(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::create_dir_all(&path).map_err(|e| format!("创建目录失败: {e}"))
    }

    fn remove_dir_all(&self, relative_path: &str) -> Result<(), String> {
        let path = self.resolve(relative_path)?;
        std::fs::remove_dir_all(&path).map_err(|e| format!("删除目录失败: {e}"))
    }

    fn list_dir(&self, relative_path: &str) -> Result<Vec<String>, String> {
        let path = self.resolve(relative_path)?;
        let entries = std::fs::read_dir(&path).map_err(|e| format!("读取目录失败: {e}"))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    fn local_path(&self, relative_path: &str) -> Option<PathBuf> {
        self.resolve(relative_path).ok()
    }
}

/// 校验相对路径，禁止绝对路径与 `..` 逃逸。
fn validate_relative_path(relative_path: &str) -> Result<(), String> {
    if relative_path.is_empty() {
        return Ok(());
    }
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("禁止绝对路径".to_string());
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err("禁止路径中包含 '..'".to_string());
            }
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => return Err("非法路径组件".to_string()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_local() -> (LocalVaultFileSystem, TempDir) {
        let dir = TempDir::new().unwrap();
        let fs = LocalVaultFileSystem::new(dir.path().to_path_buf());
        (fs, dir)
    }

    fn setup_saf() -> (SafVaultFileSystem, TempDir) {
        let dir = TempDir::new().unwrap();
        let driver = Arc::new(NoOpSafSyncDriver);
        let fs = SafVaultFileSystem::new(
            "content://tree/primary%3ASoloSoul".to_string(),
            dir.path().to_path_buf(),
            driver,
        );
        (fs, dir)
    }

    // ── LocalVaultFileSystem 基础测试 ──

    #[test]
    fn test_local_vault_file_system_read_write() {
        let (fs, _dir) = setup_local();

        fs.write_file("accounts.json", b"hello").unwrap();
        assert!(fs.exists("accounts.json").unwrap());
        assert_eq!(fs.read_file("accounts.json").unwrap(), b"hello");
    }

    #[test]
    fn test_local_vault_file_system_rejects_parent_dir() {
        let (fs, _dir) = setup_local();

        assert!(fs.read_file("../etc/passwd").is_err());
    }

    #[test]
    fn test_local_vault_file_system_list_dir() {
        let (fs, _dir) = setup_local();

        fs.write_file("a/1.txt", b"1").unwrap();
        fs.write_file("a/2.txt", b"2").unwrap();

        let mut names = fs.list_dir("a").unwrap();
        names.sort();
        assert_eq!(names, vec!["1.txt", "2.txt"]);
    }

    // ── SafVaultFileSystem 基础操作测试 ──

    #[test]
    fn test_saf_vault_file_system_read_write() {
        let (fs, _dir) = setup_saf();

        fs.write_file("accounts.json", b"hello").unwrap();
        assert!(fs.exists("accounts.json").unwrap());
        assert_eq!(fs.read_file("accounts.json").unwrap(), b"hello");
    }

    #[test]
    fn test_saf_vault_file_system_rejects_parent_dir() {
        let (fs, _dir) = setup_saf();

        assert!(fs.read_file("../etc/passwd").is_err());
    }

    #[test]
    fn test_saf_vault_file_system_rejects_absolute_path() {
        let (fs, _dir) = setup_saf();

        assert!(fs.read_file("/etc/passwd").is_err());
    }

    #[test]
    fn test_saf_vault_file_system_list_dir() {
        let (fs, _dir) = setup_saf();

        fs.write_file("a/1.txt", b"1").unwrap();
        fs.write_file("a/2.txt", b"2").unwrap();

        let mut names = fs.list_dir("a").unwrap();
        names.sort();
        assert_eq!(names, vec!["1.txt", "2.txt"]);
    }

    #[test]
    fn test_saf_vault_file_system_remove_file() {
        let (fs, _dir) = setup_saf();

        fs.write_file("tmp.txt", b"data").unwrap();
        assert!(fs.exists("tmp.txt").unwrap());

        fs.remove_file("tmp.txt").unwrap();
        assert!(!fs.exists("tmp.txt").unwrap());
    }

    #[test]
    fn test_saf_vault_file_system_remove_dir_all() {
        let (fs, _dir) = setup_saf();

        fs.write_file("acc_1/config.json", b"{}").unwrap();
        assert!(fs.exists("acc_1/config.json").unwrap());

        fs.remove_dir_all("acc_1").unwrap();
        assert!(!fs.exists("acc_1").unwrap());
    }

    #[test]
    fn test_saf_vault_file_system_create_dir_all() {
        let (fs, _dir) = setup_saf();

        fs.create_dir_all("a/b/c").unwrap();
        assert!(fs.exists("a/b/c").unwrap());
        assert!(fs.local_path("a/b/c").unwrap().is_dir());
    }

    #[test]
    fn test_saf_vault_file_system_empty_path_resolves_to_root() {
        let (fs, _dir) = setup_saf();

        let path = fs.local_path("").unwrap();
        assert!(path.is_dir());
        assert_eq!(path, fs.local_temp_dir());
    }

    // ── SafVaultFileSystem 同步委派测试 ──

    #[test]
    fn test_saf_vault_file_system_is_remote() {
        let (fs, _dir) = setup_saf();

        assert!(fs.is_remote());
    }

    #[test]
    fn test_saf_vault_file_system_sync_to_remote_delegates_to_driver() {
        let (fs, _dir) = setup_saf();

        // NoOpSafSyncDriver 总是返回 Ok(())
        assert!(fs.sync_to_remote().is_ok());
    }

    #[test]
    fn test_saf_vault_file_system_sync_from_remote_delegates_to_driver() {
        let (fs, _dir) = setup_saf();

        assert!(fs.sync_from_remote().is_ok());
    }

    #[test]
    fn test_saf_vault_file_system_tree_uri() {
        let (fs, _dir) = setup_saf();

        assert_eq!(fs.tree_uri(), "content://tree/primary%3ASoloSoul");
    }

    #[test]
    fn test_saf_vault_file_system_local_path_coincides_with_temp_dir() {
        let dir = TempDir::new().unwrap();
        let driver = Arc::new(NoOpSafSyncDriver);
        let fs = SafVaultFileSystem::new(
            "content://tree/primary%3ASoloSoul".to_string(),
            dir.path().to_path_buf(),
            driver,
        );

        // local_path 应与构造时传入的路径一致
        assert_eq!(fs.local_path("").unwrap(), dir.path());
        assert_eq!(fs.local_path("sub").unwrap(), dir.path().join("sub"));
    }
}
