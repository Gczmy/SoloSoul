//! RF-004：使用 synthetic Vault 数据捕获真正发往本地 provider 的最终 JSON。
//! 这里直接调用生产 run_chat_stream，覆盖投影、协议转换、发送和回复保存。

use super::*;
use crate::commands::llm::rag::GuideChunk;
use crate::services::llm_context::ChatContextSelection;
use serde_json::{json, Value};
use solosoul_vault::{ObjectRecord, Profile, UserTemplate};
use std::future::Future;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Notify;

const USER_TEXT: &str = "SECRET_USER：这是用户主动输入的 synthetic 内容";
const GUIDE_TEXT: &str = "RF004_SYNTHETIC_GUIDE_TEXT";
const PUBLIC_ID: &str = "rf004-public-object";
const SYNTHETIC_KEY: &str = "rf004-synthetic-provider-key";
const ANTHROPIC_REPLY: &str = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"reply\"}}\n\nevent: message_delta\ndata: {\"usage\":{\"output_tokens\":2}}\n\n";

fn messages() -> Vec<Value> {
    vec![json!({"role": "user", "content": USER_TEXT})]
}

fn selection(ids: &[&str]) -> Option<ChatContextSelection> {
    Some(ChatContextSelection::PublicProfile {
        object_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        language: "en-US".into(),
        guide_chunks: vec![GuideChunk {
            guide_id: "rf004-synthetic-guide".into(),
            guide_title: "Synthetic Guide".into(),
            chunk_text: GUIDE_TEXT.into(),
            similarity: 1.0,
        }],
    })
}

fn config(include_system_prompt: bool) -> LlmConfig {
    LlmConfig {
        providers: vec![],
        active_provider_id: None,
        ai_features_enabled: AiFeatures::default(),
        has_accepted_risk: false,
        include_system_prompt,
        use_local_embedding: false,
        local_embed_model_id: None,
    }
}

fn save_include_system_prompt(fixture: &StreamFixture, enabled: bool) {
    fixture
        .context
        .with_vault(|vault| {
            crate::commands::llm::save_config(
                vault,
                fixture.context.session.account_id(),
                &config(enabled),
            )
        })
        .unwrap();
}

fn save_profile_data(fixture: &StreamFixture, data: Vec<u8>) {
    let account = fixture.context.session.account_id();
    fixture
        .context
        .with_vault(|vault| vault.save_profile(&Profile::new_with_id(account, account, data)))
        .unwrap();
}

fn synthetic_object(account: &str, id: &str, value: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        account_id: account.into(),
        type_id: "synthetic-profile".into(),
        section_type: "identity".into(),
        name: "Synthetic Public Object".into(),
        icon_name: "document".into(),
        properties: json!({"public_value": value}),
        property_labels: Some(json!({"public_value": "public"})),
        sensitivity_level: "public".into(),
        created_at: now_iso(),
        updated_at: now_iso(),
        version: 1,
        ..Default::default()
    }
}

