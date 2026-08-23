//! WebDAV 连接器端到端集成测试（A1 · 真实协议栈验证）。
//!
//! 运行前提：本地或远程 WebDAV 服务器可达。默认对接 wsgidav 本地实例：
//!
//! ```bash
//! # 启动测试服务器（见 docs/design_map/18_云打包与云同步功能实施记录.md §A1）
//! wsgidav --config /tmp/webdav-e2e/server.json &
//!
//! # 运行（未设置环境变量时自动跳过）
//! SOLOSOUL_WEBDAV_E2E_URL=http://127.0.0.1:9988 \
//! SOLOSOUL_WEBDAV_E2E_USER=solosoul-tester \
//! SOLOSOUL_WEBDAV_E2E_PASS=test-pass-123 \
//! cargo test -p solosoul-core --test cloud_sync_webdav_e2e
//! ```
//!
//! 重点回归：大文件（>8MiB 分片阈值）上传后字节级一致 —— 标准 WebDAV 的 PUT
//! 不支持 `Content-Range` 部分写（RFC 4918 无此语义），若实现按分片顺序覆盖
//! 同一资源，最终内容只剩最后一片，本测试将立即暴露该缺陷。

use std::pin::Pin;

use solosoul_core::cloud_sync::{
    webdav::{WebDavConnector, DEFAULT_CHUNK_SIZE},
    CloudConnector, CloudSyncError, WebDavConfig,
};

fn server_url() -> Option<String> {
    std::env::var("SOLOSOUL_WEBDAV_E2E_URL").ok()
}

fn make_connector(user: &str, pass: &str) -> WebDavConnector {
    WebDavConnector::new(WebDavConfig {
        base_url: server_url().unwrap(),
        username: user.to_string(),
        password: pass.to_string(),
        root_prefix: "/SoloSoul/".to_string(),
    })
    .unwrap() // 测试用合法 URL；非法 URL 行为由 t10 专项覆盖
}

fn creds() -> (String, String) {
    (
        std::env::var("SOLOSOUL_WEBDAV_E2E_USER").unwrap_or_else(|_| "solosoul-tester".into()),
        std::env::var("SOLOSOUL_WEBDAV_E2E_PASS").unwrap_or_else(|_| "test-pass-123".into()),
    )
}

fn unique_prefix() -> String {
    format!("e2e-{}", uuid::Uuid::new_v4().simple())
}

fn tokio_main<F: std::future::Future<Output = Result<(), String>>>(f: F) -> Result<(), String> {
    if server_url().is_none() {
        eprintln!("skip: SOLOSOUL_WEBDAV_E2E_URL 未设置");
        return Ok(());
    }
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

async fn download_bytes(
    c: &WebDavConnector,
    path: &str,
) -> Result<Vec<u8>, solosoul_core::cloud_sync::CloudSyncError> {
    let mut buf = Vec::new();
    {
        let mut writer = std::io::Cursor::new(&mut buf);
        let pinned: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)> = Pin::new(&mut writer);
        c.download(path, pinned).await?;
    }
    Ok(buf)
}

async fn upload_bytes(
    c: &WebDavConnector,
    path: &str,
    data: Vec<u8>,
) -> Result<(String, String), solosoul_core::cloud_sync::CloudSyncError> {
    let size = data.len() as u64;
    let cursor = std::io::Cursor::new(data);
    c.upload(path, Box::pin(cursor), size).await
}

/// ① 连接测试：正确凭据通过；错误凭据报认证失败。
#[test]
fn t01_test_connection_and_auth_failure() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        make_connector(&user, &pass)
            .test_connection()
            .await
            .map_err(|e| e.to_string())?;
        match make_connector(&user, "definitely-wrong")
            .test_connection()
            .await
        {
            Err(solosoul_core::cloud_sync::CloudSyncError::AuthFailed(_)) => {}
            other => {
                return Err(format!(
                    "错误密码应返回 AuthFailed，实际 {:?}",
                    other.map(|_| ())
                ))
            }
        }
        Ok(())
    });
    result.unwrap();
}

