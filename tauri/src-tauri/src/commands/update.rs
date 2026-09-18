//! 应用内更新检查、可取消下载和安装命令。
//!
//! 通过 GitHub Release API 检查新版本，下载 APK 并触发系统安装。
//! 桌面端保留 updater 相同公钥验签；两端共用有界测速、低速换源、续传和取消。

use super::update_download::{cached_progress, download_file, TransferProgress};
use super::update_preferences::{
    select_source, Channel as SourceChannel, Metadata, SourcePreferences,
};
use super::update_sources::{artifact_candidates, secure_url, UpdateSources};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{ipc::Channel, Manager, Resource, ResourceId, Webview};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_updater::UpdaterExt;

const GITHUB_API: &str = "https://api.github.com/repos/Gczmy/SoloSoul/releases/latest";
/// 按 tag 拉取指定版本的 Release（P002：下载命令重新拉取元数据，不信任前端回传）。
const GITHUB_API_TAG: &str = "https://api.github.com/repos/Gczmy/SoloSoul/releases/tags/";
const USER_AGENT: &str = concat!("SoloSoul/", env!("CARGO_PKG_VERSION"));

/// GitHub 下载加速代理前缀（国内直连 GitHub Release 资产/API 不稳定时按序回退）。
///
/// 拼接规则：`<prefix> + <原始 github.com / api.github.com URL>`，例如
/// `https://ghfast.top/https://github.com/Gczmy/SoloSoul/releases/download/v2.9.2/...`。
///
/// 安全说明：下载内容无论来自直连还是代理，都会经过 minisign/SHA-256 强制校验
/// （桌面端 updater 同等验签、安卓端 P002 校验），代理不能替换未签名安装包。
///
/// 隐私披露（T004）：gh-proxy 类代理是 TLS 终止代理——连接在代理方解密后转发，
/// 因此**用户 IP、使用 SoloSoul 的事实、目标版本号、GitHub API 响应内容都会暴露
/// 给第三方代理服务商**。自有源/直连优先；清单慢响应和大包低速会触发备选探测。
/// 若用户对此敏感，可通过环境变量
/// `SOLOSOUL_PROXY_PREFIXES`（逗号分隔）覆盖为自建可信代理或留空禁用代理。
///
/// 可用性披露（T004）：① `api.github.com` 元数据请求走代理时，代理可返回陈旧/
/// 篡改的 Release JSON 软性压制升级（内容完整性不受影响——校验和与签名在 Rust 侧
/// 重新验签，属可用性面）；② 代理也可重放旧版 latest-mirror JSON 压制升级（updater
/// 只升不降，无降级风险）。
///
/// 维护注意：这些第三方代理服务存活期不稳定，失效条目应在此处替换为可用条目；
/// 清单错峰请求、大包按实际吞吐选源，单个代理失效不会阻断其他候选。
const PROXY_PREFIXES: &[&str] = &[
    "https://ghproxy.net/",
    "https://gh-proxy.com/",
    "https://ghfast.top/",
];

