//! RF-005：经过生产 providerId 网关，验证原账户配置、密钥和实际 HTTP 出口。
//! 所有账户、Provider、密钥和聊天内容均为 synthetic，网络仅使用 loopback。

use super::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Notify;

const PROVIDER_ID: &str = "rf005-synthetic-provider";
const KEY_A: &str = "rf005-synthetic-key-account-a";
const KEY_B: &str = "rf005-synthetic-key-account-b";
const USER_TEXT: &str = "RF005 synthetic user message";
const ANTHROPIC_REPLY: &str = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"reply\"}}\n\nevent: message_delta\ndata: {\"usage\":{\"output_tokens\":2}}\n\n";

fn messages() -> Vec<Value> {
    vec![json!({"role": "user", "content": USER_TEXT})]
}

pub(super) fn provider(id: &str, base_url: String, api_type: ApiType) -> ProviderConfig {
    ProviderConfig {
        id: id.into(),
        name: "RF005 Synthetic Provider".into(),
        base_url,
        model: "rf005-saved-model".into(),
        is_enabled: true,
        is_built_in: id.starts_with("builtin_"),
        api_type,
        embedding_model: None,
    }
}

pub(super) fn save_settings(
    vault: &VaultStore,
    account: &str,
    providers: Vec<ProviderConfig>,
    active_provider_id: Option<String>,
    keys: &[(&str, &str)],
) -> Result<(), String> {
    crate::commands::llm::save_config(
        vault,
        account,
        &LlmConfig {
            providers,
            active_provider_id,
            ai_features_enabled: AiFeatures::default(),
            has_accepted_risk: false,
            include_system_prompt: false,
            use_local_embedding: false,
            local_embed_model_id: None,
        },
    )?;
    for (id, key) in keys {
        crate::commands::llm::save_api_key(vault, account, id, key)?;
    }
    Ok(())
}

fn save_provider(fixture: &StreamFixture, provider: ProviderConfig, key: Option<&str>) {
    let id = provider.id.clone();
    let keys: Vec<_> = key.map(|key| (id.as_str(), key)).into_iter().collect();
    fixture
        .context
        .with_fixture_vault(|vault| {
            save_settings(
                vault,
                fixture.context.session.account_id(),
                vec![provider],
                Some(id.clone()),
                &keys,
            )
        })
        .unwrap();
}

struct CapturedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Value,
}

async fn capture_provider(
    api_type: &ApiType,
) -> (String, tokio::task::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let reply = if is_anthropic(api_type) {
        ANTHROPIC_REPLY
    } else {
        SSE_REPLY
    };
    let server = tokio::spawn(async move {
        let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .expect("Provider gateway did not connect")
            .unwrap();
        let mut raw_headers = Vec::new();
        while !raw_headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).await.unwrap();
            raw_headers.push(byte[0]);
            assert!(raw_headers.len() < 32 * 1024);
        }
        let raw_headers = String::from_utf8(raw_headers).unwrap();
        let mut lines = raw_headers.lines();
        let request_line: Vec<_> = lines.next().unwrap().split_whitespace().collect();
        let method = request_line[0].to_owned();
        let path = request_line[1].to_owned();
        let headers: HashMap<_, _> = lines
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some((name.to_ascii_lowercase(), value.trim().to_owned()))
            })
            .collect();
        let length = headers
            .get("content-length")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut bytes = vec![0; length];
        socket.read_exact(&mut bytes).await.unwrap();
        let body = serde_json::from_slice(&bytes).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            reply.len(), reply
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        CapturedRequest {
            method,
            path,
            headers,
            body,
        }
    });
    (base, server)
}