/// ② 嵌套目录递归创建（MKCOL 链）+ 目录存在时幂等。
#[test]
fn t02_ensure_dir_nested_and_idempotent() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let deep = format!("{}a/b/c/d", prefix);
        c.ensure_dir(&deep).await.map_err(|e| e.to_string())?;
        // 幂等：再次创建不报错
        c.ensure_dir(&deep).await.map_err(|e| e.to_string())?;
        // 清理
        c.delete(&prefix).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    result.unwrap();
}

/// ③ 小文件上传→HEAD 元数据→下载回读一致。
#[test]
fn t03_small_file_roundtrip() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let path = format!("{}/small.bin", prefix);

        let payload: Vec<u8> = (0..64 * 1024u32).map(|i| (i % 251) as u8).collect();
        let (_, etag) = upload_bytes(&c, &path, payload.clone())
            .await
            .map_err(|e| e.to_string())?;

        let meta = c.head(&path).await.map_err(|e| e.to_string())?;
        assert_eq!(meta.size, payload.len() as u64, "HEAD size 不一致");

        let got = download_bytes(&c, &path).await.map_err(|e| e.to_string())?;
        assert_eq!(got, payload, "小文件回读不一致");
        let _ = etag;

        c.delete(&prefix).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    result.unwrap();
}

/// ④ 大文件（超过 8MiB 分片阈值）上传后字节级一致 —— 核心回归项。
///
/// 若上传实现按 Content-Range 分片对同一资源做多次 PUT，标准服务器会以
/// 「每次整文件覆盖」处理，最终内容仅剩最后一片；本测试断言全量字节相等，
/// 直接暴露该类缺陷。
#[test]
fn t04_large_file_chunked_roundtrip() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let path = format!("{}/large.bin", prefix);

        // 2 × 分片阈值 + 尾部非对齐余量，覆盖「整片×2 + 半片」三种路径
        let total = DEFAULT_CHUNK_SIZE * 2 + 777_777;
        let mut payload = vec![0u8; total as usize];
        // 用可复现伪随机填充（xorshift），避免全零被压缩/去重掩盖问题
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        for b in payload.iter_mut() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *b = (seed & 0xFF) as u8;
        }

        upload_bytes(&c, &path, payload.clone())
            .await
            .map_err(|e| e.to_string())?;

        let meta = c.head(&path).await.map_err(|e| e.to_string())?;
        assert_eq!(meta.size, total, "大文件 HEAD size 与上传不符");

        let got = download_bytes(&c, &path).await.map_err(|e| e.to_string())?;
        assert_eq!(
            got.len() as u64,
            total,
            "下载长度 {} ≠ 上传长度 {} —— 分片上传疑似发生覆盖",
            got.len(),
            total
        );
        assert_eq!(got, payload, "大文件回读字节不一致 —— 分片覆盖缺陷实锤");

        c.delete(&prefix).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    result.unwrap();
}

/// ⑤ PROPFIND 列表：Depth 1 返回目录下文件（含大小），不含集合本身。
#[test]
fn t05_list_directory_entries() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let dir = format!("{}/listdir", prefix);
        c.ensure_dir(&dir).await.map_err(|e| e.to_string())?;

        for name in ["one.bin", "two.bin"] {
            upload_bytes(&c, &format!("{}/{}", dir, name), vec![1u8; 1024])
                .await
                .map_err(|e| e.to_string())?;
        }

        let entries = c.list(&dir).await.map_err(|e| e.to_string())?;
        let names: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
        assert!(
            names.iter().any(|n| n.ends_with("one.bin")),
            "列表缺少 one.bin，实际: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.ends_with("two.bin")),
            "列表缺少 two.bin"
        );
        for e in &entries {
            assert_eq!(e.size, 1024, "列表条目 size 应为 1024");
        }

        c.delete(&prefix).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    result.unwrap();
}

/// ⑥ 删除后资源不可再访问（HEAD 报 NotFound）。
#[test]
fn t06_delete_makes_resource_gone() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let path = format!("{}/gone.bin", prefix);
        upload_bytes(&c, &path, vec![7u8; 128])
            .await
            .map_err(|e| e.to_string())?;
        c.delete(&path).await.map_err(|e| e.to_string())?;

        match c.head(&path).await {
            Err(CloudSyncError::NotFound(_)) => {}
            other => {
                return Err(format!(
                    "删除后 HEAD 应 NotFound，实际 {:?}",
                    other.map(|m| m.size)
                ))
            }
        }
        Ok(())
    });
    result.unwrap();
}