/// T004: 代理前缀列表（可被环境变量 `SOLOSOUL_PROXY_PREFIXES` 覆盖，逗号分隔）。
///
/// U001 语义修正：**未设置**（`var` 返回 `Err`）→ 回退默认 `PROXY_PREFIXES`；
/// **显式置空或仅空白**（隐私敏感用户意图禁用代理）→ 返回空列表，仅走直连，
/// 不再回退默认——与注释/提交/报告三处「留空禁用代理」承诺一致。
/// 设置非空值时按逗号分隔去空白过滤。
fn proxy_prefixes() -> Vec<String> {
    match std::env::var("SOLOSOUL_PROXY_PREFIXES") {
        Ok(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .take(8)
            .map(str::to_string)
            .collect(),
        Err(_) => PROXY_PREFIXES.iter().map(|p| (*p).to_string()).collect(),
    }
}

/// 为给定 GitHub URL 生成下载候选列表：直连优先，随后各代理前缀。
fn download_candidates(url: &str) -> Vec<String> {
    let mut candidates = vec![url.to_string()];
    candidates.extend(proxy_prefixes().iter().map(|p| format!("{p}{url}")));
    candidates
}

/// 小清单请求错峰竞速，校验完整响应后才采用；不会被“返回 200 但正文停流”锁住。
async fn read_small_response(
    client: &reqwest::Client,
    url: &str,
    limit: usize,
) -> Result<Vec<u8>, String> {
    secure_url(url)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?
        .error_for_status()
        .map_err(|e| e.without_url().to_string())?;
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err("更新元数据超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| e.without_url().to_string())?
    {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("更新元数据超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// P003: APK 校验和（`.sha256`）的 minisign 公钥，**复用 embed 注册表密钥对**
/// （`embed_model.rs::EMBED_REGISTRY_PUBKEY_B64`，标准 minisign 格式，
/// `minisign_verify` 可解析；与桌面 updater 使用不同密钥，不能互换公钥）。
///
/// 发布侧流程：`cargo tauri signer sign -p '' <secret.key> <apk>.sha256`
/// 产出 `<apk>.sha256.minisig`（与 registry.json.minisig 同模式、同一把私钥），
/// 随 `.sha256` 一起上传到 GitHub Release。
///
/// 验签失败或签名缺失 → 校验和视为不可信，下载命令硬性拒绝，不降级校验。
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
    /// P012: 校验和不可用原因（签名缺失/验签失败/资产缺失），供前端展示可感知警告。
    pub checksum_warning: Option<String>,
    /// 是否为强制更新。当 Release body 包含 `[MANDATORY]` 标记时为 true。
    /// 强制更新会显示不可关闭的对话框，用户必须更新才能继续使用。
    pub mandatory: bool,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub apk_size: Option<i64>,
    #[serde(default)]
    pub cached_download: Option<CachedDownload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDownload {
    pub downloaded: u64,
    pub total: u64,
    pub done: bool,
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
    pub source: Option<String>,
    pub bytes_per_second: Option<u64>,
    pub phase: Option<String>,
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

// ── 可取消下载资源 ────────────────────────────────────────────

const DOWNLOAD_CANCELLED: &str = "UPDATE_DOWNLOAD_CANCELLED";

/// 先创建资源再发起下载，取消信号即使先于下载命令到达也不会丢失。
/// 每个资源仅允许启动一次，避免重用 ID 导致两个请求共享取消/完成状态。
struct UpdateDownloadOperation {
    cancelled: tokio::sync::watch::Sender<bool>,
    started: AtomicBool,
}

impl UpdateDownloadOperation {
    fn new() -> Self {
        let (cancelled, _) = tokio::sync::watch::channel(false);
        Self {
            cancelled,
            started: AtomicBool::new(false),
        }
    }

    fn cancel(&self) {
        self.cancelled.send_replace(true);
    }

    fn check_cancelled(&self) -> Result<(), String> {
        if *self.cancelled.borrow() {
            Err(DOWNLOAD_CANCELLED.to_string())
        } else {
            Ok(())
        }
    }

    async fn run<T>(
        &self,
        download: impl std::future::Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err("更新下载操作已启动，请创建新操作".to_string());
        }
        let mut cancelled = self.cancelled.subscribe();
        self.check_cancelled()?;
        tokio::select! {
            biased;
            _ = cancelled.changed() => Err(DOWNLOAD_CANCELLED.to_string()),
            result = download => {
                self.check_cancelled()?;
                result
            }
        }
    }
}

impl Resource for UpdateDownloadOperation {
    fn close(self: Arc<Self>) {
        self.cancel();
    }
}

#[tauri::command]
pub fn create_update_download(webview: Webview) -> ResourceId {
    webview
        .resources_table()
        .add(UpdateDownloadOperation::new())
}

#[tauri::command]
pub fn cancel_update_download(webview: Webview, operation_id: ResourceId) -> Result<(), String> {
    let operation = webview
        .resources_table()
        .get::<UpdateDownloadOperation>(operation_id)
        .map_err(|e| e.to_string())?;
    operation.cancel();
    Ok(())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DesktopDownloadEvent {
    #[serde(rename_all = "camelCase")]
    Transfer {
        downloaded: u64,
        total: u64,
        source: String,
        bytes_per_second: u64,
        phase: String,
    },
    Finished,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
static DESKTOP_CACHE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn remove_download_cache(part_path: &std::path::Path) {
    let _ = std::fs::remove_file(part_path);
    let mut sidecar = part_path.as_os_str().to_owned();
    sidecar.push(".json");
    let _ = std::fs::remove_file(PathBuf::from(sidecar));
}

/// 与 Tauri updater 的验签算法一致，公钥仅取编译期配置，不能由下载清单/前端替换。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn verify_desktop_signature(data: &[u8], signature: &str, pubkey: &str) -> Result<(), String> {
    use base64::Engine;
    let decode = |text: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text.trim())
            .map_err(|_| "更新签名编码错误")?;
        String::from_utf8(bytes).map_err(|_| "更新签名格式错误".into())
    };
    let key =
        minisign_verify::PublicKey::decode(&decode(pubkey)?).map_err(|_| "更新公钥格式错误")?;
    let sig =
        minisign_verify::Signature::decode(&decode(signature)?).map_err(|_| "更新签名格式错误")?;
    key.verify(data, &sig, true)
        .map_err(|_| "更新包签名校验失败，已拒绝安装".into())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedDesktopUpdate {
    rid: ResourceId,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    raw_json: serde_json::Value,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn check_desktop_sources(
    app: &tauri::AppHandle,
) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let sources = UpdateSources::load()?;
    let mut endpoints = sources.manifest_endpoints;
    let proxies = proxy_prefixes();
    if let Some(configured) = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|v| v.get("endpoints"))
        .and_then(|v| v.as_array())
    {
        for endpoint in configured.iter().filter_map(|v| v.as_str()) {
            if !endpoint.contains("/https://") || proxies.iter().any(|p| endpoint.starts_with(p)) {
                endpoints.push(endpoint.to_string());
            }
        }
    }
    // 自建代理也能读取普通清单，下载时由独立选源器决定实际通道。
    for prefix in &proxies {
        endpoints.push(format!(
            "{prefix}https://github.com/Gczmy/SoloSoul/releases/latest/download/latest.json"
        ));
    }
    let mut seen = std::collections::HashSet::new();
    endpoints.retain(|url| seen.insert(url.clone()));
    let preferences = SourcePreferences::load(app);
    let current = semver::Version::parse(&current_version()).map_err(|e| e.to_string())?;
    let selected = select_source(
        &endpoints,
        preferences.preferred(SourceChannel::Manifest, &endpoints),
        &current,
        |endpoint| {
            Box::pin(async move {
                let endpoint = secure_url(endpoint)?;
                // 插件在没有更新时返回 None；保留已解析的真实版本用于日志和陈旧源判定。
                let remote = Arc::new(std::sync::OnceLock::new());
                let observed = remote.clone();
                let value = app
                    .updater_builder()
                    .endpoints(vec![endpoint])
                    .map_err(|e| e.to_string())?
                    .version_comparator(move |current, release| {
                        let _ = observed.set(release.version.clone());
                        release.version > current
                    })
                    .timeout(std::time::Duration::from_secs(8))
                    .configure_client(|client| {
                        client
                            .connect_timeout(std::time::Duration::from_secs(3))
                            .https_only(true)
                    })
                    .build()
                    .map_err(|e| e.to_string())?
                    .check()
                    .await
                    .map_err(|e| e.to_string())?;
                let version = remote.get().cloned().ok_or("更新源未返回版本清单")?;
                Ok(Metadata { value, version })
            })
        },
    )
    .await?;
    log_version_success(
        "updater 清单",
        &selected.url,
        &current.to_string(),
        &selected.metadata.version,
    );
    preferences.remember(SourceChannel::Manifest, &selected.url, selected.full_probe);
    Ok(selected.metadata.value)
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn desktop_prepare_update(
    webview: Webview,
) -> Result<Option<PreparedDesktopUpdate>, String> {
    let Some(update) = check_desktop_sources(webview.app_handle()).await? else {
        return Ok(None);
    };
    let metadata = PreparedDesktopUpdate {
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        body: update.body.clone(),
        raw_json: update.raw_json.clone(),
        rid: webview.resources_table().add(update),
    };
    Ok(Some(metadata))
}

/// 安装包与原生 Update 绑定，前端无法把未验签字节或其他 URL 注入安装命令。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
struct VerifiedDesktopDownload {
    update: Arc<tauri_plugin_updater::Update>,
    bytes: Vec<u8>,
    installed: tokio::sync::Mutex<bool>,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl Resource for VerifiedDesktopDownload {}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn desktop_download_update(
    webview: Webview,
    update_rid: ResourceId,
    operation_id: ResourceId,
    on_event: Channel<DesktopDownloadEvent>,
) -> Result<ResourceId, String> {
    let operation = webview
        .resources_table()
        .get::<UpdateDownloadOperation>(operation_id)
        .map_err(|e| e.to_string())?;
    let update = webview
        .resources_table()
        .get::<tauri_plugin_updater::Update>(update_rid)
        .map_err(|e| e.to_string())?;
    let _guard = DESKTOP_CACHE_LOCK
        .try_lock()
        .map_err(|_| "已有更新下载正在进行中")?;
    let identity = format!(
        "desktop:{}:{}:{}",
        update.version, update.target, update.signature
    );
    let key = format!("{:x}", sha2::Sha256::digest(identity.as_bytes()));
    let cache_dir = webview
        .app_handle()
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("updates");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("创建更新缓存失败: {e}"))?;
    let part_path = cache_dir.join(format!("desktop-{key}.part"));
    // 仅清理本模块的其他版本缓存，保留当前包的断点；全程持有同一缓存锁。
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("desktop-")
                && (name.ends_with(".part") || name.ends_with(".part.json"))
                && name != format!("desktop-{key}.part")
                && name != format!("desktop-{key}.part.json")
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let sources = UpdateSources::load()?;
    let candidates = artifact_candidates(
        update.download_url.as_str(),
        &update.version,
        &sources,
        &proxy_prefixes(),
    )?;
    let pubkey = webview
        .app_handle()
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|config| config.get("pubkey"))
        .and_then(serde_json::Value::as_str)
        .ok_or("缺少编译期更新公钥")?
        .to_string();
    let bytes = operation
        .run(async {
            download_file(
                &candidates,
                &part_path,
                &identity,
                &|progress: TransferProgress| {
                    let _ = on_event.send(DesktopDownloadEvent::Transfer {
                        downloaded: progress.downloaded,
                        total: progress.total,
                        source: progress.source,
                        bytes_per_second: progress.bytes_per_second,
                        phase: progress.phase.into(),
                    });
                },
            )
            .await?;
            operation.check_cancelled()?;
            let bytes = std::fs::read(&part_path).map_err(|e| format!("读取更新缓存失败: {e}"))?;
            // Update.install() 本身不验签。自定义传输必须在创建可安装资源之前执行同等验签。
            if let Err(error) = verify_desktop_signature(&bytes, &update.signature, &pubkey) {
                remove_download_cache(&part_path);
                return Err(error);
            }
            operation.check_cancelled()?;
            remove_download_cache(&part_path);
            Ok(bytes)
        })
        .await?;
    // Finished 只在原生验签通过之后发送，前端仍以 invoke 返回值作为最终完成信号。
    let rid = webview.resources_table().add(VerifiedDesktopDownload {
        update,
        bytes,
        installed: tokio::sync::Mutex::new(false),
    });
    let _ = on_event.send(DesktopDownloadEvent::Finished);
    Ok(rid)
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn desktop_install_update(
    webview: Webview,
    download_rid: ResourceId,
) -> Result<(), String> {
    let download = webview
        .resources_table()
        .get::<VerifiedDesktopDownload>(download_rid)
        .map_err(|e| e.to_string())?;
    let mut installed = download
        .installed
        .try_lock()
        .map_err(|_| "更新安装正在进行中".to_string())?;
    if *installed {
        return Err("此更新包已经安装".to_string());
    }
    download
        .update
        .install(&download.bytes)
        .map_err(|e| e.to_string())?;
    *installed = true;
    let _ = webview.resources_table().close(download_rid);
    Ok(())
}

// ── Helper: 获取当前版本号 ──────────────────────────────────────

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// P020: 版本单调性判断——仅当 `a` 严格高于 `b` 时才视为新版本。
///
/// 任一侧解析失败（非 semver）时保守判定非新（fail-safe：宁可漏提示，不可在
/// 第三方代理重放旧 release 元数据时诱导用户降级）。GitHub tag 与 `CARGO_PKG_VERSION`
/// 均为 X.Y.Z 形式，正常路径永远走 semver 分支。
fn version_is_newer(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(va), Ok(vb)) => va > vb,
        _ => false,
    }
}

/// P020: 版本单调性归一——非新版本（低于/等于已安装版本）时归一为当前版本，
/// 前端 `latest == current` 判等即不再提示更新，杜绝代理重放旧清单的降级提示。
fn normalize_to_newer(latest: String, current: &str) -> String {
    if version_is_newer(&latest, current) {
        latest
    } else {
        current.to_string()
    }
}

/// 仅输出公开来源主机和版本，不记录带参数 URL 或账户信息。
fn log_version_success(channel: &str, url: &str, current: &str, remote: &semver::Version) {
    let source = url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_default();
    tracing::info!(
        "[updater] 成功读取版本信息：通道={}，来源={}，当前版本={}，读取版本={}，发现新版本={}",
        channel,
        source,
        current,
        remote,
        version_is_newer(&remote.to_string(), current)
    );
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

/// 下载与缓存清理使用同一把锁：检查后删除、两个窗口同时下载均不能互相覆盖文件。
static APK_CACHE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 调用方必须持有 APK_CACHE_LOCK，下载前清理也复用这条路径。
fn cleanup_stale_apk_cache_locked(
    app: &tauri::AppHandle,
    current_version: &str,
) -> Result<(), String> {
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
        // 仅处理本模块的安装包、断点与元数据，不碰其他应用缓存。
        let Some(stripped) = name_str.strip_prefix("update_") else {
            continue;
        };
        let file_version = stripped
            .strip_suffix(".apk")
            .or_else(|| stripped.strip_suffix(".part.json"))
            .or_else(|| stripped.strip_suffix(".part"));
        let orphan = stripped.contains(".part.seg") || stripped.ends_with(".part.merge");
        if orphan || file_version.is_some_and(|version| version != current_part) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

// ── Command: 检查更新 ──────────────────────────────────────────

/// 小清单/签名独立于大包传输：短连接与总时限，限制重定向为 HTTPS。
fn update_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 8 {
            attempt.error("更新请求重定向过多")
        } else if secure_url(attempt.url().as_str()).is_err() {
            attempt.error("更新重定向必须使用 HTTPS")
        } else {
            attempt.follow()
        }
    })
}

fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(8))
        .redirect(update_redirect_policy())
        .build()
        .map_err(|e| format!("创建更新客户端失败: {e}"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn fetch_github_release(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
) -> Result<GitHubRelease, String> {
    fetch_release(Some(app), client, None, false).await
}

async fn fetch_github_release_by_tag(
    client: &reqwest::Client,
    tag: &str,
) -> Result<GitHubRelease, String> {
    fetch_release(None, client, Some(tag), true).await
}

/// 优先自有 CDN 的 release.json，GitHub 与允许的代理仅作后备。
async fn fetch_release(
    app: Option<&tauri::AppHandle>,
    client: &reqwest::Client,
    tag: Option<&str>,
    require_apk: bool,
) -> Result<GitHubRelease, String> {
    let mut candidates = UpdateSources::load()?.release_endpoints(tag)?;
    let github = tag
        .map(|tag| format!("{GITHUB_API_TAG}{tag}"))
        .unwrap_or_else(|| GITHUB_API.into());
    // 静态 Release 资产适配只代理 github.com 下载、却不支持 api.github.com 的线路。
    let static_url = match tag {
        Some(tag) => {
            format!("https://github.com/Gczmy/SoloSoul/releases/download/{tag}/release.json")
        }
        None => "https://github.com/Gczmy/SoloSoul/releases/latest/download/release.json".into(),
    };
    candidates.extend(download_candidates(&static_url));
    candidates.extend(download_candidates(&github));
    let preferences = app.map(SourcePreferences::load);
    let preferred = preferences
        .as_ref()
        .and_then(|prefs| prefs.preferred(SourceChannel::Release, &candidates));
    let current = semver::Version::parse(&current_version()).map_err(|e| e.to_string())?;
    let selected = select_source(&candidates, preferred, &current, |url| {
        Box::pin(async move {
            let bytes = read_small_response(client, url, 2 * 1024 * 1024).await?;
            let release: GitHubRelease =
                serde_json::from_slice(&bytes).map_err(|e| format!("更新清单解析失败: {e}"))?;
            let raw_version = release
                .tag_name
                .strip_prefix('v')
                .unwrap_or(&release.tag_name);
            let version = semver::Version::parse(raw_version).map_err(|_| "更新清单版本号无效")?;
            if tag.is_some_and(|expected| {
                expected.strip_prefix('v').unwrap_or(expected) != raw_version
            }) {
                return Err("更新清单版本与请求版本不一致".into());
            }
            if require_apk && !has_complete_apk_assets(&release) {
                return Err("更新清单缺少完整 APK、校验和及签名资产".into());
            }
            Ok(Metadata {
                value: release,
                version,
            })
        })
    })
    .await?;
    log_version_success(
        "Release 元数据",
        &selected.url,
        &current.to_string(),
        &selected.metadata.version,
    );
    if let Some(preferences) = preferences {
        preferences.remember(SourceChannel::Release, &selected.url, selected.full_probe);
    }
    Ok(selected.metadata.value)
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
///
/// N006: 谓词收紧为仅 `ends_with(".apk")`——旧逻辑含 `contains("universal-release")`，
/// 会误命中 `xx-universal-release.apk.sha256(.minisig)` 等校验和/签名资产（若 GitHub
/// 资产排序不利 → 下载到非 APK 文件或 fail-closed 误拒）。`universal-release` 只是
/// 命名惯例，不能替代扩展名判断。
fn find_apk_metadata(release: &GitHubRelease) -> Option<&GitHubAsset> {
    // 优先可验签的版本化 APK，避免 Release 中的下载别名排在前面却没有对应校验和。
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk") && apk_has_integrity_assets(release, a))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".apk")))
}

fn apk_has_integrity_assets(release: &GitHubRelease, apk: &GitHubAsset) -> bool {
    secure_url(&apk.browser_download_url).is_ok()
        && [
            format!("{}.sha256", apk.name),
            format!("{}.sha256.minisig", apk.name),
        ]
        .iter()
        .all(|name| {
            release
                .assets
                .iter()
                .any(|a| &a.name == name && secure_url(&a.browser_download_url).is_ok())
        })
}

fn has_complete_apk_assets(release: &GitHubRelease) -> bool {
    find_apk_metadata(release).is_some_and(|apk| apk_has_integrity_assets(release, apk))
}

fn find_apk_asset(release: &GitHubRelease) -> Option<(String, Option<i64>)> {
    find_apk_metadata(release).map(|a| (a.browser_download_url.clone(), a.size))
}

/// 查找对应的 `.sha256` 校验和资产、`.sha256.minisig` 签名资产，下载并验签。
///
/// P003: 校验和不再与 APK 同通道无条件信任——发布侧已用 embed 注册表私钥
/// 对 .sha256 文件签名（cargo tauri signer sign -p ''），客户端以编译期公钥
/// 验签；验签失败或缺失签名视为校验和不可信（返回 None）。
///
/// 返回验签通过的 64 位 hex 校验和（格式: "<64位hex>  <文件名>" 或仅 hex）。
/// 返回 (校验和, 不可用原因)——P012: 验签失败/缺失不再静默吞掉，而是把原因
/// 带给调用方（check 命令转发给前端展示警告，download 命令 fail-closed 拒绝）。
async fn resolve_verified_checksum(
    client: &reqwest::Client,
    release: &GitHubRelease,
) -> (Option<String>, Option<String>) {
    let Some(apk) = find_apk_metadata(release) else {
        return (None, Some("发布未提供 APK".into()));
    };
    let Some(checksum_asset) = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", apk.name))
    else {
        return (None, Some("发布未提供对应 APK 的 .sha256 校验和".into()));
    };
    let Some(sig_asset) = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.minisig", checksum_asset.name))
    else {
        return (None, Some("发布未提供校验和签名".into()));
    };
    let sources = match UpdateSources::load() {
        Ok(s) => s,
        Err(e) => return (None, Some(e)),
    };
    let proxies = proxy_prefixes();
    let checksum_urls = artifact_candidates(
        &checksum_asset.browser_download_url,
        &release.tag_name,
        &sources,
        &proxies,
    );
    let signature_urls = artifact_candidates(
        &sig_asset.browser_download_url,
        &release.tag_name,
        &sources,
        &proxies,
    );
    let (checksum_urls, signature_urls) = match (checksum_urls, signature_urls) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return (None, Some("校验和下载地址无效".into())),
    };
    // 每个候选完整读取并验签后才参与竞速，错误/伪造响应不会抢先阻止健康源。
    let mut pending: FuturesUnordered<_> = checksum_urls
        .iter()
        .zip(&signature_urls)
        .enumerate()
        .map(|(i, (checksum_url, signature_url))| async move {
            tokio::time::sleep(std::time::Duration::from_millis(i.min(8) as u64 * 350)).await;
            let (body, signature) = tokio::try_join!(
                read_small_response(client, checksum_url, 4096),
                read_small_response(client, signature_url, 8192)
            )?;
            let signature = std::str::from_utf8(&signature).map_err(|_| "校验和签名编码无效")?;
            verify_checksum_signature(&body, signature)?;
            let body = std::str::from_utf8(&body).map_err(|_| "校验和编码无效")?;
            body.split_whitespace()
                .next()
                .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
                .map(str::to_string)
                .ok_or_else(|| "校验和必须为 64 位十六进制".to_string())
        })
        .collect();
    while let Some(result) = pending.next().await {
        if let Ok(checksum) = result {
            return (Some(checksum), None);
        }
    }
    (
        None,
        Some("所有下载源的校验和缺失或验签失败，已拒绝无校验下载".into()),
    )
}

/// 仅持久化新版本提示，旧镜像的迟到响应不能覆盖已有的新版本断点入口。
fn remember_update_info(path: &std::path::Path, info: &AndroidUpdateInfo) -> Result<(), String> {
    use std::io::Write;
    if !version_is_newer(&info.latest_version, &info.current_version) {
        return Ok(());
    }
    if std::fs::metadata(path).is_ok_and(|m| m.len() <= 2 * 1024 * 1024) {
        if let Some(previous) = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AndroidUpdateInfo>(&bytes).ok())
        {
            if version_is_newer(&previous.latest_version, &info.latest_version) {
                return Ok(());
            }
        }
    }
    let parent = path.parent().ok_or("更新信息缓存目录无效")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(info).map_err(|e| e.to_string())?;
    temporary.write_all(&bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 启动先展示上次检查结果及真实断点，同时继续在线检查；缓存不授权安装。
#[tauri::command]
pub fn android_cached_update(app: tauri::AppHandle) -> Option<AndroidUpdateInfo> {
    let path = app.path().app_cache_dir().ok()?.join("update-info.json");
    if std::fs::metadata(&path).ok()?.len() > 2 * 1024 * 1024 {
        return None;
    }
    let mut info: AndroidUpdateInfo = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    if !version_is_newer(&info.latest_version, &current_version()) {
        return None;
    }
    info.current_version = current_version();
    // 上次检查的缓存仅用于提示，下载命令仍重新拉取元数据和签名。
    info.mandatory = false;
    let identity = format!("android:{}:{}", info.latest_version, info.checksum);
    info.cached_download =
        cached_progress(&apk_part_path(&app, &info.latest_version).ok()?, &identity).map(
            |(downloaded, total)| CachedDownload {
                downloaded,
                total,
                done: false,
            },
        );
    Some(info)
}

/// 检查 GitHub Release 是否有新版本。
///
/// 仅在 Android 上有效；桌面端使用 `desktop_check_update`。
#[tauri::command]
pub async fn android_check_update(app: tauri::AppHandle) -> Result<AndroidUpdateInfo, String> {
    let current = current_version();
    let client = github_client()?;
    let release = fetch_release(Some(&app), &client, None, true).await?;

    // tag_name 格式为 "v2.6.1"，去掉 v 前缀
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();
    // P020: 版本单调性检查——拒绝低于/等于已安装版本的清单（第三方代理可重放旧
    // release 元数据压制升级或诱导降级提示）；非新版本时归一为当前版本。
    let latest = normalize_to_newer(latest, &current);

    // 查找 APK 资产；校验和走共享解析（下载 .sha256 + .minisig 并验签）
    let (apk_download_url, apk_size) = find_apk_asset(&release).unwrap_or_default();
    let apk_download_url = (!apk_download_url.is_empty()).then_some(apk_download_url);
    let (checksum, checksum_warning) = resolve_verified_checksum(&client, &release).await;

    // 缺失或无效校验和会在下载阶段硬性拒绝；检查阶段通过 warning 展示原因。

    // 检测强制更新标记：Release body 中是否包含 [MANDATORY]
    let mandatory = release
        .body
        .as_deref()
        .map(|body| body.contains("[MANDATORY]"))
        .unwrap_or(false);

    // P012: 强制更新 + 校验和不可信 → 检查阶段硬失败。
    // 强制更新不可跳过，若无法确认 APK 完整性（.sha256 缺失/签名缺失/验签失败），
    // 继续提示用户更新只会让下载命令（P002 已 fail-closed）必然失败，且用户可能
    // 在遮罩里反复点「立即更新」。此处直接阻断并给出明确原因——安全优先于可用性。
    if mandatory && checksum.is_none() {
        let reason = checksum_warning.unwrap_or_else(|| "校验和不可用".to_string());
        return Err(format!("强制更新已阻止：无法验证 APK 完整性（{reason}）"));
    }

    // 如果 Release body 包含 [MANDATORY]，在返回前移除该标记
    // 避免用户看到原始标记文本
    let clean_body = release
        .body
        .map(|body| body.replace("[MANDATORY]", "").trim().to_string())
        .filter(|s| !s.is_empty());

    // 检查本身只读：迟到/陈旧清单不得删除用户正在等待续传的新版本断点。
    let cached_download = checksum.as_ref().and_then(|checksum| {
        let path = apk_part_path(&app, &latest).ok()?;
        let identity = format!("android:{latest}:{checksum}");
        cached_progress(&path, &identity).map(|(downloaded, total)| CachedDownload {
            downloaded,
            total,
            done: false,
        })
    });

    let info = AndroidUpdateInfo {
        latest_version: latest,
        current_version: current,
        download_url: apk_download_url,
        checksum: checksum.unwrap_or_default(),
        checksum_warning,
        mandatory,
        release_notes: clean_body,
        published_at: release.published_at,
        apk_size,
        cached_download,
    };
    // 下载期间保留其版本提示；检查结果只读断点，不争用下载文件。
    if let Ok(_guard) = APK_CACHE_LOCK.try_lock() {
        if let Ok(directory) = app.path().app_cache_dir() {
            let _ = remember_update_info(&directory.join("update-info.json"), &info);
        }
    }
    Ok(info)
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
    // P020: 与 Android 路径一致的版本单调性归一（GitHub API 兜底同样防代理重放旧清单）。
    let latest = normalize_to_newer(latest, current);
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

    // 上次成功的是 Release 备用通道时，先给它短暂的独占窗口；慢/失败仍回到完整探测。
    if SourcePreferences::load(&app).prefers_release() {
        if let Ok(client) = github_client() {
            if let Ok(Ok(release)) = tokio::time::timeout(
                std::time::Duration::from_millis(900),
                fetch_github_release(&app, &client),
            )
            .await
            {
                return Ok(desktop_info_from_github_release(&current, &release));
            }
        }
    }
    match check_desktop_sources(&app).await {
        Ok(Some(update)) => {
            let body = update.body.unwrap_or_default();
            let mandatory = body.contains("[MANDATORY]");
            let notes = body.replace("[MANDATORY]", "").trim().to_string();
            Ok(DesktopUpdateInfo {
                latest_version: update.version,
                current_version: current,
                mandatory,
                release_notes: (!notes.is_empty()).then_some(notes),
                published_at: update
                    .raw_json
                    .get("pub_date")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
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
            tracing::warn!("[updater] updater 清单暂不可用，尝试 Release 备用通道: {plugin_err}");
            match github_client() {
                Ok(client) => match fetch_github_release(&app, &client).await {
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

/// 下载 APK：URL 和签名校验和均由 Rust 从指定版本清单取得，不信任 WebView 回传。
/// 与桌面端共用测速、低速换源和断点缓存；取消只停止请求，重试继续已有有效前缀。
#[tauri::command]
pub async fn android_download_apk(
    app: tauri::AppHandle,
    webview: Webview,
    version: String,
    operation_id: ResourceId,
    on_event: Channel<ApkDownloadProgress>,
) -> Result<(), String> {
    let operation = webview
        .resources_table()
        .get::<UpdateDownloadOperation>(operation_id)
        .map_err(|e| e.to_string())?;
    let version = version.strip_prefix('v').unwrap_or(&version).to_string();
    semver::Version::parse(&version).map_err(|_| "更新版本号无效")?;
    let dest = apk_cache_path(&app, &version)?;
    let part_path = apk_part_path(&app, &version)?;
    let _cache_guard = APK_CACHE_LOCK
        .try_lock()
        .map_err(|_| "已有 APK 下载正在进行中".to_string())?;
    let notify = |downloaded: u64, total: u64, status: &str| {
        app.state::<crate::update_plugin::UpdatePluginHandle<tauri::Wry>>()
            .notify_download(
                serde_json::json!({ "version": version, "downloaded": downloaded,
                "total": total, "status": status }),
            );
    };
    notify(0, 0, "preparing");
    let last_notification = std::sync::Mutex::new(std::time::Instant::now());
    let result = operation
        .run(async {
            let _ = cleanup_stale_apk_cache_locked(&app, &version);
            // URL 与 SHA-256 校验和均重新从 Rust 侧验签后的 Release 元数据取得。
            let client = github_client()?;
            let tag = if version.starts_with('v') {
                version.clone()
            } else {
                format!("v{version}")
            };
            let release = fetch_github_release_by_tag(&client, &tag).await?;
            let (download_url, _) =
                find_apk_asset(&release).ok_or_else(|| "Release 中未找到 APK 资产".to_string())?;
            let expected_checksum = resolve_verified_checksum(&client, &release)
                .await
                .0
                .ok_or_else(|| "APK 校验和不可信（签名缺失或验签失败），已拒绝下载".to_string())?;
            let sources = UpdateSources::load()?;
            let candidates =
                artifact_candidates(&download_url, &version, &sources, &proxy_prefixes())?;
            let identity = format!("android:{version}:{expected_checksum}");
            download_file(
                &candidates,
                &part_path,
                &identity,
                &|progress: TransferProgress| {
                    if let Ok(mut last) = last_notification.lock() {
                        if last.elapsed() >= std::time::Duration::from_secs(1) {
                            notify(progress.downloaded, progress.total, "downloading");
                            *last = std::time::Instant::now();
                        }
                    }
                    let percent = progress
                        .downloaded
                        .saturating_mul(100)
                        .checked_div(progress.total)
                        .unwrap_or(0);
                    let _ = on_event.send(ApkDownloadProgress {
                        progress: percent.min(99) as u32,
                        downloaded: progress.downloaded,
                        total: progress.total,
                        done: false,
                        error: None,
                        source: Some(progress.source),
                        bytes_per_second: Some(progress.bytes_per_second),
                        phase: Some(progress.phase.into()),
                    });
                },
            )
            .await?;
            let size = verify_and_finalize(&part_path, &dest, &expected_checksum, &operation)?;
            remove_download_cache(&part_path);
            Ok(size)
        })
        .await;
    let final_size = match result {
        Ok(size) => {
            notify(size, size, "done");
            size
        }
        Err(error) => {
            notify(
                0,
                0,
                if error == DOWNLOAD_CANCELLED {
                    "cancelled"
                } else {
                    "failed"
                },
            );
            return Err(error);
        }
    };
    let _ = on_event.send(ApkDownloadProgress {
        progress: 100,
        downloaded: final_size,
        total: final_size,
        done: true,
        error: None,
        source: None,
        bytes_per_second: None,
        phase: None,
    });
    Ok(())
}

/// SHA-256 校验 + 重命名落盘（下载主体后统一调用）。
///
/// 校验失败删除 part 文件并返回错误；成功则删除旧目标文件后原子重命名。
fn verify_and_finalize(
    part_path: &std::path::Path,
    dest: &std::path::Path,
    expected_checksum: &str,
    operation: &UpdateDownloadOperation,
) -> Result<u64, String> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(part_path).map_err(|e| format!("打开文件计算校验和: {e}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 8192];
    let mut size = 0u64;
    loop {
        operation.check_cancelled()?;
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取文件校验: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    let actual = format!("{:x}", hasher.finalize());
    if !expected_checksum.eq_ignore_ascii_case(&actual) {
        remove_download_cache(part_path);
        return Err(format!(
            "CHECKSUM_MISMATCH: expected {}, got {}",
            expected_checksum, actual
        ));
    }
    operation.check_cancelled()?;
    // 先删除可能存在的旧最终文件
    let _ = std::fs::remove_file(dest);
    std::fs::rename(part_path, dest).map_err(|e| format!("重命名 APK 文件失败: {e}"))?;
    Ok(size)
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

    #[test]
    fn stale_version_metadata_cannot_hide_cached_newer_update_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update-info.json");
        let mut info = AndroidUpdateInfo {
            latest_version: "2.13.3".into(),
            current_version: "2.13.1".into(),
            download_url: None,
            checksum: "verified".into(),
            checksum_warning: None,
            mandatory: false,
            release_notes: None,
            published_at: None,
            apk_size: Some(108),
            cached_download: None,
        };
        remember_update_info(&path, &info).unwrap();
        for older in ["2.13.2", "2.13.1"] {
            info.latest_version = older.into();
            remember_update_info(&path, &info).unwrap();
            let saved: AndroidUpdateInfo =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(saved.latest_version, "2.13.3");
        }
        info.latest_version = "2.13.4".into();
        remember_update_info(&path, &info).unwrap();
        let saved: AndroidUpdateInfo =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(saved.latest_version, "2.13.4");
    }

    #[tokio::test]
    async fn cancelled_before_download_never_polls_request() {
        let operation = UpdateDownloadOperation::new();
        operation.cancel();
        let polled = AtomicBool::new(false);
        let result = operation
            .run(async {
                polled.store(true, Ordering::SeqCst);
                Ok(())
            })
            .await;
        assert_eq!(result.unwrap_err(), DOWNLOAD_CANCELLED);
        assert!(!polled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn closed_download_resource_cancels_and_cannot_be_reused() {
        let operation = Arc::new(UpdateDownloadOperation::new());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (result, ()) = tokio::join!(
            operation.run(async {
                let _ = started.send(());
                std::future::pending::<Result<(), String>>().await
            }),
            async {
                ready.await.unwrap();
                Resource::close(operation.clone());
            }
        );
        assert_eq!(result.unwrap_err(), DOWNLOAD_CANCELLED);
        assert!(operation
            .run(async { Ok(()) })
            .await
            .unwrap_err()
            .contains("已启动"));
    }

    #[test]
    fn cancellation_during_verification_does_not_replace_verified_apk() {
        let dir = tempfile::tempdir().unwrap();
        let part = dir.path().join("update_test.part");
        let apk = dir.path().join("update_test.apk");
        std::fs::write(&part, b"new package").unwrap();
        std::fs::write(&apk, b"verified previous package").unwrap();
        let operation = UpdateDownloadOperation::new();
        operation.cancel();
        let checksum = format!("{:x}", sha2::Sha256::digest(b"new package"));
        assert_eq!(
            verify_and_finalize(&part, &apk, &checksum, &operation).unwrap_err(),
            DOWNLOAD_CANCELLED
        );
        remove_download_cache(&part);
        assert_eq!(std::fs::read(&apk).unwrap(), b"verified previous package");
        assert!(!part.exists());
    }

    // T004 防抖：SOLOSOUL_PROXY_PREFIXES 是进程级环境变量，涉及它的测试必须
    // 串行执行（Rust 测试默认多线程并发，set_var/remove_var 会相互干扰）。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn desktop_transport_must_verify_original_signature_and_pinned_key() {
        use base64::Engine;
        let real_sig = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUZW1YUGRnVGdqUElXektwTklqanR0NFhta25GN3FhSHI3UFh3VitLTURIU0hMeUxSbGVKc1krclNSSGZOS1FCK1FieCtZckJlckNXaHpJQ3owZlpaR051NktxN2kwWmcwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg2MTE0MTcxCWZpbGU6cDAwM190ZXN0LnNoYTI1NgpCdG5FUWQxUkVrdlVhL2VKUkhST29XU2lPVWJBQlBCOU9UbXordFpwclkyZGN6VGcyKy8ycDVxRHBJc3pkRFVXRHNwbzdjT012cTk3UXR4RmdPL1FDQT09Cg==";
        let bytes = b"deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678\n";
        let test_key = base64::engine::general_purpose::STANDARD.encode(format!(
            "untrusted comment: regression fixture\n{APK_CHECKSUM_PUBKEY}\n"
        ));
        assert!(verify_desktop_signature(bytes, real_sig, &test_key).is_ok());
        assert!(verify_desktop_signature(b"tampered package", real_sig, &test_key).is_err());
        assert!(verify_desktop_signature(bytes, "", &test_key).is_err());
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let production_key = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        // APK 校验和公钥与桌面 updater 公钥相互隔离，不可因复用下载器而混用。
        assert!(verify_desktop_signature(bytes, real_sig, production_key).is_err());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn transfer_event_serializes_absolute_progress_for_frontend() {
        let event = DesktopDownloadEvent::Transfer {
            downloaded: 512,
            total: 1024,
            source: "cdn.example".into(),
            bytes_per_second: 256,
            phase: "switching".into(),
        };
        let json = serde_json::to_value(event).unwrap();
        assert_eq!(json["event"], "Transfer");
        assert_eq!(json["data"]["downloaded"], 512);
        assert_eq!(json["data"]["bytesPerSecond"], 256);
        assert_eq!(json["data"]["phase"], "switching");
    }

    #[test]
    fn mismatched_apk_checksum_removes_untrusted_partial_but_preserves_installer() {
        let dir = tempfile::tempdir().unwrap();
        let part = dir.path().join("update_test.part");
        let dest = dir.path().join("update_test.apk");
        std::fs::write(&part, b"corrupt").unwrap();
        std::fs::write(dir.path().join("update_test.part.json"), b"{}").unwrap();
        std::fs::write(&dest, b"previous verified package").unwrap();
        let expected = format!("{:x}", sha2::Sha256::digest(b"correct"));
        assert!(
            verify_and_finalize(&part, &dest, &expected, &UpdateDownloadOperation::new()).is_err()
        );
        assert!(!part.exists());
        assert!(!dir.path().join("update_test.part.json").exists());
        assert_eq!(std::fs::read(dest).unwrap(), b"previous verified package");
    }

    /// N006 防回归：`find_apk_asset` 只匹配 `.apk` 扩展名，绝不命中校验和/
    /// 签名资产（`contains("universal-release")` 曾误命中它们）。
    #[test]
    fn test_find_apk_asset_only_matches_apk_extension() {
        let release = GitHubRelease {
            tag_name: "v2.9.2".into(),
            body: None,
            published_at: None,
            assets: vec![
                GitHubAsset {
                    name: "solo-soul-universal-release.apk.sha256".into(),
                    browser_download_url: "https://example/checksum".into(),
                    size: Some(64),
                },
                GitHubAsset {
                    name: "solo-soul-universal-release.apk.sha256.minisig".into(),
                    browser_download_url: "https://example/sig".into(),
                    size: Some(88),
                },
                GitHubAsset {
                    name: "solo-soul-universal-release.apk".into(),
                    browser_download_url: "https://example/apk".into(),
                    size: Some(52_428_800),
                },
            ],
        };
        let (url, size) = find_apk_asset(&release).expect("应命中真实 APK 资产");
        assert_eq!(url, "https://example/apk");
        assert_eq!(size, Some(52_428_800));
    }

    /// N006 防回归：无 `.apk` 资产时返回 None（旧逻辑 `contains("universal-release")`
    /// 会误把校验和资产当成 APK，导致下载到非 APK 文件）。
    #[test]
    fn test_find_apk_asset_returns_none_without_apk() {
        let release = GitHubRelease {
            tag_name: "v2.9.2".into(),
            body: None,
            published_at: None,
            assets: vec![
                GitHubAsset {
                    name: "solo-soul-universal-release.apk.sha256".into(),
                    browser_download_url: "https://example/checksum".into(),
                    size: Some(64),
                },
                GitHubAsset {
                    name: "solo-soul-universal-release.apk.sha256.minisig".into(),
                    browser_download_url: "https://example/sig".into(),
                    size: Some(88),
                },
            ],
        };
        assert!(
            find_apk_asset(&release).is_none(),
            "仅校验和/签名资产时不应命中 APK"
        );
    }

    #[test]
    fn android_metadata_requires_matching_integrity_assets_and_prefers_signed_apk() {
        let mut release = GitHubRelease {
            tag_name: "v2.13.0".into(),
            body: None,
            published_at: None,
            assets: vec![GitHubAsset {
                name: "SoloSoul_Android.apk".into(),
                browser_download_url: "https://cdn.example/alias.apk".into(),
                size: Some(1),
            }],
        };
        assert!(!has_complete_apk_assets(&release));
        for suffix in [".apk", ".apk.sha256", ".apk.sha256.minisig"] {
            release.assets.push(GitHubAsset {
                name: format!("SoloSoul_2.13.0{suffix}"),
                browser_download_url: format!("https://cdn.example/SoloSoul_2.13.0{suffix}"),
                size: Some(1),
            });
        }
        assert!(has_complete_apk_assets(&release));
        assert_eq!(
            find_apk_asset(&release).unwrap().0,
            "https://cdn.example/SoloSoul_2.13.0.apk"
        );
        release.assets.last_mut().unwrap().browser_download_url =
            "http://insecure.example/signature".into();
        assert!(!has_complete_apk_assets(&release));
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

    /// 下载候选列表：直连优先，随后各代理前缀拼接（默认代理列表）。
    #[test]
    fn test_download_candidates_direct_first_then_proxies() {
        let _guard = ENV_LOCK.lock().unwrap();
        let url = "https://github.com/Gczmy/SoloSoul/releases/download/v2.9.2/app.apk";
        let candidates = download_candidates(url);
        // 直连必须是第一个候选
        assert_eq!(candidates[0], url);
        // 代理数 = 前缀数（默认列表）
        assert_eq!(candidates.len(), 1 + PROXY_PREFIXES.len());
        // 每个代理前缀按规则拼接
        for (i, prefix) in PROXY_PREFIXES.iter().enumerate() {
            assert_eq!(candidates[i + 1], format!("{prefix}{url}"));
        }
    }

    /// T004/U001: 代理列表可被环境变量覆盖；未设置→默认；显式置空/仅空白→禁用全部代理。
    #[test]
    fn test_proxy_prefixes_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        let default = proxy_prefixes();
        assert_eq!(default.len(), PROXY_PREFIXES.len());

        // 覆盖为自建代理
        std::env::set_var("SOLOSOUL_PROXY_PREFIXES", "https://self.example.com/");
        let overridden = proxy_prefixes();
        std::env::remove_var("SOLOSOUL_PROXY_PREFIXES");
        assert_eq!(overridden, vec!["https://self.example.com/".to_string()]);

        // 逗号分隔 + 去空白
        std::env::set_var(
            "SOLOSOUL_PROXY_PREFIXES",
            " https://a.example.com/ , https://b.example.com/ ",
        );
        let multi = proxy_prefixes();
        std::env::remove_var("SOLOSOUL_PROXY_PREFIXES");
        assert_eq!(
            multi,
            vec![
                "https://a.example.com/".to_string(),
                "https://b.example.com/".to_string()
            ]
        );

        // 显式置空（禁用代理意图）→ 空列表，仅走直连（U001：不再回退默认）
        std::env::set_var("SOLOSOUL_PROXY_PREFIXES", "");
        let empty = proxy_prefixes();
        std::env::remove_var("SOLOSOUL_PROXY_PREFIXES");
        assert!(empty.is_empty());

        // 仅空白同样视为禁用
        std::env::set_var("SOLOSOUL_PROXY_PREFIXES", "  ,  ");
        let blank = proxy_prefixes();
        std::env::remove_var("SOLOSOUL_PROXY_PREFIXES");
        assert!(blank.is_empty());
    }
}
