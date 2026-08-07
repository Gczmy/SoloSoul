//! 历史快照命令 /history、/rollback。

use color_eyre::Result;
use std::time::Instant;

use crate::app::{App, AppPhase};
use crate::commands::require_unlocked;
use crate::commands::require_unlocked_with_vault;
use crate::t;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 执行 `/history <object-id>`：列出对象快照。
pub fn history(app: &mut App, object_id: Option<&str>) -> Result<()> {
    let (account_id, vault) = require_unlocked_with_vault(app)?;
    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => {
            app.error_message = Some(t!(app.i18n, "cmd-history-usage"));
            return Ok(());
        }
    };

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

    // P031：校验快照归属——快照必须属于目标对象，防止把别的对象数据套到本对象
    let owner = vault
        .get_snapshot_owner(snapshot_id)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    if owner.as_deref() != Some(object_id) {
        return Err(color_eyre::eyre::eyre!("快照不属于该对象"));
    }

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

    // 保存回滚快照（P031：序列化/保存失败不得静默留下空快照）
    let rollback_data = serde_json::to_vec(&serde_json::json!({
        "name": record.name,
        "tags": record.tags_json,
        "properties": record.properties,
    }))
    .map_err(|e| color_eyre::eyre::eyre!("序列化回滚快照失败: {}", e))?;
    vault
        .save_snapshot(
            object_id,
            "rollback",
            &rollback_data,
            "Rolled back to previous version",
        )
        .map_err(|e| color_eyre::eyre::eyre!("保存回滚快照失败: {}", e))?;
    vault
        .log_structured(
            "object_rollback",
            "object",
            Some(object_id),
            Some(&record.name),
            "user",
            Some(&format!(
                "section={} snapshot={}",
                record.section_type, snapshot_id
            )),
        )
        .map_err(|e| color_eyre::eyre::eyre!("记录操作日志失败: {}", e))?;

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

    fn make_obj(account_id: String, id: &str, name: &str) -> ObjectRecord {
        ObjectRecord {
            id: id.to_string(),
            account_id,
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: name.to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({ "title": name }),
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
        }
    }

    fn snapshot_id_of(vault: &solosoul_vault::VaultStore, object_id: &str) -> String {
        let snaps = vault.list_snapshots(object_id).unwrap();
        assert!(!snaps.is_empty(), "对象 {} 应有快照", object_id);
        snaps[0]["id"].as_str().unwrap().to_string()
    }

    /// P031：跨对象快照回滚必须被拒绝，且目标对象数据不被篡改。
    #[test]
    fn test_rollback_rejects_snapshot_from_other_object() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        let obj_a_id = format!("obj_a_{}", obj_counter());
        let obj_b_id = format!("obj_b_{}", obj_counter());
        vault
            .save_object(&make_obj(account_id.clone(), &obj_a_id, "A"))
            .unwrap();
        vault
            .save_object(&make_obj(account_id.clone(), &obj_b_id, "B"))
            .unwrap();

        // 对象 B 保存快照
        let snap_b =
            serde_json::to_vec(&serde_json::json!({"name": "B", "properties": {}})).unwrap();
        vault
            .save_snapshot(&obj_b_id, "user_edit", &snap_b, "Created")
            .unwrap();
        let snap_id = snapshot_id_of(&vault, &obj_b_id);

        // 对对象 A 回滚 B 的快照 → 拒绝
        let err = do_rollback(&mut app, &obj_a_id, &snap_id).unwrap_err();
        assert!(
            err.to_string().contains("快照不属于该对象"),
            "应拒绝跨对象快照: {}",
            err
        );

        // 对象 A 数据未被篡改
        let obj_a = vault.load_object(&obj_a_id).unwrap().unwrap();
        assert_eq!(obj_a.name, "A");
        assert_eq!(obj_a.properties["title"], "A");
    }

    /// P031 正向控制：同对象快照回滚成功。
    #[test]
    fn test_rollback_accepts_snapshot_of_same_object() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        let obj_id = format!("obj_ok_{}", obj_counter());
        vault
            .save_object(&make_obj(account_id.clone(), &obj_id, "old_name"))
            .unwrap();

        // 保存初始快照，然后把对象改成新状态
        let snap_data = serde_json::to_vec(&serde_json::json!({
            "name": "old_name",
            "tags": [],
            "properties": { "title": "old_title" },
        }))
        .unwrap();
        vault
            .save_snapshot(&obj_id, "user_edit", &snap_data, "Created")
            .unwrap();
        let snap_id = snapshot_id_of(&vault, &obj_id);

        let mut obj = vault.load_object(&obj_id).unwrap().unwrap();
        obj.name = "new_name".to_string();
        obj.properties = serde_json::json!({ "title": "new_title" });
        vault.save_object(&obj).unwrap();

        // 回滚到旧快照
        do_rollback(&mut app, &obj_id, &snap_id).unwrap();
        let rolled = vault.load_object(&obj_id).unwrap().unwrap();
        assert_eq!(rolled.name, "old_name");
        assert_eq!(rolled.properties["title"], "old_title");
        assert!(app.success_message.is_some());
    }
}
