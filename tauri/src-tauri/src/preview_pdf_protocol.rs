//! 自定义 URI scheme 协议：`solosoul-pdf://` —— 为附件 PDF 内嵌预览服务。
//!
//! # 背景（Windows 端 PDF 预览失败修复）
//!
//! 1. **WebView2/Chromium 内建 PDF 查看器无法从 `data:`/`blob:` URL 可靠渲染
//!    `<embed>`**——`data:` 无源标识 PDFium 拒绝加载；`blob:` 在 WebView2 子帧
//!    中同样不稳定（空白/无法打开）。旧实现走 `fs_read_file_as_data_url` 拿
//!    `data:application/pdf;base64,...` 塞进 `<embed>`，在 Windows 上必然失败。
//! 2. **`fs_read_file_as_data_url` 有 10 MiB 上限**（`MAX_DATA_URL_SIZE`）——真实
//!    扫描 PDF 经常超限，invoke 直接 reject，前端落入「Failed to load」错误态。
//!
//! # 方案
//!
//! 注册自定义协议 `solosoul-pdf://`，请求路径携带 URL 编码的 vault 附件绝对路径：
//!
//! - macOS/Linux：`solosoul-pdf://localhost/<urlencoded path>`
//! - Windows/Android：`http://solosoul-pdf.localhost/<urlencoded path>`
//!
//! 处理器经既有 `commands::fs::resolve_allowed_path` 白名单校验（与 fs 命令同一
//! 安全边界，动态尊重 SOLOSOUL_FS_BASE 与实际 vault 目录），扩展名守卫仅放行
//! `.pdf`，直接读取文件字节并以 `Content-Type: application/pdf` 响应。WebView2
//! 的 PDF 查看器按常规 HTTP 资源处理该响应——可靠渲染，且无 base64 膨胀、无
//! 10 MiB 上限。
//!
//! # 安全
//!
//! - 路径经 `resolve_allowed_path` 白名单校验（越界/穿越/不存在均拒绝）
//! - 扩展名守卫：非 `.pdf` 一律 404（本协议仅服务 PDF 预览）
//! - 256 MiB 读取上限防恶意超大文件拖垮进程
//! - 不响应前端任意请求体，仅按 URL 路径取白名单内文件

use tauri::http::{header, Request, Response, StatusCode};

/// 协议可服务的最大 PDF 大小（防恶意超大文件 OOM；正常文档远小于此）。
const MAX_PDF_PREVIEW_SIZE: u64 = 256 * 1024 * 1024;

/// 手动百分号解码（前端 encodeURIComponent 产出 `%XX` 大写十六进制；双方受控，
/// 无需引入 percent-encoding 依赖）。非法转义返回 Err。
fn percent_decode(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(());
            }
            let hi = hex_val(bytes[i + 1]).ok_or(())?;
            let lo = hex_val(bytes[i + 2]).ok_or(())?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 把自定义协议注册到 Tauri builder。
pub fn register<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol("solosoul-pdf", |ctx, request, responder| {
        let app = ctx.app_handle().clone();
        // 文件读取放到独立线程，避免阻塞协议回调线程。
        std::thread::spawn(move || {
            let response = handle_request(&app, request);
            responder.respond(response);
        });
    })
}

/// 处理单次协议请求（纯函数便于单测）。
fn handle_request<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    // URI path 形如 `/<urlencoded absolute path>`（两种平台 URL 形态 path 部分一致）
    let raw = request.uri().path();
    let decoded = match percent_decode(raw.trim_start_matches('/')) {
        Ok(d) => d,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "malformed url encoding"),
    };

    // 白名单校验（与 fs_read_file_as_data_url 同一安全边界；返回 canonical 路径）
    let resolved = match crate::commands::fs::resolve_allowed_path(app, &decoded) {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::FORBIDDEN, "path not allowed"),
    };

    // 扩展名守卫：仅服务 PDF
    let is_pdf = resolved
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return error_response(StatusCode::NOT_FOUND, "not a pdf");
    }

    let meta = match std::fs::metadata(&resolved) {
        Ok(m) => m,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "file missing"),
    };
    if meta.len() > MAX_PDF_PREVIEW_SIZE {
        return error_response(StatusCode::PAYLOAD_TOO_LARGE, "pdf too large");
    }

    match std::fs::read(&resolved) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/pdf")
            .header(header::CONTENT_LENGTH, bytes.len().to_string())
            .body(bytes)
            .unwrap(),
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "read failed"),
    }
}

fn error_response(status: StatusCode, msg: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(msg.as_bytes().to_vec())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_request(uri: &str) -> Request<Vec<u8>> {
        Request::builder().uri(uri).body(Vec::new()).unwrap()
    }

    /// 模拟前端 encodeURIComponent：仅保留 unreserved 字符，其余 `%XX` 大写编码。
    fn enc(path: &std::path::Path) -> String {
        let s = path.to_string_lossy();
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    #[test]
    fn percent_decode_roundtrip() {
        assert_eq!(percent_decode("a%2Fb%20c"), Ok("a/b c".to_string()));
        assert_eq!(percent_decode("plain"), Ok("plain".to_string()));
        assert_eq!(percent_decode("%GG"), Err(()));
        assert_eq!(percent_decode("trailing%"), Err(()));
    }

    #[test]
    fn serves_pdf_inside_whitelist() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("doc.pdf");
        fs::write(&path, b"%PDF-1.4 test").unwrap();

        let resp = handle_request(
            &app.handle().clone(),
            make_request(&format!("solosoul-pdf://localhost/{}", enc(&path))),
        );
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/pdf"
        );
        assert_eq!(resp.body(), b"%PDF-1.4 test");
    }

    #[test]
    fn rejects_non_pdf_extension() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("notes.txt");
        fs::write(&path, "hello").unwrap();

        let resp = handle_request(
            &app.handle().clone(),
            make_request(&format!("solosoul-pdf://localhost/{}", enc(&path))),
        );
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn rejects_missing_file() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ghost.pdf");

        let resp = handle_request(
            &app.handle().clone(),
            make_request(&format!("solosoul-pdf://localhost/{}", enc(&path))),
        );
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn rejects_oversize_pdf() {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("big.pdf");
        // 稀疏文件：大于 MAX_PDF_PREVIEW_SIZE 但占盘极少（尺寸检查先于读取）
        let f = fs::File::create(&path).unwrap();
        f.set_len(MAX_PDF_PREVIEW_SIZE + 1).unwrap();

        let resp = handle_request(
            &app.handle().clone(),
            make_request(&format!("solosoul-pdf://localhost/{}", enc(&path))),
        );
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