fn seed_public_object(fixture: &StreamFixture) {
    let account = fixture.context.session.account_id();
    let template: UserTemplate = serde_json::from_value(json!({
        "id": "rf004-synthetic-template",
        "accountId": account,
        "name": "Synthetic Template",
        "createdAt": now_iso(),
        "properties": [
            {"id": "template_public", "name": "Template public", "type": "text", "sensitivityLevel": "public"},
            {"id": "template_secret", "name": "Template secret", "type": "text", "sensitivityLevel": "critical"}
        ]
    }))
    .unwrap();
    let mut object = synthetic_object(account, PUBLIC_ID, "ALLOW_PUBLIC_VALUE");
    object.template_id = Some(template.id.clone());
    object.template_type = Some("user".into());
    object.properties = json!({
        "public_value": "ALLOW_PUBLIC_VALUE",
        "schema_public": "ALLOW_SCHEMA_VALUE",
        "template_public": "ALLOW_TEMPLATE_VALUE",
        "template_secret": "SECRET_TEMPLATE",
        "internal_value": "SECRET_INTERNAL",
        "sensitive_value": "SECRET_SENSITIVE",
        "critical_value": "SECRET_CRITICAL",
        "unknown_value": "SECRET_UNKNOWN",
        "unmarked_value": "SECRET_UNMARKED",
        "__internal": "SECRET_INTERNAL_KEY",
        "nested": {"value": "SECRET_NESTED"},
        "group": [
            {"name": "Public child", "value": "ALLOW_CHILD_VALUE", "type": "text", "sensitivityLevel": "public"},
            {"name": "Unmarked child", "value": "SECRET_CHILD_UNMARKED", "type": "text"},
            {"name": "Private child", "value": "SECRET_CHILD_PRIVATE", "type": "text", "sensitivityLevel": "critical"}
        ],
        "__fields": {
            "schema_public": {"type": "text", "sensitivityLevel": "public"},
            "group": {"type": "dynamic_group", "sensitivityLevel": "public"}
        }
    });
    object.property_labels = Some(json!({
        "public_value": "public",
        "internal_value": "internal",
        "sensitive_value": "sensitive",
        "critical_value": "critical",
        "unknown_value": "future-unknown-level",
        "__internal": "public",
        "nested": "public"
    }));
    fixture
        .context
        .with_vault(|vault| {
            vault.save_user_template(&template)?;
            vault.save_object(&object)
        })
        .unwrap();
}

/// 删除真实 SQLite 表作为故障探针；调用方只能传入本测试列出的表名。
fn drop_tables(fixture: &StreamFixture, tables: &[&str]) {
    fixture
        .context
        .with_vault(|vault| {
            let connection = rusqlite::Connection::open(vault.base_path().join("vault.db"))
                .map_err(|error| error.to_string())?;
            for table in tables {
                let sql = match *table {
                    "objects" => "DROP TABLE objects",
                    "user_templates" => "DROP TABLE user_templates",
                    "profiles" => "DROP TABLE profiles",
                    _ => panic!("Unexpected synthetic test table"),
                };
                connection
                    .execute_batch(sql)
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
        .unwrap();
}

async fn capture_server(api_type: &ApiType) -> (String, tokio::task::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let reply = if is_anthropic(api_type) {
        ANTHROPIC_REPLY
    } else {
        SSE_REPLY
    };
    let server = tokio::spawn(async move {
        let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .expect("Provider did not receive a connection")
            .unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
            assert!(headers.len() < 32 * 1024, "Unexpected request header size");
        }
        let headers = String::from_utf8(headers).unwrap();
        let length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .expect("JSON request must have Content-Length");
        let mut bytes = vec![0; length];
        socket.read_exact(&mut bytes).await.unwrap();
        let body = serde_json::from_slice(&bytes).expect("Provider must receive valid JSON");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            reply.len(),
            reply
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        body
    });
    (url, server)
}

async fn send_and_capture(
    fixture: &StreamFixture,
    api_type: ApiType,
    context_selection: Option<ChatContextSelection>,
) -> Value {
    let (url, server) = capture_server(&api_type).await;
    tokio::time::timeout(
        Duration::from_secs(10),
        run_chat_stream(
            &fixture.context,
            url,
            SYNTHETIC_KEY.into(),
            "rf004-synthetic-model".into(),
            api_type,
            messages(),
            context_selection,
        ),
    )
    .await
    .expect("Production send path timed out")
    .unwrap();
    let body = server.await.unwrap();
    let conversation = fixture.conversation();
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[1].content, "reply");
    assert_eq!(
        fixture
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.is_done)
            .count(),
        1
    );
    STATS_MAP
        .write()
        .await
        .remove(fixture.context.session.account_id());
    body
}

