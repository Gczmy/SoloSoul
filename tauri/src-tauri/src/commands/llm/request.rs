//! 共享的 LLM 请求构造与响应解析工具。
//!
//! 把 `chat_http.rs` 和 `stream.rs` 中重复的 Anthropic / OpenAI 请求构造、
//! 响应解析、错误转换提取为公共函数。

use super::*;

/// 根据 API 类型构建请求 URL。
pub fn build_api_url(base_url: &str, api_type: &ApiType) -> String {
    let base = base_url.trim_end_matches('/');
    if is_anthropic(api_type) {
        format!("{}/messages", base)
    } else {
        format!("{}/chat/completions", base)
    }
}

/// 为请求添加认证与 Content-Type 头。
pub fn add_auth_headers(
    req: reqwest::RequestBuilder,
    api_key: &str,
    api_type: &ApiType,
) -> reqwest::RequestBuilder {
    if is_anthropic(api_type) {
        req.header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
    } else {
        req.header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
    }
}

/// 构建请求体（合并 Anthropic 与 OpenAI 的消息格式）。
///
/// - Anthropic：从 messages 中分离 system 消息单独设置。
/// - OpenAI：直接使用 messages 数组。
pub fn build_request_body(
    model: &str,
    messages: Vec<serde_json::Value>,
    api_type: &ApiType,
    max_tokens: u32,
    stream: bool,
) -> serde_json::Value {
    if is_anthropic(api_type) {
        let system = messages
            .iter()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .map(|s| s.to_string());
        let chat_msgs: Vec<serde_json::Value> = messages
            .into_iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
            .collect();
        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": chat_msgs,
        });
        if stream {
            body["stream"] = serde_json::Value::Bool(true);
        }
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }
        body
    } else {
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
        });
        if stream {
            body["stream"] = serde_json::Value::Bool(true);
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }
        body
    }
}

/// 从 API 响应 JSON 中提取文本内容。
///
/// - Anthropic：`content[?(@.type=="text")].text`
/// - OpenAI：`choices[0].message.content`
pub fn extract_response_text(result: &serde_json::Value, api_type: &ApiType) -> Option<String> {
    if is_anthropic(api_type) {
        result["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|c| {
                        c.get("type").and_then(|t| t.as_str()) == Some("text")
                            || c.get("type").is_none()
                    })
                    .and_then(|c| c.get("text").and_then(|v| v.as_str()))
            })
            .map(|s| s.to_string())
    } else {
        result["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
    }
}

/// 检查 HTTP 响应状态，失败时返回格式化错误。
pub async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let url = resp.url().to_string();
    let body = resp.text().await.unwrap_or_default();
    let snippet = if body.is_empty() {
        "(empty body)".to_string()
    } else {
        body.chars().take(MAX_PREVIEW_CHARS).collect()
    };
    Err(format!("HTTP {} {} — {}", status.as_u16(), url, snippet))
}

/// 发送 HTTP POST 请求并检查响应状态。
pub async fn send_json_request(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    api_key: &str,
    api_type: &ApiType,
) -> Result<serde_json::Value, String> {
    let req = client.post(url).json(body);
    let req = add_auth_headers(req, api_key, api_type);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", url, e))?;
    let resp = check_response(resp).await?;
    resp.json()
        .await
        .map_err(|e| format!("Parse response from {}: {}", url, e))
}

/// 从非流式响应中提取 token usage（OpenAI 格式）。
pub fn extract_openai_usage(result: &serde_json::Value) -> (u64, u64) {
    let prompt = result["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let completion = result["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    (prompt, completion)
}

/// P031：SSRF 内网段判定。
///
/// 返回 `true` 表示该 IP 属于禁止外连的网段。**回环（127.0.0.0/8、::1）放行**——
/// 本地 LLM 服务器（Ollama / LM Studio / llama.cpp 均默认监听 localhost）是
/// SoloSoul 本地优先场景的核心用法；其余内网段一律拒绝：RFC1918 私网、链路本地
/// （含云元数据 169.254.169.254）、CGNAT，防止 XSS 借后端做内网探测/带凭证转发。
pub(crate) fn is_blocked_internal_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            if o[0] == 127 {
                return false; // 回环放行
            }
            o[0] == 0 // 0.0.0.0/8
                || o[0] == 10 // 10.0.0.0/8
                || (o[0] == 172 && (16..=31).contains(&o[1])) // 172.16.0.0/12
                || (o[0] == 192 && o[1] == 168) // 192.168.0.0/16
                || (o[0] == 169 && o[1] == 254) // 169.254.0.0/16（含云元数据 169.254.169.254）
                || (o[0] == 100 && (64..=127).contains(&o[1])) // 100.64.0.0/10 CGNAT
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return false;
            }
            // IPv4 映射地址（::ffff:a.b.c.d）转回 IPv4 再判
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_internal_ip(std::net::IpAddr::V4(v4));
            }
            let seg = v6.segments();
            (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 唯一本地地址
                || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 链路本地
        }
    }
}