async fn send_gateway(
    fixture: &StreamFixture,
    provider_id: &str,
    server: tokio::task::JoinHandle<CapturedRequest>,
) -> CapturedRequest {
    tokio::time::timeout(
        Duration::from_secs(10),
        run_chat_stream(&fixture.context, provider_id.into(), messages(), None),
    )
    .await
    .expect("Provider gateway timed out")
    .unwrap();
    let request = server.await.unwrap();
    let conversation = fixture.conversation();
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[1].content, "reply");
    let usage = fixture
        .context
        .with_fixture_vault(|vault| {
            load_stats_from_vault(vault, fixture.context.session.account_id())
        })
        .unwrap();
    assert_eq!(usage.usage_count, 6);
    STATS_MAP
        .write()
        .await
        .remove(fixture.context.session.account_id());
    request
}

async fn reject_before_connect(
    listeners: &[&TcpListener],
    request: impl Future<Output = Result<(), BackendError>>,
) -> String {
    tokio::pin!(request);
    let connections =
        futures::future::select_all(listeners.iter().map(|listener| Box::pin(listener.accept())));
    let error = tokio::select! {
        result = &mut request => result.expect_err("Provider gateway unexpectedly accepted request"),
        (connection, _, _) = connections => {
            connection.unwrap();
            panic!("Rejected provider gateway opened a TCP connection")
        },
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("Provider gateway rejection timed out"),
    };
    let error = serde_json::to_string(&error).unwrap();
    assert!(!error.is_empty());
    assert!(!error.contains(KEY_A));
    assert!(!error.contains(KEY_B));
    for listener in listeners {
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err(),
            "Provider observed a queued connection after rejection"
        );
    }
    error
}

async fn reject_gateway(
    fixture: &StreamFixture,
    provider_id: &str,
    listener: &TcpListener,
) -> String {
    let error = reject_before_connect(
        &[listener],
        run_chat_stream(&fixture.context, provider_id.into(), messages(), None),
    )
    .await;
    assert_eq!(fixture.conversation().messages.len(), 1);
    assert!(fixture.events.lock().unwrap().is_empty());
    assert!(!STATS_MAP
        .read()
        .await
        .contains_key(fixture.context.session.account_id()));
    error
}

