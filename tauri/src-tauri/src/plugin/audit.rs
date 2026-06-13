//! 插件审计日志
//!
//! 审计日志持久化到本地 JSONL 文件，保留最近 2000 条记录。
//! 文件权限遵循 SoloSoul 安全约定：0600。

use super::{PluginAuditAction, PluginAuditEntry, PluginError};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 默认保留的最大审计条目数
const MAX_AUDIT_ENTRIES: usize = 2000;

/// 同步审计日志器
pub struct PluginAuditLogger {
    entries: Mutex<Vec<PluginAuditEntry>>,
    path: Option<PathBuf>,
    max_entries: usize,
}

impl Default for PluginAuditLogger {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PluginAuditLogger {
    /// 创建新的审计日志器
    ///
    /// `path` 为 `None` 时不持久化（测试场景）。
    pub fn new(path: Option<PathBuf>) -> Self {
        let entries = path
            .as_ref()
            .map(|p| Self::load(p).unwrap_or_default())
            .unwrap_or_default();
        Self {
            entries: Mutex::new(entries),
            path,
            max_entries: MAX_AUDIT_ENTRIES,
        }
    }

    /// 记录一条审计日志
    pub fn log(
        &self,
        plugin_id: impl Into<String>,
        session_id: Option<impl Into<String>>,
        action: PluginAuditAction,
    ) {
        let entry = PluginAuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            plugin_id: plugin_id.into(),
            session_id: session_id.map(Into::into),
            action,
        };

        let should_truncate = {
            if let Ok(mut guard) = self.entries.lock() {
                guard.push(entry.clone());
                if guard.len() > self.max_entries {
                    let start = guard.len() - self.max_entries;
                    let trimmed: Vec<_> = guard.split_off(start);
                    *guard = trimmed;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if let Some(ref path) = self.path {
            if should_truncate {
                let _ = self.rewrite_file(path);
            } else {
                let _ = Self::append_one(path, &entry);
            }
        }
    }

    /// 获取最近的 `limit` 条审计日志
    pub fn recent(&self, limit: usize) -> Vec<PluginAuditEntry> {
        let guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        guard.iter().rev().take(limit).cloned().collect()
    }

    /// 从 JSONL 文件加载审计日志
    fn load(path: &Path) -> Result<Vec<PluginAuditEntry>, PluginError> {
        let file = fs::File::open(path).map_err(|e| PluginError::StoreError(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| PluginError::StoreError(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<PluginAuditEntry>(&line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// 追加单条审计日志到 JSONL 文件
    fn append_one(path: &Path, entry: &PluginAuditEntry) -> Result<(), PluginError> {
        Self::ensure_private_file(path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| PluginError::StoreError(e.to_string()))?;
        let line =
            serde_json::to_string(entry).map_err(|e| PluginError::StoreError(e.to_string()))?;
        writeln!(file, "{}", line).map_err(|e| PluginError::StoreError(e.to_string()))?;
        Ok(())
    }

    /// 截断后重写整个审计日志文件
    fn rewrite_file(&self, path: &Path) -> Result<(), PluginError> {
        Self::ensure_private_file(path)?;
        let entries = self.recent(self.max_entries);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| PluginError::StoreError(e.to_string()))?;
        for entry in entries.iter().rev() {
            let line =
                serde_json::to_string(entry).map_err(|e| PluginError::StoreError(e.to_string()))?;
            writeln!(file, "{}", line).map_err(|e| PluginError::StoreError(e.to_string()))?;
        }
        Ok(())
    }

    /// 确保文件存在并设置 0600 权限（Unix）
    fn ensure_private_file(path: &Path) -> Result<(), PluginError> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| PluginError::StoreError(e.to_string()))?;
            }
            fs::File::create(path).map_err(|e| PluginError::StoreError(e.to_string()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                    .map_err(|e| PluginError::StoreError(e.to_string()))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_persist_and_reload() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("plugin_audit.jsonl");
        let logger = PluginAuditLogger::new(Some(path.clone()));
        logger.log("p1", Some("s1"), PluginAuditAction::PluginRunStarted);
        logger.log(
            "p1",
            Some("s1"),
            PluginAuditAction::PluginRunCompleted { exit_code: 0 },
        );

        let logger2 = PluginAuditLogger::new(Some(path));
        let recent = logger2.recent(10);
        assert_eq!(recent.len(), 2);
        assert!(matches!(
            recent[0].action,
            PluginAuditAction::PluginRunCompleted { exit_code: 0 }
        ));
        assert!(matches!(
            recent[1].action,
            PluginAuditAction::PluginRunStarted
        ));
    }

    #[test]
    fn test_truncate_to_max_entries() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("plugin_audit.jsonl");
        let logger = PluginAuditLogger {
            entries: Mutex::new(Vec::new()),
            path: Some(path.clone()),
            max_entries: 5,
        };
        for i in 0..10 {
            logger.log(
                "p1",
                Some(format!("s{}", i)),
                PluginAuditAction::PluginRunStarted,
            );
        }
        assert_eq!(logger.recent(100).len(), 5);

        let logger2 = PluginAuditLogger::new(Some(path));
        assert_eq!(logger2.recent(100).len(), 5);
    }
}