/// 发送失败也必须发生在 TCP 建连之前，不能仅靠最终错误掩盖已外发的数据。
async fn assert_rejected_before_connect(
    listener: &TcpListener,
    request: impl Future<Output = Result<(), String>>,
) -> String {
    tokio::pin!(request);
    let error = tokio::select! {
        result = &mut request => result.expect_err("Unsafe request unexpectedly succeeded"),
        connection = listener.accept() => {
            connection.unwrap();
            panic!("Rejected request opened a provider connection")
        },
        _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("Rejection timed out"),
    };
    assert!(!error.is_empty());
    assert!(
        tokio::time::timeout(Duration::from_millis(30), listener.accept())
            .await
            .is_err(),
        "Provider observed a queued connection after rejection"
    );
    error
}

async fn reject_messages(
    fixture: &StreamFixture,
    messages: Vec<Value>,
    context_selection: Option<ChatContextSelection>,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let error = assert_rejected_before_connect(
        &listener,
        run_chat_stream(
            &fixture.context,
            url,
            SYNTHETIC_KEY.into(),
            "rf004-synthetic-model".into(),
            ApiType::OpenAI,
            messages,
            context_selection,
        ),
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

fn assert_clean_public_payload(body: &Value, api_type: &ApiType) {
    let chat = body["messages"].as_array().unwrap();
    let system_messages: Vec<_> = chat
        .iter()
        .filter(|message| message["role"] == "system")
        .collect();
    let system = if is_anthropic(api_type) {
        assert!(system_messages.is_empty());
        body["system"]
            .as_str()
            .expect("Anthropic needs one system string")
    } else {
        assert!(body.get("system").is_none());
        assert_eq!(
            system_messages.len(),
            1,
            "OpenAI needs one merged system message"
        );
        system_messages[0]["content"].as_str().unwrap()
    };
    assert!(
        system.contains("SoloSoul"),
        "Fixed system prompt is missing"
    );
    for allowed in [
        "ALLOW_PUBLIC_VALUE",
        "ALLOW_SCHEMA_VALUE",
        "ALLOW_TEMPLATE_VALUE",
        "ALLOW_CHILD_VALUE",
        GUIDE_TEXT,
    ] {
        assert_eq!(
            system.matches(allowed).count(),
            1,
            "Missing or repeated public context: {allowed}"
        );
    }
    let encoded = serde_json::to_string(body).unwrap();
    for secret in [
        "SECRET_TEMPLATE",
        "SECRET_INTERNAL",
        "SECRET_SENSITIVE",
        "SECRET_CRITICAL",
        "SECRET_UNKNOWN",
        "SECRET_UNMARKED",
        "SECRET_INTERNAL_KEY",
        "SECRET_NESTED",
        "SECRET_CHILD_UNMARKED",
        "SECRET_CHILD_PRIVATE",
    ] {
        assert!(
            !encoded.contains(secret),
            "Automatic context leaked {secret}"
        );
    }
    let user_messages: Vec<_> = chat
        .iter()
        .filter(|message| message["role"] == "user")
        .collect();
    assert_eq!(user_messages.len(), 1);
    assert_eq!(user_messages[0]["content"], USER_TEXT);
}

#[tokio::test]
async fn rf004_openai_outbound_json_contains_only_public_projection_and_user_input() {
    let fixture = StreamFixture::new();
    seed_public_object(&fixture);
    // 无 llmConfig 的已保存 Profile：兼容默认开启自动上下文。
    let body = send_and_capture(&fixture, ApiType::OpenAI, selection(&[PUBLIC_ID])).await;
    assert_clean_public_payload(&body, &ApiType::OpenAI);
}

#[tokio::test]
async fn rf004_anthropic_outbound_json_merges_public_profile_guide_and_fixed_system() {
    let fixture = StreamFixture::new();
    seed_public_object(&fixture);
    save_include_system_prompt(&fixture, true);
    let body = send_and_capture(&fixture, ApiType::Anthropic, selection(&[PUBLIC_ID])).await;
    assert_clean_public_payload(&body, &ApiType::Anthropic);
}

#[tokio::test]
async fn rf004_foreign_deleted_private_and_unknown_candidates_never_leave_host() {
    for kind in [
        "foreign",
        "deleted",
        "private",
        "unknown-sensitivity",
        "unknown-id",
    ] {
        let fixture = StreamFixture::new();
        let account = fixture.context.session.account_id();
        fixture
            .context
            .with_vault(|vault| {
                vault.save_object(&synthetic_object(account, PUBLIC_ID, "ALLOW_CANDIDATE"))?;
                if kind != "unknown-id" {
                    let mut denied =
                        synthetic_object(account, "rf004-denied", "SECRET_DENIED_OBJECT");
                    match kind {
                        "foreign" => denied.account_id = "rf004-synthetic-foreign-account".into(),
                        "deleted" => {
                            denied.is_deleted = true;
                            denied.deleted_at = Some(now_iso());
                        }
                        "private" => denied.sensitivity_level = "internal".into(),
                        "unknown-sensitivity" => {
                            denied.sensitivity_level = "future-unknown-level".into()
                        }
                        _ => unreachable!(),
                    }
                    vault.save_object(&denied)?;
                }
                Ok(())
            })
            .unwrap();
        let body = send_and_capture(
            &fixture,
            ApiType::OpenAI,
            selection(&["rf004-denied", PUBLIC_ID]),
        )
        .await;
        let encoded = serde_json::to_string(&body).unwrap();
        assert!(
            encoded.contains("ALLOW_CANDIDATE"),
            "Valid candidate lost for {kind}"
        );
        assert!(
            !encoded.contains("SECRET_DENIED_OBJECT"),
            "Rejected candidate leaked for {kind}"
        );
    }
}

#[tokio::test]
async fn rf004_empty_ids_do_not_expand_to_all_objects_or_read_template_table() {
    let fixture = StreamFixture::new();
    seed_public_object(&fixture);
    drop_tables(&fixture, &["objects", "user_templates"]);
    let body = send_and_capture(&fixture, ApiType::OpenAI, selection(&[])).await;
    let encoded = serde_json::to_string(&body).unwrap();
    assert!(!encoded.contains("ALLOW_PUBLIC_VALUE"));
    assert!(encoded.contains(GUIDE_TEXT));
    assert_eq!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| message["role"] == "system")
            .count(),
        1
    );
}

