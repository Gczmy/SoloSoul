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
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

use super::attachment::AttachmentMeta;

fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

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
    pub selected_tags: Vec<String>,        // P1: tag filter (intersection with selectedObjectIds)
    pub include_attachments: bool,         // P1: include attachment files
    pub selected_attachment_ids: Vec<String>, // P1: fine-grained attachment selection (empty = all)
    pub include_preferences: bool,         // P2: include user preferences
    pub include_behavioral: bool,          // future: include behavioral data
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
    pub attachment_selected_count: usize,
    pub estimated_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
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
pub struct AttachmentImportInfo {
    pub object_id: String,
    pub file_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptedImportPreview {
    pub objects: Vec<ObjectSummary>,
    pub conflicts: Vec<ConflictInfo>,
    pub has_preferences: bool,
    pub has_audit_log: bool,
    pub attachments: Vec<AttachmentImportInfo>,
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
                "uncategorized".to_string()
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

    // Estimate attachments (respect selected_attachment_ids filter)
    let mut attachment_count = 0usize;
    let mut attachment_selected_count = 0usize;
    if scope.include_attachments {
        let selected: std::collections::HashSet<String> = scope.selected_attachment_ids.iter().cloned().collect();
        let has_selection = !selected.is_empty();
        for rec in &records {
            let atts = load_attachments(&rec.properties);
            for att in &atts {
                if att.deleted_at.is_some() {
                    continue;
                }
                attachment_count += 1;
                if !has_selection || selected.contains(&att.id) {
                    attachment_selected_count += 1;
                    estimated_bytes += att.size_bytes;
                }
            }
        }
    }

    // Estimate preferences payload
    if scope.include_preferences {
        estimated_bytes += 4096; // rough guess
    }

    // Estimate behavioral data (audit log)
    if scope.include_behavioral {
        if let Ok(logs) = vault.list_audit_log(100000) {
            let log_json = serde_json::to_vec(&logs).unwrap_or_default();
            estimated_bytes += log_json.len() as u64;
        }
    }

    Ok(ExportEstimate {
        object_count: count,
        attachment_count,
        attachment_selected_count,
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

    // ── Verify export password is NOT the master password ──────
    match svc.verify_password(&account_id, &req.password) {
        Ok(true) => {
            return Err("Export password must be different from your master password".to_string());
        }
        Ok(false) => { /* export password is different from master password — OK */ }
        Err(e) => {
            return Err(format!("Failed to verify master password: {}", e));
        }
    }

    // ── Verify password hint does not contain the password ─────
    if let Some(ref hint) = req.password_hint {
        if !hint.is_empty() && req.password.len() >= 3 {
            let pw_lower = req.password.to_lowercase();
            let hint_lower = hint.to_lowercase();
            for window in pw_lower.as_bytes().windows(3) {
                let substr = std::str::from_utf8(window).unwrap_or("");
                if !substr.is_empty() && hint_lower.contains(substr) {
                    return Err("Password hint must not contain parts of the password".to_string());
                }
            }
        }
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

    // ── P1: Attachments ────────────────────────────────────────
    const MAX_ATTACHMENT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB
    const MAX_EXPORT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB
    const STREAMING_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MB

    let mut has_attachments = false;
    let selected_attachment_ids: std::collections::HashSet<String> = req.scope.selected_attachment_ids.iter().cloned().collect();
    let mut attachment_entries: Vec<(String, String, String, std::path::PathBuf)> = Vec::new();
    let mut total_attachment_bytes: u64 = 0;

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
                // Fine-grained selection
                if !selected_attachment_ids.is_empty() && !selected_attachment_ids.contains(&att.id) {
                    continue;
                }
                // Single attachment size limit
                if att.size_bytes > MAX_ATTACHMENT_BYTES {
                    return Err(format!("Attachment '{}' exceeds 100 MB limit", att.file_name));
                }

                let src = att
                    .vault_path
                    .as_ref()
                    .or(att.src_path.as_ref())
                    .map(|p| std::path::Path::new(p).to_path_buf())
                    .filter(|p| p.exists())
                    .or_else(|| {
                        let fallback = base_dir.join(&att.id).join(&att.file_name);
                        if fallback.exists() { Some(fallback) } else { None }
                    });

                if let Some(src) = src {
                    total_attachment_bytes += att.size_bytes;
                    attachment_entries.push((rec.id.clone(), att.id.clone(), att.file_name.clone(), src));
                }
            }
        }
    }

    // Total export size limit (payload + attachments + ~28 bytes overhead per attachment for nonce/chunk_count)
    let payload_estimate = payload_bytes.len() as u64;
    let total_export_estimate = payload_estimate + total_attachment_bytes + (attachment_entries.len() as u64 * 28);
    if total_export_estimate > MAX_EXPORT_TOTAL_BYTES {
        return Err("Total export size exceeds 1 GB limit".to_string());
    }

    // Derive attachment key via HKDF
    let att_key = if !attachment_entries.is_empty() {
        Some(solosoul_crypto::hkdf_ext::derive_hkdf_key(
            &key,
            &salt,
            b"solosoul:attachments:v1",
        ).map_err(|e| format!("derive att key: {}", e))?)
    } else {
        None
    };

    // Encrypt and write attachments
    if let Some(ref ak) = att_key {
        for (obj_id, att_id, _file_name, src_path) in &attachment_entries {
            let file_size = std::fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
            let zip_name = format!("attachments/{}/{}.enc", obj_id, att_id);
            let mut buf = Vec::new();
            let mut f = File::open(src_path).map_err(|e| format!("open attachment: {}", e))?;
            f.read_to_end(&mut buf).map_err(|e| format!("read attachment: {}", e))?;

            let enc = if file_size <= STREAMING_THRESHOLD {
                solosoul_crypto::cipher::encrypt_to_bytes(ak, &buf, None)
                    .map_err(|e| format!("encrypt attachment: {}", e))?
            } else {
                solosoul_crypto::cipher::encrypt_chunked_to_bytes(ak, &buf)
                    .map_err(|e| format!("encrypt attachment chunked: {}", e))?
            };

            zip.start_file(&zip_name, options).map_err(|e| e.to_string())?;
            zip.write_all(&enc).map_err(|e| e.to_string())?;
            has_attachments = true;
        }
    }

    // ── P2: Preferences ────────────────────────────────────────
    let mut extra_files: Vec<String> = Vec::new();
    let mut preferences_encrypted = false;
    if req.scope.include_preferences {
        if let Ok(Some(profile)) = vault.load_profile(&account_id) {
            let prefs_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
                &key,
                &salt,
                b"solosoul:preferences:v1",
            )
            .map_err(|e| format!("derive prefs key: {}", e))?;
            let prefs_enc = solosoul_crypto::cipher::encrypt_to_bytes(&prefs_key, &profile.data, None)
                .map_err(|e| format!("encrypt prefs: {}", e))?;
            zip.start_file("preferences.enc", options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&prefs_enc).map_err(|e| e.to_string())?;
            extra_files.push("preferences.enc".to_string());
            preferences_encrypted = true;
        }
    }