/// P031：连接测试类命令的附加 SSRF 复核——主机名解析后逐一检查解析地址，
/// 命中任一内网段即拒绝（防 `http://nas.local` 这类解析到内网的主机名绕过）。
/// 字面 IP 已在 `validate_llm_base_url` 同步拦截，本函数只处理域名；调用方为
/// 用户触发的连接测试命令（可接受一次 DNS 查询），流式/常驻路径不调用。
pub(crate) async fn ensure_public_llm_host(base_url: &str) -> Result<(), String> {
    let url = url::Url::parse(base_url).map_err(|e| format!("Invalid base_url: {e}"))?;
    let host = url.host_str().unwrap_or("");
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(()); // 字面 IP 已由 validate_llm_base_url 处理
    }
    let port = url.port().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("base_url host lookup failed: {e}"))?;
    for addr in addrs {
        if is_blocked_internal_ip(addr.ip()) {
            return Err(
                "base_url host resolves to a private/internal address (blocked)".to_string(),
            );
        }
    }
    Ok(())
}

/// P102：校验 LLM 请求目标 `base_url`，收窄后端网络出口。
///
/// 仅允许 http/https scheme 且含非空 host，拒绝 userinfo 与其它 scheme
/// （`javascript:`、`data:`、`file:` 等）。调用方在发起任何外连前必须先通过本校验，
/// 防止被 XSS 当作任意 URL 数据外传通道（CSP 已禁 webview 直连，后端是唯一出口）。
/// P031 追加：字面 IP 命中内网段（RFC1918/链路本地/CGNAT/云元数据）直接拒绝；
/// 回环放行以支持本地 LLM 服务器。主机名需 `ensure_public_llm_host` 异步解析复核。
/// P015 追加：非回环 host 强制 https——Bearer key 与聊天内容不得经公网明文传输，
/// 与 OCR 模型下载侧 `validate_model_base_url` 策略对齐（回环保留 http）。
pub(crate) fn validate_llm_base_url(base_url: &str) -> Result<(), String> {
    let url = url::Url::parse(base_url).map_err(|e| format!("Invalid base_url: {e}"))?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "base_url scheme must be http or https, got: {scheme}"
        ));
    }
    let host = url.host_str().unwrap_or("");
    if host.is_empty() {
        return Err("base_url must contain a host".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("base_url must not contain userinfo".to_string());
    }
    if let Some(url::Host::Ipv4(v4)) = url.host() {
        if is_blocked_internal_ip(std::net::IpAddr::V4(v4)) {
            return Err(
                "base_url host must be a public address (private/internal segments are blocked)"
                    .to_string(),
            );
        }
    } else if let Some(url::Host::Ipv6(v6)) = url.host() {
        if is_blocked_internal_ip(std::net::IpAddr::V6(v6)) {
            return Err(
                "base_url host must be a public address (private/internal segments are blocked)"
                    .to_string(),
            );
        }
    }
    // P015：非回环 host 禁止明文 http（公网传输 Bearer key 与聊天内容可被中间人窃听）；
    // 回环保留 http 以支持本地 LLM 服务器（Ollama / LM Studio / llama.cpp 默认监听 localhost）。
    if scheme == "http" && !is_loopback_host(url.host()) {
        return Err(
            "base_url over http is only allowed for loopback hosts".to_string(),
        );
    }
    Ok(())
}

