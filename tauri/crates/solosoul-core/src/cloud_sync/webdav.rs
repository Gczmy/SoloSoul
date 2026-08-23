//! WebDAV 连接器实现（Phase 2 · 云同步）。
//!
//! 支持标准 RFC 4918 WebDAV 服务器：
//! - 坚果云 (https://dav.jianguoyun.com/dav/)
//! - Nextcloud / ownCloud
//! - Alist (聚合 40+ 网盘的 WebDAV 网关)
//! - 自建 WebDAV (nginx/apache + dav 模块)

use std::pin::Pin;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Client, Method, RequestBuilder};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::cloud_sync::{
    CloudConnector, CloudObjectMeta, CloudResult, CloudSyncError, WebDavConfig,
};

/// 默认分片大小：8 MiB（坚果云/Nextcloud 均支持更大，8M 平衡内存与并发）。
pub const DEFAULT_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
/// WebDAV 连接器。
#[derive(Clone)]
pub struct WebDavConnector {
    client: Client,
    auth_header: HeaderValue,
    base_url: url::Url,
}

/// N-003：非标准方法名集中构造（`from_bytes` 对合法字节串不可失败，unwrap 集中一处）。
fn method(name: &'static [u8]) -> Method {
    Method::from_bytes(name).expect("静态 HTTP 方法名必为合法值")
}

impl WebDavConnector {
    /// N-001：构造不再 panic——`base_url` 来自用户输入（Settings 表单），
    /// 非法 URL 返回 `ConfigMissing` 而非 `expect` abort（Android Release
    /// panic=abort 下会直接闪退）。
    pub fn new(config: WebDavConfig) -> CloudResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| CloudSyncError::Internal(format!("HTTP 客户端初始化失败: {e}")))?;

        let auth = format!("{}:{}", config.username, config.password);
        // Base64 输出恒为 ASCII，HeaderValue 解析实际不可失败；防御性映射。
        let auth_header = HeaderValue::from_str(&format!("Basic {}", BASE64.encode(auth)))
            .map_err(|e| CloudSyncError::Internal(format!("认证头构造失败: {e}")))?;

        let mut base: url::Url = url::Url::parse(&config.base_url).map_err(|e| {
            CloudSyncError::ConfigMissing(format!(
                "服务器地址非法（需形如 https://host/path/）：{} ({})",
                config.base_url, e
            ))
        })?;
        if !matches!(base.scheme(), "http" | "https") {
            return Err(CloudSyncError::ConfigMissing(format!(
                "服务器地址 scheme 必须为 http/https，实际为 {}:{}",
                base.scheme(),
                config.base_url
            )));
        }
        // 确保以 '/' 结尾
        if !base.as_str().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }

        Ok(Self {
            client,
            auth_header,
            base_url: base,
        })
    }

    /// 拼接远程路径（相对路径 → 绝对 URL）。
    fn join_path(&self, remote_path: &str) -> url::Url {
        let mut url = self.base_url.clone();
        let path = remote_path.trim_start_matches('/');
        url.set_path(&format!("{}{}", self.base_url.path(), path));
        url
    }

    /// 构建带认证的请求。
    fn request(&self, method: Method, remote_path: &str) -> RequestBuilder {
        let url = self.join_path(remote_path);
        self.client
            .request(method, url)
            .header(AUTHORIZATION, self.auth_header.clone())
    }
}

