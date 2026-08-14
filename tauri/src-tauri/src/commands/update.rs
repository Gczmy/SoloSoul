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

/// GitHub 下载加速代理前缀（国内直连 GitHub Release 资产/API 不稳定时按序回退）。
///
/// 拼接规则：`<prefix> + <原始 github.com / api.github.com URL>`，例如
/// `https://ghfast.top/https://github.com/Gczmy/SoloSoul/releases/download/v2.9.2/...`。
///
/// 安全说明：下载内容无论来自直连还是代理，都会经过 minisign/SHA-256 强制校验
/// （桌面端 updater 验签、安卓端 P002 校验），因此走代理通道不引入供应链风险。
///
/// 隐私披露（T004）：gh-proxy 类代理是 TLS 终止代理——连接在代理方解密后转发，
/// 因此**用户 IP、使用 SoloSoul 的事实、目标版本号、GitHub API 响应内容都会暴露
/// 给第三方代理服务商**。直连优先的设计使国内直连可达时不会走代理，但直连受限时
/// 必然经过代理，属固有权衡，无法消除，仅在此明示。若用户对此敏感，可通过环境变量
/// `SOLOSOUL_PROXY_PREFIXES`（逗号分隔）覆盖为自建可信代理或留空禁用代理。
///
/// 可用性披露（T004）：① `api.github.com` 元数据请求走代理时，代理可返回陈旧/
/// 篡改的 Release JSON 软性压制升级（内容完整性不受影响——校验和与签名在 Rust 侧
/// 重新验签，属可用性面）；② 代理也可重放旧版 latest-mirror JSON 压制升级（updater
/// 只升不降，无降级风险）。
///
/// 维护注意：这些第三方代理服务存活期不稳定，失效条目应在此处替换为可用条目；
/// 客户端按「直连优先、逐代理回退」设计，单个代理失效只会多一次短超时，不阻断下载。
const PROXY_PREFIXES: &[&str] = &[
    "https://ghfast.top/",
    "https://ghproxy.net/",
    "https://gh-proxy.com/",
    "https://ghps.cc/",
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
            .map(str::to_string)
            .collect(),
        Err(_) => PROXY_PREFIXES.iter().map(|p| (*p).to_string()).collect(),
    }
}

/// 多线程分段下载的并发段数（移动端网络下 4 段在提速与稳定性间均衡）。
const PARALLEL_SEGMENTS: usize = 4;
/// 启用多线程分段的最小文件体积：低于该值单流下载即可，避免分段开销。
const PARALLEL_MIN_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20MB
/// 每个分段的最小体积：文件略超阈值时不生成过碎的分段。
const PARALLEL_MIN_SEGMENT_SIZE: u64 = 5 * 1024 * 1024; // 5MB
/// 分段级主通道重试次数：并行段在探测主通道（通常为直连）中途失败时不立即切中国
/// 代理，先重试同通道这么多次——避免单次网络抖动/单连接劣化把本可直连完成的段
/// 绕道到慢速代理（修复①）。单段总尝试次数 = 1 + DIRECT_SEGMENT_RETRIES。
const DIRECT_SEGMENT_RETRIES: usize = 2;

/// 直连健康测速样本大小：探测直连通道时下载该字节数用于吞吐判定。
/// 快路径（海外快直连）样本在数十毫秒内读完，开销可忽略；慢路径受探测客户端
/// 总超时（20s）约束，最坏等待有限。
const DIRECT_SAMPLE_BYTES: u64 = 1024 * 1024; // 1MB
/// 直连「健康」吞吐阈值：样本测速累计吞吐 ≥ 该值视为直连正常 → 走旧版单流直连。
/// 海外（如英国）直连通常 ≥5MB/s，国内受限直连通常 <1MB/s，阈值两侧分离清晰；
/// 恰在阈值（2MB/s）时 111.7MB 单流约 1 分钟，仍属可接受，不触发并行。
const DIRECT_MIN_SPEED_BYTES_PER_SEC: u64 = 2 * 1024 * 1024; // 2MB/s

/// 下载策略（由探测阶段判定，修复④核心）。
///
/// - [`DirectSingleStream`](DownloadStrategy::DirectSingleStream)：直连健康（206 +
///   样本测速达标）→ 走旧版单流直连路径（直连优先，代理仅作段级兜底）。
///   海外直连本就很快时不再启用并行分段与代理，消除多连接分段在移动网络上的
///   最慢段拖累与「段失败→从 0 重下」的回退放大（英国更新变慢的根因）。
/// - [`Accelerated`](DownloadStrategy::Accelerated)：直连失败或过慢（国内受限场景）→
///   启用并行分段 + 代理回退；`range_channel` 为首个支持 Range 的候选索引
///   （None = 无通道支持 Range，回退单流 + 代理）。
#[derive(Debug, Clone, Copy)]
enum DownloadStrategy {
    DirectSingleStream,
    Accelerated {
        total: u64,
        range_channel: Option<usize>,
    },
}

/// 样本测速达标判定（纯函数，便于单测）：累计吞吐 ≥ 阈值即为直连健康。
fn sample_speed_healthy(read_bytes: u64, elapsed_secs: f64) -> bool {
    elapsed_secs > 0.0 && read_bytes as f64 / elapsed_secs >= DIRECT_MIN_SPEED_BYTES_PER_SEC as f64
}

/// 直连通道 206 响应体测速：读取样本字节并按累计吞吐判定健康与否。
///
/// 快路径优化：逐块累计，一旦累计吞吐已达标（且样本窗口 ≥0.15s 避免慢启动首个
/// 分块虚高）立即返回健康，不必等满整个样本——海外快直连的探测开销 ≈ 一两个分块。
async fn direct_sample_healthy(resp: &mut reqwest::Response) -> bool {
    let start = std::time::Instant::now();
    let mut read = 0u64;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                read += chunk.len() as u64;
                if read >= DIRECT_SAMPLE_BYTES {
                    break;
                }
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed >= 0.15 && sample_speed_healthy(read, elapsed) {
                    return true;
                }
            }
            Ok(None) => break,
            Err(_) => break, // 读取失败视为过慢（交由并行/代理路径处理）
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    sample_speed_healthy(read, elapsed)
}

/// 为给定 GitHub URL 生成下载候选列表：直连优先，随后各代理前缀。
fn download_candidates(url: &str) -> Vec<String> {
    let mut candidates = vec![url.to_string()];
    candidates.extend(proxy_prefixes().iter().map(|p| format!("{p}{url}")));
    candidates
}

