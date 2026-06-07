//! Export/Import commands — P0+P1+P2: Object-level import/export with password-derived encryption
//!
//! Architecture notes (see docs §14 / §17):
//! - Export scope: page (section_type) → object. No field-level.
//! - Payload: single payload.enc encrypted with AES-256-GCM via Argon2id-derived key.
//! - Salt stored in manifest.json (hex), hint stored plaintext.
//! - P2 extras: tag filtering, preferences export, attachment export, import strategy selection.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use solosoul_vault::ObjectSummary;
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{Read, Write};
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

// ── Public types (↔ frontend) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageGroup {
    pub section_type: String,
    pub page_name: String,
    pub object_count: usize,
    pub objects: Vec<ObjectSummary>,
}

/// Scope selection transmitted from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScope {
    pub selected_page_ids: Vec<String>,    // section_types to export fully
    pub selected_object_ids: Vec<String>,  // specific object IDs
    pub selected_tags: Vec<String>,        // P2: tag filter (intersection with selectedObjectIds)
    pub include_attachments: bool,         // P2: include attachment files
    pub include_preferences: bool,         // P2: include user preferences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub password: String,
    pub password_hint: Option<String>,
    pub save_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEstimate {
    pub object_count: usize,
    pub attachment_count: usize,
    pub estimated_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewResponse {
    pub file_path: String,
    pub version: String,
    pub object_count: usize,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
    pub export_time: Option<String>,
    pub password_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptedImportPreview {
    pub objects: Vec<ObjectSummary>,
    pub conflicts: Vec<ConflictInfo>,
    pub has_preferences: bool,
    pub has_audit_log: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub object_id: String,
    pub name: String,
}

/// P2: import strategy for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStrategy {
    /// Skip conflicting objects (keep existing)
    SkipExisting,
    /// Overwrite all (imported data replaces existing)
    Overwrite,
    /// Merge: overwrite conflicts, keep non-conflicting originals
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSelection {
    pub object_id: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedImportRequest {
    pub selections: Vec<ImportSelection>,
    pub strategy: ImportStrategy,
    pub source_path: String,
    pub password: String,
}

// ── Helpers ────────────────────────────────────────────────────

fn derive_export_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    use solosoul_crypto::kdf::{derive_key, KdfConfig};
    let key_vec = derive_key(password, salt, &KdfConfig::balanced()).map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_vec);
    Ok(key)
}

/// Load attachment metadata from object properties.
fn load_attachments(props: &serde_json::Value) -> Vec<super::attachment::AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<super::attachment::AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

/// Collect all objects matching the given scope.
fn collect_scope_objects(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    scope: &ExportScope,
) -> Result<Vec<solosoul_vault::ObjectRecord>, String> {
    let all = vault.list_objects(account_id, None, None, None, false, false)?;
    let mut selected_ids: BTreeSet<String> = scope.selected_object_ids.iter().cloned().collect();

    // Add all IDs belonging to selected pages
    for summary in &all {
        if !scope.selected_page_ids.is_empty()
            && scope.selected_page_ids.contains(&summary.section_type)
        {
            selected_ids.insert(summary.id.clone());
        }
    }

    // Filter by tags (P2): if selected_tags is non-empty, keep only objects with ANY matching tag
    if !scope.selected_tags.is_empty() {
        selected_ids.retain(|id| {
            all.iter().any(|s| {
                s.id == *id
                    && s
                        .tags
                        .iter()
                        .any(|t| scope.selected_tags.contains(t))
            })
        });
    }

    let mut records = Vec::new();
    for id in &selected_ids {
        if let Ok(Some(rec)) = vault.load_object(id) {
            records.push(rec);
        }
    }
    Ok(records)
}

// ── Export commands ────────────────────────────────────────────

#[tauri::command]
pub async fn export_get_scope_tree(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<PageGroup>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let objects = vault
        .list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| format!("list_objects: {}", e))?;

    let mut groups: std::collections::BTreeMap<String, Vec<ObjectSummary>> =
        std::collections::BTreeMap::new();
    for obj in objects {
        let st = if obj.section_type.is_empty() {
            if obj.collection_type.is_empty() {
                "identity".to_string()
            } else {
                obj.collection_type.clone()
            }
        } else {
            obj.section_type.clone()
        };
        groups.entry(st).or_default().push(obj);
    }

    let page_names: std::collections::HashMap<&str, &str> = [
        ("identity", "Identity"),
        ("travel", "Travel"),
        ("financial", "Financial"),
        ("professional", "Professional"),
        ("page", "Pages"),
        ("note", "Notes"),
        ("document", "Documents"),
    ]
    .iter()
    .cloned()
    .collect();

    Ok(groups
        .into_iter()
        .map(|(st, objs)| {
            let display = page_names
                .get(st.as_str())
                .copied()
                .unwrap_or(&st)
                .to_string();
            let count = objs.len();
            PageGroup {
                section_type: st,
                page_name: display,
                object_count: count,
                objects: objs,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn export_estimate_size(
    state: State<'_, AppState>,
    account_id: String,
    scope: ExportScope,
) -> Result<ExportEstimate, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let records = collect_scope_objects(vault, &account_id, &scope)?;
    let count = records.len();
    let mut estimated_bytes: u64 = records
        .iter()
        .map(|r| {
            let props_len = serde_json::to_vec(&r.properties).unwrap_or_default().len() as u64;
            let name_len = r.name.len() as u64;
            props_len + name_len + 256
        })
        .sum();

    // Estimate attachments
    let mut attachment_count = 0usize;
    if scope.include_attachments {
        for rec in &records {
            let atts = load_attachments(&rec.properties);
            attachment_count += atts.len();
            estimated_bytes += atts.iter().map(|a| a.size_bytes).sum::<u64>();
        }
    }

    // Estimate preferences payload
    if scope.include_preferences {
        estimated_bytes += 4096; // rough guess
    }

    Ok(ExportEstimate {
        object_count: count,
        attachment_count,
        estimated_bytes,
    })
}

#[tauri::command]
pub async fn export_execute(
    state: State<'_, AppState>,
    account_id: String,
    req: ExportRequest,
) -> Result<String, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // ── Validate password ──────────────────────────────────────
    if req.password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    let has_letter = req.password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = req.password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err("Password must contain at least one letter and one digit".to_string());
    }

    // ── Collect objects ────────────────────────────────────────
    let records = collect_scope_objects(vault, &account_id, &req.scope)?;
    if records.is_empty() {
        return Err("No objects selected for export".to_string());
    }

    // ── Serialise payload ──────────────────────────────────────
    let payload = serde_json::json!({
        "objects": records.iter().map(|r| serde_json::json!({
            "id": r.id,
            "account_id": r.account_id,
            "type_id": r.type_id,
            "section_type": r.section_type,
            "name": r.name,
            "icon_name": r.icon_name,
            "parent_id": r.parent_id,
            "children_ids": r.children_ids,
            "properties": r.properties,
            "property_labels": r.property_labels,
            "sensitivity_level": r.sensitivity_level,
            "tags": r.tags_json,
            "created_at": r.created_at,
            "updated_at": r.updated_at,
            "version": r.version,
        })).collect::<Vec<_>>(),
    });
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| format!("serialize: {}", e))?;

    // ── Derive key & encrypt ──────────────────────────────────
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(&req.password, &salt)?;
    let enc_bytes = solosoul_crypto::cipher::encrypt_to_bytes(&key, &payload_bytes, None)
        .map_err(|e| format!("encrypt: {}", e))?;

    // ── Build ZIP ──────────────────────────────────────────────
    let save_path = if req.save_path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        home + &req.save_path[1..]
    } else {
        req.save_path.clone()
    };
    let zip_path = if save_path.ends_with(".solosoul") {
        save_path
    } else {
        format!("{}.solosoul", save_path)
    };
    if let Some(parent) = std::path::Path::new(&zip_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = File::create(&zip_path).map_err(|e| format!("Create ZIP: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // ── P2: Attachments ────────────────────────────────────────
    let mut has_attachments = false;
    if req.scope.include_attachments {
        for rec in &records {
            let atts = load_attachments(&rec.properties);
            if atts.is_empty() {
                continue;
            }
            let base_dir = svc.base_path().join("attachments").join(&rec.id);
            for att in &atts {
                if att.deleted_at.is_some() {
                    continue;
                }
                // Try vault_path first, then src_path
                let src = att
                    .vault_path
                    .as_ref()
                    .or(att.src_path.as_ref())
                    .map(|p| std::path::Path::new(p))
                    .filter(|p| p.exists());
                if let Some(src) = src {
                    let zip_name = format!("attachments/{}/{}", rec.id, att.file_name);
                    if let Ok(mut f) = File::open(src) {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_ok() {
                            let _ = zip.start_file(&zip_name, options);
                            let _ = zip.write_all(&buf);
                            has_attachments = true;
                        }
                    }
                } else {
                    // Fallback: look in vault structure by attachment id
                    let fallback = base_dir.join(&att.id).join(&att.file_name);
                    if fallback.exists() {
                        let zip_name = format!("attachments/{}/{}", rec.id, att.file_name);
                        if let Ok(mut f) = File::open(&fallback) {
                            let mut buf = Vec::new();
                            if f.read_to_end(&mut buf).is_ok() {
                                let _ = zip.start_file(&zip_name, options);
                                let _ = zip.write_all(&buf);
                                has_attachments = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // ── P2: Preferences ────────────────────────────────────────
    let mut extra_files: Vec<String> = Vec::new();
    let mut preferences_encrypted = false;
    if req.scope.include_preferences {
        if let Ok(Some(profile)) = vault.load_profile(&account_id) {
            let prefs_key = derive_export_key(&format!("{}_prefs_salt", req.password), &salt)?;
            let prefs_enc = solosoul_crypto::cipher::encrypt_to_bytes(&prefs_key, &profile.data, None)
                .map_err(|e| format!("encrypt prefs: {}", e))?;
            zip.start_file("preferences.enc", options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&prefs_enc).map_err(|e| e.to_string())?;
            extra_files.push("preferences.enc".to_string());
            preferences_encrypted = true;
        }
    }

    // manifest.json (plaintext)
    let manifest = serde_json::json!({
        "version": "2.0",
        "export_scope": if req.scope.selected_page_ids.is_empty() && req.scope.selected_object_ids.is_empty() { "none" } else { "partial" },
        "selected_pages": req.scope.selected_page_ids,
        "selected_objects": req.scope.selected_object_ids,
        "selected_tags": req.scope.selected_tags,
        "object_count": records.len(),
        "export_time": chrono::Utc::now().to_rfc3339(),
        "export_platform": std::env::consts::OS,
        "export_app_version": env!("CARGO_PKG_VERSION"),
        "has_attachments": has_attachments,
        "has_preferences": preferences_encrypted,
        "extra_files": extra_files,
        "password_hint": req.password_hint.unwrap_or_default(),
        "salt_hex": hex::encode(salt),
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    // payload.enc (encrypted)
    zip.start_file("payload.enc", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&enc_bytes).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| format!("ZIP finish: {}", e))?;

    let _ = vault.log_structured(
        "export_execute",
        "export",
        None,
        None,
        "user",
        Some(&format!("exported {} objects to {}", records.len(), zip_path)),
    );

    Ok(zip_path)
}

// ── Import commands ────────────────────────────────────────────

#[tauri::command]
pub async fn import_parse_package(file_path: String) -> Result<ImportPreviewResponse, String> {
    let fp = file_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let path = std::path::Path::new(&fp);
        if !path.exists() {
            return Err(format!("File not found: {}", fp));
        }
        let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|_| "Not a valid .solosoul file".to_string())?;

        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| "No manifest.json found".to_string())?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| format!("Read: {}", e))?;
        let s = String::from_utf8_lossy(&buf).to_string();
        let v: serde_json::Value = serde_json::from_str(&s)
            .map_err(|e| format!("Invalid manifest JSON: {}", e))?;

        let extra_files: Vec<String> = v
            .get("extra_files")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(ImportPreviewResponse {
            file_path: fp,
            version: v.get("version").and_then(|x| x.as_str()).unwrap_or("1.0").to_string(),
            object_count: v.get("object_count").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            has_attachments: v.get("has_attachments").and_then(|x| x.as_bool()).unwrap_or(false),
            extra_files,
            export_time: v.get("export_time").and_then(|x| x.as_str()).map(|x| x.to_string()),
            password_hint: v.get("password_hint").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(|x| x.to_string()),
        })
    }).await;

    match result {
        Ok(Ok(preview)) => Ok(preview),
        Ok(Err(e)) => Err(e),
        Err(join_err) => Err(format!("Blocking task failed: {}", join_err)),
    }
}

#[tauri::command]
pub async fn import_get_password_hint(file_path: String) -> Result<Option<String>, String> {
    let preview = import_parse_package(file_path).await?;
    Ok(preview.password_hint)
}

#[tauri::command]
pub async fn import_decrypt_preview(
    state: State<'_, AppState>,
    file_path: String,
    password: String,
) -> Result<DecryptedImportPreview, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let manifest = read_manifest(&file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    let key = derive_export_key(&password, &salt)?;
    let enc_bytes = read_file_from_zip(&file_path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_from_bytes(&key, &enc_bytes, None)
        .map_err(|_| "Decryption failed — wrong password or corrupted file".to_string())?;

    let payload: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| format!("Invalid payload: {}", e))?;

    let objects: Vec<ObjectSummary> = payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    Some(ObjectSummary {
                        id: o["id"].as_str()?.to_string(),
                        name: o["name"].as_str()?.to_string(),
                        collection_type: o["type_id"].as_str()?.to_string(),
                        section_type: o["section_type"].as_str().unwrap_or("").to_string(),
                        sensitivity_level: o["sensitivity_level"].as_str().unwrap_or("internal").to_string(),
                        created_at: o["created_at"].as_str().unwrap_or("").to_string(),
                        updated_at: o["updated_at"].as_str().unwrap_or("").to_string(),
                        is_deleted: false,
                        properties: o["properties"].clone(),
                        tags: o["tags"].as_array().map(|t| t.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut conflicts = Vec::new();
    for obj in &objects {
        if let Ok(Some(_)) = vault.load_object(&obj.id) {
            conflicts.push(ConflictInfo {
                object_id: obj.id.clone(),
                name: obj.name.clone(),
            });
        }
    }

    let has_preferences = manifest.extra_files.contains(&"preferences.enc".to_string());

    Ok(DecryptedImportPreview {
        objects,
        conflicts,
        has_preferences,
        has_audit_log: false,
    })
}

/// P0: Simple import (full import, skip conflicts)
#[tauri::command]
pub async fn import_execute(
    state: State<'_, AppState>,
    account_id: String,
    file_path: String,
    password: String,
) -> Result<usize, String> {
    import_execute_internal(state, account_id, file_path, password, ImportStrategy::SkipExisting, None).await
}

/// P2: Advanced import with object selection and strategy
#[tauri::command]
pub async fn import_execute_advanced(
    state: State<'_, AppState>,
    account_id: String,
    req: AdvancedImportRequest,
) -> Result<usize, String> {
    import_execute_internal(state, account_id, req.source_path, req.password, req.strategy, Some(req.selections)).await
}

async fn import_execute_internal(
    state: State<'_, AppState>,
    account_id: String,
    file_path: String,
    password: String,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
) -> Result<usize, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    if password.is_empty() {
        return Err("Password is required".to_string());
    }

    let manifest = read_manifest(&file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    let key = derive_export_key(&password, &salt)?;
    let enc_bytes = read_file_from_zip(&file_path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_from_bytes(&key, &enc_bytes, None)
        .map_err(|_| "Decryption failed — wrong password or corrupted file".to_string())?;

    let payload: serde_json::Value = serde_json::from_slice(&decrypted)
        .map_err(|e| format!("Invalid payload: {}", e))?;

    // Build selection set if provided
    let selected_ids: Option<BTreeSet<String>> = selections.map(|sels| {
        sels.into_iter()
            .filter(|s| s.selected)
            .map(|s| s.object_id)
            .collect()
    });

    let objects = payload["objects"].as_array().ok_or("No objects array in payload")?;
    let mut imported = 0usize;
    let now = chrono::Utc::now().to_rfc3339();

    for obj_val in objects {
        let id = obj_val["id"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }

        // Apply selection filter
        if let Some(ref sel_ids) = selected_ids {
            if !sel_ids.contains(id) {
                continue;
            }
        }

        // Check conflict & apply strategy
        let existing = vault.load_object(id).ok().flatten();
        match &strategy {
            ImportStrategy::SkipExisting => {
                if existing.is_some() {
                    continue;
                }
            }
            ImportStrategy::Overwrite => { /* always overwrite — fall through */ }
            ImportStrategy::Merge => { /* overwrite always — fall through */ }
        }

        let record = solosoul_vault::ObjectRecord {
            id: id.to_string(),
            account_id: account_id.clone(),
            type_id: obj_val["type_id"].as_str().unwrap_or("note").to_string(),
            section_type: obj_val["section_type"].as_str().unwrap_or("identity").to_string(),
            name: obj_val["name"].as_str().unwrap_or("Imported").to_string(),
            icon_name: obj_val["icon_name"].as_str().unwrap_or("document").to_string(),
            parent_id: obj_val["parent_id"].as_str().map(String::from),
            children_ids: obj_val["children_ids"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            properties: obj_val["properties"].clone(),
            property_labels: if obj_val["property_labels"].is_null() {
                None
            } else {
                Some(obj_val["property_labels"].clone())
            },
            sensitivity_level: obj_val["sensitivity_level"].as_str().unwrap_or("internal").to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: obj_val["tags"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            created_at: obj_val["created_at"].as_str().unwrap_or(&now).to_string(),
            updated_at: now.clone(),
            version: obj_val["version"].as_u64().unwrap_or(1) as u32,
        };

        vault.save_object(&record).map_err(|e| format!("save: {}", e))?;
        imported += 1;

        // P2: Import attachments if present
        if manifest.has_attachments {
            let atts = load_attachments(&record.properties);
            for att in &atts {
                let zip_name = format!("attachments/{}/{}", id, att.file_name);
                if let Ok(att_data) = read_file_from_zip(&file_path, &zip_name) {
                    // Write attachment to vault storage
                    let base_dir = svc.base_path().join("attachments").join(id);
                    let _ = std::fs::create_dir_all(&base_dir);
                    let dest = base_dir.join(&att.file_name);
                    let _ = std::fs::write(&dest, &att_data);
                }
            }
        }
    }

    // ── P2: Import preferences if present ──────────────────────
    if manifest.extra_files.contains(&"preferences.enc".to_string()) {
        let prefs_salt = hex::decode(&manifest.salt_hex).unwrap_or_default();
        let prefs_key = derive_export_key(&format!("{}_prefs_salt", password), &prefs_salt)?;
        if let Ok(prefs_enc) = read_file_from_zip(&file_path, "preferences.enc") {
            if let Ok(prefs_dec) = solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None) {
                let profile = solosoul_vault::Profile::new_with_id(&account_id, &account_id, prefs_dec.to_vec());
                let _ = vault.save_profile(&profile);
            }
        }
    }

    let _ = vault.log_structured(
        "import_execute",
        "import",
        None,
        None,
        "user",
        Some(&format!("imported {} objects from {} (strategy: {:?})", imported, file_path, strategy)),
    );

    Ok(imported)
}

// ── Internal helpers ──────────────────────────────────────────

struct ManifestData {
    salt_hex: String,
    has_attachments: bool,
    extra_files: Vec<String>,
}

fn read_manifest(file_path: &str) -> Result<ManifestData, String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Not a valid .solosoul file".to_string())?;

    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| "No manifest.json".to_string())?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| format!("Read manifest: {}", e))?;
    let s = String::from_utf8_lossy(&buf);
    let v: serde_json::Value = serde_json::from_str(&s).map_err(|e| format!("Invalid manifest: {}", e))?;

    let extra_files: Vec<String> = v
        .get("extra_files")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    Ok(ManifestData {
        salt_hex: v["salt_hex"].as_str().ok_or("Missing salt_hex")?.to_string(),
        has_attachments: v["has_attachments"].as_bool().unwrap_or(false),
        extra_files,
    })
}

fn read_file_from_zip(file_path: &str, name: &str) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid ZIP".to_string())?;
    let mut entry = archive.by_name(name).map_err(|_| format!("File not found: {}", name))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| format!("Read {}: {}", name, e))?;
    Ok(buf)
}
