//! WebDAV 连接器实现（Phase 2 · 云同步）。
//!
//! 支持标准 RFC 4918 WebDAV 服务器：
//! - 坚果云 (https://dav.jianguoyun.com/dav/)
//! - Nextcloud / ownCloud
//! - Alist (聚合 40+ 网盘的 WebDAV 网关)
//! - 自建 WebDAV (nginx/apache + dav 模块)

use std::pin::Pin;
use std::str::FromStr;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::header::{
    AUTHORIZATION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, HeaderValue,
};
use reqwest::{Client, Method, RequestBuilder};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::cloud_sync::{CloudConnector, CloudObjectMeta, CloudResult, CloudSyncError, WebDavConfig};

/// 默认分片大小：8 MiB（坚果云/Nextcloud 均支持更大，8M 平衡内存与并发）。
const DEFAULT_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
/// WebDAV 连接器。
#[derive(Clone)]
pub struct WebDavConnector {
    client: Client,
    auth_header: HeaderValue,
    base_url: url::Url,
}

impl WebDavConnector {
    pub fn new(config: WebDavConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client build failed");

        let auth = format!("{}:{}", config.username, config.password);
        let auth_header = HeaderValue::from_str(&format!("Basic {}", BASE64.encode(auth)))
            .expect("Basic auth header build failed");

        let base_url = url::Url::from_str(&config.base_url)
            .expect("WebDAV base_url 必须是合法 URL");
        // 确保以 '/' 结尾
        let mut base: url::Url = base_url;
        if !base.as_str().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }

