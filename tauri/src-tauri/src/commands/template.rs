//! User template commands (§29 模板系统重构 P1)
//!
//! Provides full CRUD for user-defined object templates stored in the
//! `user_templates` table (schema v7).  Legacy templates that were previously
//! squirrelled away inside `profile.data.preferences.userTemplates` are
//! lazily migrated the first time any template command is invoked after the
//! vault is unlocked.

use crate::commands::{current_account, vault_handle};
use crate::state::AppState;
use solosoul_vault::{PropertyType, TemplateProperty, UserTemplate};
use tauri::State;

// ---------------------------------------------------------------------------
// Legacy migration helper
// ---------------------------------------------------------------------------

/// Migrate old-style templates from Profile JSON to the dedicated table.
/// This is idempotent: if `user_templates` already has rows for the account
/// the function returns immediately.
fn migrate_legacy_templates_if_needed(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
) -> Result<(), String> {
    // Idempotency check
    let existing = vault.count_user_templates(account_id)?;
    if existing > 0 {
        // Already migrated; optionally clean up legacy JSON if still present
        cleanup_legacy_json(vault, account_id)?;
        return Ok(());
    }

    let profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        _ => return Ok(()), // nothing to migrate
    };

    if profile.data.is_empty() {
        return Ok(());
    }

    let data: serde_json::Value =
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse profile data: {}", e))?;

    let legacy_templates = data
        .get("preferences")
        .and_then(|p| p.get("userTemplates"))
        .and_then(|v| serde_json::from_value::<Vec<serde_json::Value>>(v.clone()).ok())
        .unwrap_or_default();

    if legacy_templates.is_empty() {
        tracing::warn!("Legacy migration: found profile data but no userTemplates array, skipping");
    }

    for tpl in legacy_templates {
        let name = tpl
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名模板")
            .to_string();
        let icon_id = tpl.get("iconId").and_then(|v| v.as_str()).map(String::from);

        // Legacy properties were stored as [{id, name, type}] with type always "text"
        let properties: Vec<TemplateProperty> = tpl
            .get("properties")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let id = p.get("id")?.as_str()?.to_string();
                        let name = p.get("name")?.as_str()?.to_string();
                        // Legacy templates forced everything to "text"; preserve that
                        // so existing templates don't break.
                        Some(TemplateProperty {
                            contract_field: None,
                            contract_bindings: None,
                            id,
                            name,
                            prop_type: PropertyType::Text,
                            sensitivity_level: None,
                            sensitive: None,
                            options: None,
                            deprecated_at: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let created_at = tpl
            .get("createdAt")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        let template = UserTemplate {
            contract_type_id: None,
            id: format!("utpl_{}", uuid::Uuid::new_v4().simple()),
            account_id: account_id.to_string(),
            name,
            icon_id,
            properties,
            category: Some("identity".to_string()),
            created_at,
            updated_at: None,
        };

        vault.save_user_template(&template)?;
    }

    // Clean up legacy JSON after successful migration
    cleanup_legacy_json(vault, account_id)?;

    Ok(())
}

/// Remove the legacy `preferences.userTemplates` key from Profile data.
fn cleanup_legacy_json(vault: &solosoul_vault::VaultStore, account_id: &str) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id)? {
        Some(p) => p,
        None => return Ok(()),
    };

    if profile.data.is_empty() {
        return Ok(());
    }

    let mut data: serde_json::Value =
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?;

    if let Some(prefs) = data.get_mut("preferences").and_then(|v| v.as_object_mut()) {
        if prefs.remove("userTemplates").is_some() {
            profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
            profile.updated_at = chrono::Utc::now();
            profile.version += 1;
            vault.save_profile(&profile)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// IPC commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn template_create(
    state: State<'_, AppState>,
    name: String,
    icon_id: Option<String>,
    category: Option<String>,
    properties: Vec<TemplateProperty>,
    contract_type_id: Option<String>,
) -> Result<String, String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let template = UserTemplate {
        contract_type_id,
        id: format!("utpl_{}", uuid::Uuid::new_v4().simple()),
        account_id: account_id.clone(),
        name,
        icon_id,
        properties,
        category,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };

    vault.save_user_template(&template)?;

    let _ = vault.log_structured(
        "template_create",
        "template",
        Some(&template.id),
        Some(&template.name),
        "user",
        Some(&format!("name={}", template.name)),
    );

    Ok(template.id)
}