    // ── P2: Behavioral data (audit log) ────────────────────────
    let mut behavioral_encrypted = false;
    if req.scope.include_behavioral {
        if let Ok(logs) = vault.list_audit_log(100000) {
            let logs_json = serde_json::to_vec(&logs).unwrap_or_default();
            if !logs_json.is_empty() {
                let behav_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    &key,
                    &salt,
                    b"solosoul:behavioral:v1",
                )
                .map_err(|e| format!("derive behavioral key: {}", e))?;
                let behav_enc = solosoul_crypto::cipher::encrypt_to_bytes(&behav_key, &logs_json, None)
                    .map_err(|e| format!("encrypt behavioral: {}", e))?;
                zip.start_file("behavioral.enc", options)
                    .map_err(|e| e.to_string())?;
                zip.write_all(&behav_enc).map_err(|e| e.to_string())?;
                extra_files.push("behavioral.enc".to_string());
                behavioral_encrypted = true;
            }
        }
    }

    // manifest.json (plaintext)
    let manifest = serde_json::json!({
        "version": "2.0",
        "export_scope": "partial",
        "selected_pages": req.scope.selected_page_ids,
        "selected_objects": req.scope.selected_object_ids,
        "selected_tags": req.scope.selected_tags,
        "object_count": records.len(),
        "export_time": chrono::Utc::now().to_rfc3339(),
        "export_platform": std::env::consts::OS,
        "export_app_version": env!("CARGO_PKG_VERSION"),
        "has_attachments": has_attachments,
        "has_preferences": preferences_encrypted,
        "has_behavioral": behavioral_encrypted,
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

// ── Attachment info for export UI ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub id: String,
    pub file_name: String,
    pub size_bytes: u64,
}

#[tauri::command]
pub async fn export_get_attachments(
    state: State<'_, AppState>,
    _account_id: String,
    object_id: String,
) -> Result<Vec<AttachmentInfo>, String> {
    let svc = state.vault_service.read().await;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    let obj = vault.load_object(&object_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Object {} not found", object_id))?;

    let atts = load_attachments(&obj.properties);
    let result: Vec<AttachmentInfo> = atts
        .into_iter()
        .filter(|a| a.deleted_at.is_none())
        .map(|a| AttachmentInfo {
            id: a.id,
            file_name: a.file_name,
            size_bytes: a.size_bytes,
        })
        .collect();

    Ok(result)
}

// ── Import commands ────────────────────────────────────────────

#[tauri::command]
pub async fn import_parse_package(file_path: String) -> Result<ImportPreview, String> {
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

        Ok(ImportPreview {
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

    // Build attachment preview list from payload
    let mut attachments = Vec::new();
    if manifest.has_attachments {
        for obj in &objects {
            let atts = load_attachments(&obj.properties);
            for att in &atts {
                if att.deleted_at.is_some() {
                    continue;
                }
                attachments.push(AttachmentImportInfo {
                    object_id: obj.id.clone(),
                    file_name: att.file_name.clone(),
                    size_bytes: att.size_bytes,
                });
            }
        }
    }

    Ok(DecryptedImportPreview {
        objects,
        conflicts,
        has_preferences,
        has_audit_log: false,
        attachments,
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
    let package_ids = build_package_ids(&payload);
    let mut imported = 0usize;
    let mut imported_object_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
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

        // Resolve cross-scope RelationProperty references
        let mut properties = obj_val["properties"].clone();
        resolve_cross_scope_references(&mut properties, &package_ids);

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
            properties,
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
        imported_object_ids.insert(id.to_string());
    }

    // ── P1: Import attachments (encrypted) ─────────────────────
    if manifest.has_attachments {
        // Derive attachment key via HKDF
        let att_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
            &key,
            &salt,
            b"solosoul:attachments:v1",
        ).map_err(|e| format!("derive att key: {}", e))?;

        // Build old att_id -> meta map from payload objects
        let mut att_meta_map: std::collections::HashMap<(String, String), AttachmentMeta> = std::collections::HashMap::new();
        for obj_val in objects {
            let obj_id = obj_val["id"].as_str().unwrap_or("");
            if obj_id.is_empty() { continue; }
            let empty_props = serde_json::Map::new();
            let props = obj_val["properties"].as_object().unwrap_or(&empty_props);
            let atts = load_attachments(&serde_json::Value::Object(props.clone()));
            for att in &atts {
                att_meta_map.insert((obj_id.to_string(), att.id.clone()), att.clone());
            }
        }

        // Open ZIP and iterate attachments
        let zip_file = File::open(&file_path).map_err(|e| format!("open zip: {}", e))?;
        let mut archive = ZipArchive::new(zip_file).map_err(|e| format!("invalid zip: {}", e))?;
        let att_prefix = "attachments/";
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            if !name.starts_with(att_prefix) || name.ends_with('/') {
                continue;
            }
            let rel = &name[att_prefix.len()..]; // "objId/attId.enc"
            let parts: Vec<&str> = rel.split('/').collect();
            if parts.len() != 2 {
                continue;
            }
            let obj_id = parts[0];
            let enc_name = parts[1];
            let old_att_id = match enc_name.strip_suffix(".enc") {
                Some(id) => id,
                None => continue,
            };

            // Skip if object was not imported (e.g. SkipExisting)
            if !imported_object_ids.contains(obj_id) {
                continue;
            }

            let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
                Some(m) => m,
                None => continue,
            };

            let mut enc_data = Vec::new();
            f.read_to_end(&mut enc_data).map_err(|e| e.to_string())?;

            // Decrypt (try chunked first for larger files)
            const STREAMING_THRESHOLD: usize = 10 * 1024 * 1024; // 10 MB
            let att_data = if enc_data.len() > (STREAMING_THRESHOLD + 28) {
                solosoul_crypto::cipher::decrypt_chunked_from_bytes(&att_key, &enc_data)
                    .map_err(|e| format!("decrypt attachment chunked: {}", e))?
            } else {
                match solosoul_crypto::cipher::decrypt_from_bytes(&att_key, &enc_data, None) {
                    Ok(d) => d,
                    Err(_) => solosoul_crypto::cipher::decrypt_chunked_from_bytes(&att_key, &enc_data)
                        .map_err(|e| format!("decrypt attachment fallback: {}", e))?,
                }
            };

            // Save to vault storage
            let new_att_id = generate_id();
            let dest = svc.base_path().join("attachments").join(obj_id).join(&new_att_id);
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            let file_path_dest = dest.join(&old_meta.file_name);
            std::fs::write(&file_path_dest, &att_data).map_err(|e| e.to_string())?;

            // Update object's __attachments property
            let mut obj = vault.load_object(obj_id).map_err(|e| format!("get object: {}", e))?
                .ok_or_else(|| format!("object {} not found", obj_id))?;
            let mut atts = load_attachments(&obj.properties);
            atts.push(AttachmentMeta {
                id: new_att_id,
                object_id: obj_id.to_string(),
                file_name: old_meta.file_name.clone(),
                mime_type: old_meta.mime_type.clone(),
                size_bytes: att_data.len() as u64,
                created_at: now.clone(),
                deleted_at: None,
                src_path: Some(file_path_dest.to_string_lossy().to_string()),
                vault_path: Some(file_path_dest.to_string_lossy().to_string()),
            });
            let att_json = serde_json::to_value(&atts).map_err(|e| e.to_string())?;
            match &mut obj.properties {
                serde_json::Value::Object(map) => {
                    map.insert("__attachments".to_string(), att_json);
                }
                _ => {
                    let mut map = serde_json::Map::new();
                    map.insert("__attachments".to_string(), att_json);
                    obj.properties = serde_json::Value::Object(map);
                }
            }
            vault.save_object(&obj).map_err(|e| e.to_string())?;
        }
    }

    // ── P2: Import preferences if present ──────────────────────
    if manifest.extra_files.contains(&"preferences.enc".to_string()) {
        let prefs_salt = hex::decode(&manifest.salt_hex).unwrap_or_default();
        let prefs_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
            &key,
            &prefs_salt,
            b"solosoul:preferences:v1",
        )
        .map_err(|e| format!("derive prefs key: {}", e))?;
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

// ── Cross-scope reference resolution ─────────────────────────

/// Build a set of all object IDs present in the imported package.
fn build_package_ids(payload: &serde_json::Value) -> std::collections::HashSet<String> {
    payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Recursively scan a JSON value for RelationProperty references.
/// If a relation targets an object not in `package_ids`, downgrade it to a text remark.
fn resolve_value_references(
    value: &mut serde_json::Value,
    package_ids: &std::collections::HashSet<String>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            // Check if this object looks like a RelationProperty
            let is_relation = obj
                .get("type")
                .or_else(|| obj.get("__type"))
                .or_else(|| obj.get("kind"))
                .and_then(|v| v.as_str())
                == Some("relation");
            if is_relation {
                let target_id = obj
                    .get("targetId")
                    .or_else(|| obj.get("id"))
                    .or_else(|| obj.get("objectId"))
                    .and_then(|v| v.as_str());
                if let Some(tid) = target_id {
                    if !package_ids.contains(tid) {
                        // Downgrade to text remark as per §4 cross-scope reference handling
                        *value = serde_json::Value::String(format!("[引用对象未导出: {}]", tid));
                        return;
                    }
                }
            }
            // Recurse into nested objects
            for (_k, v) in obj.iter_mut() {
                resolve_value_references(v, package_ids);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_value_references(item, package_ids);
            }
        }
        _ => {}
    }
}

/// Scan object properties and downgrade any cross-scope relation references.
fn resolve_cross_scope_references(
    properties: &mut serde_json::Value,
    package_ids: &std::collections::HashSet<String>,
) {
    if let Some(map) = properties.as_object_mut() {
        for (_key, value) in map.iter_mut() {
            resolve_value_references(value, package_ids);
        }
    }
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
