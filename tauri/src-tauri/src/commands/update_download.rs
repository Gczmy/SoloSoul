//! 更新包的有界测速、故障换源和持久断点续传；最终验签由调用方负责。
//!
//! 不创建下载任务：外层取消并丢弃 future 时，响应和探测 future 同步释放。
//! 文件使用同步小块写入，避免 tokio::fs 已提交的后台写操作在取消后继续修改缓存。

use futures::{stream, StreamExt};
use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE, RANGE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;

/// 仅暴露源主机名，避免向前端泄漏重定向的签名查询参数。
pub(super) struct TransferProgress {
    pub downloaded: u64,
    pub total: u64,
    pub source: String,
    pub bytes_per_second: u64,
    pub phase: &'static str,
}

type ProgressCallback<'a> = &'a (dyn Fn(TransferProgress) + Send + Sync);

#[derive(Clone)]
struct Policy {
    connect_timeout: Duration,
    header_timeout: Duration,
    probe_timeout: Duration,
    idle_timeout: Duration,
    slow_window: Duration,
    sample_bytes: u64,
    healthy_speed: u64,
    minimum_speed: u64,
    maximum_size: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(3),
            header_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_millis(1800),
            idle_timeout: Duration::from_secs(3),
            slow_window: Duration::from_secs(4),
            sample_bytes: 256 * 1024,
            healthy_speed: 512 * 1024,
            minimum_speed: 64 * 1024,
            maximum_size: 512 * 1024 * 1024,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    version: u8,
    identity: String,
    total: u64,
    source: String,
    etag: Option<String>,
}

struct Cache {
    metadata: CacheMetadata,
    downloaded: u64,
}

#[derive(Clone)]
struct Probe {
    url: String,
    total: u64,
    speed: u64,
    complete_sample: bool,
}

struct ResponseInfo {
    total: u64,
    etag: Option<String>,
    partial: bool,
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn metadata_path(part_path: &Path) -> PathBuf {
    let mut name = part_path.as_os_str().to_os_string();
    name.push(".json");
    name.into()
}

fn source_name(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_default()
}

fn allowed_url(url: &reqwest::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        return false;
    }
    if url.scheme() == "https" {
        return true;
    }
    // HTTP 仅供本文件的真实本机 fixture 使用，发布构建始终要求 HTTPS。
    #[cfg(test)]
    if url.scheme() == "http" {
        return url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
    }
    false
}

fn client(policy: &Policy) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("SoloSoul-Updater")
        .connect_timeout(policy.connect_timeout)
        // 不设置整个请求的总超时：大安装包允许持续下载超过数分钟。
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 8 || !allowed_url(attempt.url()) {
                attempt.error("不安全的更新包重定向")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| "无法创建更新下载连接".to_string())
}

fn load_cache(path: &Path, identity: &str, policy: &Policy) -> Result<Cache, String> {
    let identity = digest(identity);
    let sidecar = metadata_path(path);
    // 不读取任意大小的 sidecar，损坏的元数据只会导致重新下载。
    let metadata = std::fs::metadata(&sidecar)
        .ok()
        .filter(|info| info.len() <= 16 * 1024)
        .and_then(|_| std::fs::read(&sidecar).ok())
        .and_then(|bytes| serde_json::from_slice::<CacheMetadata>(&bytes).ok());
    let length = std::fs::metadata(path).ok().map(|info| info.len());
    if let (Some(metadata), Some(downloaded)) = (metadata, length) {
        if metadata.version == 1
            && metadata.identity == identity
            && metadata.total > 0
            && metadata.total <= policy.maximum_size
            && downloaded <= metadata.total
        {
            return Ok(Cache {
                metadata,
                downloaded,
            });
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "无法创建更新缓存目录".to_string())?;
    }
    File::create(path).map_err(|_| "无法重置更新缓存".to_string())?;
    match std::fs::remove_file(&sidecar) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("无法重置更新缓存元数据".to_string()),
    }
    Ok(Cache {
        metadata: CacheMetadata {
            version: 1,
            identity,
            total: 0,
            source: String::new(),
            etag: None,
        },
        downloaded: 0,
    })
}

