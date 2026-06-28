use super::*;

// ── Export commands ────────────────────────────────────────────

#[tauri::command]
pub async fn export_get_scope_tree(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<PageGroup>, String> {
    let vault = vault_handle(&state)?;

    let objects = vault
        .list_objects(&account_id, None, None, None, false, false)
        .map_err(|e| format!("list_objects: {}", e))?;

    // Collect custom page-defining objects (type_id = "page") into a lookup
    // page_id -> (page_name, icon_name)
    let mut custom_pages: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    let mut custom_page_order: Vec<String> = Vec::new();
    for obj in &objects {
        if obj.collection_type == "page" && !custom_pages.contains_key(&obj.id) {
            custom_pages.insert(obj.id.clone(), (obj.name.clone(), obj.icon_name.clone()));
            custom_page_order.push(obj.id.clone());
        }
    }

    let custom_page_ids: std::collections::HashSet<String> = custom_pages.keys().cloned().collect();

    let mut groups: std::collections::HashMap<String, Vec<ObjectSummary>> =
        std::collections::HashMap::new();

    for obj in objects {
        let group_key = if obj.collection_type == "page" {
            // Page definition object — group under its own ID so it merges with children
            obj.id.clone()
        } else if !obj.section_type.is_empty() && custom_page_ids.contains(&obj.section_type) {
            // Object belongs to a custom page — use page ID as group key
            obj.section_type.clone()
        } else if !obj.section_type.is_empty() {
            obj.section_type.clone()
        } else if !obj.collection_type.is_empty() {
            obj.collection_type.clone()
        } else {
            "uncategorized".to_string()
        };
        groups.entry(group_key).or_default().push(obj);
    }

    // System page display names (sidebar order)
    let system_sections: &[&str] = &[
        "identity",
        "travel",
        "financial",
        "professional",
        "note",
        "document",
    ];
    let page_names: std::collections::HashMap<&str, &str> = [
        ("identity", "Identity"),
        ("travel", "Travel"),
        ("financial", "Financial"),
        ("professional", "Professional"),
        ("note", "Notes"),
        ("document", "Documents"),
    ]
    .iter()
    .cloned()
    .collect();

    let mut result = Vec::new();

    // 1. System sections in sidebar order
    for key in system_sections {
        if let Some(objs) = groups.remove(*key) {
            let display = page_names.get(key).copied().unwrap_or(key).to_string();
            result.push(PageGroup {
                section_type: key.to_string(),
                page_name: display,
                object_count: objs.len(),
                objects: objs,
            });
        }
    }

    // 2. Custom page groups in order they appear from list_objects
    for page_id in &custom_page_order {
        if let Some(objs) = groups.remove(page_id.as_str()) {
            let page_name = &custom_pages[page_id].0;
            result.push(PageGroup {
                section_type: page_id.clone(),
                page_name: page_name.clone(),
                object_count: objs.len(),
                objects: objs,
            });
        }
    }

    // 3. Any remaining groups (uncategorized, etc.)
    let mut remaining: Vec<(String, Vec<ObjectSummary>)> = groups.into_iter().collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    for (st, objs) in remaining {
        result.push(PageGroup {
            section_type: st.clone(),
            page_name: st,
            object_count: objs.len(),
            objects: objs,
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn export_estimate_size(
    state: State<'_, AppState>,
    account_id: String,
    scope: ExportScope,
) -> Result<ExportEstimate, String> {
    let vault = vault_handle(&state)?;

    let records = collect_scope_objects(&vault, &account_id, &scope)?;
    let count = records.len();
    let mut estimated_bytes: u64 = records
        .iter()
        .map(|r| {
            let props_len = serde_json::to_vec(&r.properties).unwrap_or_default().len() as u64;
            let name_len = r.name.len() as u64;
            props_len + name_len + 256
        })
        .sum();

    // Estimate attachments (only explicitly selected attachment IDs are counted)
    let mut attachment_count = 0usize;
    let mut attachment_selected_count = 0usize;
    if scope.include_attachments {
        let selected: std::collections::HashSet<String> =
            scope.selected_attachment_ids.iter().cloned().collect();
        for rec in &records {
            let atts = load_attachments(&rec.properties);
            for att in &atts {
                if att.deleted_at.is_some() {
                    continue;
                }
                attachment_count += 1;
                if selected.contains(&att.id) {
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
        if let Ok(logs) = vault.list_audit_log(MAX_AUDIT_LOG_EXPORT) {
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
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    // ── Validate password ──────────────────────────────────────
    if req.password.len() < 8 {
        return Err(export_err("PASSWORD_TOO_SHORT"));
    }
    let has_letter = req.password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = req.password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(export_err("PASSWORD_REQUIRE_LETTER_DIGIT"));
    }

    // ── Verify export password is NOT the master password ──────
    match svc.verify_password(&account_id, &req.password) {
        Ok(true) => {
            return Err(export_err("SAME_AS_MASTER_PASSWORD"));
        }
        Ok(false) => { /* export password is different from master password — OK */ }
        Err(e) => {
            return Err(export_err_with_detail(
                "MASTER_VERIFY_FAILED",
                &e.to_string(),
            ));
        }
    }

    // ── Collect objects ────────────────────────────────────────
    let records = collect_scope_objects(vault, &account_id, &req.scope)?;
    if records.is_empty() {
        return Err(export_err("NO_OBJECTS_SELECTED"));
    }

    // ── Collect referenced templates ────────────────────────────
    let template_ids: std::collections::BTreeSet<String> = records
        .iter()
        .filter_map(|r| r.template_id.clone())
        .collect();
    let templates: Vec<serde_json::Value> = template_ids
        .iter()
        .filter_map(|tid| {
            vault
                .load_user_template(tid)
                .ok()
                .flatten()
                .and_then(|tpl| serde_json::to_value(&tpl).ok())
        })
        .collect();

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
            "template_id": r.template_id,
            "template_type": r.template_type,
        })).collect::<Vec<_>>(),
        "templates": templates,
    });
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| format!("serialize: {}", e))?;

    // ── Derive key & encrypt ──────────────────────────────────
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(&req.password, &salt)?;
    // Payload is encrypted via streaming chunked cipher to avoid holding the full
    // ciphertext in memory simultaneously with the plaintext (P1-023).

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

    let mut has_attachments = false;
    let selected_attachment_ids: std::collections::HashSet<String> =
        req.scope.selected_attachment_ids.iter().cloned().collect();
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
                // Fine-grained selection: only export explicitly selected attachments
                if !selected_attachment_ids.contains(&att.id) {
                    continue;
                }
                // Single attachment size limit
                if att.size_bytes > MAX_ATTACHMENT_BYTES {
                    return Err(export_err_with_detail(
                        "ATTACHMENT_TOO_LARGE",
                        &att.file_name,
                    ));
                }

                let src = att
                    .vault_path
                    .as_ref()
                    .or(att.src_path.as_ref())
                    .map(|p| std::path::Path::new(p).to_path_buf())
                    .filter(|p| p.exists())
                    .or_else(|| {
                        let fallback = base_dir.join(&att.id).join(&att.file_name);
                        if fallback.exists() {
                            Some(fallback)
                        } else {
                            None
                        }
                    });

                if let Some(src) = src {
                    validate_attachment_path(svc.base_path().join("attachments").as_path(), &src)?;
                    total_attachment_bytes += att.size_bytes;
                    attachment_entries.push((
                        rec.id.clone(),
                        att.id.clone(),
                        att.file_name.clone(),
                        src,
                    ));
                }
            }
        }
    }

    // Total export size limit (payload + attachments + ~28 bytes overhead per attachment for nonce/chunk_count)
    let payload_estimate = payload_bytes.len() as u64;
    let total_export_estimate =
        payload_estimate + total_attachment_bytes + (attachment_entries.len() as u64 * 28);
    if total_export_estimate > MAX_EXPORT_TOTAL_BYTES {
        return Err(export_err("TOTAL_SIZE_EXCEEDED"));
    }

    // Derive attachment key via HKDF
    let att_key = if !attachment_entries.is_empty() {
        Some(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .map_err(|e| format!("derive att key: {}", e))?,
        )
    } else {
        None
    };

    // Encrypt and write attachments
    if let Some(ref ak) = att_key {
        for (obj_id, att_id, _file_name, src_path) in &attachment_entries {
            let file_size = std::fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
            let zip_name = format!("attachments/{}/{}.enc", obj_id, att_id);
            zip.start_file(&zip_name, options)
                .map_err(|e| e.to_string())?;

            // Always use streaming chunked encryption for attachments to avoid
            // holding both plaintext and ciphertext in memory (P1-023).
            let mut f = File::open(src_path).map_err(|e| format!("open attachment: {}", e))?;
            let mut reader = std::io::BufReader::new(&mut f);
            solosoul_crypto::cipher::encrypt_chunked_stream(ak, file_size, &mut reader, &mut zip)
                .map_err(|e| format!("encrypt attachment: {}", e))?;
            has_attachments = true;
        }
    }

    // ── P2: Preferences ────────────────────────────────────────
    let mut extra_files: Vec<String> = Vec::new();
    let mut preferences_encrypted = false;
    if req.scope.include_preferences {
        if let Ok(Some(profile)) = vault.load_profile(&account_id) {
            let prefs_key =
                solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:preferences:v1")
                    .map_err(|e| format!("derive prefs key: {}", e))?;
            let prefs_enc =
                solosoul_crypto::cipher::encrypt_to_bytes(&prefs_key, &profile.data, None)
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
        if let Ok(logs) = vault.list_audit_log(MAX_AUDIT_LOG_EXPORT) {
            let logs_json = serde_json::to_vec(&logs).unwrap_or_default();
            if !logs_json.is_empty() {
                let behav_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
                    &key,
                    &salt,
                    b"solosoul:behavioral:v1",
                )
                .map_err(|e| format!("derive behavioral key: {}", e))?;
                let behav_enc =
                    solosoul_crypto::cipher::encrypt_to_bytes(&behav_key, &logs_json, None)
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
    let has_templates = !templates.is_empty();
    let manifest = serde_json::json!({
        "version": "2.0",
        "export_scope": if req.scope.full { "full" } else { "partial" },
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
        "has_templates": has_templates,
        "extra_files": extra_files,
        "password_hint": req.password_hint.unwrap_or_default(),
        "salt_hex": hex::encode(salt),
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    // payload.enc (encrypted via streaming chunked cipher — P1-023)
    zip.start_file("payload.enc", options)
        .map_err(|e| e.to_string())?;
    {
        let mut cursor = std::io::Cursor::new(&payload_bytes);
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut cursor,
            &mut zip,
        )
        .map_err(|e| format!("encrypt payload stream: {}", e))?;
    }

    zip.finish().map_err(|e| format!("ZIP finish: {}", e))?;

    let _ = vault.log_structured(
        "export_execute",
        "export",
        None,
        None,
        "user",
        Some(&format!(
            "exported {} objects to {}",
            records.len(),
            zip_path
        )),
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
    let vault = vault_handle(&state)?;

    let obj = vault
        .load_object(&object_id)
        .map_err(|e| e.to_string())?
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