#[tokio::test]
async fn rf004_saved_opt_out_skips_objects_and_templates_in_real_send_path() {
    let fixture = StreamFixture::new();
    seed_public_object(&fixture);
    save_include_system_prompt(&fixture, false);
    drop_tables(&fixture, &["objects", "user_templates"]);
    let body = send_and_capture(&fixture, ApiType::OpenAI, selection(&[PUBLIC_ID])).await;
    assert_eq!(body["messages"], json!(messages()));
    assert!(body.get("system").is_none());
}

#[tokio::test]
async fn rf004_missing_and_explicit_none_skip_profile_objects_and_templates() {
    for context_selection in [None, Some(ChatContextSelection::None)] {
        let fixture = StreamFixture::new();
        seed_public_object(&fixture);
        drop_tables(&fixture, &["objects", "user_templates", "profiles"]);
        let body = send_and_capture(&fixture, ApiType::OpenAI, context_selection).await;
        assert_eq!(body["messages"], json!(messages()));
        assert!(body.get("system").is_none());
    }
}

#[tokio::test]
async fn rf004_corrupt_saved_profile_or_llm_config_fails_without_connecting() {
    let mut invalid_config = serde_json::to_value(config(true)).unwrap();
    invalid_config["includeSystemPrompt"] = json!("not-a-boolean");
    for corrupt in [
        b"not-json".to_vec(),
        serde_json::to_vec(&json!({"preferences": {"llmConfig": invalid_config}})).unwrap(),
        serde_json::to_vec(&json!({"preferences": {"llmConfig": []}})).unwrap(),
    ] {
        let fixture = StreamFixture::new();
        seed_public_object(&fixture);
        save_profile_data(&fixture, corrupt);
        reject_messages(&fixture, messages(), selection(&[PUBLIC_ID])).await;
    }
}