fn save_metadata(path: &Path, metadata: &CacheMetadata) -> Result<(), String> {
    let bytes = serde_json::to_vec(metadata).map_err(|_| "无法编码更新缓存元数据".to_string())?;
    let target = metadata_path(path);
    let mut temporary = tempfile::NamedTempFile::new_in(target.parent().ok_or("缓存目录无效")?)
        .map_err(|_| "无法创建更新缓存元数据".to_string())?;
    temporary
        .write_all(&bytes)
        .map_err(|_| "无法写入更新缓存元数据".to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| "无法保存更新缓存元数据".to_string())?;
    temporary
        .persist(&target)
        .map_err(|_| "无法提交更新缓存元数据".to_string())?;
    Ok(())
}

/// 只读断点快照，不因检查更新重置或清理下载中的文件。
pub(super) fn cached_progress(path: &Path, identity: &str) -> Option<(u64, u64)> {
    let sidecar = metadata_path(path);
    if std::fs::metadata(&sidecar).ok()?.len() > 16 * 1024 {
        return None;
    }
    let metadata: CacheMetadata = serde_json::from_slice(&std::fs::read(sidecar).ok()?).ok()?;
    let downloaded = std::fs::metadata(path).ok()?.len();
    (metadata.version == 1
        && metadata.identity == digest(identity)
        && metadata.total > 0
        && metadata.total <= Policy::default().maximum_size
        && downloaded <= metadata.total)
        .then_some((downloaded, metadata.total))
}

fn header_u64(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Result<Option<u64>, String> {
    response
        .headers()
        .get(name)
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| "更新包长度标头无效".to_string())
        })
        .transpose()
}

fn parse_content_range(value: &str) -> Result<(u64, u64, u64), String> {
    let parsed = value
        .strip_prefix("bytes ")
        .and_then(|value| value.split_once('/'))
        .and_then(|(range, total)| {
            range
                .split_once('-')
                .map(|(start, end)| (start, end, total))
        })
        .and_then(|(start, end, total)| {
            Some((
                start.parse::<u64>().ok()?,
                end.parse::<u64>().ok()?,
                total.parse::<u64>().ok()?,
            ))
        })
        .filter(|(start, end, total)| *total > 0 && start <= end && end < total);
    parsed.ok_or_else(|| "更新包 Content-Range 无效".to_string())
}

fn response_info(
    response: &reqwest::Response,
    start: u64,
    requested_end: Option<u64>,
    policy: &Policy,
) -> Result<ResponseInfo, String> {
    if response
        .headers()
        .get(CONTENT_ENCODING)
        .is_some_and(|value| value != "identity")
    {
        return Err("更新包使用了不支持的内容编码".to_string());
    }
    let content_length = header_u64(response, CONTENT_LENGTH)?;
    let partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let total = if partial {
        let raw = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| "更新包缺少 Content-Range".to_string())?;
        let (actual_start, end, total) = parse_content_range(raw)?;
        let expected_end = requested_end.unwrap_or(total - 1).min(total - 1);
        if actual_start != start || end != expected_end {
            return Err("更新包续传范围与请求不一致".to_string());
        }
        if content_length.is_some_and(|length| length != end - actual_start + 1) {
            return Err("更新包 Content-Length 与范围不一致".to_string());
        }
        total
    } else if response.status() == reqwest::StatusCode::OK {
        if response.headers().contains_key(CONTENT_RANGE) {
            return Err("更新包 200 响应携带了异常范围".to_string());
        }
        // 身份与确切大小共同绑定持久前缀，不接受无法预先界定大小的无限流。
        content_length.ok_or_else(|| "更新包缺少确定大小".to_string())?
    } else {
        return Err(format!(
            "更新包请求返回 HTTP {}",
            response.status().as_u16()
        ));
    };
    if total == 0 || total > policy.maximum_size {
        return Err("更新包大小超出允许范围".to_string());
    }
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with('"') && value.ends_with('"'))
        .map(str::to_owned);
    Ok(ResponseInfo {
        total,
        etag,
        partial,
    })
}

fn emit(callback: ProgressCallback<'_>, cache: &Cache, url: &str, speed: u64, phase: &'static str) {
    callback(TransferProgress {
        downloaded: cache.downloaded,
        total: cache.metadata.total,
        source: source_name(url),
        bytes_per_second: speed,
        phase,
    });
}