/// 逐个尝试候选 URL，返回第一个 2xx 响应。
///
/// 用于小体积请求（API 元数据、校验和、签名文件）——候选依次快速失败，
/// 全部失败时返回聚合错误。
async fn request_first_ok(
    client: &reqwest::Client,
    candidates: &[String],
) -> Result<reqwest::Response, String> {
    let mut last_err = String::new();
    for url in candidates {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(resp),
            Ok(resp) => last_err = format!("HTTP {}", resp.status()),
            Err(e) => last_err = format!("{e}"),
        }
    }
    Err(format!("所有下载通道均不可用: {last_err}"))
}

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
    /// P012: 校验和不可用原因（签名缺失/验签失败/资产缺失），供前端展示可感知警告。
    pub checksum_warning: Option<String>,
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

/// U003: 进程内「APK 下载进行中」标志——Tauri commands 并发执行，用户在分段下载
/// 进行中触发 `android_check_update`（AboutPage/横幅）时，若 cleanup 照常执行会删除
/// `download_range_to_file` **正在写入**的 `.part.seg{i}`。下载进行中 cleanup 直接跳过
/// （后果可自愈：合并 open 失败 → 回退单流 → SHA-256 终检兜底，但避免浪费带宽/进度归零）。
static APK_DOWNLOAD_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// V003: Drop guard（scopeguard 式，无需外部依赖）——无论正常返回、`?` 提前返回还是
/// panic unwind，离开作用域时都恢复 `APK_DOWNLOAD_ACTIVE`，杜绝标志永久置位导致
/// cleanup 此后整体失效。
struct ApkDownloadActiveGuard;

impl Drop for ApkDownloadActiveGuard {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering;
        APK_DOWNLOAD_ACTIVE.store(false, Ordering::Relaxed);
    }
}

/// 清理非当前版本的 APK 缓存文件，避免旧版本安装包占用空间并被误用。
///
/// 下载进行中（`APK_DOWNLOAD_ACTIVE` 为 true）时**整体跳过**，保护正在写入的
/// `.part.seg{i}`；调用点均为下载开始前或检查更新时，下次下载前仍会正常清理。
fn cleanup_stale_apk_cache(app: &tauri::AppHandle, current_version: &str) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    if APK_DOWNLOAD_ACTIVE.load(Ordering::Relaxed) {
        return Ok(());
    }
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
        // 仅处理 update_<version>.apk / .part / .part.seg{i} 格式
        let is_update_file = (name_str.starts_with("update_") && name_str.ends_with(".apk"))
            || (name_str.starts_with("update_") && name_str.ends_with(".part"))
            || (name_str.starts_with("update_") && name_str.contains(".part.seg"))
            // merge_seg_files 的合并临时文件（进程中断残留即孤儿）
            || (name_str.starts_with("update_") && name_str.ends_with(".part.merge"));
        if !is_update_file {
            continue;
        }
        if let Some(stripped) = name_str.strip_prefix("update_") {
            let file_version = stripped
                .strip_suffix(".apk")
                .or_else(|| stripped.strip_suffix(".part"))
                .or_else(|| stripped.split(".part.seg").next())
                .unwrap_or(stripped);
            // ① 旧版本文件一律删除；② 当前版本的 `.part.seg{i}` 孤儿分段文件与
            // `.part.merge` 合并临时文件也删除（T003）：进程中断（kill/崩溃）残留的
            // seg/merge 文件纯属垃圾累积；本函数仅在下载开始前调用（检查更新时/下载前），
            // 无并发写入，删除安全。当前版本的 `.part`（单流断点）与 `.apk` 保留。
            let is_orphan_seg = name_str.contains(".part.seg");
            if file_version != current_part || is_orphan_seg {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

// ── Command: 检查更新 ──────────────────────────────────────────

/// 创建 GitHub API 请求客户端（带 UA、连接超时与总超时）。
///
/// 用于 API 元数据/校验和/签名等小体积请求。候选回退（直连 + 多代理）可能依次
/// 尝试多个 URL，因此连接超时设 10s、总超时 60s（避免单个卡死拖垮整条链路）。
fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
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
///
/// 直连失败时自动回退到代理前缀（`api.github.com` 在部分地区同样受限）。
async fn fetch_github_release_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<GitHubRelease, String> {
    let candidates = download_candidates(url);
    let resp = request_first_ok(client, &candidates).await?;

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
///
/// N006: 谓词收紧为仅 `ends_with(".apk")`——旧逻辑含 `contains("universal-release")`，
/// 会误命中 `xx-universal-release.apk.sha256(.minisig)` 等校验和/签名资产（若 GitHub
/// 资产排序不利 → 下载到非 APK 文件或 fail-closed 误拒）。`universal-release` 只是
/// 命名惯例，不能替代扩展名判断。
fn find_apk_asset(release: &GitHubRelease) -> Option<(String, Option<i64>)> {
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".apk"))
        .map(|a| (a.browser_download_url.clone(), a.size))
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
    // P012: 资产匹配收紧——仅以 `.sha256` 结尾且排除 `.minisig` 签名文件
    // （旧逻辑 `contains("sha256")` 会误匹配 `xx.sha256.minisig`）。
    let Some(checksum_asset) = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".sha256") && !a.name.ends_with(".minisig"))
    else {
        return (
            None,
            Some("发布未提供 .sha256 校验和资产，无法确认 APK 完整性".to_string()),
        );
    };
    // N006: 签名资产谓词收紧——统一以 `.sha256.minisig` 结尾（`.apk.sha256.minisig`
    // 本身已以此结尾），去掉宽松的 `contains`（会误命中 `foo.minisig` 等无关资产）。
    let sig_asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".sha256.minisig"));

    // 下载校验和文件（约 64 字节）与签名文件（直连失败回退代理通道）
    let body = match request_first_ok(
        client,
        &download_candidates(&checksum_asset.browser_download_url),
    )
    .await
    {
        Ok(resp) => resp.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    let sig_text = match sig_asset {
        Some(asset) => {
            match request_first_ok(client, &download_candidates(&asset.browser_download_url)).await
            {
                Ok(resp) => resp.text().await.unwrap_or_default(),
                Err(_) => String::new(),
            }
        }
        None => String::new(),
    };

    let sig_ok =
        !sig_text.is_empty() && verify_checksum_signature(body.as_bytes(), &sig_text).is_ok();
    if !sig_ok {
        tracing::warn!(
            "[updater] APK checksum minisign signature missing or invalid — checksum rejected"
        );
        return (
            None,
            Some("校验和签名缺失或验签失败，无法确认 APK 完整性".to_string()),
        );
    }

    // 验签通过后，才提取 64 位 hex 作为校验和。
    let checksum = body
        .split_whitespace()
        .next()
        .filter(|token| token.len() == 64)
        .map(|s| s.to_string());
    let warning = checksum
        .is_none()
        .then(|| "校验和文件格式异常，无法解析 64 位 hex".to_string());
    (checksum, warning)
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
    let (checksum, checksum_warning) = resolve_verified_checksum(&client, &release).await;

    // 如果找到 checksum 但解析失败（如文件格式异常），也返回空字符串，
    // 客户端仍可正常下载，只是不进行校验——原因经 checksum_warning 展示给用户。

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
        checksum_warning,
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
    // U002: 显式设置 15s 请求超时（与前端 check() 的 UPDATE_REQUEST_TIMEOUT_MS 一致）——
    // 插件默认无超时，直连黑洞（hang 而非 RST）时 endpoint 回退不触发，AboutPage 会
    // 永久卡在 checking 态；超时后插件自动尝试下一个 endpoint。
    // 注：timeout 在 UpdaterBuilder 上（updater() 是已构建的 Updater，无此方法），
    // 故经 updater_builder() 构建。
    // V002: build 失败不提前返回——与旧 `app.updater()` 行为一致，将错误放入
    // updater_result 走下方 GitHub Release API 兜底分支，保证 AboutPage 仍能给出版本信息。
    let updater_result = match app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(u) => u.check().await.map_err(|e| format!("检查更新失败: {e}")),
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
/// 下载策略（直连健康优先，国内 GitHub 直连受限场景优化，修复④）：
/// - **直连健康 → 旧版单流直连**：探测阶段直连返回 206 且样本测速达标（≥2MB/s）时
///   直接单流直连——海外（如英国）直连本就很快，不再启用并行分段/代理，避免多连接
///   分段在移动网络上的最慢段拖累与失败回退放大（英国更新变慢的根因）；
/// - **直连失败/过慢 → 多通道回退 + 并行分段**：直连受限（国内场景）时，候选 URL =
///   直连 + 各代理前缀自动回退；服务器支持 HTTP Range 且文件足够大（>20MB）时切成
///   至多 4 段并发拉取，不支持 Range 或小文件退回单流；
/// - **单流断点续传**：已存在 `.part` 时带 `Range` 头从断点继续。
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
        .0
        .ok_or_else(|| "APK 校验和不可信（签名缺失或验签失败），已拒绝下载".to_string())?;

    // 下载候选通道：直连优先 + 代理回退（元数据/校验和/APK 全链路同策略）
    let candidates = download_candidates(&download_url);

    // 探测下载通道并判定策略：直连健康→单流直连；直连失败/过慢→并行+代理
    let strategy = probe_download(&candidates).await?; // U003: 标记下载进行中（cleanup 据此跳过，避免并发删除正在写入的 .seg 文件）。
                                                       // 探测阶段已完成（不写 seg），从下载主体开始标记。
    use std::sync::atomic::Ordering;
    APK_DOWNLOAD_ACTIVE.store(true, Ordering::Relaxed);
    let _active_guard = ApkDownloadActiveGuard; // V003: Drop 时恢复标志，panic 路径也不泄漏
    let result = download_apk_to_part(
        &app,
        &candidates,
        strategy,
        &part_path,
        &dest,
        &expected_checksum,
    )
    .await;
    let final_size = result?;

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

/// 构造并行下载的段级候选顺序（修复①）。
///
/// 探测主通道（`idx`，通常为直连）排最前并重复 `1 + DIRECT_SEGMENT_RETRIES` 次：
/// 段在主通道中途失败时先重试同通道（网络抖动/单连接劣化多为瞬时，重试即恢复），
/// 全部重试仍失败才轮到其余候选（代理），避免直连可用场景的段被绕道到慢速代理。
/// 其余候选保持原有相对顺序（`idx+1..` 优先于 `..idx`）。
fn parallel_candidate_order(candidates: &[String], idx: usize) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::with_capacity(candidates.len() + DIRECT_SEGMENT_RETRIES);
    for _ in 0..=DIRECT_SEGMENT_RETRIES {
        ordered.push(candidates[idx].clone());
    }
    ordered.extend(candidates[idx + 1..].iter().cloned());
    ordered.extend(candidates[..idx].iter().cloned());
    ordered
}