#[tauri::command]
pub async fn template_update(
    state: State<'_, AppState>,
    template_id: String,
    name: Option<String>,
    icon_id: Option<String>,
    category: Option<String>,
    properties: Option<Vec<TemplateProperty>>,
    contract_type_id: Option<String>,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let mut template = vault
        .load_user_template(&template_id)?
        .ok_or_else(|| "模板不存在".to_string())?;

    if template.account_id != account_id {
        return Err("无权修改此模板".to_string());
    }

    if let Some(n) = name {
        template.name = n;
    }
    if let Some(i) = icon_id {
        template.icon_id = Some(i);
    }
    if let Some(c) = category {
        template.category = Some(c);
    }
    if let Some(p) = properties {
        template.properties = p;
    }
    if let Some(ct) = contract_type_id {
        // 传入空字符串表示清除 contract_type_id
        if ct.is_empty() {
            template.contract_type_id = None;
        } else {
            template.contract_type_id = Some(ct);
        }
    }
    template.updated_at = Some(chrono::Utc::now().to_rfc3339());

    vault.save_user_template(&template)?;

    let _ = vault.log_structured(
        "template_update",
        "template",
        Some(&template_id),
        Some(&template.name),
        "user",
        None,
    );

    Ok(())
}

#[tauri::command]
pub async fn template_check_field_usage(
    state: State<'_, AppState>,
    template_id: String,
    field_key: String,
) -> Result<serde_json::Value, String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    // Verify template ownership
    let template = vault
        .load_user_template(&template_id)?
        .ok_or_else(|| "模板不存在".to_string())?;
    if template.account_id != account_id {
        return Err("无权查看此模板".to_string());
    }

    let (active, soft_deleted) = vault.check_field_usage(&account_id, &field_key)?;
    Ok(serde_json::json!({
        "active": active,
        "softDeleted": soft_deleted,
    }))
}

#[tauri::command]
pub async fn template_delete(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let template = vault
        .load_user_template(&template_id)?
        .ok_or_else(|| "模板不存在".to_string())?;

    if template.account_id != account_id {
        return Err("无权删除此模板".to_string());
    }

    // Load retention period and build TrashItem
    let period = load_trash_retention(&vault, &account_id);
    let retention_ms = retention_ms(&period);
    let now_ms = chrono::Utc::now().timestamp_millis();

    let template_data =
        serde_json::to_vec(&template).map_err(|e| format!("序列化模板失败: {}", e))?;

    let trash = solosoul_vault::TrashItem {
        id: format!("trash_{}", uuid::Uuid::new_v4()),
        item_type: "template".to_string(),
        original_id: template_id.clone(),
        original_parent_id: None,
        original_section_type: template.category.clone(),
        original_sort_order: None,
        data: template_data,
        deleted_at: now_ms,
        expires_at: Some(now_ms + retention_ms),
        deleted_by: "user".to_string(),
        name_snapshot: template.name.clone(),
        icon_snapshot: template.icon_id.clone(),
    };

    vault.save_trash_item(&trash)?;
    vault.delete_user_template(&template_id)?;

    let _ = vault.log_structured(
        "template_delete",
        "template",
        Some(&template_id),
        Some(&template.name),
        "user",
        Some(&format!("name={}", template.name)),
    );

    Ok(())
}