async fn probe(client: &reqwest::Client, url: &str, policy: &Policy) -> Result<Probe, String> {
    let mut response = tokio::time::timeout(
        policy.header_timeout,
        client
            .get(url)
            .header(ACCEPT_ENCODING, "identity")
            .header(RANGE, format!("bytes=0-{}", policy.sample_bytes - 1))
            .send(),
    )
    .await
    .map_err(|_| "下载源测速连接超时".to_string())?
    .map_err(|_| "下载源测速连接失败".to_string())?;
    let info = response_info(&response, 0, Some(policy.sample_bytes - 1), policy)?;
    // DNS/TLS/首响应使用独立预算，不能挤占实际读取样本的测速窗口。
    let started = Instant::now();
    let deadline = started + policy.probe_timeout;
    let sample_size = policy.sample_bytes.min(info.total);
    let mut received = 0;
    while received < sample_size {
        match tokio::time::timeout_at(deadline, response.chunk()).await {
            Ok(Ok(Some(bytes))) => {
                received += bytes.len() as u64;
                if received > info.total || (info.partial && received > sample_size) {
                    return Err("测速响应超过声明大小".to_string());
                }
            }
            Ok(Ok(None)) => return Err("测速响应提前结束".to_string()),
            Ok(Err(_)) => return Err("测速响应读取失败".to_string()),
            Err(_) if received > 0 => break,
            Err(_) => return Err("下载源测速停流".to_string()),
        }
    }
    // 至少按 50ms 计算：避免一次已缓冲 chunk 的微秒级读取产生夸大的速度。
    let speed =
        (received.min(sample_size) as f64 / started.elapsed().as_secs_f64().max(0.05)) as u64;
    Ok(Probe {
        url: url.to_string(),
        total: info.total,
        speed,
        complete_sample: info.total <= policy.sample_bytes && received >= info.total,
    })
}

async fn probe_alternatives(
    client: &reqwest::Client,
    urls: Vec<String>,
    policy: &Policy,
) -> Vec<Probe> {
    let mut probes: Vec<_> = stream::iter(
        urls.into_iter()
            .map(|url| async move { probe(client, &url, policy).await.ok() }),
    )
    .buffer_unordered(3)
    .filter_map(|result| async { result })
    .collect()
    .await;
    probes.sort_by_key(|probe| std::cmp::Reverse(probe.speed));
    probes
}

async fn transfer(
    client: &reqwest::Client,
    selected: &Probe,
    path: &Path,
    cache: &mut Cache,
    policy: &Policy,
    callback: ProgressCallback<'_>,
) -> Result<u64, String> {
    if cache.metadata.total != 0 && cache.metadata.total != selected.total {
        return Err("下载源大小与已缓存安装包不一致".to_string());
    }
    let source = digest(&selected.url);
    let mut request = client
        .get(&selected.url)
        .header(ACCEPT_ENCODING, "identity")
        .header(RANGE, format!("bytes={}-", cache.downloaded));
    let validator = (cache.downloaded > 0 && cache.metadata.source == source)
        .then(|| cache.metadata.etag.clone())
        .flatten();
    if let Some(etag) = &validator {
        request = request.header(IF_RANGE, etag);
    }
    let mut response = tokio::time::timeout(policy.header_timeout, request.send())
        .await
        .map_err(|_| "下载源响应超时".to_string())?
        .map_err(|_| "下载源连接失败".to_string())?;
    let info = response_info(&response, cache.downloaded, None, policy)?;
    if info.total != selected.total {
        return Err("下载源在测速后改变了安装包大小".to_string());
    }
    if info.partial
        && validator
            .as_ref()
            .is_some_and(|etag| info.etag.as_ref() != Some(etag))
    {
        return Err("下载源忽略 If-Range 且安装包 ETag 已改变".to_string());
    }
    // 服务器忽略 Range 或 If-Range 失效而返回 200 时，从零覆盖，绝不追加完整包。
    let start = if info.partial { cache.downloaded } else { 0 };
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(start == 0)
        .open(path)
        .map_err(|_| "无法打开更新缓存".to_string())?;
    file.seek(SeekFrom::Start(start))
        .map_err(|_| "无法定位更新缓存".to_string())?;
    cache.downloaded = start;
    cache.metadata.total = info.total;
    cache.metadata.source = source;
    cache.metadata.etag = info.etag;
    save_metadata(path, &cache.metadata)?;
    emit(callback, cache, &selected.url, 0, "downloading");
    let mut window_start = Instant::now();
    let mut last_data = window_start;
    let mut last_report = window_start;
    let mut window_bytes = 0u64;
    loop {
        let deadline = (last_data + policy.idle_timeout).min(window_start + policy.slow_window);
        let next = tokio::time::timeout_at(deadline, response.chunk()).await;
        match next {
            Ok(Ok(Some(bytes))) => {
                if cache.downloaded + bytes.len() as u64 > info.total {
                    return Err("更新包响应超过声明大小".to_string());
                }
                // 每次轮询中的写入同步完成；drop future 后不遗留文件写任务。
                file.write_all(&bytes)
                    .map_err(|_| "无法写入更新缓存".to_string())?;
                cache.downloaded += bytes.len() as u64;
                window_bytes += bytes.len() as u64;
                last_data = Instant::now();
                if last_report.elapsed() >= Duration::from_millis(200) {
                    let speed = (window_bytes as f64
                        / window_start.elapsed().as_secs_f64().max(0.001))
                        as u64;
                    emit(callback, cache, &selected.url, speed, "downloading");
                    last_report = Instant::now();
                }
            }
            Ok(Ok(None)) => {
                if cache.downloaded != info.total {
                    return Err("更新包下载不完整".to_string());
                }
                file.sync_all()
                    .map_err(|_| "无法保存完整更新包".to_string())?;
                let speed =
                    (window_bytes as f64 / window_start.elapsed().as_secs_f64().max(0.001)) as u64;
                emit(callback, cache, &selected.url, speed, "downloading");
                return Ok(cache.downloaded);
            }
            Ok(Err(_)) => return Err("更新包连接中断，可继续断点下载".to_string()),
            Err(_) if last_data.elapsed() >= policy.idle_timeout => {
                return Err("下载源持续停流".to_string())
            }
            Err(_) => {}
        }
        if window_start.elapsed() >= policy.slow_window {
            let speed =
                (window_bytes as f64 / window_start.elapsed().as_secs_f64().max(0.001)) as u64;
            if speed < policy.minimum_speed {
                return Err("下载源持续低速".to_string());
            }
            window_start = Instant::now();
            window_bytes = 0;
        }
    }
}