/// 下载主体：按探测策略执行（直连健康→单流直连；需加速→并行分段或单流），
/// 随后 SHA-256 校验并返回最终大小。
///
/// 抽为独立 async 函数使 `android_download_apk` 的 U003 活动标志在错误路径也能恢复
/// （调用方 await 后无论 Ok/Err 都 store(false)）。
async fn download_apk_to_part(
    app: &tauri::AppHandle,
    candidates: &[String],
    strategy: DownloadStrategy,
    part_path: &std::path::Path,
    dest: &std::path::Path,
    expected_checksum: &str,
) -> Result<u64, String> {
    match strategy {
        // 修复④：直连健康 → 旧版单流直连（候选仍直连优先，代理仅作段级兜底）
        DownloadStrategy::DirectSingleStream => {
            download_apk_single_stream(app, candidates, part_path).await?;
        }
        // 直连失败/过慢（国内受限场景）→ 启用并行分段 + 代理回退
        DownloadStrategy::Accelerated {
            total,
            range_channel,
        } => {
            if let Some(idx) = range_channel {
                if total >= PARALLEL_MIN_FILE_SIZE {
                    // 并行时优先复用探测确认支持 Range 的通道，且主通道（通常直连）
                    // 重复 DIRECT_SEGMENT_RETRIES 次排最前（修复①）：段中途失败先
                    // 重试同通道而非立即切中国代理。
                    let ordered = parallel_candidate_order(candidates, idx);
                    // 并行失败（代理限流/中途断连等）时回退单流重试一次：
                    // parallel 已把完成段合并为 part_path 前缀（修复②），单流可断点
                    // 续传；单流再失败则错误传播，由调用方处理。
                    if let Err(e) = download_apk_parallel(app, &ordered, total, part_path).await {
                        tracing::warn!("[updater] 分段并行下载失败，回退单流重试: {e}");
                        download_apk_single_stream(app, candidates, part_path).await?;
                    }
                } else {
                    download_apk_single_stream(app, candidates, part_path).await?;
                }
            } else {
                download_apk_single_stream(app, candidates, part_path).await?;
            }
        }
    }

    // SHA-256 校验（P002：校验和由 Rust 侧验签获取，必非空，强制校验）+ 落盘
    let final_size = verify_and_finalize(part_path, dest, expected_checksum)?;
    Ok(final_size)
}

/// 创建 APK 下载专用客户端：连接超时 15s + 总超时 120s。
///
/// 旧实现曾无总超时，服务器中途停流时 `chunk().await` 会永久挂起（reqwest 无读超时）；
/// 120s 总超时保证任何停流/死链都会失败并被上层「候选回退 / 分段失败」路径处理。
fn download_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