#[tauri::command]
pub async fn template_restore(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<String, String> {
    let vault = vault_handle(&state)?;

    let trash = vault.get_trash_item(&trash_id)?.ok_or("回收站项目不存在")?;

    if trash.item_type != "template" {
        return Err("该回收站项目不是模板".to_string());
    }

    let template: solosoul_vault::UserTemplate =
        serde_json::from_slice(&trash.data).map_err(|e| format!("反序列化模板失败: {}", e))?;

    if vault.load_user_template(&template.id)?.is_some() {
        return Err("该模板已存在，无需恢复".to_string());
    }

    vault.save_user_template(&template)?;
    vault.delete_trash_item(&trash_id)?;

    let _ = vault.log_structured(
        "template_restore",
        "template",
        Some(&template.id),
        Some(&template.name),
        "user",
        None,
    );

    Ok(template.id)
}

#[tauri::command]
pub async fn template_get(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<UserTemplate, String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let template = vault
        .load_user_template(&template_id)?
        .ok_or_else(|| "模板不存在".to_string())?;

    if template.account_id != account_id {
        return Err("无权查看此模板".to_string());
    }

    Ok(template)
}

#[tauri::command]
pub async fn template_list(state: State<'_, AppState>) -> Result<Vec<UserTemplate>, String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    // Lazy migration: if this is the first template call after unlock,
    // migrate legacy Profile-JSON templates into the new table.
    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let templates = vault.list_user_templates(&account_id)?;
    Ok(templates)
}

#[tauri::command]
pub async fn template_save_from_object(
    state: State<'_, AppState>,
    object_id: String,
    template_name: String,
    icon_id: Option<String>,
) -> Result<String, String> {
    let vault = vault_handle(&state)?;

    let account_id = current_account(&state)?;

    migrate_legacy_templates_if_needed(&vault, &account_id)?;

    let record = vault.load_object(&object_id)?.ok_or("Object not found")?;

    let properties: Vec<TemplateProperty> =
        if let serde_json::Value::Object(ref props) = record.properties {
            props
                .iter()
                .map(|(key, value)| {
                    let prop_type = PropertyType::infer_from_value(value, key);
                    TemplateProperty {
                        contract_field: None,
                        contract_bindings: None,
                        id: key.clone(),
                        name: key.clone(),
                        prop_type,
                        sensitivity_level: None,
                        sensitive: None,
                        options: None,
                        deprecated_at: None,
                    }
                })
                .collect()
        } else {
            vec![]
        };

    let template = UserTemplate {
        contract_type_id: None,
        id: format!("utpl_{}", uuid::Uuid::new_v4().simple()),
        account_id: account_id.clone(),
        name: template_name,
        icon_id: icon_id.or(Some(record.icon_name)),
        properties,
        category: Some(record.section_type.clone()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: None,
    };

    vault.save_user_template(&template)?;

    let _ = vault.log_structured(
        "template_save_from_object",
        "template",
        Some(&template.id),
        Some(&template.name),
        "user",
        None,
    );

    Ok(template.id)
}

// ── Trash retention helpers (delegated to shared snapshot.rs) ─

fn load_trash_retention(vault: &solosoul_vault::VaultStore, account_id: &str) -> String {
    super::object::snapshot::load_trash_retention(vault, account_id)
}

fn retention_ms(period: &str) -> i64 {
    super::object::snapshot::retention_ms(period)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── retention_ms ──────────────────────────────────────────────

    #[test]
    fn test_retention_ms_default_30d() {
        assert_eq!(retention_ms(""), 30 * 24 * 3600 * 1000);
        assert_eq!(retention_ms("invalid"), 30 * 24 * 3600 * 1000);
        assert_eq!(retention_ms("30d"), 30 * 24 * 3600 * 1000);
    }

    #[test]
    fn test_retention_ms_60d() {
        assert_eq!(retention_ms("60d"), 60 * 24 * 3600 * 1000);
    }

    #[test]
    fn test_retention_ms_half_year() {
        assert_eq!(retention_ms("half_year"), 180 * 24 * 3600 * 1000);
    }

    #[test]
    fn test_retention_ms_one_year() {
        assert_eq!(retention_ms("one_year"), 365 * 24 * 3600 * 1000);
    }

    #[test]
    fn test_retention_ms_never() {
        assert_eq!(retention_ms("never"), i64::MAX);
    }

    // ── load_trash_retention ──────────────────────────────────────

    #[test]
    fn test_load_trash_retention_default_when_no_profile() {
        let (vault, _dir) = setup_vault();
        let result = load_trash_retention(&vault, "nonexistent");
        assert_eq!(result, "30d");
    }

    #[test]
    fn test_load_trash_retention_from_profile() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {"trashRetention": "60d"}
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        let result = load_trash_retention(&vault, "acc-1");
        assert_eq!(result, "60d");
    }

    #[test]
    fn test_load_trash_retention_default_when_missing_key() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({"preferences": {}});
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        let result = load_trash_retention(&vault, "acc-1");
        assert_eq!(result, "30d");
    }

    #[test]
    fn test_load_trash_retention_default_when_empty_data() {
        let (vault, _dir) = setup_vault();
        let profile = solosoul_vault::Profile::new_with_id("acc-1", "Test", vec![]);
        vault.save_profile(&profile).unwrap();

        let result = load_trash_retention(&vault, "acc-1");
        assert_eq!(result, "30d");
    }

    // ── cleanup_legacy_json ───────────────────────────────────────

    #[test]
    fn test_cleanup_legacy_json_removes_user_templates() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {
                "userTemplates": [{"name": "Old"}],
                "other": "value"
            }
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        cleanup_legacy_json(&vault, "acc-1").unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert!(parsed["preferences"]["userTemplates"].is_null());
        assert_eq!(parsed["preferences"]["other"], "value");
    }

    #[test]
    fn test_cleanup_legacy_json_no_op_when_no_templates() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({"preferences": {"other": "value"}});
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        cleanup_legacy_json(&vault, "acc-1").unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert_eq!(parsed["preferences"]["other"], "value");
    }

    #[test]
    fn test_cleanup_legacy_json_no_op_when_no_preferences() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({"other": "value"});
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        cleanup_legacy_json(&vault, "acc-1").unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert_eq!(parsed["other"], "value");
    }

    #[test]
    fn test_cleanup_legacy_json_idempotent() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {"userTemplates": [{"name": "T"}]}
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        // Run twice
        cleanup_legacy_json(&vault, "acc-1").unwrap();
        cleanup_legacy_json(&vault, "acc-1").unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert!(parsed["preferences"]["userTemplates"].is_null());
    }

    // ── migrate_legacy_templates_if_needed ────────────────────────

    #[test]
    fn test_migrate_legacy_templates_if_needed_creates_templates() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {
                "userTemplates": [
                    {"name": "Contact", "iconId": "user",
                     "properties": [{"id": "f1", "name": "Name", "type": "text"}]}
                ]
            }
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        migrate_legacy_templates_if_needed(&vault, "acc-1").unwrap();

        let templates = vault.list_user_templates("acc-1").unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Contact");
        assert_eq!(templates[0].icon_id.as_deref(), Some("user"));
        assert_eq!(templates[0].properties.len(), 1);
        assert_eq!(templates[0].properties[0].id, "f1");
    }

    #[test]
    fn test_migrate_legacy_templates_idempotent() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {
                "userTemplates": [{"name": "Card", "properties": []}]
            }
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        // First run: migrates
        migrate_legacy_templates_if_needed(&vault, "acc-1").unwrap();
        // Second run: idempotent — already has templates, skips migration
        migrate_legacy_templates_if_needed(&vault, "acc-1").unwrap();

        let templates = vault.list_user_templates("acc-1").unwrap();
        assert_eq!(templates.len(), 1, "Should not create duplicate templates");
    }

    #[test]
    fn test_migrate_legacy_templates_no_op_when_no_legacy() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({"preferences": {}});
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        migrate_legacy_templates_if_needed(&vault, "acc-1").unwrap();

        let templates = vault.list_user_templates("acc-1").unwrap();
        assert_eq!(templates.len(), 0);
    }

    #[test]
    fn test_migrate_legacy_templates_handles_empty_properties() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::json!({
            "preferences": {
                "userTemplates": [{"name": "Empty", "properties": null}]
            }
        });
        let profile = solosoul_vault::Profile::new_with_id(
            "acc-1",
            "Test",
            serde_json::to_vec(&data).unwrap(),
        );
        vault.save_profile(&profile).unwrap();

        migrate_legacy_templates_if_needed(&vault, "acc-1").unwrap();

        let templates = vault.list_user_templates("acc-1").unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].properties.len(), 0);
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn setup_vault() -> (solosoul_vault::VaultStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let config = solosoul_vault::VaultConfig::new("test", dir.path().to_path_buf())
            .with_data_key([0x42u8; 32]);
        let vault = solosoul_vault::VaultStore::open(config).unwrap();
        (vault, dir)
    }
}
