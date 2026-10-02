//! RF-317：生产发送/HTTP 边界的失败分类；全为 synthetic + loopback。
use super::*;
use serde_json::{json, Value};
use tokio::net::TcpListener;

async fn response(
    status: u16,
    body: &str,
    content_type: &str,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_owned();
    let content_type = content_type.to_owned();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
        }
        let length = String::from_utf8_lossy(&headers)
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        socket.read_exact(&mut vec![0; length]).await.unwrap();
        let response = format!("HTTP/1.1 {status} Synthetic\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    (url, server)
}
fn assert_safe(error: &BackendError) {
    let wire = serde_json::to_string(error).unwrap();
    let diagnostic = format!("{error:?} {error}");
    for secret in ["RF317_SYNTHETIC", "127.0.0.1", "Bearer", "synthetic-key"] {
        assert!(!wire.contains(secret), "{wire}");
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
}
#[tokio::test]
async fn rf317_http_rejections_use_status_codes_without_response_or_url_in_wire() {
    for (status, key) in [
        (401, "rejected"),
        (429, "rateLimited"),
        (500, "unavailable"),
    ] {
        let fixture = StreamFixture::new();
        let (url, server) = response(
            status,
            "RF317_SYNTHETIC_SECRET_RESPONSE",
            "application/json",
        )
        .await;
        let error = run_resolved_chat_stream(
            &fixture.context,
            url,
            "synthetic-key".into(),
            "model".into(),
            ApiType::OpenAI,
            vec![json!({"role":"user","content":"synthetic"})],
            None,
        )
        .await
        .unwrap_err();
        assert_safe(&error);
        let expected: Value =
            serde_json::from_str(include_str!("../../contracts/rf317-fixtures.json")).unwrap();
        assert_eq!(serde_json::to_value(&error).unwrap(), expected[key]);
        assert_eq!(fixture.conversation().messages.len(), 1);
        {
            let events = fixture.events.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].failure.as_ref().unwrap(), &error);
            assert!(!serde_json::to_string(&*events)
                .unwrap()
                .contains("RF317_SYNTHETIC"));
        }
        server.await.unwrap();
    }
}
#[tokio::test]
async fn rf317_non_stream_http_boundary_uses_the_same_safe_rejection() {
    let (url, server) = response(401, "RF317_SYNTHETIC_SECRET_RESPONSE", "application/json").await;
    let error = request::send_json_request(
        &reqwest::Client::new(),
        &url,
        &json!({}),
        "synthetic-key",
        &ApiType::OpenAI,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, Code::LlmProviderRejected);
    assert_safe(&error);
    server.await.unwrap();
}
#[tokio::test]
async fn rf317_connection_failure_cannot_save_reply_or_leak_request_address() {
    let fixture = StreamFixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/RF317_SYNTHETIC_PRIVATE_PATH",
        listener.local_addr().unwrap()
    );
    drop(listener);
    let error = run_resolved_chat_stream(
        &fixture.context,
        url,
        "synthetic-key".into(),
        "model".into(),
        ApiType::OpenAI,
        vec![json!({"role":"user","content":"synthetic"})],
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, Code::LlmNetworkFailed);
    assert!(error.retryable);
    assert_safe(&error);
    assert_eq!(fixture.conversation().messages.len(), 1);
}
#[tokio::test]
async fn rf317_generated_reply_survives_real_database_write_failure() {
    let fixture = StreamFixture::new();
    let connection =
        rusqlite::Connection::open(fixture.context.session.vault().base_path().join("vault.db"))
            .unwrap();
    connection.execute_batch("CREATE TRIGGER rf317_fail BEFORE INSERT ON llm_conversations BEGIN SELECT RAISE(ABORT, 'RF317_SYNTHETIC_STORAGE_DETAIL'); END;").unwrap();
    drop(connection);
    let (url, server) = response(200, SSE_REPLY, "text/event-stream").await;
    run_resolved_chat_stream(
        &fixture.context,
        url,
        "synthetic-key".into(),
        "model".into(),
        ApiType::OpenAI,
        vec![json!({"role":"user","content":"synthetic"})],
        None,
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert_eq!(fixture.conversation().messages.len(), 1);
    let events = fixture.events.lock().unwrap().clone();
    assert_eq!(
        events.iter().map(|e| e.chunk.as_str()).collect::<String>(),
        "reply"
    );
    let failures: Vec<_> = events.iter().filter_map(|e| e.failure.as_ref()).collect();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, Code::LlmReplySaveFailed);
    assert_eq!(
        events.last().unwrap().error.as_deref(),
        Some("__LLM_PERSIST_FAILED__: LLM_REPLY_SAVE_FAILED")
    );
    assert!(!serde_json::to_string(&events)
        .unwrap()
        .contains("RF317_SYNTHETIC_STORAGE_DETAIL"));
    STATS_MAP
        .write()
        .await
        .remove(fixture.context.session.account_id());
}
#[tokio::test]
async fn rf317_sse_provider_error_is_failure_and_does_not_save_an_empty_reply() {
    let fixture = StreamFixture::new();
    let (url, server) = response(
        200,
        "event: error\ndata: {\"error\":{\"message\":\"RF317_SYNTHETIC_SECRET_RESPONSE\"}}\n\n",
        "text/event-stream",
    )
    .await;
    let error = run_resolved_chat_stream(
        &fixture.context,
        url,
        "synthetic-key".into(),
        "model".into(),
        ApiType::Anthropic,
        vec![json!({"role":"user","content":"synthetic"})],
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, Code::LlmProviderRejected);
    assert_safe(&error);
    assert_eq!(fixture.conversation().messages.len(), 1);
    server.await.unwrap();
}
#[tokio::test]
async fn rf317_expired_stream_cannot_emit_or_save_to_a_new_session() {
    let fixture = StreamFixture::new();
    fixture.context.service.read().unwrap().lock();
    let error = fixture
        .context
        .emit("RF317_SYNTHETIC_LATE".into(), true, None)
        .unwrap_err();
    assert_eq!(error.code, Code::SessionExpired);
    assert_safe(&error);
    assert!(fixture.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn rf317_request_timeout_has_a_safe_retryable_code() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (socket, _) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
        wait.await.unwrap();
        drop(socket);
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(200))
        .build()
        .unwrap();
    let error =
        request::send_json_request(&client, &url, &json!({}), "synthetic-key", &ApiType::OpenAI)
            .await
            .unwrap_err();
    assert_eq!(error.code, Code::LlmTimeout);
    assert!(error.retryable);
    assert_safe(&error);
    release.send(()).unwrap();
    server.await.unwrap();
}
#[tokio::test]
async fn rf317_saved_provider_errors_are_classified_without_parsing_core_messages() {
    let fixture = StreamFixture::new();
    let error = run_chat_stream(
        &fixture.context,
        "missing-provider".into(),
        vec![json!({"role":"user","content":"synthetic"})],
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, Code::LlmProviderNotConfigured);
    assert_safe(&error);
    let mut provider = super::rf005::provider(
        "disabled-provider",
        "http://localhost".into(),
        ApiType::OpenAI,
    );
    provider.is_enabled = false;
    // public-in-test helper below builds real saved config, not an injected command error.
    fixture
        .context
        .with_fixture_vault(|vault| {
            super::rf005::save_settings(
                vault,
                fixture.context.session.account_id(),
                vec![provider],
                Some("disabled-provider".into()),
                &[],
            )
        })
        .unwrap();
    let error = run_chat_stream(
        &fixture.context,
        "disabled-provider".into(),
        vec![json!({"role":"user","content":"synthetic"})],
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, Code::LlmProviderDisabled);
    assert_safe(&error);
}

#[test]
fn rf317_generated_reply_save_rejection_keeps_root_maintenance_gate_and_reports_invoke_failure() {
    let fixture = StreamFixture::new();
    fixture.context.emit("reply".into(), true, None).unwrap();
    let before = fixture.conversation();
    let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(
        fixture.context.session.root_owner(),
    )
    .unwrap();
    let failure = persist_conversation_reply(&fixture.context, "reply", &[]).unwrap_err();
    assert_eq!(failure.code, Code::LlmReplySaveFailed);
    assert_safe(&failure);
    assert_eq!(fixture.events.lock().unwrap().len(), 1);
    drop(maintenance);
    assert_eq!(
        serde_json::to_value(fixture.conversation()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}