/// 探测下载候选通道并判定下载策略（修复④核心）。
///
/// - **直连（候选 0）返回 206**：下载 `DIRECT_SAMPLE_BYTES` 样本测速——
///   - 吞吐 ≥ 阈值 → 直连健康 → [`DownloadStrategy::DirectSingleStream`]
///     （海外快直连不再并行/绕代理）；
///   - 吞吐不足 → 直连过慢（国内受限直连）→ [`DownloadStrategy::Accelerated`]
///     （并行 over 直连，代理兜底）。
/// - **直连失败/非 206** → 继续探测代理：首个 206 的代理作为并行通道；
/// - **所有候选均不支持 Range（200）** → 记录首个可连通候选大小，回退单流；
/// - **全部候选失败** → 聚合错误。
///
/// 探测用独立 client：连接/总超时均短（样本最大 1MB），避免卡住整条链路。
async fn probe_download(candidates: &[String]) -> Result<DownloadStrategy, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let mut last_err = String::new();
    let mut first_ok_size: Option<u64> = None;
    for (i, url) in candidates.iter().enumerate() {
        let resp = match client
            .get(url)
            .header("Range", format!("bytes=0-{}", DIRECT_SAMPLE_BYTES - 1))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("通道 {i} 失败: {e}");
                continue;
            }
        };
        if resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let Some(total) = parse_content_range_total(&resp) else {
                last_err = format!("通道 {i} Content-Range 解析失败");
                continue;
            };
            if i == 0 {
                // 直连通道：样本测速判定健康与否
                let mut resp = resp;
                if direct_sample_healthy(&mut resp).await {
                    tracing::info!(
                        "[updater] 直连健康（样本测速 ≥ {}B/s），走单流直连",
                        DIRECT_MIN_SPEED_BYTES_PER_SEC
                    );
                    return Ok(DownloadStrategy::DirectSingleStream);
                }
                tracing::warn!(
                    "[updater] 直连过慢（样本测速 < {}B/s），启用并行加速（总大小 {total} 字节）",
                    DIRECT_MIN_SPEED_BYTES_PER_SEC
                );
                return Ok(DownloadStrategy::Accelerated {
                    total,
                    range_channel: Some(0),
                });
            }
            // 代理通道支持 Range：作为并行下载通道（段级失败回退其余候选）
            tracing::info!("[updater] 探测通道 {i}（代理）支持 Range，文件总大小 {total} 字节");
            return Ok(DownloadStrategy::Accelerated {
                total,
                range_channel: Some(i),
            });
        } else if resp.status().is_success() {
            // 该通道忽略 Range 返回 200（完整 body），记录大小作为单流兜底
            if first_ok_size.is_none() {
                first_ok_size = resp.content_length();
            }
            last_err = format!("通道 {i} 不支持 Range（HTTP 200）");
        } else {
            last_err = format!("通道 {i} HTTP {}", resp.status());
        }
    }
    match first_ok_size {
        Some(len) => Ok(DownloadStrategy::Accelerated {
            total: len,
            range_channel: None,
        }),
        None => Err(format!("所有下载通道探测失败: {last_err}")),
    }
}

/// 计算分段下载的 `[start, end]` 闭区间列表（HTTP Range 语义）。
///
/// 段数在 `PARALLEL_SEGMENTS` 与「每段不小于 `PARALLEL_MIN_SEGMENT_SIZE`」间取小，
/// 避免文件略超阈值时生成过碎分段；余数分配到前若干段。
fn compute_segments(total: u64) -> Vec<(u64, u64)> {
    if total == 0 {
        return vec![];
    }
    let max_segments = total.div_ceil(PARALLEL_MIN_SEGMENT_SIZE);
    let count = (PARALLEL_SEGMENTS as u64).min(max_segments).max(1) as usize;
    let base = total / count as u64;
    let rem = total % count as u64;
    let mut segments = Vec::with_capacity(count);
    let mut start = 0u64;
    for i in 0..count {
        let len = base + if (i as u64) < rem { 1 } else { 0 };
        segments.push((start, start + len - 1));
        start += len;
    }
    segments
}

/// 按序合并 seg 文件到 `part_path` 并删除已合并的分段文件，返回合并字节数。
///
/// 先合并到同目录临时文件（`part_path` + `.merge`），成功后原子 `rename` 覆盖
/// `part_path`：任何失败都不触碰既有 `part_path`（T002：并行失败不销毁单流遗留
/// 断点），临时文件在失败路径自行清理。成功路径合并全部段；并行失败路径（修复②）
/// 仅合并已完成的前缀段。
fn merge_seg_files(part_path: &std::path::Path, seg_paths: &[PathBuf]) -> Result<u64, String> {
    let tmp_path = {
        let mut p = part_path.as_os_str().to_owned();
        p.push(".merge");
        PathBuf::from(p)
    };
    let result = (|| -> Result<u64, String> {
        let mut out = std::fs::File::create(&tmp_path).map_err(|e| format!("创建合并文件: {e}"))?;
        let mut merged = 0u64;
        for p in seg_paths {
            let mut f = std::fs::File::open(p).map_err(|e| format!("打开分段文件: {e}"))?;
            merged += std::io::copy(&mut f, &mut out).map_err(|e| format!("合并分段失败: {e}"))?;
            drop(f);
            let _ = std::fs::remove_file(p);
        }
        drop(out);
        Ok(merged)
    })();
    match result {
        Ok(merged) => std::fs::rename(&tmp_path, part_path)
            .map(|()| merged)
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!("合并文件落位失败: {e}")
            }),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// 多线程分段下载：将 `[0, total)` 均分为多段并发拉取，全部成功后按序合并到 `part_path`。
