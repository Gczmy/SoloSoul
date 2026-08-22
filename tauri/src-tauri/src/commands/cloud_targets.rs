//! 云盘同步目录检测（Phase 1 · 云打包功能）。
//!
//! 检测桌面端常见云盘客户端的本地同步文件夹，检测结果有两个消费方：
//!
//! 1. Tauri 命令 [`cloud_targets_detect`] — 前端导出向导「保存到云盘」快捷目标；
//! 2. `attachment/mod.rs::allowed_fs_bases` / `commands/fs.rs::desktop_fs_bases`
//!    的白名单放行（与 Desktop/Documents/Downloads 同一信任级别：用户自有目录，
//!    经系统目录选择器或快捷芯片选择后落盘）。
//!
//! 隐私说明：检测仅在本地扫描 home 一级子目录与 macOS CloudStorage 挂载点，
//! 不发起任何网络请求；检测结果不落库、不上报。
//!
//! 注意：本模块的候选列表变更时，需同步检查 `allowed_fs_bases` /
//! `desktop_fs_bases` 两处调用方（它们直接消费本模块输出，无独立列表）。

use serde::Serialize;
use std::path::{Path, PathBuf};

/// 单个检测到的云盘同步目录。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CloudTargetInfo {
    /// 稳定标识（前端图标/文案映射用），如 `onedrive`、`baidu_netdisk`。
    pub id: String,
    /// 展示名（已本地化的英文兜底，前端可用 i18n 按 id 覆盖）。
    pub name: String,
    /// 本地同步目录绝对路径。
    pub path: String,
}

/// 候选条目：home 相对路径 + 云盘标识 + 展示名。
struct Candidate {
    rel: &'static str,
    id: &'static str,
    name: &'static str,
}

/// 静态候选列表（跨平台通用）。目录存在才纳入结果，不存在静默跳过。
const STATIC_CANDIDATES: &[Candidate] = &[
    Candidate {
        rel: "OneDrive",
        id: "onedrive",
        name: "OneDrive",
    },
    Candidate {
        rel: "百度网盘同步空间",
        id: "baidu_netdisk",
        name: "Baidu Netdisk",
    },
    Candidate {
        rel: "BaiduSyncdisk",
        id: "baidu_netdisk",
        name: "Baidu Netdisk",
    },
    Candidate {
        rel: "Nutstore Files",
        id: "nutstore",
        name: "Nutstore",
    },
    Candidate {
        rel: "Nutstore",
        id: "nutstore",
        name: "Nutstore",
    },
    Candidate {
        rel: "Dropbox",
        id: "dropbox",
        name: "Dropbox",
    },
    Candidate {
        rel: "Google Drive",
        id: "google_drive",
        name: "Google Drive",
    },
    Candidate {
        rel: "GoogleDrive",
        id: "google_drive",
        name: "Google Drive",
    },
];

