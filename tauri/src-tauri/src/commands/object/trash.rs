use crate::commands::vault_handle;
use crate::state::AppState;
use tauri::State;

use super::*;

/// Result returned by object_restore / trash_restore describing what happened.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_id: String,
    pub name: String,
    pub cascaded_page_name: Option<String>,
    pub cascaded_count: u32,
    pub rebuilt_page_name: Option<String>,
}
#[tauri::command]
pub async fn object_trash_list(
    state: State<'_, AppState>,
    account_id: String,
    since: Option<i64>,
) -> Result<Vec<solosoul_vault::TrashItemSummary>, String> {
    let _ = account_id;
    let vault = vault_handle(&state)?;
    vault.list_trash_items(None, since)
}

/// Read the user's language setting from plaintext UI preferences.
fn get_ui_language(svc: &solosoul_core::vault_service::VaultService) -> String {
    let path = svc.base_path().join("ui_preferences.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<serde_json::Value>(&content) {
                if prefs.is_object() {
                    if let Some(lang) = prefs.get("language").and_then(|v| v.as_str()) {
                        return lang.to_string();
                    }
                }
            }
        }
    }
    "en-US".to_string()
}

/// Built-in profile section identifiers. Anything else is treated as a custom page UUID.
const BUILT_IN_SECTIONS: &[&str] = &["identity", "travel", "financial", "professional"];

fn is_built_in_section(section_type: &str) -> bool {
    BUILT_IN_SECTIONS.contains(&section_type)
}

/// Find a page-type trash item whose original_id matches the given section_type (page UUID).
fn find_page_in_trash(
    vault: &solosoul_vault::VaultStore,
    page_id: &str,
) -> Result<Option<solosoul_vault::TrashItem>, String> {
    let all = vault.list_trash_items(None, None)?;
    for item in &all {
        if item.item_type == "page" && item.original_id == page_id {
            return vault.get_trash_item(&item.id);
        }
    }
    Ok(None)
}

/// Find all object trash items whose original_section_type matches the given section_type.
fn find_child_objects_in_trash(
    vault: &solosoul_vault::VaultStore,
    section_type: &str,
) -> Result<Vec<solosoul_vault::TrashItem>, String> {
    let all = vault.list_trash_items(None, None)?;
    let mut out = Vec::new();
    for item in &all {
        if item.item_type == "object" {
            if let Ok(Some(full)) = vault.get_trash_item(&item.id) {
                if full.original_section_type.as_deref() == Some(section_type) {
                    out.push(full);
                }
            }
        }
    }
    Ok(out)
}

/// Get the "(restored)" suffix localized to the user's language.
pub(crate) fn restored_suffix(language: &str) -> &'static str {
    match language {
        "zh-CN" => "（已恢复）",
        _ => " (restored)",
    }
}

/// Default localized name used when rebuilding a page stub whose original name is unknown.
pub(crate) fn recovered_page_name(language: &str) -> &'static str {
    match language {
        "zh-CN" => "已恢复的页面",
        _ => "Recovered Page",
    }
}