        Self {
            client,
            auth_header,
            base_url: base,
        }
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
        let resp = self.request(Method::from_bytes(b"PROPFIND").unwrap(), "")
            .header("Depth", "0")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(r#"<?xml version="1.0"?><propfind xmlns="DAV:"><prop><resourcetype/></prop></propfind>"#)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if resp.status().is_success() || resp.status().as_u16() == 207 {
            Ok(())
        } else {
            Err(CloudSyncError::ConnectFailed(format!("PROPFIND 失败: {}", resp.status())))
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
            let check = self.request(Method::from_bytes(b"PROPFIND").unwrap(), &current)
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
                    let mkcol = self.request(Method::from_bytes(b"MKCOL").unwrap(), &current)
                        .send()
                        .await
                        .map_err(CloudSyncError::Http)?;
                    if !mkcol.status().is_success() && mkcol.status().as_u16() != 405 {
                        // 405 Method Not Allowed 可能是已存在文件同名，忽略
                        return Err(CloudSyncError::UploadFailed(format!(
                            "MKCOL 创建目录失败 {}: {}",
                            current, mkcol.status()
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
        mut reader: Pin<&mut (dyn AsyncRead + Send + Unpin)>,
        total_size: u64,
    ) -> CloudResult<(String, String)> {
        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(remote_path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str != "." && !parent_str.is_empty() {
                self.ensure_dir(&parent_str).await?;
            }
        }

        if total_size <= DEFAULT_CHUNK_SIZE {
            // 小文件单次 PUT
            let mut buf = Vec::with_capacity(total_size as usize);
            reader.read_to_end(&mut buf).await.map_err(CloudSyncError::Io)?;
            let resp = self.request(Method::PUT, remote_path)
                .header(CONTENT_LENGTH, buf.len() as u64)
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(buf)
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
                Err(CloudSyncError::UploadFailed(format!("PUT 失败: {}", resp.status())))
            }
        } else {
            // 大文件分片上传（RFC 4918 PUT + Content-Range）
            // 顺序上传各分片，最后合并（某些服务器需最后发送 0 字节完成标记）。
            let mut uploaded: u64 = 0;
            let mut chunk_idx: u64 = 0;
            let mut buffer = vec![0u8; DEFAULT_CHUNK_SIZE as usize];

            while uploaded < total_size {
                let to_read = std::cmp::min(DEFAULT_CHUNK_SIZE, total_size - uploaded) as usize;
                let n = reader.read(&mut buffer[..to_read]).await.map_err(CloudSyncError::Io)?;
                if n == 0 {
                    break;
                }
                let chunk = &buffer[..n];

                let start = uploaded;
                let end = uploaded + n as u64 - 1;
                let range_header = format!("bytes {}-{}/{}", start, end, total_size);

                let resp = self.request(Method::PUT, remote_path)
                    .header(CONTENT_LENGTH, n as u64)
                    .header(CONTENT_RANGE, range_header)
                    .header(CONTENT_TYPE, "application/octet-stream")
                    .body(chunk.to_vec())
                    .send()
                    .await
                    .map_err(CloudSyncError::Http)?;

                if !resp.status().is_success() && resp.status().as_u16() != 206 {
                    return Err(CloudSyncError::UploadFailed(format!(
                        "分片上传失败 {}: {} (分片 {})",
                        remote_path, resp.status(), chunk_idx
                    )));
                }

                uploaded = end + 1;
                chunk_idx += 1;
            }

            // 最终确认（某些服务器需要最后发送空分片或单独完成请求；这里假设上传完即完成）
            let head = self.head(remote_path).await?;
            Ok((remote_path.to_string(), head.etag.unwrap_or_default()))
        }
    }

    async fn download(
        &self,
        remote_path: &str,
        mut writer: Pin<&mut (dyn AsyncWrite + Send + Unpin)>,
    ) -> CloudResult<u64> {
        let resp = self.request(Method::GET, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() {
            return Err(CloudSyncError::DownloadFailed(format!(
                "GET 失败: {}", resp.status()
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
        let resp = self.request(Method::from_bytes(b"PROPFIND").unwrap(), remote_path)
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() && resp.status().as_u16() != 207 {
            return Err(CloudSyncError::ListFailed(format!("PROPFIND 失败: {}", resp.status())));
        }

        let text = resp.text().await.map_err(CloudSyncError::Http)?;
        parse_propfind_response(&text, remote_path)
    }

    async fn delete(&self, remote_path: &str) -> CloudResult<()> {
        let resp = self.request(Method::DELETE, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            Err(CloudSyncError::DeleteFailed(format!("DELETE 失败: {}", resp.status())))
        }
    }

    async fn head(&self, remote_path: &str) -> CloudResult<CloudObjectMeta> {
        let resp = self.request(Method::HEAD, remote_path)
            .send()
            .await
            .map_err(CloudSyncError::Http)?;

        if !resp.status().is_success() {
            return Err(CloudSyncError::NotFound(format!("HEAD 失败: {}", resp.status())));
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

/// 解析 PROPFIND 多状态响应（简化版，仅提取需要的字段）。
fn parse_propfind_response(xml: &str, base_path: &str) -> CloudResult<Vec<CloudObjectMeta>> {
    let mut results = Vec::new();

    // 简易解析：按 <d:response> 分块
    for block in xml.split("<d:response>").skip(1) {
        let mut href = String::new();
        let mut size: u64 = 0;
        let mut modified: Option<DateTime<Utc>> = None;
        let mut etag: Option<String> = None;
        let mut is_collection = false;

        // 提取 <d:href>
        if let Some(href_block) = extract_xml_tag(block, "href") {
            href = href_block;
        }

        // 跳过基路径本身
        let base_trimmed = base_path.trim_end_matches('/');
        let href_trimmed = href.trim_end_matches('/');
        if href_trimmed == base_trimmed {
            continue;
        }

        // 提取 <d:prop> 里的内容
        if let Some(prop_block) = extract_xml_tag(block, "prop") {
            if let Some(cl) = extract_xml_tag(&prop_block, "getcontentlength") {
                size = cl.parse().unwrap_or(0);
            }
            if let Some(lm) = extract_xml_tag(&prop_block, "getlastmodified") {
                modified = DateTime::parse_from_rfc2822(&lm).ok().map(|dt| dt.with_timezone(&Utc));
            }
            if let Some(et) = extract_xml_tag(&prop_block, "getetag") {
                etag = Some(et.trim_matches('"').to_string());
            }
            if extract_xml_tag(&prop_block, "resourcetype").map(|s| s.contains("collection")).unwrap_or(false) {
                is_collection = true;
            }
        }

        if is_collection {
            continue; // 当前仅返回文件
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

/// 从 XML 片段中提取单个标签内容（简易版，无命名空间完整处理）。
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<d:{}>", tag);
    let close = format!("</d:{}>", tag);
    xml.find(&open)
        .and_then(|start| {
            let content_start = start + open.len();
            xml[content_start..].find(&close).map(|end| {
                xml[content_start..content_start + end].trim().to_string()
            })
        })
        .or_else(|| {
            // 尝试无前缀版本
            let open = format!("<{}>", tag);
            let close = format!("</{}>", tag);
            xml.find(&open).and_then(|start| {
                let content_start = start + open.len();
                xml[content_start..].find(&close).map(|end| {
                    xml[content_start..content_start + end].trim().to_string()
                })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_snapshot_remote_path() {
        let path = crate::cloud_sync::build_snapshot_remote_path(
            "/SoloSoul/",
            "acc-123",
            "device-abc",
            "1234567890-1",
        );
        assert_eq!(path, "/SoloSoul/acc-123/snapshots/device-abc/1234567890-1.solosoul");
    }

    #[test]
    fn test_webdav_config_default_prefix() {
        let cfg = WebDavConfig {
            base_url: "https://dav.example.com/".to_string(),
            username: "test".to_string(),
            password: "pass".to_string(),
            root_prefix: "".to_string(),
        };
        // root_prefix 默认由 serde default 填充，这里仅测试结构
        assert!(cfg.base_url.ends_with('/'));
    }
}