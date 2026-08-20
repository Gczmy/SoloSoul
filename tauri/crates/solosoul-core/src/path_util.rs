//! 路径规范化工具函数
//!
//! 本模块提供路径规范化功能，用于安全地比较路径是否位于某个工作区目录下。
//! 提取自 `solosoul-plugin` crate 和 `tauri/src-tauri` 中重复的 `resolve_path` / `is_under_workspace` 实现。
//!
//! 核心场景：插件水印功能中验证输入/输出路径是否在插件临时工作区内，
//! 防止路径穿越泄露 Vault 存储或系统文件。

use std::path::{Path, PathBuf};

/// 规范化路径，对不存在的路径尝试规范化其最近存在的祖先后再拼接末尾组件。
///
/// 这样可处理 macOS `/tmp` -> `/private/tmp` 的符号链接，同时允许尚未创建
/// 的输出路径（如 `.watermarked.pdf`）通过 `is_path_under_workspace` 检查。
///
/// # 安全性
///
/// 对不存在的最终路径，手动消除 `..` / `.` 组件，防止路径穿越绕过
/// `is_path_under_workspace` 检查。
pub fn resolve_path(path: &Path) -> PathBuf {
    if let Ok(p) = std::fs::canonicalize(path) {
        return p;
    }
    // 找到最近存在的祖先，规范化后再把缺失的组件拼回来。
    let mut existing = path;
    let mut suffix = Vec::new();
    while !existing.exists() {
        if let Some(file_name) = existing.file_name() {
            suffix.push(file_name);
        }
        match existing.parent() {
            Some(parent) => existing = parent,
            None => break,
        }
    }
    let base = std::fs::canonicalize(existing).unwrap_or_else(|_| existing.to_path_buf());
    let result = suffix
        .into_iter()
        .rev()
        .fold(base, |acc, name| acc.join(name));

    // 尝试规范化最终路径；若失败（文件不存在），手动消除 `..` / `.` 组件
    // 防止路径穿越绕过 is_path_under_workspace 检查。
    std::fs::canonicalize(&result).unwrap_or_else(|_| {
        let mut normalized = PathBuf::new();
        for component in result.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
        }
        normalized
    })
}

/// 判断 path 是否落在 workspace_dir 目录下（均经 `resolve_path` 规范化）。
///
/// 两路径都先通过 `resolve_path` 规范化，再使用 `starts_with` 比较。
/// 适用于插件水印、文件输出等场景中验证路径安全性。
pub fn is_path_under_workspace(workspace_dir: &Path, path: &Path) -> bool {
    let canonical_ws = resolve_path(workspace_dir);
    let canonical_path = resolve_path(path);
    canonical_path.starts_with(&canonical_ws)
}

/// 净化附件/导入文件名（P023 共享实现）。
///
/// 平台无关地拒绝含路径分隔符的名字（`/` 与 `\\`）——Unix 上 `\\` 不是分隔符，
/// 仅靠 `Path::file_name()` 无法剥离 `..\\..\\evil.txt` 中的反斜杠，因此必须
/// 显式拒绝；再取末段组件作为兜底；拒绝空名与 `.`/`..`。
///
/// P023 统一：sync（solosoul-sync）、导入（export_import）、附件落盘
/// （src-tauri attachment crud/share/import_plugin）、插件附件（solosoul-plugin）
/// 四处净化实现收敛到本函数，消除「同款安全控制强度不一致」。
pub fn sanitize_file_name(file_name: &str) -> Result<String, String> {
    // P014: 错误消息不回显原始文件名——含路径的输入放进错误链会外溢用户目录结构，
    // 与「错误不携带完整路径」约定不一致（attachment_import_plugin 同款约定）。
    if file_name.contains('/') || file_name.contains('\\') {
        return Err("附件文件名无效: 名称不得包含路径分隔符".to_string());
    }
    let safe = Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty() && s != "." && s != "..")
        .ok_or_else(|| "附件文件名无效: 名称为空或非法".to_string())?;
    Ok(safe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_path_existing() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();
        let resolved = resolve_path(&file_path);
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("test.txt"));
    }

    #[test]
    fn test_resolve_path_non_existent() {
        let dir = TempDir::new().unwrap();
        let non_existent = dir.path().join("not_exist").join("file.txt");
        let resolved = resolve_path(&non_existent);
        assert!(resolved.ends_with("file.txt"));
    }

    #[test]
    fn test_resolve_path_dot_dot_eliminated() {
        let dir = TempDir::new().unwrap();
        // Create a path with ../.. traversal outside the temp dir
        let dir_path = dir.path().to_path_buf();
        let sub_dir = dir_path.join("a").join("b");
        fs::create_dir_all(&sub_dir).unwrap();

        let traversal = sub_dir.join("..").join("..").join("..");
        let resolved = resolve_path(&traversal);
        // The resolved path should NOT contain "a/b" after normalization
        assert!(!resolved.to_string_lossy().contains("/a/b/.."));
    }

    #[test]
    fn test_is_path_under_workspace_inside() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file = sub.join("data.txt");
        fs::write(&file, "data").unwrap();

        assert!(is_path_under_workspace(dir.path(), &file));
    }

    #[test]
    fn test_is_path_under_workspace_outside() {
        let dir = TempDir::new().unwrap();
        assert!(!is_path_under_workspace(dir.path(), &PathBuf::from("/tmp")));
    }

    #[test]
    fn test_resolve_path_traversal_normalized() {
        let dir = TempDir::new().unwrap();
        let outside = dir.path().join("..").join("..").join("etc");
        let resolved = resolve_path(&outside);
        // After normalization, the path should not contain the temp dir
        assert!(!resolved.starts_with(dir.path()));
    }

    #[test]
    fn test_is_path_under_workspace_with_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let non_existent = dir.path().join("future.txt");
        // File doesn't exist yet, but should still be considered under workspace
        assert!(is_path_under_workspace(dir.path(), &non_existent));
    }

    #[test]
    fn test_sanitize_file_name_accepts_normal_name() {
        assert_eq!(sanitize_file_name("doc.pdf").unwrap(), "doc.pdf");
        assert_eq!(sanitize_file_name("...").unwrap(), "...");
    }

    #[test]
    fn test_sanitize_file_name_rejects_path_separators() {
        assert!(sanitize_file_name("../../evil.txt").is_err());
        assert!(sanitize_file_name("..\\..\\evil.txt").is_err());
        assert!(sanitize_file_name("a/b.txt").is_err());
        assert!(sanitize_file_name("a\\b.txt").is_err());
    }

    #[test]
    fn test_sanitize_file_name_error_does_not_echo_raw_name() {
        // P014: 错误消息不得回显含路径的原始输入（用户目录结构不外溢到错误链）
        let err = sanitize_file_name("/Users/someone/private/..\\evil.txt").unwrap_err();
        assert!(!err.contains("Users"), "错误不应回显原始文件名，got: {err}");
        assert!(
            !err.contains("evil.txt"),
            "错误不应回显原始文件名，got: {err}"
        );
        let err2 = sanitize_file_name("../").unwrap_err();
        assert!(!err2.contains(".."), "错误不应回显原始文件名，got: {err2}");
    }

    #[test]
    fn test_sanitize_file_name_rejects_dot_and_empty() {
        assert!(sanitize_file_name("").is_err());
        assert!(sanitize_file_name(".").is_err());
        assert!(sanitize_file_name("..").is_err());
    }
}