/// 判断 host 是否为回环：`localhost` 域名、IPv4 127.0.0.0/8、IPv6 ::1。
fn is_loopback_host(host: Option<url::Host<&str>>) -> bool {
    match host {
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(v4)) => v4.octets()[0] == 127,
        Some(url::Host::Ipv6(v6)) => v6.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_base_url_accepts_https() {
        assert!(validate_llm_base_url("https://api.openai.com/v1").is_ok());
    }

    #[test]
    fn test_validate_base_url_accepts_localhost_http() {
        // Ollama 本地端点允许 http + localhost。
        assert!(validate_llm_base_url("http://localhost:11434/v1").is_ok());
        assert!(validate_llm_base_url("http://127.0.0.1:11434/v1").is_ok());
    }

    #[test]
    fn test_validate_base_url_rejects_non_http_schemes() {
        for bad in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "ftp://example.com/v1",
            "gopher://example.com/x",
        ] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject scheme: {bad}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_rejects_missing_host() {
        // `https:///path` 会被 WHATWG URL 规范化为主机 "path"（非空），不算缺失 host。
        for bad in ["https://", "http://", "https:// ", "http:// "] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject missing host: {bad:?}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_rejects_userinfo() {
        assert!(validate_llm_base_url("https://user:pass@evil.com/v1").is_err());
        assert!(validate_llm_base_url("https://user@evil.com/v1").is_err());
    }

    #[test]
    fn test_validate_base_url_rejects_garbage() {
        for bad in ["", "   ", "not-a-url", "https://exa mple.com"] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject garbage: {bad:?}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_rejects_internal_literal_ips() {
        // P031：字面内网 IP 拒绝（回环除外）
        for bad in [
            "http://10.0.0.1/v1",
            "http://172.16.0.1/v1",
            "http://172.31.255.254/v1",
            "http://192.168.1.5/v1",
            "http://169.254.169.254/latest/meta-data",
            "http://100.64.0.1/v1",
            "http://0.0.0.0/v1",
            "http://[fc00::1]/v1",
            "http://[fe80::1]/v1",
            "http://[::ffff:192.168.1.5]/v1",
        ] {
            assert!(
                validate_llm_base_url(bad).is_err(),
                "should reject internal IP: {bad}"
            );
        }
    }

    #[test]
    fn test_validate_base_url_accepts_loopback_and_public_ips() {
        // P031：回环放行（本地 LLM 服务器）+ 公网字面 IP 放行（https）
        for good in [
            "http://127.0.0.1:11434/v1",
            "http://[::1]:11434/v1",
            "https://8.8.8.8/v1",
            "https://1.1.1.1/v1",
            "https://[2001:4860:4860::8888]/v1",
        ] {
            assert!(validate_llm_base_url(good).is_ok(), "should accept: {good}");
        }
    }

    #[test]
    fn test_validate_base_url_rejects_public_http() {
        // P015：非回环 host 强制 https——公网 http 明文传输密钥/内容被拒，与 OCR 侧一致
        for bad in [
            "http://api.openai.com/v1",
            "http://8.8.8.8/v1",
            "http://1.1.1.1/v1",
            "http://[2001:4860:4860::8888]/v1",
            "http://example.com:8080/v1",
        ] {
            assert!(validate_llm_base_url(bad).is_err(), "should reject: {bad}");
        }
        // 回环 + localhost 仍允许 http
        assert!(validate_llm_base_url("http://localhost:11434/v1").is_ok());
        assert!(validate_llm_base_url("http://127.0.0.2:11434/v1").is_ok());
    }

    #[test]
    fn test_is_blocked_internal_ip() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

        // 回环放行
        assert!(!is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            127, 0, 0, 1
        ))));
        assert!(!is_blocked_internal_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));

        // 私网拒绝
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            10, 1, 2, 3
        ))));
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            172, 16, 0, 1
        ))));
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            172, 31, 0, 1
        ))));
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            192, 168, 0, 1
        ))));
        // 链路本地 / 云元数据 / CGNAT / 0.0.0.0/8
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            169, 254, 169, 254
        ))));
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            100, 64, 0, 1
        ))));
        assert!(is_blocked_internal_ip(IpAddr::V4(Ipv4Addr::new(
            0, 0, 0, 0
        ))));

        // IPv6：ULA / 链路本地 / IPv4 映射
        assert!(is_blocked_internal_ip("fc00::1".parse().unwrap()));
        assert!(is_blocked_internal_ip("fd12:3456::1".parse().unwrap()));
        assert!(is_blocked_internal_ip("fe80::1".parse().unwrap()));
        assert!(is_blocked_internal_ip(
            "::ffff:192.168.1.5".parse().unwrap()
        ));

        // 公网放行
        assert!(!is_blocked_internal_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_blocked_internal_ip(
            "2001:4860:4860::8888".parse().unwrap()
        ));
        assert!(!is_blocked_internal_ip("1.2.3.4".parse().unwrap()));
    }

    #[tokio::test]
    async fn test_ensure_public_llm_host() {
        // 字面 IP 短路（回环放行）
        assert!(ensure_public_llm_host("http://127.0.0.1:11434/v1")
            .await
            .is_ok());
        // localhost 解析到回环 → 放行
        assert!(ensure_public_llm_host("http://localhost:11434/v1")
            .await
            .is_ok());
        // 不存在的域名 → 解析失败报错
        assert!(
            ensure_public_llm_host("http://definitely-not-a-real-domain-xyz123.invalid/v1")
                .await
                .is_err()
        );
    }
}