///
/// 每段写入独立的 `.seg{i}` 临时文件，共用原子进度计数并发出进度事件；
/// 任一段失败即中止其余任务，并把**已完成的段（必为连续前缀）合并为 `part_path`**
/// 断点供回退单流续传（修复②），不再从 0 重下。
async fn download_apk_parallel(
    app: &tauri::AppHandle,
    candidates: &[String],
    total: u64,
    part_path: &std::path::Path,
) -> Result<(), String> {
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    let segments = compute_segments(total);
    let seg_count = segments.len();
    let client = download_client()?;
    let downloaded = Arc::new(AtomicU64::new(0));

    // 各段临时文件路径：part_path.seg{i}
    let seg_paths: Vec<PathBuf> = (0..seg_count)
        .map(|i| {
            let mut p = part_path.as_os_str().to_owned();
            p.push(format!(".seg{i}"));
            PathBuf::from(p)
        })
        .collect();

    let mut handles = Vec::with_capacity(seg_count);
    for (i, &(start, end)) in segments.iter().enumerate() {
        let client = client.clone();
        let candidates = candidates.to_vec();
        let path = seg_paths[i].clone();
        let sink = ProgressSink {
            app: app.clone(),
            downloaded: downloaded.clone(),
            last_reported: std::sync::atomic::AtomicU64::new(0),
            file_total: total,
        };
        handles.push(tokio::spawn(async move {
            download_range_to_file(&client, &candidates, start, end, &path, &sink).await
        }));
    }
    // 顺序等待各段结果（0..seg_count）；任一段失败即中止其余仍在运行的分段任务
    // （否则它们会继续写已清理的 .seg 文件并 emit 进度事件）。
    // 因按序等待，首个失败段下标即「已完成段数」：0..fail_idx 均已成功写盘。
    let mut iter = handles.into_iter();
    let mut failure: Option<(usize, String)> = None;
    for (i, handle) in iter.by_ref().enumerate() {
        let result = handle
            .await
            .map_err(|e| format!("分段下载任务异常: {e}"))
            .and_then(|r| r.map_err(|e| format!("分段下载失败: {e}")));
        if let Err(e) = result {
            failure = Some((i, e));
            break;
        }
    }
    if let Some((fail_idx, e)) = failure {
        // U005: abort() 是异步信号——任务可能已越过最后的 await 点仍在执行
        // （写完文件才返回），若立刻删 seg 文件，任务收尾时可能重建孤儿 .seg。
        // 先 abort 全部剩余句柄，再逐个 await（忽略结果）确保完全停止，之后清理。
        let remaining: Vec<_> = iter.collect();
        for handle in &remaining {
            handle.abort();
        }
        for handle in remaining {
            let _ = handle.await;
        }
        // 修复②：已完成段（0..fail_idx）构成连续前缀，合并为 part_path 断点供
        // 回退单流续传（Range: bytes={prefix}-），避免从 0 重下整个文件。
        // 仅当前缀长于既有 .part（上次单流遗留断点）时才覆盖，否则保留更长断点。
        if fail_idx > 0 {
            let existing = std::fs::metadata(part_path).map(|m| m.len()).unwrap_or(0);
            let prefix_len: u64 = seg_paths[..fail_idx]
                .iter()
                .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
                .sum();
            if prefix_len > existing {
                match merge_seg_files(part_path, &seg_paths[..fail_idx]) {
                    Ok(merged) => {
                        tracing::info!("[updater] 并行失败后保留 {merged} 字节前缀供单流断点续传")
                    }
                    // 合并失败：part_path 未被触碰（临时文件已由 merge_seg_files 清理），
                    // 既有断点原样保留（T002），回退单流仍可从原断点续传。
                    Err(merge_err) => tracing::warn!(
                        "[updater] 并行失败后合并已完成分段失败（既有断点保留）: {merge_err}"
                    ),
                }
            }
        }
        // 清理分段文件：合并路径的成功段已随合并删除，失败段/被中止段/未合并段
        // 在这里清理（remove_file 对已删除文件报错可忽略）；**保留 part_path**（T002）。
        for p in &seg_paths {
            let _ = std::fs::remove_file(p);
        }
        return Err(e);
    }

    // 全部成功：按序合并分段 → part_path（合并函数随合并删除各段文件）
    let merged = merge_seg_files(part_path, &seg_paths)?;
    if merged != total {
        let _ = std::fs::remove_file(part_path);
        return Err(format!("分段合并大小不符: 期望 {total}, 实际 {merged}"));
    }
    tracing::info!("[updater] 分段下载完成，{seg_count} 段共 {merged} 字节");
    Ok(())
}

/// 分段下载的进度上报集合（打包传输，避免函数参数过多）。
struct ProgressSink {
    app: tauri::AppHandle,
    downloaded: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// 已上报过的最大累计值——并发段可能乱序 fetch_add，只上报上升值保证进度条单调不回退。
    last_reported: std::sync::atomic::AtomicU64,
    file_total: u64,
}

impl ProgressSink {
    /// 累计本次分块字节数并发出进度事件（单调不回退）。
    fn report(&self, chunk_len: u64) {
        use std::sync::atomic::Ordering;
        let done = self.downloaded.fetch_add(chunk_len, Ordering::Relaxed) + chunk_len;
        if self.file_total > 0 {
            // 仅当本次累计值超过上次上报值时再 emit，避免并发段乱序导致进度条回退
            let prev = self.last_reported.load(Ordering::Relaxed);
            if done <= prev {
                return;
            }
            self.last_reported.store(done, Ordering::Relaxed);
            let pct = (done as f64 / self.file_total as f64 * 100.0) as u32;
            let _ = self.app.emit(
                "apk-download-progress",
                ApkDownloadProgress {
                    progress: pct.min(100),
                    downloaded: done,
                    total: self.file_total,
                    done: false,
                    error: None,
                },
            );
        }
    }
}

/// 下载单个 Range 段到文件，写入期间通过 `ProgressSink` 累计进度并发出进度事件。
///
/// 逐通道尝试：某通道返回 206 即成功；返回其他状态（如不支持 Range 的 200）
/// 或网络错误则切换到下一候选通道。
async fn download_range_to_file(
    client: &reqwest::Client,
    candidates: &[String],
    start: u64,
    end: u64,
    path: &std::path::Path,
    sink: &ProgressSink,
) -> Result<(), String> {
    let mut last_err = String::new();
    for url in candidates {
        let req = client
            .get(url)
            .header("Range", format!("bytes={start}-{end}"));
        let resp = match req.send().await {
            Ok(r) if r.status() == reqwest::StatusCode::PARTIAL_CONTENT => r,
            Ok(r) => {
                last_err = format!("通道不支持 Range（HTTP {}）", r.status());
                continue;
            }
            Err(e) => {
                last_err = format!("{e}");
                continue;
            }
        };
        let mut seg_file = std::fs::File::create(path).map_err(|e| format!("创建分段文件: {e}"))?;
        // 写入部分文件；流中途失败或写入失败时记录错误并**切换到下一候选通道**，
        // 而不是直接失败——候选通道限流/中途断连可借此恢复（T002）。
        let mut stream = resp;
        let mut written = 0u64;
        let mut seg_error: Option<String> = None;
        loop {
            let chunk = match stream.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(e) => {
                    seg_error = Some(format!("分段分块读取失败: {e}"));
                    break;
                }
            };
            use std::io::Write;
            if let Err(e) = seg_file.write_all(&chunk) {
                seg_error = Some(format!("写入分段失败: {e}"));
                break;
            }
            written += chunk.len() as u64;
            sink.report(chunk.len() as u64);
        }
        // 校验本段实际收到的字节数，提前暴露截断/短响应（避免合并阶段才报错且无法定位）
        let expected = end - start + 1;
        if seg_error.is_none() && written != expected {
            seg_error = Some(format!(
                "分段 {start}-{end} 截断: 期望 {expected} 字节, 实际 {written}"
            ));
        }
        if let Some(e) = seg_error {
            // 本候选通道失败：清理该段部分文件，切换下一候选重试本段。
            // 注：已 report 的字节数不回退，进度条在失败重试路径上可能短暂偏高，
            // 属可接受的近似（最终以合并校验为准）。
            last_err = e;
            let _ = std::fs::remove_file(path);
            continue;
        }
        return Ok(());
    }
    Err(format!("分段 {start}-{end} 所有通道失败: {last_err}"))
}