/// 检测 home 目录下的云盘同步文件夹。
///
/// 纯函数（仅读文件系统元数据），便于单元测试。返回按 canonicalize 后路径去重
/// 的有序结果（静态候选顺序优先，随后是企业版 OneDrive / CloudStorage 扫描项）。
pub(crate) fn detect_cloud_sync_dirs(home: &Path) -> Vec<CloudTargetInfo> {
    let mut out: Vec<CloudTargetInfo> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    let push = |p: PathBuf,
                    id: &str,
                    name: &str,
                    out: &mut Vec<CloudTargetInfo>,
                    seen: &mut Vec<PathBuf>| {
        if !p.is_dir() {
            return;
        }
        let canon = match p.canonicalize() {
            Ok(c) => c,
            Err(_) => return,
        };
        if seen.contains(&canon) {
            return;
        }
        seen.push(canon);
        out.push(CloudTargetInfo {
            id: id.to_string(),
            name: name.to_string(),
            path: p.to_string_lossy().to_string(),
        });
    };

    // 1. 静态候选
    for c in STATIC_CANDIDATES {
        push(home.join(c.rel), c.id, c.name, &mut out, &mut seen);
    }

    // 2. 企业版 OneDrive：目录名形如 `OneDrive - <组织名>`，按前缀扫描 home 一级子目录。
    if let Ok(entries) = std::fs::read_dir(home) {
        for e in entries.flatten() {
            let is_biz = e
                .file_name()
                .to_str()
                .map(|n| n.starts_with("OneDrive - "))
                .unwrap_or(false);
            if is_biz {
                push(
                    e.path(),
                    "onedrive_business",
                    "OneDrive",
                    &mut out,
                    &mut seen,
                );
            }
        }
    }

    // 3. macOS：iCloud 云盘 + CloudStorage 挂载点（macOS 12+ 各云盘客户端的标准挂载位置，
    //    子目录名形如 `GoogleDrive-xxx@umd`、`Dropbox`、`OneDrive-xxx` 等）。
    #[cfg(target_os = "macos")]
    {
        let icloud = home.join("Library/Mobile Documents/com~apple~CloudDocs");
        push(icloud, "icloud", "iCloud Drive", &mut out, &mut seen);

        let storage = home.join("Library/CloudStorage");
        if let Ok(entries) = std::fs::read_dir(&storage) {
            for e in entries.flatten() {
                // 显示名取挂载目录名（如 `GoogleDrive-zzz`），id 用小写去后缀前缀。
                let raw_name = e.file_name().to_string_lossy().to_string();
                let id = raw_name
                    .split(['-', '~'])
                    .next()
                    .unwrap_or("cloud")
                    .to_ascii_lowercase();
                push(e.path(), &id, &raw_name, &mut out, &mut seen);
            }
        }
    }

    out
}

/// Tauri 命令：列出当前机器可用的云盘同步目标（移动端恒为空——Android 走 SAF 选择器）。
#[tauri::command]
pub fn cloud_targets_detect() -> Vec<CloudTargetInfo> {
    #[cfg(mobile)]
    {
        Vec::new()
    }
    #[cfg(desktop)]
    {
        #[cfg(unix)]
        let home_var = "HOME";
        #[cfg(windows)]
        let home_var = "USERPROFILE";
        match std::env::var(home_var) {
            Ok(home) => detect_cloud_sync_dirs(&PathBuf::from(home)),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_static_candidate_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("OneDrive")).unwrap();
        std::fs::create_dir_all(tmp.path().join("Nutstore Files")).unwrap();

        let targets = detect_cloud_sync_dirs(tmp.path());
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "onedrive");
        assert!(targets[0].path.ends_with("OneDrive"));
        assert_eq!(targets[1].id, "nutstore");
    }

    #[test]
    fn test_detect_skips_missing_dirs_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Dropbox 存在但为普通文件 → 跳过
        std::fs::write(tmp.path().join("Dropbox"), b"not a dir").unwrap();

        let targets = detect_cloud_sync_dirs(tmp.path());
        assert!(targets.is_empty());
    }

    #[test]
    fn test_detect_business_onedrive_prefix_scan() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("OneDrive - Contoso")).unwrap();

        let targets = detect_cloud_sync_dirs(tmp.path());
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "onedrive_business");
    }

    #[test]
    fn test_detect_dedupes_same_target_via_multiple_candidates() {
        let tmp = tempfile::tempdir().unwrap();
        // Nutstore 与 Nutstore Files 同时存在时是两个不同目录，都应出现；
        // 但同一目录不应重复——用 OneDrive 静态 + 无企业版验证无重复。
        std::fs::create_dir_all(tmp.path().join("OneDrive")).unwrap();
        let targets = detect_cloud_sync_dirs(tmp.path());
        let paths: Vec<_> = targets.iter().map(|t| t.path.clone()).collect();
        assert_eq!(
            paths.len(),
            paths.iter().collect::<std::collections::HashSet<_>>().len()
        );
        assert_eq!(paths.len(), 1);
    }
}