#[tokio::test]
async fn rf004_saved_profile_load_error_fails_closed_before_provider_connection() {
    let fixture = StreamFixture::new();
    seed_public_object(&fixture);
    drop_tables(&fixture, &["profiles"]);
    reject_messages(&fixture, messages(), selection(&[PUBLIC_ID])).await;
}

#[tokio::test]
async fn rf004_client_system_and_unknown_roles_are_rejected_before_connecting() {
    for role in ["system", "developer", "tool", "unknown"] {
        let fixture = StreamFixture::new();
        reject_messages(
            &fixture,
            vec![
                json!({"role": role, "content": "SECRET_CLIENT_SYSTEM"}),
                messages().remove(0),
            ],
            None,
        )
        .await;
    }
}

#[tokio::test]
async fn rf004_non_text_or_missing_chat_fields_are_rejected_before_connecting() {
    for malformed in [
        json!({"role": "user", "content": {"text": "SECRET_STRUCTURED_CONTENT"}}),
        json!({"role": "user", "content": null}),
        json!({"role": "user"}),
        json!({"content": "SECRET_MISSING_ROLE"}),
    ] {
        let fixture = StreamFixture::new();
        reject_messages(&fixture, vec![malformed], None).await;
    }
}

#[tokio::test]
async fn rf004_lock_or_account_switch_after_projection_prevents_any_connection() {
    for switch_account in [false, true] {
        let mut fixture = StreamFixture::new();
        seed_public_object(&fixture);
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let account = fixture.context.session.account_id().to_owned();
        let other = format!("acc_{}", uuid::Uuid::new_v4().simple());
        let pending = run_chat_stream(
            &fixture.context,
            url,
            SYNTHETIC_KEY.into(),
            "rf004-synthetic-model".into(),
            ApiType::OpenAI,
            messages(),
            selection(&[PUBLIC_ID]),
        );
        tokio::pin!(pending);
        tokio::select! {
            _ = reached.notified() => {},
            result = &mut pending => panic!("Request completed before projection barrier: {result:?}"),
            _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("Projection barrier timed out"),
        }
        {
            let service = fixture.context.service.read().unwrap();
            service.lock();
            if switch_account {
                service
                    .create_account_with_id(&other, "Synthetic B", "password456", None)
                    .unwrap();
                // 新账户同 ID 对象不能被旧请求重读或外发。
                let vault = service.get_vault_store().unwrap();
                vault
                    .save_object(&synthetic_object(&other, PUBLIC_ID, "SECRET_NEW_ACCOUNT"))
                    .unwrap();
            }
        }
        release.notify_one();
        assert_rejected_before_connect(&listener, &mut pending).await;
        assert!(fixture.events.lock().unwrap().is_empty());
        let stats = STATS_MAP.read().await;
        assert!(!stats.contains_key(&account));
        assert!(!stats.contains_key(&other));
        drop(stats);
        let service = fixture.context.service.read().unwrap();
        if switch_account {
            let vault = service.get_vault_store().unwrap();
            assert!(vault.list_profiles().unwrap().is_empty());
            assert!(vault.list_conversations(&other).unwrap().is_empty());
        }
        service.unlock(&account, "password123").unwrap();
        let vault = service.get_vault_store().unwrap();
        let stored = vault
            .load_conversation(&account, &fixture.context.conversation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Conversation>(&stored)
                .unwrap()
                .messages
                .len(),
            1
        );
        assert_eq!(
            load_stats_from_vault(&vault, &account).unwrap().usage_count,
            5
        );
    }
}

