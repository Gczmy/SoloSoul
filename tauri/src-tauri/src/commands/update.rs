//! Android 应用内更新命令。
//!
//! 通过 GitHub Release API 检查新版本，下载 APK 并触发系统安装。
//! 桌面端仍使用 `@tauri-apps/plugin-updater`，本模块仅对 Android 有效。

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_updater::UpdaterExt;

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
    /// SHA-256 校验和（hex 编码），用于下载后验证 APK 完整性。
    /// 如果 Release 中没有对应的 `.sha256` 资产，则为空字符串。
    pub checksum: String,
    /// 是否为强制更新。当 Release body 包含 `[MANDATORY]` 标记时为 true。
    /// 强制更新会显示不可关闭的对话框，用户必须更新才能继续使用。
    pub mandatory: bool,
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

/// 桌面端更新检查结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopUpdateInfo {
    pub latest_version: String,
    pub current_version: String,
    /// 是否为强制更新（Release body 包含 `[MANDATORY]` 标记）。
    pub mandatory: bool,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

// ── Helper: 获取当前版本号 ──────────────────────────────────────

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Helper: APK 缓存路径 ──────────────────────────────────────

/// 将版本号转换为安全的文件名字符串。
fn version_to_file_part(version: &str) -> String {
    version.replace(
        |c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_',
        "_",
    )
}

/// 获取 APK 最终文件路径（应用缓存目录下的 `update_{version}.apk`）。
fn apk_cache_path(app: &tauri::AppHandle, version: &str) -> Result<PathBuf, String> {
    let file_name = format!("update_{}.apk", version_to_file_part(version));
    let cache = app
        .path()
        .resolve(file_name, tauri::path::BaseDirectory::Cache)
        .map_err(|e| format!("无法解析缓存目录: {e}"))?;
    Ok(cache)
}

/// 获取 APK 部分下载文件路径（`update_{version}.part`），用于断点续传。
/// 下载完成后会重命名为 `update_{version}.apk`。
fn apk_part_path(app: &tauri::AppHandle, version: &str) -> Result<PathBuf, String> {
    let mut path = apk_cache_path(app, version)?;
    path.set_extension("part");
    Ok(path)
}

