//! 进程级 Vault 排他锁。
//!
//! 用于防止 CLI 与 GUI 同时写入同一数据目录导致 SQLite 冲突。
//! 锁文件为 `{base_path}/.lock`，通过 `fs2::FileExt::try_lock_exclusive` 实现跨平台排他锁。
//!
//! 移动端（Android/iOS）通常为单实例运行，暂不提供跨进程锁；通过应用级单实例
//!（Android `singleTask` launchMode）避免并发写入。

use std::path::Path;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use fs2::FileExt;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::fs::{File, OpenOptions};

/// 进程级排他锁。创建成功即代表当前进程持有锁，`Drop` 时自动释放。
pub struct ProcessLock {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[allow(dead_code)]
    file: File,
}

impl ProcessLock {
    /// 尝试获取排他锁。若锁已被其他进程持有，返回错误。
    pub fn acquire(base_path: &Path) -> Result<Self, String> {
        let lock_path = base_path.join(".lock");
        std::fs::create_dir_all(base_path)
            .map_err(|e| format!("无法创建数据目录 {}: {}", base_path.display(), e))?;

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)
                .map_err(|e| format!("无法打开锁文件 {}: {}", lock_path.display(), e))?;

            file.try_lock_exclusive()
                .map_err(|_| "Vault 正被其他进程使用，请关闭后再试".to_string())?;

            Ok(Self { file })
        }

        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            // 移动端单实例由 Android manifest / iOS 生命周期保证，锁仅作占位。
            Ok(Self {})
        }
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
            let _lock = ProcessLock::acquire(&base).unwrap();

            // 同一线程/进程再次获取应失败（仅桌面端）
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let result = ProcessLock::acquire(&base);
                assert!(result.is_err());
            }
        }

        // Drop 后应能重新获取
        let lock = ProcessLock::acquire(&base);
        assert!(lock.is_ok());
    }
}