/// U005: 单流中途失败后，下一候选续传应使用的起始字节偏移。
///
/// - `Some(len)`（文件实际长度，V005）：write_all 部分写入失败时文件实际长度可能
///   大于计数断点，以文件系统实际字节边界为准，供下一候选 `Range: bytes={len}-` 续传。
/// - `None`（文件缺失/不可读，V005-R1）：返回 0 强制重下——若沿用计数断点，下一候选
///   206 续传会对缺失文件 `append(true).open()` 直接 NotFound 终止整个下载；置 0 后
///   下一候选不带 Range 头、走「重新下载」分支自愈。
///
/// 抽为纯函数便于单测：断点语义（实际长度优先 / 缺失强制重下）可独立验证。
fn next_resume_offset(part_len: Option<u64>) -> u64 {
    part_len.unwrap_or(0)
}

/// 单流下载（不支持 Range 或文件较小）：候选通道回退 + 断点续传 + 进度事件。
/// 成功返回后 `part_path` 即完整文件，交由调用方校验落盘。
async fn download_apk_single_stream(
    app: &tauri::AppHandle,
    candidates: &[String],
    part_path: &std::path::Path,
) -> Result<(), String> {
    // 检查是否有已下载的部分文件，用于断点续传
    // U005: mut——流中途失败切下一候选时更新为「已写字节数」作为新断点，
    // 下一候选从新断点续传（不再丢失已下载字节）。
    let mut existing_size = if part_path.exists() {
        let meta = std::fs::metadata(part_path).map_err(|e| format!("读取部分文件元数据: {e}"))?;
        let size = meta.len();
        // 部分文件体积异常（超过普通 APK 大小）时忽略
        if size > 0 && size < 300_000_000 {
            size
        } else {
            let _ = std::fs::remove_file(part_path);
            0
        }
    } else {
        0
    };

    let client = download_client()?;
    let mut last_err = String::new();
    for url in candidates {
        // 构建请求：如果有已下载的部分，添加 Range 头
        let mut req = client.get(url);
        if existing_size > 0 {
            req = req.header("Range", format!("bytes={}-", existing_size));
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("{e}");
                continue;
            }
        };
        let status = resp.status();

        // 处理响应：断点续传 vs 重新下载，并解析完整文件大小
        let (mut file, initial_offset, file_total) = if existing_size > 0
            && status == reqwest::StatusCode::PARTIAL_CONTENT
        {
            // ── 服务器支持 Range，续传 ──
            let remaining = resp.content_length().unwrap_or(0);
            let full_size_from_header =
                parse_content_range_total(&resp).unwrap_or(existing_size + remaining);
            let file = std::fs::OpenOptions::new()
                .append(true)
                .open(part_path)
                .map_err(|e| format!("打开部分文件追加: {e}"))?;
            (file, existing_size, full_size_from_header)
        } else {
            // ── 不支持续传或没有部分文件，重新下载 ──
            if existing_size > 0 {
                // 服务器不支持 Range，删除旧文件重新下载
                let _ = std::fs::remove_file(part_path);
            }
            if !status.is_success() {
                last_err = format!("HTTP {}", status);
                continue;
            }
            let chunk_total = resp.content_length().unwrap_or(0);
            let file = std::fs::File::create(part_path).map_err(|e| format!("创建文件: {e}"))?;
            (file, 0u64, chunk_total)
        };

        // ── 流式下载（统一处理续传和新下载） ──
        let mut new_bytes: u64 = 0;
        let mut stream = resp;
        let mut stream_err: Option<String> = None;
        loop {
            let chunk = match stream.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(e) => {
                    stream_err = Some(format!("下载分块失败: {e}"));
                    break;
                }
            };
            use std::io::Write;
            if let Err(e) = file.write_all(&chunk) {
                stream_err = Some(format!("写入分块失败: {e}"));
                break;
            }
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
        // U005: 流中途失败（分块读取/写入错误）→ 保留已写字节作为断点，切下一候选；
        // 若本候选走的是「重新下载」分支（删旧文件重下），文件已存在且大小为
        // initial_offset + new_bytes，断点同样有效。成功则返回。
        // V005: 断点以 metadata().len() 为准——write_all 部分写入失败时（已写部分字节
        // 但未计入 new_bytes）文件实际长度可能大于计数断点，若按计数断点续传会从错误
        // 偏移追加、在末尾重复拼接未计数字节导致 part 损坏（SHA-256 终检虽能兜住但
        // 浪费一次全量重下）。文件系统实际长度即精确已持久化字节边界。
        // V005-R1: metadata 读取失败（文件可能已缺失，如写失败后被并发删除）时
        // **不能**回退计数断点——若沿用计数断点，下一候选 206 续传会对缺失文件
        // `append(true).open()` 直接 NotFound 以 `?` 终止整个函数，并非修复记录所称
        // 「走重新下载分支自愈」（自愈仅在服务器不支持 Range 时成立）；置 0 强制重下。
        if let Some(e) = stream_err {
            last_err = e;
            existing_size = next_resume_offset(std::fs::metadata(part_path).ok().map(|m| m.len()));
            continue;
        }
        return Ok(());
    }
    Err(format!("单流下载所有通道失败: {last_err}"))
}

