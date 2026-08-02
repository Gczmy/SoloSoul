use super::request;
use super::*;

#[tauri::command]
pub async fn llm_check_connection(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<bool, String> {
    // P102：网络出口收窄——scheme/host 校验（此命令仅发送固定问候，不携带 Vault 数据，
    // 只需 URL 合法即可，允许测试未保存的新 provider）。
    request::validate_llm_base_url(&base_url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "Client error")?;
    let url = request::build_api_url(&base_url, &api_type);
    let body = request::build_request_body(
        &model,
        vec![serde_json::json!({"role": "user", "content": "Hi"})],
        &api_type,
        1,
        false,
    );
    let req = request::add_auth_headers(client.post(&url).json(&body), &api_key, &api_type);
    match req.send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn llm_test_provider(
    base_url: String,
    api_key: String,
    model: String,
    api_type: ApiType,
) -> Result<String, String> {
    // P102：网络出口收窄——scheme/host 校验（仅发送固定问候，允许测试未保存的新 provider）。
    request::validate_llm_base_url(&base_url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client: {}", e))?;

    let url = request::build_api_url(&base_url, &api_type);
    let body = request::build_request_body(
        &model,
        vec![serde_json::json!({"role": "user", "content": "Hello"})],
        &api_type,
        10,
        false,
    );
    let result = request::send_json_request(&client, &url, &body, &api_key, &api_type).await?;
    let text = request::extract_response_text(&result, &api_type).unwrap_or("ok".to_string());
    Ok(text)
}