/// Restore a single non-deleted object from a trash item.
/// Returns the restored ObjectRecord and whether the ID was changed due to a name conflict.
fn restore_single_object(
    vault: &solosoul_vault::VaultStore,
    trash: &solosoul_vault::TrashItem,
    lang: &str,
) -> Result<(solosoul_vault::ObjectRecord, bool), String> {
    let record_data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("Invalid trash data: {}", e))?;

    // 回收站中保存的对象数据使用 camelCase 键，需兼容旧版 snake_case fallback。
    let get_str = |snake: &str, camel: &str| {
        record_data[camel]
            .as_str()
            .or_else(|| record_data[snake].as_str())
    };
    let get_array = |snake: &str, camel: &str| -> Option<&Vec<serde_json::Value>> {
        record_data[camel]
            .as_array()
            .or_else(|| record_data[snake].as_array())
    };

    let target_section = trash
        .original_section_type
        .as_deref()
        .or_else(|| record_data["sectionType"].as_str())
        .or_else(|| record_data["section_type"].as_str())
        .unwrap_or("identity");

    // Check if a non-deleted object with the same name exists in the target section (conflict)
    let account_id = get_str("account_id", "accountId").unwrap_or("imported");
    let objects = vault
        .list_objects(
            account_id,
            None,
            None,
            Some(&trash.name_snapshot),
            false,
            false,
        )
        .unwrap_or_default();
    let exists = objects
        .iter()
        .any(|o| o.name == trash.name_snapshot && o.section_type == target_section);

    let suffix = restored_suffix(lang);

    let new_id = if exists {
        format!(
            "{}_{}",
            trash.original_id,
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("restored")
        )
    } else {
        trash.original_id.clone()
    };

    let new_name = if exists {
        format!("{}{}", trash.name_snapshot, suffix)
    } else {
        trash.name_snapshot.clone()
    };

    let restore_contract_type_id =
        inherit_contract_type_id(vault, get_str("template_id", "templateId"));

    let now = chrono::Utc::now().to_rfc3339();
    let record = solosoul_vault::ObjectRecord {
        contract_type_id: restore_contract_type_id,
        id: new_id.clone(),
        account_id: get_str("account_id", "accountId")
            .unwrap_or("imported")
            .to_string(),
        type_id: get_str("type_id", "typeId").unwrap_or("note").to_string(),
        section_type: target_section.to_string(),
        name: new_name,
        icon_name: get_str("icon_name", "iconName")
            .unwrap_or("document")
            .to_string(),
        parent_id: get_str("parent_id", "parentId").map(String::from),
        children_ids: get_array("children_ids", "childrenIds")
            .map(|a: &Vec<serde_json::Value>| {
                a.iter()
                    .filter_map(|v: &serde_json::Value| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        properties: record_data["properties"].clone(),
        property_labels: if record_data["propertyLabels"].is_null() {
            if record_data["property_labels"].is_null() {
                None
            } else {
                Some(record_data["property_labels"].clone())
            }
        } else {
            Some(record_data["propertyLabels"].clone())
        },
        sensitivity_level: get_str("sensitivity_level", "sensitivityLevel")
            .unwrap_or("internal")
            .to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: record_data["tags"]
            .as_array()
            .map(|a: &Vec<serde_json::Value>| {
                a.iter()
                    .filter_map(|v: &serde_json::Value| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        template_id: get_str("template_id", "templateId").map(String::from),
        template_type: get_str("template_type", "templateType").map(String::from),
        template_hash: get_str("template_hash", "templateHash").map(String::from),
        ignored_template_hash: get_str("ignored_template_hash", "ignoredTemplateHash")
            .map(String::from),
        created_at: get_str("created_at", "createdAt")
            .unwrap_or(&now)
            .to_string(),
        updated_at: now.clone(),
        version: record_data["version"].as_u64().unwrap_or(1) as u32,
    };

    vault.save_object(&record)?;
    if new_id != trash.original_id {
        let _ = vault.copy_snapshots(&trash.original_id, &new_id);
    }

    Ok((record, exists))
}

/// Build and persist a page stub so that objects whose original custom page was permanently
/// deleted can still be restored into a page with the same UUID.
fn rebuild_page_stub(
    vault: &solosoul_vault::VaultStore,
    page_id: &str,
    account_id: &str,
    page_name: &str,
    icon_name: &str,
) -> Result<solosoul_vault::ObjectRecord, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let record = solosoul_vault::ObjectRecord {
        id: page_id.to_string(),
        account_id: account_id.to_string(),
        type_id: "page".to_string(),
        section_type: page_id.to_string(),
        name: page_name.to_string(),
        icon_name: icon_name.to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({}),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        ignored_template_hash: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        contract_type_id: None,
    };
    vault.save_object(&record)?;
    Ok(record)
}

/// Restore an object from trash. Handles conflict: if an object with the same ID
/// already exists, restore as a new copy with name appended " (restored)".
/// When the object's original custom page is missing, the page is either restored
/// from trash (cascade) or rebuilt as a stub (same UUID).
#[tauri::command]
pub async fn object_restore(
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    let trash = vault
        .get_trash_item(&trash_id)?
        .ok_or("Trash item not found")?;

    let fallback_lang = get_ui_language(&svc);
    let lang = lang.as_deref().unwrap_or(&fallback_lang);

    // Restore a page-type item: restore the page itself, then cascade child objects.
    if trash.item_type == "page" {
        let (page_record, _) = restore_single_object(vault, &trash, lang)?;
        let page_id = page_record.id.clone();
        let page_name = page_record.name.clone();
        vault.delete_trash_item(&trash_id)?;
        let _ = vault.log_structured(
            "page_restore",
            "page",
            Some(&page_id),
            Some(&page_name),
            "user",
            Some("count=0"),
        );

        let mut cascaded_count = 0u32;
        let children = find_child_objects_in_trash(vault, &page_id)?;
        for child_trash in &children {
            if child_trash.id == trash_id {
                continue;
            }
            if let Ok((_, _)) = restore_single_object(vault, child_trash, lang) {
                vault.delete_trash_item(&child_trash.id)?;
                let _ = vault.log_structured(
                    "object_restore",
                    "object",
                    Some(&child_trash.original_id),
                    Some(&child_trash.name_snapshot),
                    "user",
                    Some(&format!("cascaded_from_page={}", page_id)),
                );
                cascaded_count += 1;
            }
        }

        return Ok(RestoreOutcome {
            restored_id: page_id,
            name: page_name,
            cascaded_page_name: None,
            cascaded_count,
            rebuilt_page_name: None,
        });
    }

    // Object-type item: resolve the target section/page.
    let record_data: serde_json::Value =
        serde_json::from_slice(&trash.data).map_err(|e| format!("Invalid trash data: {}", e))?;
    let get_str = |snake: &str, camel: &str| {
        record_data[camel]
            .as_str()
            .or_else(|| record_data[snake].as_str())
    };
    let account_id = get_str("account_id", "accountId")
        .unwrap_or("imported")
        .to_string();
    let target_section = trash
        .original_section_type
        .as_deref()
        .or_else(|| record_data["sectionType"].as_str())
        .or_else(|| record_data["section_type"].as_str())
        .unwrap_or("identity")
        .to_string();

    let mut cascaded_page_name: Option<String> = None;
    let mut rebuilt_page_name: Option<String> = None;

    // If the object belongs to a custom page (UUID section), ensure the page exists.
    if !is_built_in_section(&target_section) && uuid::Uuid::parse_str(&target_section).is_ok() {
        let page_exists = vault
            .load_object(&target_section)
            .ok()
            .flatten()
            .map(|o| !o.is_deleted)
            .unwrap_or(false);

        if !page_exists {
            if let Ok(Some(page_trash)) = find_page_in_trash(vault, &target_section) {
                // Cascade-restore the page (and its other children will be handled by the
                // page-type branch above if the user restores the page directly; here we
                // restore only the page object itself so this object has a valid home).
                let (page_record, _) = restore_single_object(vault, &page_trash, lang)?;
                vault.delete_trash_item(&page_trash.id)?;
                cascaded_page_name = Some(page_record.name.clone());
                let _ = vault.log_structured(
                    "page_restore",
                    "page",
                    Some(&page_record.id),
                    Some(&page_record.name),
                    "user",
                    Some("cascaded_from_object_restore"),
                );
            } else {
                // Page was permanently deleted: rebuild a stub with the original UUID.
                let raw_name = record_data["parentPageName"].as_str().unwrap_or("");
                let page_name = if raw_name.is_empty() {
                    recovered_page_name(lang).to_string()
                } else {
                    raw_name.to_string()
                };
                let page_icon = record_data["parentPageIcon"].as_str().unwrap_or("folder");
                let stub =
                    rebuild_page_stub(vault, &target_section, &account_id, &page_name, page_icon)?;
                rebuilt_page_name = Some(stub.name.clone());
                let _ = vault.log_structured(
                    "page_create",
                    "page",
                    Some(&stub.id),
                    Some(&stub.name),
                    "user",
                    Some("rebuilt_stub_from_object_restore"),
                );
            }
        }
    }

    let (record, was_conflict) = restore_single_object(vault, &trash, lang)?;
    vault.delete_trash_item(&trash_id)?;
    let _ = vault.log_structured(
        "object_restore",
        "object",
        Some(&trash.original_id),
        Some(&trash.name_snapshot),
        "user",
        Some(&format!(
            "section={} was_conflict={}",
            target_section, was_conflict
        )),
    );

    Ok(RestoreOutcome {
        restored_id: record.id,
        name: record.name,
        cascaded_page_name,
        cascaded_count: 0,
        rebuilt_page_name,
    })
}

#[tauri::command]
pub async fn object_purge(state: State<'_, AppState>, object_id: String) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    let (obj_name, obj_section) = vault
        .load_object(&object_id)
        .ok()
        .flatten()
        .map(|r| (r.name, r.section_type))
        .unwrap_or_default();
    vault.delete_object(&object_id, false)?;
    vault.delete_trash_item(&object_id).ok();
    let _ = vault.log_structured(
        "object_purge",
        "object",
        Some(&object_id),
        Some(&obj_name),
        "user",
        Some(&format!("section={}", obj_section)),
    );
    Ok(())
}

#[tauri::command]
pub async fn trash_restore(
    state: State<'_, AppState>,
    trash_id: String,
    lang: Option<String>,
) -> Result<RestoreOutcome, String> {
    object_restore(state, trash_id, lang).await
}

/// Permanently delete a trash item (by trash_id → looks up original_id).
#[tauri::command]
pub async fn trash_permanent_delete(
    state: State<'_, AppState>,
    trash_id: String,
) -> Result<(), String> {
    let vault = vault_handle(&state)?;

    if let Ok(Some(trash)) = vault.get_trash_item(&trash_id) {
        if trash.item_type != "template" {
            vault.delete_object(&trash.original_id, false)?;
        }
        let _ = vault.log_structured(
            "trash_permanent_delete",
            "trash_item",
            Some(&trash_id),
            Some(&trash.name_snapshot),
            "user",
            Some(&format!("original_id={}", trash.original_id)),
        );
        vault.delete_trash_item(&trash_id).ok();
        return Ok(());
    }
    vault.delete_trash_item(&trash_id).ok();
    let _ = vault.log_structured(
        "trash_permanent_delete",
        "trash_item",
        Some(&trash_id),
        None,
        "user",
        None,
    );
    Ok(())
}

/// Delete a page (section_type) and all its objects into trash.
/// If `page_object_id` is provided, the custom page object is also deleted.
#[tauri::command]
pub async fn page_delete(
    state: State<'_, AppState>,
    account_id: String,
    section_type: String,
    page_object_id: Option<String>,
) -> Result<usize, String> {
    let vault = vault_handle(&state)?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let period = load_trash_retention(&vault, &account_id);
    let retention_ms = retention_ms(&period);
    let mut count = 0usize;

    let mut page_name = String::new();

    // Delete the custom page object itself if provided
    if let Some(pid) = &page_object_id {
        if let Ok(Some(rec)) = vault.load_object(pid) {
            page_name = rec.name.clone();
            let trash = solosoul_vault::TrashItem {
                id: format!("trash_{}", uuid::Uuid::new_v4()),
                item_type: "page".to_string(),
                original_id: rec.id.clone(),
                original_parent_id: None,
                original_section_type: Some(rec.section_type.clone()),
                original_sort_order: None,
                data: serde_json::to_vec(&serde_json::json!({
                    "id": rec.id,
                    "accountId": rec.account_id,
                    "typeId": rec.type_id,
                    "sectionType": rec.section_type,
                    "name": rec.name,
                    "iconName": rec.icon_name,
                    "parentId": rec.parent_id,
                    "childrenIds": rec.children_ids,
                    "properties": rec.properties,
                    "propertyLabels": rec.property_labels,
                    "sensitivityLevel": rec.sensitivity_level,
                    "tags": rec.tags_json,
                    "createdAt": rec.created_at,
                    "updatedAt": rec.updated_at,
                    "version": rec.version,
                    "templateId": rec.template_id,
                    "templateType": rec.template_type,
                    "contractTypeId": rec.contract_type_id,
                    "templateHash": rec.template_hash,
                }))
                .unwrap_or_default(),
                deleted_at: now_ms,
                expires_at: Some(now_ms + retention_ms),
                deleted_by: "user".to_string(),
                name_snapshot: rec.name.clone(),
                icon_snapshot: Some(rec.icon_name),
            };
            let _ = vault.save_trash_item(&trash);
            vault.delete_object(pid, true)?;
            count += 1;
        }
    }

    // Delete all objects in this section_type
    let objects = vault
        .list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| format!("list: {}", e))?;
    for obj in &objects {
        if obj.section_type == section_type || obj.collection_type == section_type {
            if page_name.is_empty() {
                page_name = section_type.clone();
            }
            if let Ok(Some(rec)) = vault.load_object(&obj.id) {
                let full_record = serde_json::json!({
                    "id": rec.id,
                    "accountId": rec.account_id,
                    "typeId": rec.type_id,
                    "sectionType": rec.section_type,
                    "name": rec.name,
                    "iconName": rec.icon_name,
                    "parentId": rec.parent_id,
                    "childrenIds": rec.children_ids,
                    "properties": rec.properties,
                    "propertyLabels": rec.property_labels,
                    "sensitivityLevel": rec.sensitivity_level,
                    "tags": rec.tags_json,
                    "createdAt": rec.created_at,
                    "updatedAt": rec.updated_at,
                    "version": rec.version,
                    "templateId": rec.template_id,
                    "templateType": rec.template_type,
                    "contractTypeId": rec.contract_type_id,
                    "templateHash": rec.template_hash,
                    "parentPageName": page_name,
                    "parentPageIcon": rec.icon_name,
                });
                let trash = solosoul_vault::TrashItem {
                    id: format!("trash_{}", uuid::Uuid::new_v4()),
                    item_type: "object".to_string(),
                    original_id: rec.id.clone(),
                    original_parent_id: None,
                    original_section_type: Some(rec.section_type.clone()),
                    original_sort_order: None,
                    data: serde_json::to_vec(&full_record).unwrap_or_default(),
                    deleted_at: now_ms,
                    expires_at: Some(now_ms + retention_ms),
                    deleted_by: "user".to_string(),
                    name_snapshot: rec.name.clone(),
                    icon_snapshot: Some(rec.icon_name.clone()),
                };
                let _ = vault.save_trash_item(&trash);
                vault.delete_object(&obj.id, true)?;
                count += 1;
            }
        }
    }

    let _ = vault.log_structured(
        "page_delete",
        "page",
        Some(&section_type),
        if page_name.is_empty() {
            None
        } else {
            Some(&page_name)
        },
        "user",
        Some(&format!("count={}", count)),
    );
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{VaultConfig, VaultStore};

    fn setup_vault() -> (VaultStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    fn make_trash(
        id: &str,
        item_type: &str,
        original_id: &str,
        section_type: Option<&str>,
    ) -> solosoul_vault::TrashItem {
        solosoul_vault::TrashItem {
            id: id.to_string(),
            item_type: item_type.to_string(),
            original_id: original_id.to_string(),
            original_parent_id: None,
            original_section_type: section_type.map(|s| s.to_string()),
            original_sort_order: None,
            data: serde_json::to_vec(&serde_json::json!({"name": id})).unwrap_or_default(),
            deleted_at: 1,
            expires_at: None,
            deleted_by: "user".to_string(),
            name_snapshot: id.to_string(),
            icon_snapshot: None,
        }
    }

    #[test]
    fn test_rebuild_page_stub_persists_with_same_uuid() {
        let (vault, _dir) = setup_vault();
        let page_id = uuid::Uuid::new_v4().to_string();
        let stub =
            rebuild_page_stub(&vault, &page_id, "acc-1", "Recovered Page", "folder").unwrap();
        assert_eq!(stub.id, page_id);
        assert_eq!(stub.section_type, page_id);
        assert_eq!(stub.type_id, "page");
        assert_eq!(stub.name, "Recovered Page");

        let loaded = vault.load_object(&page_id).unwrap().unwrap();
        assert_eq!(loaded.name, "Recovered Page");
        assert_eq!(loaded.icon_name, "folder");
        assert!(!loaded.is_deleted);
    }

    #[test]
    fn test_find_page_in_trash_by_original_id() {
        let (vault, _dir) = setup_vault();
        let page_id = uuid::Uuid::new_v4().to_string();
        let page_trash = make_trash("trash-page", "page", &page_id, Some(&page_id));
        vault.save_trash_item(&page_trash).unwrap();

        let found = find_page_in_trash(&vault, &page_id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().original_id, page_id);

        let not_found = find_page_in_trash(&vault, "non-existent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_child_objects_in_trash_matches_section_type() {
        let (vault, _dir) = setup_vault();
        let page_id = uuid::Uuid::new_v4().to_string();
        let child1 = make_trash("trash-child-1", "object", "obj-1", Some(&page_id));
        let child2 = make_trash("trash-child-2", "object", "obj-2", Some(&page_id));
        let other = make_trash("trash-other", "object", "obj-3", Some("other-section"));
        vault.save_trash_item(&child1).unwrap();
        vault.save_trash_item(&child2).unwrap();
        vault.save_trash_item(&other).unwrap();

        let children = find_child_objects_in_trash(&vault, &page_id).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|c| c.original_id == "obj-1"));
        assert!(children.iter().any(|c| c.original_id == "obj-2"));
    }

    #[test]
    fn test_recovered_page_name_localized() {
        assert_eq!(recovered_page_name("zh-CN"), "已恢复的页面");
        assert_eq!(recovered_page_name("en-US"), "Recovered Page");
        assert_eq!(recovered_page_name("ja-JP"), "Recovered Page");
    }

    #[test]
    fn test_restored_suffix_localized() {
        assert_eq!(restored_suffix("zh-CN"), "（已恢复）");
        assert_eq!(restored_suffix("en-US"), " (restored)");
    }
}
