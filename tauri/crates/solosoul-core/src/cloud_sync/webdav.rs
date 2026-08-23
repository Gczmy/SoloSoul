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

impl WebDavConnector {
    /// 单次流式 PUT（A1 修正版）。`if_match` 非空时附带 If-Match 条件头：
    /// 远端 ETag 不匹配返回 412 → [`CloudSyncError::Conflict`]（B-05 并发保护）。
    async fn put_stream(
        &self,
        remote_path: &str,
        reader: Pin<Box<dyn AsyncRead + Send>>,
        total_size: u64,
        if_match: Option<&str>,
    ) -> CloudResult<(String, String)> {
        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(remote_path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str != "." && !parent_str.is_empty() {
                self.ensure_dir(&parent_str).await?;
            }
        }

        // 标准 WebDAV PUT 不支持 Content-Range 部分写（RFC 4918 无此语义）：
        // 单次流式 PUT，请求体边读边发，内存常驻恒定。
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let mut req = self
            .request(Method::PUT, remote_path)
            .header(CONTENT_LENGTH, total_size)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(body);
        if let Some(etag) = if_match {
            req = req.header("If-Match", format!("\"{}\"", etag));
        }
        let resp = req.send().await.map_err(CloudSyncError::Http)?;
        let status = resp.status();
        if status.as_u16() == 412 {
            return Err(CloudSyncError::Conflict(
                "远端已被其他设备更新（ETag 不匹配），需重拉合并后重试".to_string(),
            ));
        }
        if status.is_success() {
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
        // N-003：父目录创建由 put_stream 统一处理，此处不再重复
        self.put_stream(remote_path, reader, total_size, None).await
    }

    /// B-05：条件上传（If-Match）。ETag 不匹配返回 Conflict。
    async fn upload_if_match(
        &self,
        remote_path: &str,
        reader: Pin<Box<dyn AsyncRead + Send>>,
        total_size: u64,
        if_match: Option<&str>,
    ) -> CloudResult<(String, String)> {
        self.put_stream(remote_path, reader, total_size, if_match)
            .await
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
/// B-02：改用 roxmltree 按本地名匹配（忽略命名空间前缀），替代手写前缀剥离 +
/// 字符串提取——对属性顺序/自闭合/编码变体更鲁棒。href 仍需百分号解码。
fn parse_propfind_response(xml: &str, base_path: &str) -> CloudResult<Vec<CloudObjectMeta>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| CloudSyncError::ListFailed(format!("PROPFIND 响应 XML 解析失败: {e}")))?;

    fn local_name<'a, 'input>(node: &'a roxmltree::Node<'a, 'input>) -> &'a str {
        // tag_name().name() 即本地名（不含前缀），与命名空间无关
        node.tag_name().name()
    }

    let mut results = Vec::new();
    for response_node in doc.descendants().filter(|n| local_name(n) == "response") {
        let href = response_node
            .children()
            .find(|n| local_name(n) == "href")
            .and_then(|n| n.text())
            .map(percent_decode)
            .unwrap_or_default();

        // 跳过基路径自身（归一化前导/尾随斜杠后比较）
        let base_trimmed = base_path.trim_matches('/');
        let href_trimmed = href.trim_matches('/');
        if href_trimmed == base_trimmed {
            continue;
        }

        let mut size: u64 = 0;
        let mut modified: Option<DateTime<Utc>> = None;
        let mut etag: Option<String> = None;
        let mut is_collection = false;

        for prop in response_node
            .descendants()
            .filter(|n| local_name(n) == "prop")
        {
            if let Some(n) = prop
                .children()
                .find(|n| local_name(n) == "getcontentlength")
            {
                size = n.text().and_then(|t| t.trim().parse().ok()).unwrap_or(0);
            }
            if let Some(n) = prop.children().find(|n| local_name(n) == "getlastmodified") {
                modified = n
                    .text()
                    .and_then(|t| DateTime::parse_from_rfc2822(t.trim()).ok())
                    .map(|dt| dt.with_timezone(&Utc));
            }
            if let Some(n) = prop.children().find(|n| local_name(n) == "getetag") {
                etag = n.text().map(|t| t.trim().trim_matches('"').to_string());
            }
            if let Some(n) = prop.children().find(|n| local_name(n) == "resourcetype") {
                is_collection = n.children().any(|child| local_name(&child) == "collection");
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

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_parse_propfind_wsgidav_style_uppercase_prefix() {
        // wsgidav 真实输出：大写 D: 前缀 + 目录双 propstat（404/200）
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
 <D:response>
  <D:href>/e2e/listdir/</D:href>
  <D:propstat><D:prop><D:getcontentlength /><D:getetag /></D:prop><D:status>HTTP/1.1 404 Not Found</D:status></D:propstat>
  <D:propstat><D:prop><D:getlastmodified>Sun, 23 Aug 2026 13:20:43 GMT</D:getlastmodified><D:resourcetype><D:collection /></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>
 </D:response>
 <D:response>
  <D:href>/e2e/listdir/one.bin</D:href>
  <D:propstat><D:prop><D:getcontentlength>1024</D:getcontentlength><D:getetag>abc123</D:getetag><D:resourcetype /></D:prop></D:propstat>
 </D:response>
</D:multistatus>"#;
        let items = parse_propfind_response(xml, "/e2e/listdir").unwrap();
        assert_eq!(items.len(), 1, "目录自身应被跳过");
        assert_eq!(items[0].size, 1024);
        assert_eq!(items[0].path, "/e2e/listdir/one.bin");
        assert_eq!(items[0].etag.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_parse_propfind_ns0_prefix_and_relative_base() {
        // ns0 前缀 + 调用方传相对路径（无前导斜杠）——B-01 前实测踩过的坑
        let xml = r#"<ns0:multistatus xmlns:ns0="DAV:"><ns0:response><ns0:href>/dbg2/one.bin</ns0:href><ns0:propstat><ns0:prop><ns0:getcontentlength>10</ns0:getcontentlength></ns0:prop></ns0:propstat></ns0:response></ns0:multistatus>"#;
        let items = parse_propfind_response(xml, "dbg2").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].size, 10);
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("/a%20b/%E4%B8%AD.txt"), "/a b/中.txt");
        assert_eq!(percent_decode("/plain.bin"), "/plain.bin");
    }
}