#[test]
fn rf004_context_selection_accepts_camel_case_json_and_rejects_incomplete_requests() {
    let none: ChatContextSelection = serde_json::from_str(r#"{"mode":"none"}"#).unwrap();
    assert!(matches!(none, ChatContextSelection::None));
    let payload: Value = serde_json::from_str(
        r#"{
            "mode": "publicProfile",
            "objectIds": ["rf004-json-object"],
            "language": "zh-CN",
            "guideChunks": [{
                "guideId": "rf004-json-guide",
                "guideTitle": "反序列化标题",
                "chunkText": "反序列化正文🪶",
                "similarity": 0.75
            }]
        }"#,
    )
    .unwrap();
    let decoded: ChatContextSelection = serde_json::from_value(payload.clone()).unwrap();
    match &decoded {
        ChatContextSelection::PublicProfile {
            object_ids,
            language,
            guide_chunks,
        } => {
            assert_eq!(object_ids, &["rf004-json-object"]);
            assert_eq!(language, "zh-CN");
            assert_eq!(guide_chunks.len(), 1);
            assert_eq!(guide_chunks[0].guide_id, "rf004-json-guide");
            assert_eq!(guide_chunks[0].guide_title, "反序列化标题");
            assert_eq!(guide_chunks[0].chunk_text, "反序列化正文🪶");
            assert_eq!(guide_chunks[0].similarity, 0.75);
        }
        ChatContextSelection::None => panic!("publicProfile decoded as none"),
    }
    assert_eq!(serde_json::to_value(decoded).unwrap(), payload);
    assert!(
        serde_json::from_value::<ChatContextSelection>(json!({"mode": "unknownMode"})).is_err()
    );
    for field in ["mode", "objectIds", "language", "guideChunks"] {
        let mut incomplete = payload.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ChatContextSelection>(incomplete).is_err(),
            "Missing selection field {field} unexpectedly accepted"
        );
    }
    for field in ["guideId", "guideTitle", "chunkText", "similarity"] {
        let mut incomplete = payload.clone();
        incomplete["guideChunks"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<ChatContextSelection>(incomplete).is_err(),
            "Missing guide field {field} unexpectedly accepted"
        );
    }
}

fn outgoing_openai_system(body: &Value) -> &str {
    let messages = body["messages"].as_array().unwrap();
    let systems: Vec<_> = messages
        .iter()
        .filter(|message| message["role"] == "system")
        .collect();
    assert_eq!(
        systems.len(),
        1,
        "Truncation must preserve a single system message"
    );
    assert!(messages
        .iter()
        .any(|message| message["role"] == "user" && message["content"] == USER_TEXT));
    systems[0]["content"].as_str().unwrap()
}

fn unicode_guide(title: String, text: String) -> GuideChunk {
    GuideChunk {
        guide_id: "rf004-unicode-guide".into(),
        guide_title: title,
        chunk_text: text,
        similarity: 0.75,
    }
}

