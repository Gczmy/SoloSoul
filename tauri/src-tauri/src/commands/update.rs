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
/// 按 tag 拉取指定版本的 Release（P002：下载命令重新拉取元数据，不信任前端回传）。
const GITHUB_API_TAG: &str = "https://api.github.com/repos/Gczmy/SoloSoul/releases/tags/";
const USER_AGENT: &str = "SoloSoul/2.6.1";

/// P003: APK 校验和（`.sha256`）的 minisign 公钥，**复用 embed 注册表密钥对**
/// （`embed_model.rs::EMBED_REGISTRY_PUBKEY_B64`，标准 minisign 格式，
/// `minisign_verify` 可解析；tauri.conf.json 的 updater pubkey 是 Tauri 自定义
/// 格式，与 `minisign_verify::Signature::decode` 不兼容，不可用于此路径）。
///
/// 发布侧流程：`cargo tauri signer sign -p '' <secret.key> <apk>.sha256`
/// 产出 `<apk>.sha256.minisig`（与 registry.json.minisig 同模式、同一把私钥），
/// 随 `.sha256` 一起上传到 GitHub Release。
///
/// 验签失败或签名缺失 → 校验和视为不可信（返回空串，客户端失去完整性校验，
/// 不阻断下载），杜绝「同通道替换 APK + 校验和」的静默降级。
const APK_CHECKSUM_PUBKEY: &str = "RWTemXPdgTgjPGuPgRxV+e3ng0NH2lgS8HzRbmi0XSlyjYXKI6zGkvXD";

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
    fetch_github_release_url(client, GITHUB_API).await
}

/// 按 tag 拉取指定版本的 Release（P002：下载命令重新拉取元数据，不信任前端回传）。
async fn fetch_github_release_by_tag(
    client: &reqwest::Client,
    tag: &str,
) -> Result<GitHubRelease, String> {
    fetch_github_release_url(client, &format!("{GITHUB_API_TAG}{tag}")).await
}

/// 请求 GitHub Release API 并解析 Release（latest 或按 tag）。
async fn fetch_github_release_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<GitHubRelease, String> {
    let resp = client
        .get(url)
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

/// P003: 校验 APK 校验和文件的 minisign 签名（与 embed_model 的
/// `verify_registry_signature` 同模式，发布侧用 `npx tauri signer sign` 签名）。
///
/// `tauri signer sign` 输出的 `.sig` 是 **base64 包裹的 minisign 明文**
/// （客户端先 base64 解码得到 `untrusted comment: ...` 开头的标准 minisign
/// 签名，`minisign_verify::Signature::decode` 才能解析），因此本函数先解码再验签。
fn verify_checksum_signature(checksum_bytes: &[u8], sig_text: &str) -> Result<(), String> {
    let public_key = minisign_verify::PublicKey::from_base64(APK_CHECKSUM_PUBKEY)
        .map_err(|e| format!("APK checksum public key parse failed: {e}"))?;
    // 先 base64 解码 tauri signer 输出，再交给 minisign_verify
    let sig_plain =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, sig_text.trim())
            .map_err(|e| format!("APK checksum signature base64 decode failed: {e}"))?;
    let sig_plain = String::from_utf8(sig_plain)
        .map_err(|e| format!("APK checksum signature UTF-8 decode failed: {e}"))?;
    let signature = minisign_verify::Signature::decode(&sig_plain)
        .map_err(|e| format!("APK checksum signature decode failed: {e}"))?;
    public_key
        .verify(checksum_bytes, &signature, false)
        .map_err(|e| format!("APK checksum signature verification failed: {e}"))
}

/// 从 Release 资产中查找 APK 下载资产，返回其 URL 与大小（克隆值，避免借用阻塞后续字段移动）。
fn find_apk_asset(release: &GitHubRelease) -> Option<(String, Option<i64>)> {
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk") || a.name.contains("universal-release"))
        .map(|a| (a.browser_download_url.clone(), a.size))
}