/// SHA-256 校验 + 重命名落盘（下载主体后统一调用）。
///
/// 校验失败删除 part 文件并返回错误；成功则删除旧目标文件后原子重命名。
fn verify_and_finalize(
    part_path: &std::path::Path,
    dest: &std::path::Path,
    expected_checksum: &str,
) -> Result<u64, String> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(part_path).map_err(|e| format!("打开文件计算校验和: {e}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 8192];
    let mut size = 0u64;
    loop {
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
        let _ = std::fs::remove_file(part_path);
        return Err(format!(
            "CHECKSUM_MISMATCH: expected {}, got {}",
            expected_checksum, actual
        ));
    }
    // 先删除可能存在的旧最终文件
    let _ = std::fs::remove_file(dest);
    std::fs::rename(part_path, dest).map_err(|e| format!("重命名 APK 文件失败: {e}"))?;
    Ok(size)
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

    /// U005/V005: 单流中途失败后断点以文件实际长度为准（write_all 部分写入失败时
    /// 实际字节数可能大于计数断点）；V005-R1: 文件缺失（metadata 失败）→ 0 强制重下
    /// （若沿用计数断点，下一候选 206 续传会对缺失文件 append 直接 NotFound 终止下载）。
    #[test]
    fn test_next_resume_offset() {
        // 文件存在：以文件系统实际长度为断点（部分写入失败时可能大于计数断点）
        assert_eq!(next_resume_offset(Some(1024)), 1024);
        assert_eq!(next_resume_offset(Some(5_002_048)), 5_002_048);
        // 文件缺失/不可读（V005-R1）：强制重新下载（0 → 下一候选不带 Range 头）
        assert_eq!(next_resume_offset(None), 0);
    }

    /// 分段计算：文件恰为段数整数倍 → 均匀分段，首尾闭合区间连续。
    #[test]
    fn test_compute_segments_even_split() {
        // 80MB / 4 段 = 每段 20MB
        let segs = compute_segments(80 * 1024 * 1024);
        assert_eq!(segs.len(), PARALLEL_SEGMENTS);
        let expected: Vec<(u64, u64)> = (0..4)
            .map(|i| {
                let start = i as u64 * 20 * 1024 * 1024;
                (start, start + 20 * 1024 * 1024 - 1)
            })
            .collect();
        assert_eq!(segs, expected);
        // 相邻区间无缝衔接且覆盖全部字节
        for w in segs.windows(2) {
            assert_eq!(w[0].1 + 1, w[1].0);
        }
        assert_eq!(segs.last().unwrap().1, 80 * 1024 * 1024 - 1);
    }

    /// 分段计算：总大小不能被段数整除时，余数分配到前若干段，区间仍连续覆盖。
    #[test]
    fn test_compute_segments_remainder_handling() {
        let total = 4 * 1024 * 1024 + 3; // 4MB+3 字节，远小于 20MB 阈值
        let segs = compute_segments(total);
        // 因每段最小 5MB，4MB 文件只分 1 段
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0], (0, total - 1));

        // 22MB：仍按 4 段（每段 ~5.5MB 满足最小 5MB）
        let total2 = 22 * 1024 * 1024;
        let segs2 = compute_segments(total2);
        assert_eq!(segs2.len(), PARALLEL_SEGMENTS);
        for w in segs2.windows(2) {
            assert_eq!(w[0].1 + 1, w[1].0);
        }
        assert_eq!(segs2.last().unwrap().1, total2 - 1);
    }

    /// 分段计算：文件略超 20MB 阈值（如 21MB）时受每段最小 5MB 约束，段数收缩到 4；
    /// 文件非常大时仍不超过 PARALLEL_SEGMENTS 段。
    ///
    /// 注意：`compute_segments` 自身不感知 `PARALLEL_MIN_FILE_SIZE`（20MB 并行阈值），
    /// 那是调用点 `android_download_apk`（经 `download_apk_to_part`）的职责；此处只按
    /// 「每段最小 PARALLEL_MIN_SEGMENT_SIZE」收缩段数——19MB 按 5MB/段 得 ceil(19/5)=4 段，
    /// 与 20MB 阈值无关。
    #[test]
    fn test_compute_segments_caps_at_max_segments() {
        // 500MB：每段 5MB 上限为 100 段，但被 PARALLEL_SEGMENTS 封顶为 4
        let segs = compute_segments(500 * 1024 * 1024);
        assert_eq!(segs.len(), PARALLEL_SEGMENTS);
        // 0 字节：无段
        assert!(compute_segments(0).is_empty());
        // 19MB：低于调用点 20MB 并行阈值，但 compute_segments 本身仍按每段最小 5MB 分 4 段
        assert_eq!(compute_segments(19 * 1024 * 1024).len(), 4);
    }

    /// 修复④：样本测速达标判定——海外快直连（≈20MB/s）健康；恰在阈值（2MB/s）健康；
    /// 国内受限直连（≈300KB/s）过慢；零耗时/零字节防御性判过慢。
    #[test]
    fn test_sample_speed_healthy_verdicts() {
        assert!(sample_speed_healthy(5 * 1024 * 1024, 0.25)); // 20MB/s
        assert!(sample_speed_healthy(1024 * 1024, 0.5)); // 恰 2MB/s 阈值
        assert!(!sample_speed_healthy(300 * 1024, 1.0)); // 300KB/s 过慢
        assert!(!sample_speed_healthy(0, 0.0));
        assert!(!sample_speed_healthy(1024 * 1024, 0.0)); // 零耗时防御
    }

    // ── 修复④ 探测策略集成测试（本地 HTTP/1.1 服务器，不触网） ──────────────

    /// 启动本地 HTTP/1.1 服务器，模拟 GitHub 资产（64MB，支持 Range）。
    /// `slow` 时按 ≈320KB/s（每 64KB sleep 200ms）分块吐样本字节，模拟直连过慢；
    /// `ignore_range` 时忽略 Range 返回 200（完整 body，模拟不支持 Range 的通道）。
    /// 返回 base URL；服务器任务在后台 tokio 任务中持续接受连接。
    async fn spawn_range_server(slow: bool, ignore_range: bool) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        const TOTAL: u64 = 64 * 1024 * 1024; // 64MB 模拟 APK
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind 本地端口");
        let addr = listener.local_addr().expect("取监听地址");
        let base = format!("http://{addr}");
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    // 读请求头（到 \r\n\r\n 或上限）
                    let mut buf = [0u8; 4096];
                    let mut request: Vec<u8> = Vec::new();
                    loop {
                        match sock.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => request.extend_from_slice(&buf[..n]),
                        }
                        if request.windows(4).any(|w| w == b"\r\n\r\n") || request.len() > 8192 {
                            break;
                        }
                    }
                    // 解析 Range: bytes=start-end
                    let req_text = String::from_utf8_lossy(&request);
                    let (start, end) = req_text
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                        .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_string()))
                        .and_then(|v| v.strip_prefix("bytes=").map(|r| r.to_string()))
                        .and_then(|r| {
                            r.split_once('-')
                                .map(|(s, e)| (s.to_string(), e.to_string()))
                        })
                        .map(|(s, e)| (s.parse().unwrap_or(0), e.parse().unwrap_or(0)))
                        .unwrap_or((0, 1024 * 1024));
                    let end = end.min(TOTAL - 1);
                    let len = end.saturating_sub(start) + 1;
                    let headers = if ignore_range {
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {TOTAL}\r\nConnection: close\r\n\r\n")
                    } else {
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{TOTAL}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
                        )
                    };
                    if sock.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    // 200（不支持 Range）场景客户端不会读 body（探测只取状态/大小），
                    // 只写前 1MB 即可让探测正常完成，避免无谓写满 64MB。
                    let body_len = if ignore_range {
                        len.min(1024 * 1024)
                    } else {
                        len
                    };
                    let mut sent = 0u64;
                    while sent < body_len {
                        // 快路径用大块单次写入（最小化 syscall 开销，降低 CI 慢机误判慢的风险）
                        let chunk = if slow {
                            (body_len - sent).min(64 * 1024) as usize
                        } else {
                            (body_len - sent).min(1024 * 1024) as usize
                        };
                        if sock.write_all(&vec![0u8; chunk]).await.is_err() {
                            return;
                        }
                        sent += chunk as u64;
                        if slow {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        }
                    }
                    let _ = sock.shutdown().await;
                });
            }
        });
        base
    }

    /// 取一个必然连接失败的端口（绑定后立即释放）。
    async fn closed_port() -> u16 {
        tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind")
            .local_addr()
            .expect("取端口")
            .port()
    }

    /// 修复④：直连健康（206 + 样本测速达标）→ 单流直连策略（不再并行/绕代理）。
    #[tokio::test]
    async fn test_probe_direct_healthy_returns_single_stream() {
        let base = spawn_range_server(false, false).await;
        let candidates = vec![format!("{base}/app.apk")];
        let strategy = probe_download(&candidates).await.expect("探测应成功");
        assert!(
            matches!(strategy, DownloadStrategy::DirectSingleStream),
            "快直连应走单流直连，实际 {strategy:?}"
        );
    }

    /// 修复④：直连过慢（206 但样本测速不足）→ 并行加速（range_channel=0，代理兜底）。
    #[tokio::test]
    async fn test_probe_direct_slow_returns_accelerated_parallel() {
        let base = spawn_range_server(true, false).await;
        let candidates = vec![format!("{base}/app.apk")];
        let strategy = probe_download(&candidates).await.expect("探测应成功");
        match strategy {
            DownloadStrategy::Accelerated {
                total,
                range_channel,
            } => {
                assert_eq!(range_channel, Some(0), "直连过慢应并行 over 直连");
                assert!(total > 0);
            }
            other => panic!("直连过慢应返回 Accelerated，实际 {other:?}"),
        }
    }

    /// 修复④：直连失败 → 回退代理通道（首个 206 的代理索引）。
    #[tokio::test]
    async fn test_probe_direct_down_falls_back_to_proxy() {
        let base = spawn_range_server(false, false).await;
        let dead = format!("http://127.0.0.1:{}/x", closed_port().await);
        let candidates = vec![dead, format!("{base}/app.apk")];
        let strategy = probe_download(&candidates).await.expect("探测应成功");
        match strategy {
            DownloadStrategy::Accelerated {
                total,
                range_channel,
            } => {
                assert_eq!(range_channel, Some(1), "直连失败应回退代理通道");
                assert!(total > 0);
            }
            other => panic!("直连失败应回退代理，实际 {other:?}"),
        }
    }

    /// 修复④：全部候选不可用 → 聚合错误。
    #[tokio::test]
    async fn test_probe_all_candidates_down_errors() {
        let candidates = vec![
            format!("http://127.0.0.1:{}/a", closed_port().await),
            format!("http://127.0.0.1:{}/b", closed_port().await),
        ];
        assert!(probe_download(&candidates).await.is_err());
    }

    /// 修复④：所有候选均不支持 Range（200）→ 单流回退（range_channel=None）。
    #[tokio::test]
    async fn test_probe_no_range_support_falls_back_to_single_stream() {
        let base = spawn_range_server(false, true).await;
        let candidates = vec![format!("{base}/a"), format!("{base}/b")];
        let strategy = probe_download(&candidates).await.expect("探测应成功");
        match strategy {
            DownloadStrategy::Accelerated {
                total,
                range_channel,
            } => {
                assert!(range_channel.is_none(), "不支持 Range 应回退单流");
                assert!(total > 0);
            }
            other => panic!("200 通道应返回 Accelerated{{None}}，实际 {other:?}"),
        }
    }

    /// 修复①：并行段级候选顺序——主通道（通常直连）重复 1+DIRECT_SEGMENT_RETRIES 次
    /// 排最前（失败先重试同通道而非立即切代理），其余候选保持原有相对顺序。
    #[test]
    fn test_parallel_candidate_order_direct_retried_first() {
        let candidates: Vec<String> = (0..5).map(|i| format!("url{i}")).collect();
        // idx=0（直连主通道）：直连 ×(1+RETRIES)，随后其余候选
        let ordered = parallel_candidate_order(&candidates, 0);
        let mut expected = vec![candidates[0].clone(); 1 + DIRECT_SEGMENT_RETRIES];
        expected.extend(candidates[1..].iter().cloned());
        assert_eq!(ordered, expected);
        // idx=2（代理主通道）：主通道 ×(1+RETRIES)，随后 idx+1.. 再 ..idx
        let ordered2 = parallel_candidate_order(&candidates, 2);
        let mut expected2 = vec![candidates[2].clone(); 1 + DIRECT_SEGMENT_RETRIES];
        expected2.extend(candidates[3..].iter().cloned());
        expected2.extend(candidates[..2].iter().cloned());
        assert_eq!(ordered2, expected2);
    }

    /// 修复②：合并 seg 文件按序拼接并删除源文件（成功路径全量 / 失败路径前缀共用）。
    #[test]
    fn test_merge_seg_files_concatenates_and_removes() {
        let dir = std::env::temp_dir().join(format!("solosoul_seg_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let segs: Vec<PathBuf> = (0..3)
            .map(|i| {
                let p = dir.join(format!("part.seg{i}"));
                std::fs::write(&p, vec![b'a' + i as u8; 100]).unwrap();
                p
            })
            .collect();
        let part = dir.join("part");
        let merged = merge_seg_files(&part, &segs).unwrap();
        assert_eq!(merged, 300);
        let data = std::fs::read(&part).unwrap();
        assert_eq!(data.len(), 300);
        // 按序拼接：前 100 字节 a、次 100 字节 b、末 100 字节 c
        assert!(data.iter().take(100).all(|&b| b == b'a'));
        assert!(data.iter().skip(100).take(100).all(|&b| b == b'b'));
        assert!(data.iter().skip(200).all(|&b| b == b'c'));
        // 源 seg 文件已删除
        for p in &segs {
            assert!(!p.exists());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