/// ⑦ 上行路径不存在时的错误语义：GET 缺失资源报 DownloadFailed/NotFound 而非 panic。
#[test]
fn t07_download_missing_is_typed_error() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let path = format!("{}missing/never-exists.bin", unique_prefix());
        let mut buf = Vec::new();
        {
            let mut writer = std::io::Cursor::new(&mut buf);
            let pinned: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)> =
                Pin::new(&mut writer);
            match c.download(&path, pinned).await {
                Err(CloudSyncError::DownloadFailed(_) | CloudSyncError::NotFound(_)) => {}
                other => {
                    return Err(format!(
                        "缺失资源应返回类型化错误，实际 {:?}",
                        other.map(|_| ())
                    ))
                }
            }
        }
        Ok(())
    });
    result.unwrap();
}

/// ⑩ N-001 回归：非法服务器 URL 返回类型化错误而非 panic。
#[test]
fn t10_invalid_url_returns_err_not_panic() {
    // 不依赖服务器，纯构造路径
    for bad in [
        "",
        "not-a-url",
        "http://",
        "ftp://x",
        "https://host with space",
    ] {
        let result = std::panic::catch_unwind(|| {
            WebDavConnector::new(WebDavConfig {
                base_url: bad.to_string(),
                username: "u".into(),
                password: "p".into(),
                root_prefix: "/SoloSoul/".to_string(),
            })
        });
        match result {
            Ok(Err(solosoul_core::cloud_sync::CloudSyncError::ConfigMissing(_))) => {}
            Ok(Ok(_)) => panic!("URL {:?} 应被拒绝", bad),
            Ok(Err(other)) => panic!("URL {:?} 应返回 ConfigMissing，实际 {:?}", bad, other),
            Err(_) => panic!("URL {:?} 构造发生 panic —— N-001 回归", bad),
        }
    }
}

/// ⑧ B-05 回归：If-Match 条件上传——正确 ETag 成功、过期 ETag 返回 Conflict。
#[test]
fn t08_upload_if_match_conflict_detection() {
    let result = tokio_main(async {
        let (user, pass) = creds();
        let c = make_connector(&user, &pass);
        let prefix = unique_prefix();
        let path = format!("{}/cond.json", prefix);
        c.ensure_dir(&prefix).await.map_err(|e| e.to_string())?;

        use std::io::Cursor;
        use std::pin::Pin;
        // 各版本使用不同长度：wsgidav 等 ETag 基于 mtime+size，
        // 同尺寸同秒重写会产生相同 ETag 导致无法触发 412。
        // 首次无条件写入 v1（7 字节）
        let (_, etag_v1) = upload_bytes(&c, &path, br#"{"v":1}"#.to_vec())
            .await
            .map_err(|e| e.to_string())?;
        assert!(!etag_v1.is_empty(), "服务器应返回 ETag");

        // 正确 ETag → 成功（8 字节，size 变化确保 ETag 必然更新）
        let (_, _etag_v2) = {
            let cursor = Cursor::new(br#"{"v":22}"#.to_vec());
            c.upload_if_match(&path, Box::pin(cursor), 8, Some(&etag_v1))
                .await
                .map_err(|e| e.to_string())?
        };

        // 用旧 ETag 再写（9 字节）→ 当前 ETag 已变为非 etag_v1 → 应返回 Conflict
        let stale = Cursor::new(br#"{"v":333}"#.to_vec());
        match c
            .upload_if_match(&path, Box::pin(stale), 9, Some(&etag_v1))
            .await
        {
            Err(solosoul_core::cloud_sync::CloudSyncError::Conflict(_)) => {}
            other => {
                return Err(format!(
                    "过期 ETag 应返回 Conflict，实际 {:?}",
                    other.map(|_| ())
                ))
            }
        }

        // None = 无条件写 → 也应成功
        let none_write = Cursor::new(br#"{"v":4444}"#.to_vec());
        c.upload_if_match(&path, Box::pin(none_write), 10, None)
            .await
            .map_err(|e| e.to_string())?;

        c.delete(&prefix).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    result.unwrap();
}