/// 查找对应的 `.sha256` 校验和资产、`.sha256.minisig` 签名资产，下载并验签。
///
/// P003: 校验和不再与 APK 同通道无条件信任——发布侧已用 embed 注册表私钥
/// 对 .sha256 文件签名（cargo tauri signer sign -p ''），客户端以编译期公钥
/// 验签；验签失败或缺失签名视为校验和不可信（返回 None）。
///
/// 返回验签通过的 64 位 hex 校验和（格式: "<64位hex>  <文件名>" 或仅 hex）。
async fn resolve_verified_checksum(
    client: &reqwest::Client,
    release: &GitHubRelease,
) -> Option<String> {
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk.sha256") || a.name.contains("sha256"))?;
    let sig_asset = release.assets.iter().find(|a| {
        a.name.ends_with(".sha256.minisig")
            || a.name.ends_with(".apk.sha256.minisig")
            || a.name.contains(".sha256.minisig")
    });

    // 下载校验和文件（约 64 字节）与签名文件
    let body = match client
        .get(&checksum_asset.browser_download_url)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
        _ => String::new(),
    };
    let sig_text = match sig_asset {
        Some(asset) => match client.get(&asset.browser_download_url).send().await {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            _ => String::new(),
        },
        None => String::new(),
    };

    let sig_ok =
        !sig_text.is_empty() && verify_checksum_signature(body.as_bytes(), &sig_text).is_ok();
    if !sig_ok {
        tracing::warn!(
            "[updater] APK checksum minisign signature missing or invalid — checksum rejected"
        );
        return None;
    }

    // 验签通过后，才提取 64 位 hex 作为校验和。
    body.split_whitespace()
        .next()
        .filter(|token| token.len() == 64)
        .map(|s| s.to_string())
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

    // 查找 APK 资产；校验和走共享解析（下载 .sha256 + .minisig 并验签）
    let (apk_download_url, apk_size) = find_apk_asset(&release).unwrap_or_default();
    let apk_download_url = (!apk_download_url.is_empty()).then_some(apk_download_url);
    let checksum = resolve_verified_checksum(&client, &release).await;

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
        download_url: apk_download_url,
        checksum: checksum.unwrap_or_default(),
        mandatory,
        release_notes: clean_body,
        published_at: release.published_at,
        apk_size,
    })
}

// ── Command: 桌面端检查更新 ─────────────────────────────────────

/// 桌面端检查更新：版本检测复用 Tauri updater 插件（读取 latest.json），
/// Release notes 通过 GitHub Release API 补全，行为与 Android 对齐。
/// 从 GitHub Release API 结果构建桌面端更新信息（updater 插件兜底路径，与 Android 逻辑对齐）。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn desktop_info_from_github_release(current: &str, release: &GitHubRelease) -> DesktopUpdateInfo {
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();
    let mandatory = release
        .body
        .as_deref()
        .map(|body| body.contains("[MANDATORY]"))
        .unwrap_or(false);
    let clean_body = release
        .body
        .as_ref()
        .map(|body| body.replace("[MANDATORY]", "").trim().to_string())
        .filter(|s| !s.is_empty());
    DesktopUpdateInfo {
        latest_version: latest,
        current_version: current.to_string(),
        mandatory,
        release_notes: clean_body,
        published_at: release.published_at.clone(),
    }
}

/// 桌面端检查更新：版本检测首选 Tauri updater 插件（latest.json + 签名校验）。
///
/// updater 插件路径依赖 `github.com` 的 release 下载端点（302 → release-assets），
/// 在部分地区可能不稳定或不可达。失败时记录日志并回退到 GitHub Release API
/// （与 Android 同路径，仅需 `api.github.com`），保证「关于页面」仍能给出版本信息。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn desktop_check_update(app: tauri::AppHandle) -> Result<DesktopUpdateInfo, String> {
    let current = current_version();

    // 1. 首选：通过 updater 插件检测是否有可用更新（latest.json 中的版本与签名）
    let updater_result = match app.updater() {
        Ok(updater) => updater
            .check()
            .await
            .map_err(|e| format!("检查更新失败: {e}")),
        Err(e) => Err(format!("初始化更新器失败: {e}")),
    };

    match updater_result {
        Ok(Some(update)) => {
            tracing::info!(
                "[updater] 检测到新版本 {}（当前 {}）",
                update.version,
                current
            );
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
        Ok(None) => {
            tracing::info!("[updater] 已是最新版本（{}）", current);
            Ok(DesktopUpdateInfo {
                latest_version: current.clone(),
                current_version: current,
                mandatory: false,
                release_notes: None,
                published_at: None,
            })
        }
        Err(plugin_err) => {
            // 3. 兜底：通过 GitHub Release API 检测版本（仅需 api.github.com）
            tracing::error!("[updater] updater 插件检查失败，回退 GitHub API: {plugin_err}");
            match github_client() {
                Ok(client) => match fetch_github_release(&client).await {
                    Ok(release) => Ok(desktop_info_from_github_release(&current, &release)),
                    Err(fallback_err) => Err(format!(
                        "检查更新失败: {plugin_err}（GitHub API 兜底失败: {fallback_err}）"
                    )),
                },
                Err(client_err) => Err(format!(
                    "检查更新失败: {plugin_err}（创建 HTTP 客户端失败: {client_err}）"
                )),
            }
        }
    }
}

// ── Command: 下载 APK ──────────────────────────────────────────