fn assert_provider_request(
    request: &CapturedRequest,
    api_type: &ApiType,
    base_path: &str,
    key: &str,
) {
    assert_eq!(request.method, "POST");
    let suffix = if is_anthropic(api_type) {
        "/messages"
    } else {
        "/chat/completions"
    };
    assert_eq!(request.path, format!("{base_path}{suffix}"));
    assert_eq!(request.body["model"], "rf005-saved-model");
    assert_eq!(request.body["messages"], json!(messages()));
    assert_eq!(request.body["stream"], true);
    assert_eq!(
        request.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    if is_anthropic(api_type) {
        assert_eq!(
            request.headers.get("x-api-key").map(String::as_str),
            Some(key)
        );
        assert_eq!(
            request.headers.get("anthropic-version").map(String::as_str),
            Some("2023-06-01")
        );
        assert!(!request.headers.contains_key("authorization"));
    } else {
        assert_eq!(
            request.headers.get("authorization"),
            Some(&format!("Bearer {key}"))
        );
        assert!(!request.headers.contains_key("x-api-key"));
    }
    let body = serde_json::to_string(&request.body).unwrap();
    assert!(!body.contains(KEY_A));
    assert!(!body.contains(KEY_B));
}

#[tokio::test]
async fn rf005_saved_provider_controls_openai_and_anthropic_url_model_and_auth() {
    for api_type in [ApiType::OpenAI, ApiType::Anthropic] {
        let fixture = StreamFixture::new();
        let (base, server) = capture_provider(&api_type).await;
        let selected = provider(PROVIDER_ID, format!("{base}/saved/v1/"), api_type.clone());
        let mut decoy = provider(
            "rf005-active-decoy",
            format!("{base}/decoy"),
            ApiType::OpenAI,
        );
        decoy.model = "rf005-decoy-model".into();
        fixture
            .context
            .with_fixture_vault(|vault| {
                save_settings(
                    vault,
                    fixture.context.session.account_id(),
                    vec![selected, decoy],
                    Some("rf005-active-decoy".into()),
                    &[(PROVIDER_ID, KEY_A), ("rf005-active-decoy", KEY_B)],
                )
            })
            .unwrap();
        let request = send_gateway(&fixture, PROVIDER_ID, server).await;
        assert_provider_request(&request, &api_type, "/saved/v1", KEY_A);
    }
}

#[tokio::test]
async fn rf005_historical_gui_ids_and_custom_id_honor_saved_overrides() {
    for id in [
        "builtin_openai",
        "builtin_anthropic",
        "builtin_ollama",
        "builtin_deepseek",
        "builtin_alibaba",
        "rf005-legacy-custom",
    ] {
        let fixture = StreamFixture::new();
        // 故意统一覆写成 Anthropic，验证保存的 apiType 也覆盖内置默认值。
        let (base, server) = capture_provider(&ApiType::Anthropic).await;
        save_provider(
            &fixture,
            provider(id, format!("{base}/historical/v1"), ApiType::Anthropic),
            Some(KEY_A),
        );
        let request = send_gateway(&fixture, id, server).await;
        assert_provider_request(&request, &ApiType::Anthropic, "/historical/v1", KEY_A);
    }
}

#[tokio::test]
async fn rf005_loopback_services_without_saved_api_keys_remain_usable() {
    for api_type in [ApiType::OpenAI, ApiType::Anthropic] {
        let fixture = StreamFixture::new();
        let (base, server) = capture_provider(&api_type).await;
        save_provider(
            &fixture,
            provider(PROVIDER_ID, format!("{base}/local/v1"), api_type.clone()),
            None,
        );
        let request = send_gateway(&fixture, PROVIDER_ID, server).await;
        assert_eq!(request.body["model"], "rf005-saved-model");
        assert_eq!(request.body["messages"], json!(messages()));
        let suffix = if is_anthropic(&api_type) {
            "/messages"
        } else {
            "/chat/completions"
        };
        assert_eq!(request.path, format!("/local/v1{suffix}"));
        if let Some(authorization) = request.headers.get("authorization") {
            assert_eq!(authorization, "Bearer");
        }
        if let Some(key) = request.headers.get("x-api-key") {
            assert!(key.is_empty());
        }
    }
}

#[tokio::test]
async fn rf005_unknown_and_disabled_provider_ids_never_fall_back_to_active_provider() {
    for disabled in [false, true] {
        let fixture = StreamFixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let active = provider("rf005-active", base.clone(), ApiType::OpenAI);
        let mut selected = provider(PROVIDER_ID, base, ApiType::OpenAI);
        selected.is_enabled = !disabled;
        fixture
            .context
            .with_fixture_vault(|vault| {
                save_settings(
                    vault,
                    fixture.context.session.account_id(),
                    vec![selected, active],
                    Some("rf005-active".into()),
                    &[(PROVIDER_ID, KEY_A), ("rf005-active", KEY_B)],
                )
            })
            .unwrap();
        let requested = if disabled {
            PROVIDER_ID
        } else {
            "rf005-unknown-id"
        };
        reject_gateway(&fixture, requested, &listener).await;
    }
}

#[tokio::test]
async fn rf005_provider_existing_only_in_another_account_is_not_resolved() {
    let mut fixture = StreamFixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let original = fixture.context.session.account_id().to_owned();
    let other = format!("acc_{}", uuid::Uuid::new_v4().simple());
    let service = fixture.context.service.clone();
    {
        let service = service.read().unwrap();
        service.lock();
        service
            .create_account_with_id(&other, "RF005 Synthetic B", "password456", None)
            .unwrap();
        save_settings(
            &service.get_vault_store().unwrap(),
            &other,
            vec![provider(PROVIDER_ID, base, ApiType::OpenAI)],
            Some(PROVIDER_ID.into()),
            &[(PROVIDER_ID, KEY_B)],
        )
        .unwrap();
        service.unlock(&original, "password123").unwrap();
    }
    // 新捕获 A 的有效会话，避免仅靠旧 generation 失效就让跨账户查询测试通过。
    let sink = fixture.events.clone();
    fixture.context = StreamContext::capture(
        &service,
        &original,
        "conversation".into(),
        None,
        move |event| {
            sink.lock().unwrap().push(event);
            Ok(())
        },
    )
    .unwrap();
    reject_gateway(&fixture, PROVIDER_ID, &listener).await;
}

#[tokio::test]
async fn rf005_invalid_saved_profile_configuration_or_key_map_is_rejected() {
    for corrupt_kind in ["profile-json", "config-shape", "config-field", "key-map"] {
        let fixture = StreamFixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        save_provider(
            &fixture,
            provider(PROVIDER_ID, base, ApiType::OpenAI),
            Some(KEY_A),
        );
        fixture
            .context
            .with_fixture_vault(|vault| {
                let account = fixture.context.session.account_id();
                let mut profile = vault.load_profile(account)?.unwrap();
                if corrupt_kind == "profile-json" {
                    profile.data = b"not-json".to_vec();
                } else {
                    let mut data: Value = serde_json::from_slice(&profile.data).unwrap();
                    match corrupt_kind {
                        "config-shape" => data["preferences"]["llmConfig"] = json!([]),
                        "config-field" => {
                            data["preferences"]["llmConfig"]["providers"][0]["isEnabled"] =
                                json!("true")
                        }
                        "key-map" => data["preferences"]["llmApiKeys"] = json!({(PROVIDER_ID): 17}),
                        _ => unreachable!(),
                    }
                    profile.data = serde_json::to_vec(&data).unwrap();
                }
                vault.save_profile(&profile)
            })
            .unwrap();
        reject_gateway(&fixture, PROVIDER_ID, &listener).await;
    }
}

#[tokio::test]
async fn rf005_profile_storage_failure_is_not_treated_as_missing_configuration() {
    let fixture = StreamFixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    save_provider(
        &fixture,
        provider(PROVIDER_ID, base, ApiType::OpenAI),
        Some(KEY_A),
    );
    fixture
        .context
        .with_fixture_vault(|vault| {
            let connection = rusqlite::Connection::open(vault.base_path().join("vault.db"))
                .map_err(|error| error.to_string())?;
            connection
                .execute_batch("DROP TABLE profiles")
                .map_err(|error| error.to_string())
        })
        .unwrap();
    reject_gateway(&fixture, PROVIDER_ID, &listener).await;
}

#[tokio::test]
async fn rf005_saved_disallowed_urls_still_fail_before_any_provider_connection() {
    for kind in ["userinfo", "non-http-scheme"] {
        let fixture = StreamFixture::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let invalid = if kind == "userinfo" {
            format!("http://synthetic-user:synthetic-password@{address}/v1")
        } else {
            format!("ftp://{address}/v1")
        };
        // 直接存旧配置，以覆盖历史脏数据绕过保存时 URL 校验的场景。
        save_provider(
            &fixture,
            provider(PROVIDER_ID, invalid, ApiType::OpenAI),
            Some(KEY_A),
        );
        reject_gateway(&fixture, PROVIDER_ID, &listener).await;
    }
}

#[tokio::test]
async fn rf005_lock_or_switch_after_resolution_never_reuses_old_or_new_account_keys() {
    for switch in [false, true] {
        let mut fixture = StreamFixture::new();
        let old_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let new_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let old_base = format!("http://{}/old/v1", old_listener.local_addr().unwrap());
        let new_base = format!("http://{}/new/v1", new_listener.local_addr().unwrap());
        save_provider(
            &fixture,
            provider(PROVIDER_ID, old_base, ApiType::OpenAI),
            Some(KEY_A),
        );
        let reached = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let hook_reached = reached.clone();
        let hook_release = release.clone();
        fixture.context.before_send = Some(Arc::new(move || {
            let reached = hook_reached.clone();
            let release = hook_release.clone();
            Box::pin(async move {
                reached.notify_one();
                release.notified().await;
            })
        }));
        let original = fixture.context.session.account_id().to_owned();
        let other = format!("acc_{}", uuid::Uuid::new_v4().simple());
        let pending = run_chat_stream(&fixture.context, PROVIDER_ID.into(), messages(), None);
        tokio::pin!(pending);
        tokio::select! {
            _ = reached.notified() => {},
            result = &mut pending => panic!("Request ended before provider resolution barrier: {result:?}"),
            _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("Provider resolution barrier timed out"),
        }
        let new_profile_before = {
            let service = fixture.context.service.read().unwrap();
            service.lock();
            if switch {
                service
                    .create_account_with_id(&other, "RF005 Synthetic B", "password456", None)
                    .unwrap();
                let vault = service.get_vault_store().unwrap();
                save_settings(
                    &vault,
                    &other,
                    vec![provider(PROVIDER_ID, new_base, ApiType::Anthropic)],
                    Some(PROVIDER_ID.into()),
                    &[(PROVIDER_ID, KEY_B)],
                )
                .unwrap();
                Some(vault.load_profile(&other).unwrap().unwrap().data)
            } else {
                None
            }
        };
        release.notify_one();
        reject_before_connect(&[&old_listener, &new_listener], &mut pending).await;
        assert!(fixture.events.lock().unwrap().is_empty());
        let stats = STATS_MAP.read().await;
        assert!(!stats.contains_key(&original));
        assert!(!stats.contains_key(&other));
        drop(stats);
        let service = fixture.context.service.read().unwrap();
        if switch {
            let vault = service.get_vault_store().unwrap();
            assert_eq!(
                Some(vault.load_profile(&other).unwrap().unwrap().data),
                new_profile_before
            );
            assert!(vault.list_conversations(&other).unwrap().is_empty());
        }
        service.unlock(&original, "password123").unwrap();
        let vault = service.get_vault_store().unwrap();
        let conversation = vault
            .load_conversation(&original, &fixture.context.conversation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Conversation>(&conversation)
                .unwrap()
                .messages
                .len(),
            1
        );
        assert_eq!(
            load_stats_from_vault(&vault, &original)
                .unwrap()
                .usage_count,
            5
        );
        assert_eq!(
            crate::commands::llm::load_api_keys(&vault, &original)
                .unwrap()
                .get(PROVIDER_ID)
                .map(String::as_str),
            Some(KEY_A)
        );
    }
}

#[test]
fn rf905_stream_short_callback_rejects_existing_maintenance_without_updating_original_store() {
    let fixture = StreamFixture::new();
    let context = &fixture.context;
    let account = context.session.account_id();
    let original = context.session.vault();
    let before = original.load_profile(account).unwrap().unwrap();
    let maintenance =
        solosoul_core::import_activity::begin_owned_root_maintenance(original.root_owner())
            .unwrap();
    let called = std::sync::atomic::AtomicBool::new(false);
    let result = context.with_fixture_vault(|vault| {
        called.store(true, std::sync::atomic::Ordering::SeqCst);
        vault.update_profile_prefs(account, |prefs| {
            prefs.insert("rf905-write".into(), json!("must not be written"));
            Ok(())
        })
    });
    assert_eq!(result.unwrap_err().code, Code::VaultBusy);
    assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    let after = original.load_profile(account).unwrap().unwrap();
    assert_eq!(after.data, before.data);
    assert_eq!(after.version, before.version);
    drop(maintenance);
    context
        .with_fixture_vault(|vault| {
            vault.update_profile_prefs(account, |prefs| {
                prefs.insert("rf905-write".into(), json!("accepted after maintenance"));
                Ok(())
            })
        })
        .unwrap();
    let after = original.load_profile(account).unwrap().unwrap();
    let value: Value = serde_json::from_slice(&after.data).unwrap();
    assert_eq!(
        value["preferences"]["rf905-write"],
        "accepted after maintenance"
    );
}
