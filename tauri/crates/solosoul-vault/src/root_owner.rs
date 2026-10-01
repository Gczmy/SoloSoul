//! RF905：真实 Vault 根目录的 Native 所有者。
//! 不按路径隐式复用；同一应用内的多句柄必须显式克隆已有 Arc。
//! 桌面锁使用标准库 File::try_lock（最低 Rust 1.89），与既有 fs2 flock/LockFileEx 协议互操作。
//! Android/iOS 保持 no-op 边界，仅固定本地 cache root 的句柄寿命，不锁远端 SAF provider。
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct VaultRootOwner {
    root: PathBuf,
    id: uuid::Uuid,
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    _file: std::fs::File,
}

impl VaultRootOwner {
    pub fn acquire(root: &Path) -> Result<Arc<Self>, String> {
        // 仅建立目录与锁文件；账户/config/SQLite 迁移须在成功取得 owner 之后。
        std::fs::create_dir_all(root).map_err(|_| "VAULT_DIRECTORY_UNAVAILABLE")?;
        let root = root
            .canonicalize()
            .map_err(|_| "VAULT_DIRECTORY_UNAVAILABLE")?;
        if !root.is_dir() {
            return Err("VAULT_DIRECTORY_UNAVAILABLE".into());
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let path = root.join(".lock");
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                    return Err("VAULT_DIRECTORY_LOCK_INVALID".into());
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("VAULT_DIRECTORY_LOCK_FAILED".into()),
            }
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .map_err(|_| "VAULT_DIRECTORY_LOCK_FAILED")?;
            match file.try_lock() {
                Ok(()) => {}
                Err(std::fs::TryLockError::WouldBlock) => return Err("VAULT_DIRECTORY_BUSY".into()),
                Err(std::fs::TryLockError::Error(_)) => {
                    return Err("VAULT_DIRECTORY_LOCK_FAILED".into())
                }
            }
            Ok(Arc::new(Self {
                root,
                id: uuid::Uuid::new_v4(),
                _file: file,
            }))
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            Ok(Arc::new(Self {
                root,
                id: uuid::Uuid::new_v4(),
            }))
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
    pub fn is_process_locked(&self) -> bool {
        cfg!(not(any(target_os = "android", target_os = "ios")))
    }

    /// 路径必须已存在且在真实 root 内；失败不能打开或迁移 SQLite。
    pub(crate) fn vault_path(&self, path: &Path) -> Result<PathBuf, String> {
        let path = path
            .canonicalize()
            .map_err(|_| "VAULT_DIRECTORY_UNAVAILABLE")?;
        if !path.is_dir() || !path.starts_with(&self.root) {
            return Err("VAULT_ROOT_MISMATCH".into());
        }
        Ok(path)
    }
}