#[async_trait]
impl CloudConnector for WebDavConnector {
    fn connector_type(&self) -> &'static str {
        "webdav"
    }

    async fn test_connection(&self) -> CloudResult<()> {
        // PROPFIND 根目录，Depth: 0
        let resp = self.request(method(b"PROPFIND"), "")
            .header("Depth", "0")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(r#"<?xml version="1.0"?><propfind xmlns="DAV:"><prop><resourcetype/></prop></propfind>"#)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        match resp.status().as_u16() {
            207 | 200 => Ok(()),
            401 | 403 => Err(CloudSyncError::AuthFailed(
                "WebDAV 认证失败（用户名/密码错误）".to_string(),
            )),
            s => Err(CloudSyncError::ConnectFailed(format!(
                "PROPFIND 失败: {}",
                s
            ))),
        }
    }

    async fn ensure_dir(&self, remote_path: &str) -> CloudResult<()> {
        // WebDAV 规范：MKCOL 创建集合（目录），父目录必须存在。
        // 这里递归创建父目录链。
        let segments: Vec<&str> = remote_path.trim_matches('/').split('/').collect();
        let mut current = String::new();
        for seg in segments {
            current.push('/');
            current.push_str(seg);
            // PROPFIND 检查是否已存在
            let check = self.request(method(b"PROPFIND"), &current)
                .header("Depth", "0")
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(r#"<?xml version="1.0"?><propfind xmlns="DAV:"><prop><resourcetype/></prop></propfind>"#)
                .send()
                .await;
            match check {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 207 => {
                    // 已存在，继续下一级
                }
                _ => {
                    // 不存在，MKCOL 创建
                    let mkcol = self
                        .request(method(b"MKCOL"), &current)
                        .send()
                        .await
                        .map_err(CloudSyncError::Http)?;
                    if !mkcol.status().is_success() && mkcol.status().as_u16() != 405 {
                        // 405 Method Not Allowed 可能是已存在文件同名，忽略
                        return Err(CloudSyncError::UploadFailed(format!(
                            "MKCOL 创建目录失败 {}: {}",
                            current,
                            mkcol.status()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    async fn upload(
        &self,
        remote_path: &str,
        reader: Pin<Box<dyn AsyncRead + Send>>,
        total_size: u64,
    ) -> CloudResult<(String, String)> {
        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(remote_path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str != "." && !parent_str.is_empty() {
                self.ensure_dir(&parent_str).await?;
            }
        }

        // A1 实测修正：标准 WebDAV PUT 不支持 Content-Range 部分写（RFC 4918 无此
        // 语义，wsgidav 返回 400、坚果云同样拒绝）。改为单次流式 PUT：请求体经
        // ReaderStream 边读边发，内存常驻恒定，任意大小文件均适用。
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let resp = self
            .request(Method::PUT, remote_path)
            .header(CONTENT_LENGTH, total_size)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(body)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if resp.status().is_success() {
            let etag = resp
                .headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_default();
            Ok((remote_path.to_string(), etag))
        } else {
            Err(CloudSyncError::UploadFailed(format!(
                "PUT 失败: {}",
                resp.status()
            )))
        }
    }

    async fn download(
        &self,
        remote_path: &str,
        mut writer: Pin<&mut (dyn AsyncWrite + Send + Unpin)>,
    ) -> CloudResult<u64> {
        let resp = self
            .request(Method::GET, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() {
            return Err(CloudSyncError::DownloadFailed(format!(
                "GET 失败: {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream();
        let mut total: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(CloudSyncError::Http)?;
            writer.write_all(&bytes).await.map_err(CloudSyncError::Io)?;
            total += bytes.len() as u64;
        }
        writer.flush().await.map_err(CloudSyncError::Io)?;
        Ok(total)
    }

    async fn list(&self, remote_path: &str) -> CloudResult<Vec<CloudObjectMeta>> {
        // PROPFIND Depth: 1
        let body = r#"<?xml version="1.0"?><propfind xmlns="DAV:"><prop><getcontentlength/><getlastmodified/><getetag/><resourcetype/></prop></propfind>"#;
        let resp = self
            .request(method(b"PROPFIND"), remote_path)
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() && resp.status().as_u16() != 207 {
            return Err(CloudSyncError::ListFailed(format!(
                "PROPFIND 失败: {}",
                resp.status()
            )));
        }

        let text = resp.text().await.map_err(CloudSyncError::Http)?;
        parse_propfind_response(&text, remote_path)
    }

    async fn delete(&self, remote_path: &str) -> CloudResult<()> {
        let resp = self
            .request(Method::DELETE, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            Err(CloudSyncError::DeleteFailed(format!(
                "DELETE 失败: {}",
                resp.status()
            )))
        }
    }

    async fn head(&self, remote_path: &str) -> CloudResult<CloudObjectMeta> {
        let resp = self
            .request(Method::HEAD, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() {
            return Err(CloudSyncError::NotFound(format!(
                "HEAD 失败: {}",
                resp.status()
            )));
        }

        let headers = resp.headers();
        let size = headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let modified = headers
            .get("Last-Modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let etag = headers
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());

        Ok(CloudObjectMeta {
            path: remote_path.to_string(),
            size,
            modified,
            etag,
        })
    }
}

/// 解析 PROPFIND 多状态响应。
///
/// A1 实测修正：不同服务器命名空间前缀不一（wsgidav 用 `D:`、其他常见 `d:`/`ns0:`），
/// 先将所有标签的命名空间前缀剥离归一化为无前缀形式，再做简易提取。
fn parse_propfind_response(xml: &str, base_path: &str) -> CloudResult<Vec<CloudObjectMeta>> {
    let normalized = strip_xml_ns_prefixes(xml);

    let mut results = Vec::new();
    for block in normalized.split("<response>").skip(1) {
        let mut href = String::new();
        let mut size: u64 = 0;
        let mut modified: Option<DateTime<Utc>> = None;
        let mut etag: Option<String> = None;
        let mut is_collection = false;

        if let Some(h) = extract_xml_tag(block, "href") {
            // wsgidav 等 server 返回的 href 可能经 URL 编码，先解码再比较
            href = percent_decode(&h);
        }

        let base_trimmed = base_path.trim_end_matches('/');
        // A1 实测修正：调用方可能传相对路径（无前导 /），而 href 恒为绝对路径；
        // 双方均剥离前导 / 后再比较，否则基路径目录自身会混入结果（size=0）。
        let href_trimmed = href.trim_start_matches('/').trim_end_matches('/');
        if href_trimmed == base_trimmed.trim_start_matches('/') {
            continue; // 跳过基路径自身
        }

        if let Some(prop_block) = extract_xml_tag(block, "prop") {
            if let Some(cl) = extract_xml_tag(&prop_block, "getcontentlength") {
                size = cl.trim().parse().unwrap_or(0);
            }
            if let Some(lm) = extract_xml_tag(&prop_block, "getlastmodified") {
                modified = DateTime::parse_from_rfc2822(lm.trim())
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            }
            if let Some(et) = extract_xml_tag(&prop_block, "getetag") {
                etag = Some(et.trim().trim_matches('"').to_string());
            }
            if let Some(rt) = extract_xml_tag(&prop_block, "resourcetype") {
                is_collection = rt.contains("collection");
            }
        }

        if is_collection {
            continue; // 仅返回文件
        }

        results.push(CloudObjectMeta {
            path: href,
            size,
            modified: modified.unwrap_or_else(Utc::now),
            etag,
        });
    }

    Ok(results)
}

/// 剥离 XML 标签的命名空间前缀：`<D:response>` / `<ns0:href ...>` → `<response>` / `<href ...>`。
///
/// 仅重写标签名的 `prefix:` 部分，属性原样保留；文本节点不触碰。
fn strip_xml_ns_prefixes(xml: &str) -> String {
    let bytes = xml.as_bytes();
    let mut out = String::with_capacity(xml.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(rel) = xml[i..].find('>') {
                let end = i + rel; // '>' 位置
                out.push_str(&rewrite_tag(&xml[i..=end]));
                i = end + 1;
                continue;
            }
        }
        // 非 ASCII 字符按字节拷贝会破坏 UTF-8；以 char 边界推进
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&xml[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1, // 连续字节不应单独出现，防御性回退
    }
}

/// 重写单个标签（含 `<` 与 `>`）：去掉标签名中的命名空间前缀。
fn rewrite_tag(tag: &str) -> String {
    debug_assert!(tag.starts_with('<') && tag.ends_with('>'));
    let inner = &tag[1..tag.len() - 1];
    let (is_close, body) = if let Some(rest) = inner.strip_prefix('/') {
        (true, rest)
    } else {
        (false, inner)
    };
    // 自闭合 `<D:x/>`
    let self_close = body.ends_with('/');
    let body_trimmed = if self_close {
        &body[..body.len() - 1]
    } else {
        body
    };

    // 标签名 = 第一个空白或结尾之前的部分；仅当其中含 ':' 且前缀为合法 NCName 时剥离
    let name_end = body_trimmed
        .find(|ch: char| ch.is_whitespace())
        .unwrap_or(body_trimmed.len());
    let (name, rest) = body_trimmed.split_at(name_end);
    let cleaned_name = match name.split_once(':') {
        Some((_prefix, local)) => local,
        None => name,
    };

    let mut s = String::with_capacity(tag.len());
    s.push('<');
    if is_close {
        s.push('/');
    }
    s.push_str(cleaned_name);
    s.push_str(rest);
    if self_close {
        s.push('/');
    }
    s.push('>');
    s
}

/// 极简百分号解码（WebDAV href 常见场景：路径段中的空格/中文/特殊字符）。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// 从 XML 片段提取首个 `<name ...>...</name>` 的内容（兼容带属性的开放标签）。
fn extract_xml_tag(xml: &str, name: &str) -> Option<String> {
    let open_candidates = [format!("<{}>", name), format!("<{} ", name)];
    for open in &open_candidates {
        if let Some(start) = xml.find(open.as_str()) {
            // 开放标签内容起点：跳过到 '>' 之后
            let content_start = start + open.len().min(xml.len() - start);
            let content_start = if open.ends_with('>') {
                content_start
            } else {
                match xml[start..].find('>') {
                    Some(gt) => start + gt + 1,
                    None => continue,
                }
            };
            let close = format!("</{}>", name);
            if let Some(end_rel) = xml[content_start..].find(close.as_str()) {
                return Some(
                    xml[content_start..content_start + end_rel]
                        .trim()
                        .to_string(),
                );
            }
        }
    }
    None
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_strip_ns_prefixes_variants() {
        let input = r#"<?xml version="1.0"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/a/one.bin</D:href><D:propstat><D:prop><D:getcontentlength>1024</D:getcontentlength></D:prop></D:propstat></D:response></D:multistatus>"#;
        let out = strip_xml_ns_prefixes(input);
        assert!(out.contains("<response>"), "大写前缀应被剥离: {}", out);
        assert!(out.contains("<href>/a/one.bin</href>"));
        assert!(out.contains("<getcontentlength>1024</getcontentlength>"));

        let ns0 = r#"<ns0:multistatus xmlns:ns0="DAV:"><ns0:response/></ns0:multistatus>"#;
        let out2 = strip_xml_ns_prefixes(ns0);
        assert!(out2.contains("<response/>"));
        assert!(out2.contains("<multistatus "));
    }

    #[test]
    fn test_parse_propfind_wsgidav_style() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
 <D:response>
  <D:href>/e2e/listdir/</D:href>
  <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat>
 </D:response>
 <D:response>
  <D:href>/e2e/listdir/one.bin</D:href>
  <D:propstat><D:prop><D:getcontentlength>1024</D:getcontentlength><D:resourcetype/></D:prop></D:propstat>
 </D:response>
</D:multistatus>"#;
        let items = parse_propfind_response(xml, "/e2e/listdir").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].size, 1024);
        assert_eq!(items[0].path, "/e2e/listdir/one.bin");
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("/a%20b/%E4%B8%AD.txt"), "/a b/中.txt");
        assert_eq!(percent_decode("/plain.bin"), "/plain.bin");
    }
}
