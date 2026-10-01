//! RF-303：同一 JSON fixture 同时由 Rust serde 和生成 TypeScript 检验。
use super::{ChatContextSelection, LlmStreamPayload};
use serde_json::{json, Value};
use solosoul_core::llm::config::{Conversation, ConversationSummary};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

#[test]
fn rf303_event_identity_and_nullable_error_are_serialized_exactly() {
    let payload = LlmStreamPayload {
        account_id: "synthetic-account".into(),
        session_generation: 42,
        conversation_id: "synthetic-conversation".into(),
        request_id: "synthetic-request".into(),
        chunk: "最后一段正文".into(),
        is_done: true,
        error: None,
    };
    let complete = serde_json::to_value(&payload).unwrap();
    assert_eq!(complete, fixture()["complete"]);
    assert_eq!(complete.as_object().unwrap().len(), 7);
    assert!(complete.get("error").unwrap().is_null());
}

#[test]
fn rf303_done_is_not_a_persistence_success_or_an_upstream_failure() {
    let data = fixture();
    for (key, is_done, error) in [
        (
            "persistFailed",
            true,
            "__LLM_PERSIST_FAILED__: synthetic storage failure",
        ),
        ("upstreamFailed", false, "HTTP 500: synthetic"),
    ] {
        let payload = LlmStreamPayload {
            account_id: "synthetic-account".into(),
            session_generation: 42,
            conversation_id: "synthetic-conversation".into(),
            request_id: "synthetic-request".into(),
            chunk: String::new(),
            is_done,
            error: Some(error.into()),
        };
        assert_eq!(serde_json::to_value(payload).unwrap(), data[key]);
    }
}

#[test]
fn rf303_legacy_roles_temporary_history_and_optional_deletion_round_trip() {
    let data = fixture();
    let conversation: Conversation = serde_json::from_value(data["conversation"].clone()).unwrap();
    assert!(conversation.is_temporary);
    assert_eq!(conversation.messages[0].role, "system");
    assert_eq!(conversation.messages[1].role, "legacy-tool-role");
    assert!(conversation.deleted_at.is_none());
    assert_eq!(
        serde_json::to_value(&conversation).unwrap(),
        data["conversation"]
    );
    let summary = ConversationSummary {
        id: conversation.id.clone(),
        name: conversation.name.clone(),
        updated_at: conversation.updated_at.clone(),
        message_count: conversation.messages.len(),
        deleted_at: None,
    };
    assert_eq!(serde_json::to_value(summary).unwrap(), data["summary"]);
    let mut restored = data["conversation"].clone();
    restored["deletedAt"] = Value::Null;
    let restored: Conversation = serde_json::from_value(restored).unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        data["conversation"]
    );
    let mut deleted = conversation;
    deleted.deleted_at = Some("2026-10-01T00:00:00Z".into());
    assert_eq!(
        serde_json::to_value(&deleted).unwrap()["deletedAt"],
        "2026-10-01T00:00:00Z"
    );
    let decoded: Conversation =
        serde_json::from_value(serde_json::to_value(deleted).unwrap()).unwrap();
    assert_eq!(decoded.deleted_at.as_deref(), Some("2026-10-01T00:00:00Z"));
}

#[test]
fn rf303_context_variants_keep_the_existing_camel_case_wire_shape() {
    let data = fixture();
    for key in ["none", "publicProfile"] {
        let selection: ChatContextSelection = serde_json::from_value(data[key].clone()).unwrap();
        assert_eq!(serde_json::to_value(selection).unwrap(), data[key]);
    }
    for key in ["objectIds", "language", "guideChunks"] {
        let mut incomplete = data["publicProfile"].clone();
        incomplete.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<ChatContextSelection>(incomplete).is_err());
    }
    assert!(serde_json::from_value::<ChatContextSelection>(json!({"mode":"unknown"})).is_err());
}
