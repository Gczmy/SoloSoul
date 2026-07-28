//! Android 应用内更新命令。
//!
//! 通过 GitHub Release API 检查新版本，下载 APK 并触发系统安装。
//! 桌面端仍使用 `@tauri-apps/plugin-updater`，本模块仅对 Android 有效。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

const GITHUB_API: &str = "https://api.github.com/repos/Gczmy/SoloSoul/releases/latest";
const USER_AGENT: &str = "SoloSoul/2.6.1";

// ── Types ──────────────────────────────────────────────────────

/// GitHub Release API 返回的顶层结构（仅提取所需字段）。
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    /// 下载 URL（直接 GitHub Release 资产链接）。
    browser_download_url: String,
    size: Option<i64>,
}

/// 更新检查结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidUpdateInfo {
    pub latest_version: String,
    pub current_version: String,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub apk_size: Option<i64>,
}

/// APK 下载进度事件负载。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkDownloadProgress {
    pub progress: u32,
    pub downloaded: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

// ── Helper: 获取当前版本号 ──────────────────────────────────────

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Helper: APK 缓存路径 ──────────────────────────────────────

/// 获取 APK 下载目标路径（应用缓存目录）。
fn apk_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let cache = app
        .path()
        .resolve("update.apk", tauri::path::BaseDirectory::Cache)
        .map_err(|e| format!("无法解析缓存目录: {e}"))?;
    Ok(cache)
}

/// 获取已下载的 APK 文件大小（字节），不存在则返回 0。
pub fn apk_downloaded_size(app: &tauri::AppHandle) -> Result<u64, String> {
    let path = apk_cache_path(app)?;
    if path.exists() {
        Ok(std::fs::metadata(&path)
            .map_err(|e| format!("读取文件大小: {e}"))?
            .len())
    } else {
        Ok(0)
    }
}

/// 删除已下载的 APK 缓存。
pub fn delete_apk_cache(app: &tauri::AppHandle) -> Result<(), String> {
    let path = apk_cache_path(app)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除 APK 缓存失败: {e}"))?;
    }
    Ok(())
}

// ── Command: 检查更新 ──────────────────────────────────────────

/// 检查 GitHub Release 是否有新版本。
///
/// 仅在 Android 上有效；桌面端使用 `@tauri-apps/plugin-updater`。
#[tauri::command]
pub async fn android_check_update(
    _app: tauri::AppHandle,
) -> Result<AndroidUpdateInfo, String> {
    let current = current_version();
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = client
        .get(GITHUB_API)
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回 HTTP {}", resp.status()));
    }

    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("解析 GitHub Release 响应失败: {e}"))?;

    // tag_name 格式为 "v2.6.1"，去掉 v 前缀
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    // 查找 APK 资产
    let apk_asset = release.assets.into_iter().find(|a| {
        a.name.ends_with(".apk") || a.name.contains("universal-release")
    });

    Ok(AndroidUpdateInfo {
        latest_version: latest,
        current_version: current,
        download_url: apk_asset.as_ref().map(|a| a.browser_download_url.clone()),
        release_notes: release.body,
        published_at: release.published_at,
        apk_size: apk_asset.and_then(|a| a.size),
    })
}

// ── Command: 下载 APK ──────────────────────────────────────────

/// 从指定 URL 下载 APK 文件到缓存目录，并发送进度事件。
///
/// 事件名：`apk-download-progress`
#[tauri::command]
pub async fn android_download_apk(
    app: tauri::AppHandle,
    download_url: String,
) -> Result<(), String> {
    let dest = apk_cache_path(&app)?;

    // 清理旧文件
    let _ = std::fs::remove_file(&dest);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let mut resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("请求 APK 下载失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("APK 下载返回 HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let tmp_path = dest.with_extension("tmp");
    let mut file =
        std::fs::File::create(&tmp_path).map_err(|e| format!("创建临时文件失败: {e}"))?;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("下载分块失败: {e}"))?
    {
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("写入分块失败: {e}"))?;
        downloaded += chunk.len() as u64;

        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
            let _ = app.emit(
                "apk-download-progress",
                ApkDownloadProgress {
                    progress: pct.min(100),
                    downloaded,
                    total,
                    done: false,
                    error: None,
                },
            );
        }
    }

    // 临时文件重命名为目标文件（原子操作）
    std::fs::rename(&tmp_path, &dest).map_err(|e| format!("重命名 APK 文件失败: {e}"))?;

    // 发送完成事件
    let _ = app.emit(
        "apk-download-progress",
        ApkDownloadProgress {
            progress: 100,
            downloaded,
            total,
            done: true,
            error: None,
        },
    );

    Ok(())
}

// ── Command: 安装 APK ──────────────────────────────────────────

/// 获取已下载的 APK 文件路径，用于安装。
#[tauri::command]
pub async fn android_get_apk_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = apk_cache_path(&app)?;
    if !path.exists() {
        return Err("APK 文件不存在，请先下载".to_string());
    }
    Ok(path.to_string_lossy().to_string())
}

/// 检查 APK 是否已下载。
#[tauri::command]
pub async fn android_is_apk_downloaded(app: tauri::AppHandle) -> Result<bool, String> {
    let path = apk_cache_path(&app)?;
    Ok(path.exists())
}
