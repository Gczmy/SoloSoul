use super::{code, HttpResult};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::time::Duration;

pub(crate) async fn perform_http_async(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    body: &str,
) -> Result<HttpResult, i32> {
    let mut headers = HeaderMap::new();
    if serde_json::from_str::<serde_json::Value>(body).is_ok() {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    }

    let method =
        reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| code::INVALID_ARGUMENT)?;

    let mut req = client.request(method, url).headers(headers);
    if !body.is_empty() {
        req = req.body(body.to_string());
    }

    let resp = req
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|_| code::NETWORK_TIMEOUT)?;

    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| code::NETWORK_TIMEOUT)?;

    Ok(HttpResult {
        status,
        body,
        error_code: None,
    })
}

/// 执行同步阻塞的 HTTP POST 请求
pub(crate) fn perform_http_post(
    client: &reqwest::Client,
    url: &str,
    body: &str,
) -> Result<String, String> {
    let mut headers = HeaderMap::new();
    if serde_json::from_str::<serde_json::Value>(body).is_ok() {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    }

    block_on(async {
        let resp = client
            .post(url)
            .headers(headers)
            .body(body.to_string())
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let text = resp.text().await.map_err(|e| e.to_string())?;
        Ok(text)
    })
    .map_err(|_| "无法在当前线程执行网络请求".to_string())?
}

/// 在当前 Tokio 运行时上阻塞执行 Future（插件 Host Function 运行在 spawn_blocking 线程）
pub(crate) fn block_on<F: std::future::Future>(future: F) -> Result<F::Output, ()> {
    tokio::runtime::Handle::try_current()
        .map_err(|_| ())
        .map(|handle| handle.block_on(future))
}