#[tokio::test]
async fn rf004_outbound_unicode_prompt_and_guide_bounds_keep_readable_prefixes() {
    // 四条短片段总长远低于提示预算，第四条只能因数量边界而被排除。
    {
        let fixture = StreamFixture::new();
        let guides = [
            ("萤源一", "萤片一🌱"),
            ("萤源二", "萤片二🌲"),
            ("萤源三", "萤片三🌳"),
            ("萤源四", "萤片四🌴"),
        ];
        let body = send_and_capture(
            &fixture,
            ApiType::OpenAI,
            Some(ChatContextSelection::PublicProfile {
                object_ids: vec![],
                language: "zh-CN".into(),
                guide_chunks: guides
                    .iter()
                    .map(|(title, text)| unicode_guide((*title).into(), (*text).into()))
                    .collect(),
            }),
        )
        .await;
        let system = outgoing_openai_system(&body);
        assert!(system.contains("SoloSoul"));
        for (title, text) in &guides[..3] {
            assert!(system.contains(*title));
            assert!(system.contains(*text));
        }
        assert!(!system.contains(guides[3].0));
        assert!(!system.contains(guides[3].1));
        assert_eq!(system.matches("【文档片段 ").count(), 3);
        assert!(!system.contains("以下内容因长度限制被截断"));
    }

    // 最后一个允许字符为非 BMP 字符；截断按 Unicode scalar 而不是字节数。
    {
        let fixture = StreamFixture::new();
        let title_prefix = format!("{}🦚", "鱻".repeat(119));
        let text_prefix = format!("{}🪷", "龘".repeat(1499));
        let body = send_and_capture(
            &fixture,
            ApiType::OpenAI,
            Some(ChatContextSelection::PublicProfile {
                object_ids: vec![],
                language: "zh-CN".into(),
                guide_chunks: vec![unicode_guide(
                    format!("{title_prefix}RF004_TITLE_OVERFLOW"),
                    format!("{text_prefix}RF004_BODY_OVERFLOW"),
                )],
            }),
        )
        .await;
        let system = outgoing_openai_system(&body);
        assert!(system.contains(&format!("《{title_prefix}》")));
        assert!(system.contains(&format!("```text\n{text_prefix}\n```")));
        assert_eq!(system.matches('鱻').count(), 119);
        assert_eq!(system.matches('龘').count(), 1499);
        assert!(!system.contains("RF004_TITLE_OVERFLOW"));
        assert!(!system.contains("RF004_BODY_OVERFLOW"));
        assert!(!system.contains("以下内容因长度限制被截断"));
        assert!(system.chars().count() <= 3000);
    }

    // 长对象名称突破七段基础提示的预算；首个完整对象保留，后续尾部被截断。
    {
        let fixture = StreamFixture::new();
        let account = fixture.context.session.account_id();
        let ids = [
            "rf004-long-name-one",
            "rf004-long-name-two",
            "rf004-long-name-three",
        ];
        fixture
            .context
            .with_vault(|vault| {
                for (index, id) in ids.iter().enumerate() {
                    let mut object = synthetic_object(account, id, "公开值");
                    object.name = format!("碑首{index}{}碑尾{index}", "篆".repeat(900));
                    vault.save_object(&object)?;
                }
                Ok(())
            })
            .unwrap();
        let body = send_and_capture(
            &fixture,
            ApiType::OpenAI,
            Some(ChatContextSelection::PublicProfile {
                object_ids: ids.iter().map(|id| (*id).into()).collect(),
                language: "zh-CN".into(),
                guide_chunks: vec![],
            }),
        )
        .await;
        let system = outgoing_openai_system(&body);
        assert!(system.contains("碑首0"));
        assert!(system.contains("碑尾0"));
        assert!(!system.contains("碑尾2"));
        assert!(system.ends_with("（上下文过长，部分内容已省略）"));
        assert!(
            system.chars().count() > 1000,
            "Unicode names were cut as bytes"
        );
        assert!(system.chars().count() <= 1500);
        assert!(
            system.len() > 1500,
            "UTF-8 byte length should exceed character budget"
        );
    }

    // 每条指南自身都合法，合并之后超过总预算，保留首片尾部并提示合并截断。
    {
        let fixture = StreamFixture::new();
        let body = send_and_capture(
            &fixture,
            ApiType::OpenAI,
            Some(ChatContextSelection::PublicProfile {
                object_ids: vec![],
                language: "zh-CN".into(),
                guide_chunks: vec![
                    unicode_guide("羽源一".into(), format!("{}首片尾🦢", "翎".repeat(1400))),
                    unicode_guide("羽源二".into(), format!("{}次片尾🦜", "黻".repeat(1400))),
                    unicode_guide("羽源三".into(), format!("{}末片尾🦩", "叒".repeat(1400))),
                ],
            }),
        )
        .await;
        let system = outgoing_openai_system(&body);
        assert!(system.contains("SoloSoul"));
        assert!(system.contains("首片尾🦢"));
        assert!(!system.contains("次片尾🦜"));
        assert!(!system.contains("末片尾🦩"));
        assert!(system.ends_with("（以下内容因长度限制被截断）"));
        assert!(
            system.chars().count() > 2000,
            "Unicode guides were cut as bytes"
        );
        assert!(system.chars().count() <= 3000);
        assert!(
            system.len() > 3000,
            "UTF-8 byte length should exceed character budget"
        );
    }
}
