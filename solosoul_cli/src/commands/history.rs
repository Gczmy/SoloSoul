//! 历史快照命令 /history、/rollback。

use color_eyre::Result;
use std::time::Instant;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked;
use crate::t;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 执行 `/history <object-id>`：列出对象快照。
pub fn history(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let account_id = require_unlocked(app)?;
    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-history-usage"));
            return Ok(());
        }
    };

    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    // 校验对象存在且属于当前账户
    match vault
        .load_object(&object_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
    {
        Some(record) if record.account_id == account_id && !record.is_deleted => {
            let snapshots = vault
                .list_snapshots(&object_id)
                .map_err(|e| color_eyre::eyre::eyre!(e))?;
            app.previous_phase = Some(app.phase.clone());
            app.phase = AppPhase::HistoryList {
                object_id,
                snapshots,
                selected: 0,
            };
            Ok(())
        }
        _ => {
            app.error_message = Some(t!(app.i18n, "cmd-object-not-found", id = object_id));
            Ok(())
        }
    }
}

/// 执行 `/rollback <object-id> <snapshot-id>`：回滚到指定快照。
pub fn rollback(app: &mut App, object_id: Option<&str>, snapshot_id: Option<&str>) -> Result<()> {
    require_unlocked(app)?;
    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-rollback-usage"));
            return Ok(());
        }
    };
    let snapshot_id = match snapshot_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-rollback-usage"));
            return Ok(());
        }
    };

    prompt::open(
        app,
        PromptSpec::Confirm {
            message: t!(
                app.i18n,
                "cmd-rollback-confirm",
                obj = &object_id,
                snap = &snapshot_id
            ),
            default_yes: false,
        },
        Box::new(move |app, result| {
            if let PromptResult::Confirm(true) = result {
                if let Err(e) = do_rollback(app, &object_id, &snapshot_id) {
                    app.error_message = Some(t!(app.i18n, "cmd-operation-failed", err = e));
                }
            }
        }),
    );

    Ok(())
}

fn do_rollback(app: &mut App, object_id: &str, snapshot_id: &str) -> Result<()> {
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| color_eyre::eyre::eyre!("Vault 未打开"))?;

    let data = vault
        .get_snapshot(snapshot_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
        .ok_or_else(|| color_eyre::eyre::eyre!("快照不存在"))?;
    let snapshot: serde_json::Value = serde_json::from_slice(&data)
        .map_err(|e| color_eyre::eyre::eyre!("解析快照失败: {}", e))?;

    let mut record = vault
        .load_object(object_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
        .ok_or_else(|| color_eyre::eyre::eyre!("对象不存在"))?;

    if let Some(name) = snapshot["name"].as_str() {
        record.name = name.to_string();
    }
    if let Some(tags) = snapshot["tags"].as_array() {
        record.tags_json = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if !snapshot["properties"].is_null() {
        record.properties = snapshot["properties"].clone();
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    record.version += 1;

    vault
        .save_object(&record)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    // 保存回滚快照
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
    }))
    .unwrap_or_default();
    let _ = vault.save_snapshot(
        object_id,
        "rollback",
        &rollback_data,
        "Rolled back to previous version",
    );
    let _ = vault.log_structured(
        "object_rollback",
        "object",
        Some(object_id),
        Some(&record.name),
        "user",
        Some(&format!(
            "section={} snapshot={}",
            record.section_type, snapshot_id
        )),
    );

    app.success_message = Some((
        t!(
            app.i18n,
            "cmd-rollback-complete",
            name = &record.name,
            snap = snapshot_id
        ),
        Instant::now(),
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::{ObjectRecord, VaultService};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    static OBJ_COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn obj_counter() -> usize {
        OBJ_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_history_missing_id() {
        let (mut app, _id, _dir) = unlocked_app();
        history(&mut app, None).unwrap();
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_history_lists_snapshots() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        let obj = ObjectRecord {
            id: format!("obj_test_{}", obj_counter()),
            account_id,
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "原名称".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({ "title": "old" }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            template_hash: None,
            ignored_template_hash: None,
        };
        vault.save_object(&obj).unwrap();

        // 保存一个初始快照
        let snapshot_data = serde_json::to_vec(&serde_json::json!({
            "name": obj.name,
            "tags": obj.tags_json,
            "properties": obj.properties,
        }))
        .unwrap();
        let _ = vault.save_snapshot(&obj.id, "user_edit", &snapshot_data, "Created");

        history(&mut app, Some(&obj.id)).unwrap();
        match &app.phase {
            AppPhase::HistoryList { snapshots, .. } => {
                assert!(!snapshots.is_empty());
            }
            _ => panic!("expected HistoryList"),
        }
    }

    #[test]
    fn test_rollback_without_args() {
        let (mut app, _id, _dir) = unlocked_app();
        rollback(&mut app, None, None).unwrap();
        assert!(app.error_message.is_some());
    }
}
