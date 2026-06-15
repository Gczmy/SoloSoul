//! 进程级 Vault 排他锁。
//!
//! 用于防止 CLI 与 GUI 同时写入同一 `~/.solosoul` 数据目录导致 SQLite 冲突。
//! 锁文件为 `{base_path}/.lock`，通过 `fs2::FileExt::try_lock_exclusive` 实现跨平台排他锁。

use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// 进程级排他锁。创建成功即代表当前进程持有锁，`Drop` 时自动释放。
pub struct ProcessLock {
    #[allow(dead_code)]
    file: File,
    path: PathBuf,
}

impl ProcessLock {
    /// 尝试获取排他锁。若锁已被其他进程持有，返回错误。
    pub fn acquire(base_path: &Path) -> Result<Self, String> {
        let lock_path = base_path.join(".lock");
        std::fs::create_dir_all(base_path)
            .map_err(|e| format!("无法创建数据目录 {}: {}", base_path.display(), e))?;

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| format!("无法打开锁文件 {}: {}", lock_path.display(), e))?;

        file.try_lock_exclusive()
            .map_err(|_| "Vault 正被其他进程使用，请关闭后再试".to_string())?;

        Ok(Self {
            file,
            path: lock_path,
        })
    }

    /// 返回锁文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_acquire_and_drop_releases_lock() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".solosoul");

        {
            let lock = ProcessLock::acquire(&base).unwrap();
            assert!(lock.path().exists());

            // 同一线程/进程再次获取应失败
            let result = ProcessLock::acquire(&base);
            assert!(result.is_err());
        }

        // Drop 后应能重新获取
        let lock = ProcessLock::acquire(&base);
        assert!(lock.is_ok());
    }
}