/// 从指定 URL 下载 APK 文件到缓存目录，并发送进度事件。
///
/// 支持**断点续传**：如果缓存目录中已存在部分下载的 `update.part` 文件，
/// 会自动通过 HTTP `Range` 请求头从断点处继续下载。
///
/// P002: 下载 URL 与 SHA-256 校验和**不信任前端回传**——WebView 被 XSS 控制时
/// 可诱导下载任意 URL 的 APK 并触发系统安装流程。因此本命令仅接收 `version`，
/// 在 Rust 侧按 `releases/tags/v{version}` 重新拉取 GitHub Release 元数据，
/// 复用 `resolve_verified_checksum` 重新验签；元数据缺失或验签失败则 **fail-closed**
/// 拒绝下载（绝不降级为「无校验下载」）。下载完成后仍强制 SHA-256 校验。
///
/// 事件名：`apk-download-progress`
#[tauri::command]
pub async fn android_download_apk(app: tauri::AppHandle, version: String) -> Result<(), String> {
    let dest = apk_cache_path(&app, &version)?;
    let part_path = apk_part_path(&app, &version)?;

    // 下载前再次清理旧版本缓存，确保不会把旧版本的 .part/.apk 混淆。
    let _ = cleanup_stale_apk_cache(&app, &version);

    // P002: Rust 侧重新拉取 release 元数据，提取 APK 下载地址并重新验签校验和。
    let client = github_client()?;
    let tag = if version.starts_with('v') {
        version.clone()
    } else {
        format!("v{}", version)
    };
    let release = fetch_github_release_by_tag(&client, &tag).await?;
    let (download_url, _apk_size) =
        find_apk_asset(&release).ok_or_else(|| "Release 中未找到 APK 资产".to_string())?;
    let expected_checksum = resolve_verified_checksum(&client, &release)
        .await
        .ok_or_else(|| "APK 校验和不可信（签名缺失或验签失败），已拒绝下载".to_string())?;
    let should_verify = true;

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

    // ── SHA-256 校验（P002：校验和由 Rust 侧验签获取，必非空，强制校验） ──
    if should_verify {
        use std::io::Read;
        let expected = expected_checksum;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// P003 防回归：编译期 APK 校验和公钥必须能被 minisign_verify 解析。
    /// 曾误用 tauri.conf.json 的 updater pubkey（Tauri 自定义格式），
    /// minisign_verify 无法解析——此测试确保公钥为标准 minisign 格式。
    #[test]
    fn test_apk_checksum_pubkey_is_parseable() {
        minisign_verify::PublicKey::from_base64(APK_CHECKSUM_PUBKEY)
            .expect("编译期公钥必须可解析（标准 minisign 格式）");
    }

    /// P003 防回归：签名文本必须能被 base64 解码 + Signature::decode 解析
    /// （标准 minisign 格式），篡改/空签名应被拒绝。
    #[test]
    fn test_verify_checksum_signature_rejects_tampered() {
        // 非 base64 文本
        let bogus_sig = "not-base64!!!";
        assert!(verify_checksum_signature(b"some checksum bytes", bogus_sig).is_err());

        // 空签名同样被拒
        assert!(verify_checksum_signature(b"x", "").is_err());
    }

    /// P003 端到端：真实签名（embed-registry 私钥签发，tauri signer 输出）验签通过。
    /// 测试数据为发布侧用 embed 私钥对固定校验和文本签名后的真实产物（2026-08-07
    /// 采集），与客户端运行时下载 `.sha256` + `.sha256.minisig` 后的验签链路一致。
    #[test]
    fn test_verify_checksum_signature_end_to_end() {
        // tauri signer sign 输出（base64 包裹的 minisign 明文）
        let real_sig = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUZW1YUGRnVGdqUElXektwTklqanR0NFhta25GN3FhSHI3UFh3VitLTURIU0hMeUxSbGVKc1krclNSSGZOS1FCK1FieCtZckJlckNXaHpJQ3owZlpaR051NktxN2kwWmcwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg2MTE0MTcxCWZpbGU6cDAwM190ZXN0LnNoYTI1NgpCdG5FUWQxUkVrdlVhL2VKUkhST29XU2lPVWJBQlBCOU9UbXordFpwclkyZGN6VGcyKy8ycDVxRHBJc3pkRFVXRHNwbzdjT012cTk3UXR4RmdPL1FDQT09Cg==";
        // 被签名的校验和文件内容（`<64位hex>\n`）
        let checksum_bytes = b"deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678\n";
        assert!(
            verify_checksum_signature(checksum_bytes, real_sig).is_ok(),
            "真实签名应验签通过"
        );

        // 篡改校验和内容 → 验签必须失败
        let tampered = b"deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345679\n";
        assert!(verify_checksum_signature(tampered, real_sig).is_err());
    }

    /// 校验和提取逻辑：仅接受 64 位 hex 首 token。
    #[test]
    fn test_checksum_token_extraction() {
        let ok: Option<String> =
            "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90  file.apk"
                .split_whitespace()
                .next()
                .filter(|token| token.len() == 64)
                .map(|s| s.to_string());
        assert!(ok.is_some());

        let short: Option<String> = "abc"
            .split_whitespace()
            .next()
            .filter(|token| token.len() == 64)
            .map(|s| s.to_string());
        assert!(short.is_none());
    }
}