/// candidates 应由可信更新元数据构造，按直连/官方源优先排序。
/// identity 须包含版本、平台和可信签名（Android 为已验签 SHA-256）。
/// 成功仅表示完整传输；调用方必须验签，验签失败时删除 .part 及 .part.json。
pub(super) async fn download_file(
    candidates: &[String],
    part_path: &Path,
    identity: &str,
    on_progress: ProgressCallback<'_>,
) -> Result<u64, String> {
    download_with_policy(
        candidates,
        part_path,
        identity,
        on_progress,
        &Policy::default(),
    )
    .await
}

async fn download_with_policy(
    candidates: &[String],
    part_path: &Path,
    identity: &str,
    on_progress: ProgressCallback<'_>,
    policy: &Policy,
) -> Result<u64, String> {
    let mut urls = Vec::new();
    for candidate in candidates.iter().take(12) {
        if reqwest::Url::parse(candidate).is_ok_and(|url| allowed_url(&url))
            && !urls.contains(candidate)
        {
            urls.push(candidate.clone());
        }
    }
    if urls.is_empty() {
        return Err("没有安全可用的更新下载地址".to_string());
    }
    let mut cache = load_cache(part_path, identity, policy)?;
    if cache.downloaded > 0 {
        if let Some(index) = urls
            .iter()
            .position(|url| digest(url) == cache.metadata.source)
        {
            urls.swap(0, index);
        }
        if cache.downloaded == cache.metadata.total {
            emit(on_progress, &cache, &urls[0], 0, "downloading");
            return Ok(cache.downloaded);
        }
    }
    let client = client(policy)?;
    let first = urls.remove(0);
    emit(on_progress, &cache, &first, 0, "probing");
    let initial = probe(&client, &first, policy).await;
    let mut last_error = initial.as_ref().err().cloned().unwrap_or_default();
    let mut probes = Vec::new();
    if let Ok(initial) = initial {
        probes.push(initial);
    }
    let healthy = probes
        .first()
        .is_some_and(|probe| probe.speed >= policy.healthy_speed || probe.complete_sample);
    // 海外健康直连不接触代理；只有实测慢/失败或中途退化才并发测速备选。
    if !healthy {
        emit(on_progress, &cache, &first, 0, "switching");
        probes.extend(probe_alternatives(&client, std::mem::take(&mut urls), policy).await);
        probes.sort_by_key(|probe| std::cmp::Reverse(probe.speed));
    }
    let mut attempted = Vec::new();
    loop {
        for selected in std::mem::take(&mut probes) {
            emit(
                on_progress,
                &cache,
                &selected.url,
                selected.speed,
                "downloading",
            );
            match transfer(
                &client,
                &selected,
                part_path,
                &mut cache,
                policy,
                on_progress,
            )
            .await
            {
                Ok(size) => return Ok(size),
                Err(error) => last_error = format!("{}: {error}", source_name(&selected.url)),
            }
            attempted.push(selected);
            emit(on_progress, &cache, &first, 0, "switching");
        }
        if urls.is_empty() {
            break;
        }
        probes = probe_alternatives(&client, std::mem::take(&mut urls), policy).await;
    }
    // 没有更快的可用通道时，最后一轮允许持续低速完成，避免要求用户反复手动继续。
    // 只关闭低速换源；停流、响应头超时、协议校验和外层取消均保持不变。
    // 每个通道至多再重连一次，保留已有前缀，坏连接仍受有界尝试和 idle_timeout 约束。
    let fallback_policy = Policy {
        minimum_speed: 0,
        ..policy.clone()
    };
    for selected in attempted {
        match transfer(
            &client,
            &selected,
            part_path,
            &mut cache,
            &fallback_policy,
            on_progress,
        )
        .await
        {
            Ok(size) => return Ok(size),
            Err(error) => last_error = format!("{}: {error}", source_name(&selected.url)),
        }
    }
    Err(format!(
        "所有下载源均未能完成更新包下载；已保留断点。{last_error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const PACKAGE_SIZE: usize = 32 * 1024;

    fn policy() -> Policy {
        Policy {
            connect_timeout: Duration::from_millis(150),
            header_timeout: Duration::from_millis(200),
            probe_timeout: Duration::from_millis(150),
            idle_timeout: Duration::from_millis(100),
            slow_window: Duration::from_millis(120),
            sample_bytes: 1024,
            healthy_speed: 8 * 1024,
            minimum_speed: 4 * 1024,
            maximum_size: 64 * 1024,
        }
    }

    fn package() -> Vec<u8> {
        (0..PACKAGE_SIZE).map(|index| (index % 251) as u8).collect()
    }

    #[derive(Clone, Copy)]
    enum Behavior {
        Normal,
        SlowHeaders,
        IgnoreRange,
        SlowProbe,
        StallProbe,
        StallBody,
        SlowBody,
        TrickleBody,
        TruncateBody,
        WrongRange,
        WrongLength,
        Oversized,
        UnknownLength,
        ChangedEtag,
    }

    #[derive(Clone, Debug)]
    struct Request {
        start: usize,
        end: Option<usize>,
        if_range: Option<String>,
    }

    impl Request {
        fn parse(raw: &str) -> Self {
            let headers: Vec<_> = raw
                .lines()
                .filter_map(|line| line.split_once(':'))
                .map(|(key, value)| (key.to_ascii_lowercase(), value.trim()))
                .collect();
            let range = headers
                .iter()
                .find(|(key, _)| key == "range")
                .map(|(_, value)| *value)
                .unwrap_or("bytes=0-");
            let (start, end) = range
                .strip_prefix("bytes=")
                .unwrap()
                .split_once('-')
                .unwrap();
            Self {
                start: start.parse().unwrap(),
                end: end.parse().ok(),
                if_range: headers
                    .iter()
                    .find(|(key, _)| key == "if-range")
                    .map(|(_, value)| (*value).to_string()),
            }
        }
    }

    struct Server {
        url: String,
        requests: Arc<Mutex<Vec<Request>>>,
        task: tokio::task::JoinHandle<()>,
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    impl Server {
        async fn start(behavior: Behavior) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/package", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded = requests.clone();
            let task = tokio::spawn(async move {
                // 子连接也由 JoinSet 管理，fixture 释放后不遗留后台任务。
                let mut connections = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        accepted = listener.accept() => {
                            let Ok((mut socket, _)) = accepted else { return; };
                            let recorded = recorded.clone();
                            connections.spawn(async move {
                                let mut raw = Vec::new();
                                let mut buffer = [0u8; 1024];
                                while !raw.windows(4).any(|window| window == b"\r\n\r\n") {
                                    let Ok(count) = socket.read(&mut buffer).await else { return; };
                                    if count == 0 || raw.len() > 16 * 1024 { return; }
                                    raw.extend_from_slice(&buffer[..count]);
                                }
                                let request = Request::parse(&String::from_utf8_lossy(&raw));
                                recorded.lock().unwrap().push(request.clone());
                                serve(&mut socket, request, behavior).await;
                            });
                        }
                        _ = connections.join_next(), if !connections.is_empty() => {}
                    }
                }
            });
            Self {
                url,
                requests,
                task,
            }
        }

        fn transfer_requests(&self) -> Vec<Request> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.end.is_none())
                .cloned()
                .collect()
        }
    }

    async fn serve(socket: &mut tokio::net::TcpStream, request: Request, behavior: Behavior) {
        if matches!(behavior, Behavior::SlowHeaders) {
            tokio::time::sleep(Duration::from_millis(180)).await;
        }
        let data = package();
        let is_probe = request.end.is_some();
        if matches!(behavior, Behavior::UnknownLength) {
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nendless")
                .await;
            return;
        }
        if matches!(behavior, Behavior::Oversized) {
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 65537\r\nConnection: close\r\n\r\n")
                .await;
            return;
        }
        if request.start >= data.len() {
            let _ = socket.write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            return;
        }
        let full = matches!(behavior, Behavior::IgnoreRange);
        let start = if full { 0 } else { request.start };
        let end = if full {
            data.len() - 1
        } else {
            request.end.unwrap_or(data.len() - 1).min(data.len() - 1)
        };
        let body = &data[start..=end];
        let etag = if matches!(behavior, Behavior::ChangedEtag) {
            "new"
        } else {
            "package-v1"
        };
        let mut headers = if full {
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n", body.len())
        } else {
            let reported_start = if !is_probe && matches!(behavior, Behavior::WrongRange) {
                start + 1
            } else {
                start
            };
            let length = if !is_probe && matches!(behavior, Behavior::WrongLength) {
                body.len() - 1
            } else {
                body.len()
            };
            format!("HTTP/1.1 206 Partial Content\r\nContent-Length: {length}\r\nContent-Range: bytes {reported_start}-{end}/{}\r\n", data.len())
        };
        headers.push_str(&format!("ETag: \"{etag}\"\r\nConnection: close\r\n\r\n"));
        if socket.write_all(headers.as_bytes()).await.is_err() {
            return;
        }
        if is_probe && matches!(behavior, Behavior::StallProbe) {
            tokio::time::sleep(Duration::from_secs(1)).await;
            return;
        }
        if is_probe && matches!(behavior, Behavior::SlowProbe) {
            if socket.write_all(&body[..1]).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            return;
        }
        if !is_probe && matches!(behavior, Behavior::StallBody | Behavior::TruncateBody) {
            let _ = socket.write_all(&body[..2048.min(body.len())]).await;
            if matches!(behavior, Behavior::StallBody) {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            return;
        }
        if !is_probe && matches!(behavior, Behavior::TrickleBody) {
            for byte in body {
                if socket.write_all(&[*byte]).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            return;
        }
        if !is_probe && matches!(behavior, Behavior::SlowBody) {
            for chunk in body.chunks(1024) {
                if socket.write_all(chunk).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
            return;
        }
        let _ = socket.write_all(body).await;
    }

    fn seed_cache(path: &Path, identity: &str, source: &str, prefix: usize) {
        std::fs::write(path, &package()[..prefix]).unwrap();
        save_metadata(
            path,
            &CacheMetadata {
                version: 1,
                identity: digest(identity),
                total: PACKAGE_SIZE as u64,
                source: digest(source),
                etag: Some("\"package-v1\"".to_string()),
            },
        )
        .unwrap();
    }

    #[tokio::test]
    async fn healthy_direct_download_does_not_contact_alternatives() {
        let direct = Server::start(Behavior::Normal).await;
        let alternate = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let result = download_with_policy(
            &[direct.url.clone(), alternate.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await;
        assert_eq!(result.unwrap(), PACKAGE_SIZE as u64);
        assert_eq!(std::fs::read(path).unwrap(), package());
        assert!(alternate.requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn slow_sample_selects_actual_fast_alternative() {
        let slow = Server::start(Behavior::SlowProbe).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        download_with_policy(
            &[slow.url.clone(), fast.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert!(slow.transfer_requests().is_empty());
        assert_eq!(fast.transfer_requests().len(), 1);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn slow_headers_do_not_consume_the_body_probe_budget() {
        let direct = Server::start(Behavior::SlowHeaders).await;
        let alternate = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let mut configuration = policy();
        configuration.header_timeout = Duration::from_millis(500);
        configuration.probe_timeout = Duration::from_millis(100);
        download_with_policy(
            &[direct.url.clone(), alternate.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &configuration,
        )
        .await
        .unwrap();
        assert!(alternate.requests.lock().unwrap().is_empty());
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn stalled_alternatives_are_probed_concurrently() {
        let direct = Server::start(Behavior::StallProbe).await;
        let slow_a = Server::start(Behavior::StallProbe).await;
        let slow_b = Server::start(Behavior::StallProbe).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let configuration = policy();
        let candidates = [
            direct.url.clone(),
            slow_a.url.clone(),
            slow_b.url.clone(),
            fast.url.clone(),
        ];
        let download =
            download_with_policy(&candidates, &path, "signed-1", &|_| {}, &configuration);
        // 首个备选收到请求后，其余备选也应启动；不依赖完整下载的墙钟耗时。
        let observe_concurrency = async {
            loop {
                if !slow_a.requests.lock().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
            assert!(!slow_b.requests.lock().unwrap().is_empty());
            assert!(!fast.requests.lock().unwrap().is_empty());
        };
        tokio::join!(
            async {
                download.await.unwrap();
            },
            observe_concurrency
        );
        assert_eq!(fast.transfer_requests().len(), 1);
    }

    #[tokio::test]
    async fn stalled_transfer_switches_source_and_keeps_prefix() {
        let stalled = Server::start(Behavior::StallBody).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        download_with_policy(
            &[stalled.url.clone(), fast.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert_eq!(fast.transfer_requests()[0].start, 2048);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn trickling_transfer_switches_without_waiting_for_idle() {
        let slow = Server::start(Behavior::TrickleBody).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let mut configuration = policy();
        configuration.idle_timeout = Duration::from_secs(2);
        let started = Instant::now();
        download_with_policy(
            &[slow.url.clone(), fast.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &configuration,
        )
        .await
        .unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(fast.transfer_requests()[0].start > 0);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn short_body_resumes_from_another_source() {
        let broken = Server::start(Behavior::TruncateBody).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        download_with_policy(
            &[broken.url.clone(), fast.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert_eq!(fast.transfer_requests()[0].start, 2048);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn all_slow_sources_finish_in_the_final_fallback_pass() {
        let direct = Server::start(Behavior::SlowBody).await;
        let alternate = Server::start(Behavior::SlowBody).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let mut configuration = policy();
        configuration.minimum_speed = 128 * 1024;
        download_with_policy(
            &[direct.url.clone(), alternate.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &configuration,
        )
        .await
        .unwrap();
        let direct_requests = direct.transfer_requests();
        let alternate_requests = alternate.transfer_requests();
        assert_eq!(direct_requests.len(), 2);
        assert_eq!(alternate_requests.len(), 1);
        assert!(alternate_requests[0].start > 0);
        assert!(direct_requests[1].start > alternate_requests[0].start);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn final_fallback_still_rejects_a_stalled_source() {
        let server = Server::start(Behavior::StallBody).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            download_with_policy(
                std::slice::from_ref(&server.url),
                &path,
                "signed-1",
                &|_| {},
                &policy(),
            ),
        )
        .await
        .expect("最后一轮仍应在 idle_timeout 内退出停流连接");
        assert!(result.unwrap_err().contains("持续停流"));
        assert_eq!(server.transfer_requests().len(), 2);
        assert_eq!(std::fs::read(path).unwrap(), package()[..4096]);
    }

    #[tokio::test]
    async fn persisted_prefix_resumes_with_if_range_on_previous_source() {
        let direct = Server::start(Behavior::Normal).await;
        let previous = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        seed_cache(&path, "signed-1", &previous.url, 4096);
        download_with_policy(
            &[direct.url.clone(), previous.url.clone()],
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert!(direct.requests.lock().unwrap().is_empty());
        assert_eq!(previous.transfer_requests()[0].start, 4096);
        assert_eq!(
            previous.transfer_requests()[0].if_range.as_deref(),
            Some("\"package-v1\"")
        );
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn ignored_range_restarts_instead_of_appending() {
        let server = Server::start(Behavior::IgnoreRange).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        seed_cache(&path, "signed-1", &server.url, 4096);
        download_with_policy(
            std::slice::from_ref(&server.url),
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert_eq!(server.transfer_requests()[0].start, 4096);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[tokio::test]
    async fn invalid_range_and_length_never_append() {
        for behavior in [
            Behavior::WrongRange,
            Behavior::WrongLength,
            Behavior::ChangedEtag,
        ] {
            let server = Server::start(behavior).await;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("update.part");
            seed_cache(&path, "signed-1", &server.url, 4096);
            assert!(download_with_policy(
                std::slice::from_ref(&server.url),
                &path,
                "signed-1",
                &|_| {},
                &policy()
            )
            .await
            .is_err());
            assert_eq!(std::fs::read(path).unwrap(), package()[..4096]);
        }
    }

    #[tokio::test]
    async fn invalid_identity_and_corrupt_metadata_discard_old_prefix() {
        for corrupt in [false, true] {
            let server = Server::start(Behavior::Normal).await;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("update.part");
            seed_cache(&path, "old-signature", &server.url, 4096);
            if corrupt {
                std::fs::write(metadata_path(&path), b"{broken").unwrap();
            }
            download_with_policy(
                std::slice::from_ref(&server.url),
                &path,
                "new-signature",
                &|_| {},
                &policy(),
            )
            .await
            .unwrap();
            assert_eq!(server.transfer_requests()[0].start, 0);
            assert_eq!(std::fs::read(path).unwrap(), package());
        }
    }

    #[tokio::test]
    async fn oversized_or_unbounded_responses_are_rejected_without_writing() {
        for behavior in [Behavior::Oversized, Behavior::UnknownLength] {
            let server = Server::start(behavior).await;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("update.part");
            assert!(download_with_policy(
                std::slice::from_ref(&server.url),
                &path,
                "signed-1",
                &|_| {},
                &policy()
            )
            .await
            .is_err());
            assert_eq!(std::fs::metadata(path).unwrap().len(), 0);
        }
    }

    #[tokio::test]
    async fn dropping_download_stops_writes_and_next_call_resumes() {
        let slow = Server::start(Behavior::TrickleBody).await;
        let fast = Server::start(Behavior::Normal).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.part");
        let mut configuration = policy();
        configuration.slow_window = Duration::from_secs(5);
        let written = tokio::sync::Notify::new();
        let on_progress = |progress: TransferProgress| {
            if progress.downloaded > 0 {
                written.notify_one();
            }
        };
        {
            let candidates = [slow.url.clone()];
            let download =
                download_with_policy(&candidates, &path, "signed-1", &on_progress, &configuration);
            tokio::pin!(download);
            tokio::select! {
                _ = &mut download => panic!("slow response must still be downloading"),
                _ = written.notified() => {},
                _ = tokio::time::sleep(Duration::from_secs(2)) => panic!("download never wrote a prefix"),
            }
        }
        let length = std::fs::metadata(&path).unwrap().len();
        assert!(length > 0 && length < PACKAGE_SIZE as u64);
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(std::fs::metadata(&path).unwrap().len(), length);
        download_with_policy(
            std::slice::from_ref(&fast.url),
            &path,
            "signed-1",
            &|_| {},
            &policy(),
        )
        .await
        .unwrap();
        assert_eq!(fast.transfer_requests()[0].start as u64, length);
        assert_eq!(std::fs::read(path).unwrap(), package());
    }

    #[test]
    fn cached_progress_uses_file_length_and_never_changes_other_version_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("package.part");
        seed_cache(&path, "signed-1", "https://example.com/file", 4096);
        assert_eq!(
            cached_progress(&path, "signed-1"),
            Some((4096, PACKAGE_SIZE as u64))
        );
        assert_eq!(cached_progress(&path, "signed-2"), None);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 4096);
        std::fs::write(&path, vec![0u8; PACKAGE_SIZE + 1]).unwrap();
        assert_eq!(cached_progress(&path, "signed-1"), None);
    }

    #[test]
    fn content_range_is_strict_and_overflow_safe() {
        assert_eq!(parse_content_range("bytes 4-9/10").unwrap(), (4, 9, 10));
        for invalid in [
            "bytes 0-1/*",
            "bytes 9-4/10",
            "bytes 0-10/10",
            "items 0-1/10",
            "bytes 0-18446744073709551615/18446744073709551615",
        ] {
            assert!(parse_content_range(invalid).is_err());
        }
    }

    #[test]
    fn remote_http_and_embedded_credentials_are_rejected() {
        for invalid in [
            "http://example.com/package",
            "https://user:password@example.com/package",
            "ftp://example.com/package",
        ] {
            assert!(!allowed_url(&reqwest::Url::parse(invalid).unwrap()));
        }
        assert!(allowed_url(
            &reqwest::Url::parse("https://example.com/package").unwrap()
        ));
    }
}