/// 清理非当前版本的 APK 缓存文件，避免旧版本安装包占用空间并被误用。
fn cleanup_stale_apk_cache(app: &tauri::AppHandle, current_version: &str) -> Result<(), String> {
    let cache_dir = app
        .path()
        .resolve("", tauri::path::BaseDirectory::Cache)
        .map_err(|e| format!("无法解析缓存目录: {e}"))?;
    let Ok(entries) = std::fs::read_dir(&cache_dir) else {
        return Ok(());
    };

    let current_part = version_to_file_part(current_version);
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "update.apk" || name_str == "update.part" {
            // 旧版无版本缓存，直接删除
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        // 仅处理 update_<version>.apk / update_<version>.part 格式
        let is_update_file = (name_str.starts_with("update_") && name_str.ends_with(".apk"))
            || (name_str.starts_with("update_") && name_str.ends_with(".part"));
        if !is_update_file {
            continue;
        }
        if let Some(stripped) = name_str.strip_prefix("update_") {
            let file_version = stripped
                .strip_suffix(".apk")
                .or_else(|| stripped.strip_suffix(".part"))
                .unwrap_or(stripped);
            if file_version != current_part {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

/// 获取已下载完成的 APK 大小（字节），不存在则返回 0。
pub fn apk_downloaded_size(app: &tauri::AppHandle, version: &str) -> Result<u64, String> {
    let path = apk_cache_path(app, version)?;
    if path.exists() {
        Ok(std::fs::metadata(&path)
            .map_err(|e| format!("读取文件大小: {e}"))?
            .len())
    } else {
        Ok(0)
    }
}

/// 删除已下载的 APK 缓存（同时清理最终文件和部分文件）。
pub fn delete_apk_cache(app: &tauri::AppHandle, version: &str) -> Result<(), String> {
    for path in [apk_cache_path(app, version)?, apk_part_path(app, version)?] {
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("删除缓存失败 ({}): {e}", path.display()))?;
        }
    }
    Ok(())
}

// ── Command: 检查更新 ──────────────────────────────────────────

/// 创建 GitHub API 请求客户端（带 UA 与 15s 超时）。
fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

/// 请求 GitHub Release API 并解析最新 Release。
async fn fetch_github_release(client: &reqwest::Client) -> Result<GitHubRelease, String> {
    let resp = client
        .get(GITHUB_API)
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回 HTTP {}", resp.status()));
    }

    resp.json()
        .await
        .map_err(|e| format!("解析 GitHub Release 响应失败: {e}"))
}

/// 检查 GitHub Release 是否有新版本。
///
/// 仅在 Android 上有效；桌面端使用 `desktop_check_update`。
#[tauri::command]
pub async fn android_check_update(app: tauri::AppHandle) -> Result<AndroidUpdateInfo, String> {
    let current = current_version();
    let client = github_client()?;
    let release = fetch_github_release(&client).await?;

    // tag_name 格式为 "v2.6.1"，去掉 v 前缀
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    // 查找 APK 资产
    let apk_asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk") || a.name.contains("universal-release"));

    // 查找对应的 .sha256 校验和资产，并下载其内容
    let checksum = if let Some(checksum_asset) = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk.sha256") || a.name.contains("sha256"))
    {
        // 使用已有的 async client 下载校验和文件（约 64 字节）
        let url = &checksum_asset.browser_download_url;
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                // 格式: "<64位hex>  <文件名>" 或仅 "<64位hex>"
                let body = resp.text().await.unwrap_or_default();
                body.split_whitespace()
                    .next()
                    .filter(|token| token.len() == 64)
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    } else {
        None
    };

    // 如果找到 checksum 但解析失败（如文件格式异常），也返回空字符串
    // 客户端仍可正常下载，只是不进行校验

    // 检测强制更新标记：Release body 中是否包含 [MANDATORY]
    let mandatory = release
        .body
        .as_deref()
        .map(|body| body.contains("[MANDATORY]"))
        .unwrap_or(false);

    // 如果 Release body 包含 [MANDATORY]，在返回前移除该标记
    // 避免用户看到原始标记文本
    let clean_body = release
        .body
        .map(|body| body.replace("[MANDATORY]", "").trim().to_string())
        .filter(|s| !s.is_empty());

    // 检查到新版本后，立即清理旧版本缓存，避免旧 APK 被误当作已下载。
    let _ = cleanup_stale_apk_cache(&app, &latest);

    Ok(AndroidUpdateInfo {
        latest_version: latest,
        current_version: current,
        download_url: apk_asset.map(|a| a.browser_download_url.clone()),
        checksum: checksum.unwrap_or_default(),
        mandatory,
        release_notes: clean_body,
        published_at: release.published_at,
        apk_size: apk_asset.and_then(|a| a.size),
    })
}

// ── Command: 桌面端检查更新 ─────────────────────────────────────

/// 桌面端检查更新：版本检测复用 Tauri updater 插件（读取 latest.json），
/// Release notes 通过 GitHub Release API 补全，行为与 Android 对齐。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn desktop_check_update(app: tauri::AppHandle) -> Result<DesktopUpdateInfo, String> {
    let current = current_version();

    // 1. 通过 updater 插件检测是否有可用更新（latest.json 中的版本与签名）
    let updater = app
        .updater()
        .map_err(|e| format!("初始化更新器失败: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("检查更新失败: {e}"))?;

    let Some(update) = update else {
        return Ok(DesktopUpdateInfo {
            latest_version: current.clone(),
            current_version: current,
            mandatory: false,
            release_notes: None,
            published_at: None,
        });
    };

    // 2. 通过 GitHub Release API 补全 release notes（失败不阻塞，仅缺 notes）
    let (release_notes, mandatory, published_at) = match github_client() {
        Ok(client) => match fetch_github_release(&client).await {
            Ok(release) => {
                let mandatory = release
                    .body
                    .as_deref()
                    .map(|body| body.contains("[MANDATORY]"))
                    .unwrap_or(false);
                let clean_body = release
                    .body
                    .map(|body| body.replace("[MANDATORY]", "").trim().to_string())
                    .filter(|s| !s.is_empty());
                (clean_body, mandatory, release.published_at)
            }
            Err(e) => {
                tracing::warn!("[updater] 获取 GitHub Release notes 失败: {e}");
                (None, false, None)
            }
        },
        Err(e) => {
            tracing::warn!("[updater] 创建 GitHub 客户端失败: {e}");
            (None, false, None)
        }
    };

    Ok(DesktopUpdateInfo {
        latest_version: update.version,
        current_version: current,
        mandatory,
        release_notes,
        published_at,
    })
}

// ── Command: 下载 APK ──────────────────────────────────────────

/// 从指定 URL 下载 APK 文件到缓存目录，并发送进度事件。
///
/// 支持**断点续传**：如果缓存目录中已存在部分下载的 `update.part` 文件，
/// 会自动通过 HTTP `Range` 请求头从断点处继续下载。
///
/// 如果提供了 `expected_checksum`（非空字符串），下载完成后会读取完整文件
/// 计算 SHA-256 并校验。验证失败则删除部分文件并返回错误。
///
/// 事件名：`apk-download-progress`
#[tauri::command]
pub async fn android_download_apk(
    app: tauri::AppHandle,
    version: String,
    download_url: String,
    expected_checksum: Option<String>,
) -> Result<(), String> {
    let dest = apk_cache_path(&app, &version)?;
    let part_path = apk_part_path(&app, &version)?;

    // 下载前再次清理旧版本缓存，确保不会把旧版本的 .part/.apk 混淆。
    let _ = cleanup_stale_apk_cache(&app, &version);
    let should_verify = expected_checksum
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // 检查是否有已下载的部分文件，用于断点续传
    let existing_size = if part_path.exists() {
        let meta = std::fs::metadata(&part_path).map_err(|e| format!("读取部分文件元数据: {e}"))?;
        let size = meta.len();
        // 部分文件体积异常（超过普通 APK 大小）时忽略
        if size > 0 && size < 300_000_000 {
            size
        } else {
            let _ = std::fs::remove_file(&part_path);
            0
        }
    } else {
        0
    };

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 构建请求：如果有已下载的部分，添加 Range 头
    let mut req = client.get(&download_url);
    if existing_size > 0 {
        req = req.header("Range", format!("bytes={}-", existing_size));
    }

    let mut resp = req.send().await.map_err(|e| format!("请求下载失败: {e}"))?;
    let status = resp.status();

    // 处理响应：断点续传 vs 重新下载，并解析完整文件大小
    let (mut file, initial_offset, file_total) =
        if existing_size > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
            // ── 服务器支持 Range，续传 ──
            let remaining = resp.content_length().unwrap_or(0);
            let full_size_from_header =
                parse_content_range_total(&resp).unwrap_or(existing_size + remaining);
            let file = std::fs::OpenOptions::new()
                .append(true)
                .open(&part_path)
                .map_err(|e| format!("打开部分文件追加: {e}"))?;
            (file, existing_size, full_size_from_header)
        } else {
            // ── 不支持续传或没有部分文件，重新下载 ──
            if existing_size > 0 {
                // 服务器不支持 Range，删除旧文件重新下载
                let _ = std::fs::remove_file(&part_path);
            }
            if !status.is_success() {
                return Err(format!("APK 下载返回 HTTP {}", status));
            }
            let chunk_total = resp.content_length().unwrap_or(0);
            let file = std::fs::File::create(&part_path).map_err(|e| format!("创建文件: {e}"))?;
            (file, 0u64, chunk_total)
        };

    // ── 流式下载（统一处理续传和新下载） ──
    let mut new_bytes: u64 = 0;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("下载分块失败: {e}"))?
    {
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("写入分块失败: {e}"))?;
        new_bytes += chunk.len() as u64;

        // 发送进度事件（包含总量和百分比，供前端进度条使用）
        let downloaded = initial_offset + new_bytes;
        if file_total > 0 {
            let pct = (downloaded as f64 / file_total as f64 * 100.0) as u32;
            let _ = app.emit(
                "apk-download-progress",
                ApkDownloadProgress {
                    progress: pct.min(100),
                    downloaded,
                    total: file_total,
                    done: false,
                    error: None,
                },
            );
        }
    }

    // 文件下载完成，关闭文件句柄
    drop(file);

    let final_size = initial_offset + new_bytes;

    // ── SHA-256 校验 ──
    if should_verify {
        use std::io::Read;
        let expected = expected_checksum.unwrap_or_default();
        let mut file =
            std::fs::File::open(&part_path).map_err(|e| format!("打开文件计算校验和: {e}"))?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| format!("读取文件校验: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let actual = format!("{:x}", hasher.finalize());
        if !expected.eq_ignore_ascii_case(&actual) {
            let _ = std::fs::remove_file(&part_path);
            return Err(format!(
                "CHECKSUM_MISMATCH: expected {}, got {}",
                expected, actual
            ));
        }
    }

    // ── 重命名为最终文件 ──
    // 先删除可能存在的旧最终文件
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&part_path, &dest).map_err(|e| format!("重命名 APK 文件失败: {e}"))?;

    // 发送完成事件
    let _ = app.emit(
        "apk-download-progress",
        ApkDownloadProgress {
            progress: 100,
            downloaded: final_size,
            total: final_size,
            done: true,
            error: None,
        },
    );

    Ok(())
}

/// 从 HTTP 响应中解析 `Content-Range` 响应头，提取完整文件大小。
/// 格式: `bytes {start}-{end}/{total}`
fn parse_content_range_total(resp: &reqwest::Response) -> Option<u64> {
    let header = resp.headers().get(reqwest::header::CONTENT_RANGE)?;
    let value = header.to_str().ok()?;
    // 提取 `/` 之后的部分
    let total = value.split('/').nth(1)?;
    // 处理 `*` 通配（极少见，但防御性处理）
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}

// ── Command: 安装 APK ──────────────────────────────────────────

/// 获取已下载的 APK 文件路径，用于安装。
#[tauri::command]
pub async fn android_get_apk_path(
    app: tauri::AppHandle,
    version: String,
) -> Result<String, String> {
    let path = apk_cache_path(&app, &version)?;
    if !path.exists() {
        return Err("APK 文件不存在，请先下载".to_string());
    }
    Ok(path.to_string_lossy().to_string())
}

/// 检查 APK 是否已下载。
#[tauri::command]
pub async fn android_is_apk_downloaded(
    app: tauri::AppHandle,
    version: String,
) -> Result<bool, String> {
    let path = apk_cache_path(&app, &version)?;
    Ok(path.exists())
}
