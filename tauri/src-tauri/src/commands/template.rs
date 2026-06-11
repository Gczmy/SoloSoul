//! User template commands (§29 模板系统重构 P1)
//!
//! Provides full CRUD for user-defined object templates stored in the
//! `user_templates` table (schema v7).  Legacy templates that were previously
//! squirrelled away inside `profile.data.preferences.userTemplates` are
//! lazily migrated the first time any template command is invoked after the
//! vault is unlocked.

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
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

    let template = UserTemplate {
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
        None,
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
) -> Result<(), String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

    let template = vault
        .load_user_template(&template_id)?
        .ok_or_else(|| "模板不存在".to_string())?;

    if template.account_id != account_id {
        return Err("无权删除此模板".to_string());
    }

    // Load retention period and build TrashItem
    let period = load_trash_retention(vault, &account_id);
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
        None,
    );

    Ok(())
}

#[tauri::command]
pub async fn template_restore(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    // Lazy migration: if this is the first template call after unlock,
    // migrate legacy Profile-JSON templates into the new table.
    migrate_legacy_templates_if_needed(vault, &account_id)?;

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
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let account_id = svc.get_current_account().ok_or("No unlocked account")?;

    migrate_legacy_templates_if_needed(vault, &account_id)?;

    let record = vault.load_object(&object_id)?.ok_or("Object not found")?;

    let properties: Vec<TemplateProperty> =
        if let serde_json::Value::Object(ref props) = record.properties {
            props
                .iter()
                .map(|(key, value)| {
                    let prop_type = PropertyType::infer_from_value(value, key);
                    TemplateProperty {
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

// ── Trash retention helpers (mirrored from object.rs) ────────

fn load_trash_retention(vault: &solosoul_vault::VaultStore, account_id: &str) -> String {
    if let Ok(Some(profile)) = vault.load_profile(account_id) {
        if !profile.data.is_empty() {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) {
                if let Some(ret) = data
                    .pointer("/preferences/trashRetention")
                    .and_then(|v| v.as_str())
                {
                    return ret.to_string();
                }
            }
        }
    }
    "30d".to_string()
}

fn retention_ms(period: &str) -> i64 {
    match period {
        "60d" => 60 * 24 * 3600 * 1000i64,
        "half_year" => 180 * 24 * 3600 * 1000i64,
        "one_year" => 365 * 24 * 3600 * 1000i64,
        "never" => i64::MAX,
        _ => 30 * 24 * 3600 * 1000i64,
    }
}
