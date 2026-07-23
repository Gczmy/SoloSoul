//! Vault 文件系统抽象层
//!
//! 将 Vault 所需的文件操作抽象为 trait，使上层业务代码（VaultService、
//! VaultStore 等）不再直接依赖 `std::fs`。桌面端/App-private 模式使用
//! `LocalVaultFileSystem`；Android 用户自定义目录模式后续通过 SAF-backed
//! 实现替换。

use std::path::{Path, PathBuf};

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

    #[test]
    fn test_local_vault_file_system_read_write() {
        let dir = TempDir::new().unwrap();
        let fs = LocalVaultFileSystem::new(dir.path().to_path_buf());

        fs.write_file("accounts.json", b"hello").unwrap();
        assert!(fs.exists("accounts.json").unwrap());
        assert_eq!(fs.read_file("accounts.json").unwrap(), b"hello");
    }

    #[test]
    fn test_local_vault_file_system_rejects_parent_dir() {
        let dir = TempDir::new().unwrap();
        let fs = LocalVaultFileSystem::new(dir.path().to_path_buf());

        assert!(fs.read_file("../etc/passwd").is_err());
    }

    #[test]
    fn test_local_vault_file_system_list_dir() {
        let dir = TempDir::new().unwrap();
        let fs = LocalVaultFileSystem::new(dir.path().to_path_buf());

        fs.write_file("a/1.txt", b"1").unwrap();
        fs.write_file("a/2.txt", b"2").unwrap();

        let mut names = fs.list_dir("a").unwrap();
        names.sort();
        assert_eq!(names, vec!["1.txt", "2.txt"]);
    }
}
