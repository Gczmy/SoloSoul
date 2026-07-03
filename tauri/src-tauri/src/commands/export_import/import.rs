use super::*;

// ── Import commands ────────────────────────────────────────────

#[tauri::command]
pub async fn import_parse_package(file_path: String) -> Result<ImportPreview, String> {
    let fp = file_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let path = std::path::Path::new(&fp);
        if !path.exists() {
            return Err(import_err_with_detail("FILE_NOT_FOUND", &fp));
        }
        let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|_| import_err("INVALID_PACKAGE"))?;

        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| import_err("MISSING_MANIFEST"))?;
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("Read: {}", e))?;
        let s = String::from_utf8_lossy(&buf).to_string();
        let v: serde_json::Value =
            serde_json::from_str(&s).map_err(|e| format!("Invalid manifest JSON: {}", e))?;

        let extra_files: Vec<String> = v
            .get("extra_files")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ImportPreview {
            file_path: fp,
            version: v
                .get("version")
                .and_then(|x| x.as_str())
                .unwrap_or("1.0")
                .to_string(),
            object_count: v.get("object_count").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            has_attachments: v
                .get("has_attachments")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            extra_files,
            export_time: v
                .get("export_time")
                .and_then(|x| x.as_str())
                .map(|x| x.to_string()),
            password_hint: v
                .get("password_hint")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(|x| x.to_string()),
        })
    })
    .await;

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
    let vault = vault_handle(&state)?;

    let manifest = read_manifest(&file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    let key = derive_export_key(&password, &salt)?;
    let enc_bytes = read_file_from_zip(&file_path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc_bytes)
        .map_err(|_| import_err("DECRYPT_FAILED"))?;

    let payload: serde_json::Value =
        serde_json::from_slice(&decrypted).map_err(|e| format!("Invalid payload: {}", e))?;

    let objects: Vec<ObjectSummary> = payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    Some(ObjectSummary {
                        contract_type_id: o["contract_type_id"].as_str().map(String::from),
                        id: o["id"].as_str()?.to_string(),
                        name: o["name"].as_str()?.to_string(),
                        collection_type: o["type_id"].as_str()?.to_string(),
                        section_type: o["section_type"].as_str().unwrap_or("").to_string(),
                        sensitivity_level: o["sensitivity_level"]
                            .as_str()
                            .unwrap_or("internal")
                            .to_string(),
                        created_at: o["created_at"].as_str().unwrap_or("").to_string(),
                        updated_at: o["updated_at"].as_str().unwrap_or("").to_string(),
                        is_deleted: false,
                        template_id: o["template_id"].as_str().map(String::from),
                        template_type: o["template_type"].as_str().map(String::from),
                        icon_name: o["icon_name"].as_str().unwrap_or("document").to_string(),
                        properties: o["properties"].clone(),
                        property_labels: None,
                        tags: o["tags"]
                            .as_array()
                            .map(|t| {
                                t.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut conflicts = Vec::new();
    for obj in &objects {
        if let Ok(Some(existing)) = vault.load_object(&obj.id) {
            // Soft-deleted objects are in trash and should not be treated as conflicts.
            if !existing.is_deleted {
                conflicts.push(ConflictInfo {
                    object_id: obj.id.clone(),
                    name: obj.name.clone(),
                });
            }
        }
    }

    let has_preferences = manifest
        .extra_files
        .contains(&"preferences.enc".to_string());

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
                    id: att.id.clone(),
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
) -> Result<ImportResult, String> {
    import_execute_internal(
        state,
        account_id,
        file_path,
        password,
        ImportStrategy::SkipExisting,
        None,
        None,
    )
    .await
}

/// P2: Advanced import with object selection and strategy
#[tauri::command]
pub async fn import_execute_advanced(
    state: State<'_, AppState>,
    account_id: String,
    req: AdvancedImportRequest,
) -> Result<ImportResult, String> {
    import_execute_internal(
        state,
        account_id,
        req.source_path,
        req.password,
        req.strategy,
        Some(req.selections),
        req.selected_attachment_ids,
    )
    .await
}

async fn import_execute_internal(
    state: State<'_, AppState>,
    account_id: String,
    file_path: String,
    password: String,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
) -> Result<ImportResult, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let vault_guard = svc.get_vault_store().ok_or("Vault not unlocked")?;
    let vault = vault_guard.as_ref();

    if password.is_empty() {
        return Err(import_err("PASSWORD_REQUIRED"));
    }

    let manifest = read_manifest(&file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    let key = derive_export_key(&password, &salt)?;
    let enc_bytes = read_file_from_zip(&file_path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc_bytes)
        .map_err(|_| import_err("DECRYPT_FAILED"))?;

    let payload: serde_json::Value =
        serde_json::from_slice(&decrypted).map_err(|e| format!("Invalid payload: {}", e))?;

    // Build selection set if provided
    let selected_ids: Option<BTreeSet<String>> = selections.map(|sels| {
        sels.into_iter()
            .filter(|s| s.selected)
            .map(|s| s.object_id)
            .collect()
    });

    let objects = payload["objects"]
        .as_array()
        .ok_or("No objects array in payload")?;
    let package_ids = build_package_ids(&payload);
    // ── Rebuild referenced templates (snapshot isolation by content hash) ──
    let mut template_id_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if let Some(templates) = payload["templates"].as_array() {
        let now = chrono::Utc::now().to_rfc3339();
        for tpl_val in templates {
            match serde_json::from_value::<solosoul_vault::UserTemplate>(tpl_val.clone()) {
                Ok(mut tpl) => {
                    let original_id = tpl.id.clone();
                    let hash = solosoul_core::export_import::user_template_content_hash(&tpl);

                    // 去重：检查是否有完全一致的已有模板（含系统预置模板）
                    let local_id = if let Some(existing) = vault
                        .find_user_template_by_content_hash(&account_id, &hash)
                        .map_err(|e| e.to_string())?
                    {
                        existing.id
                    } else {
                        let imported_id =
                            solosoul_core::export_import::imported_template_id(&original_id, &hash);
                        if vault
                            .load_user_template(&imported_id)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            tpl.id = imported_id.clone();
                            tpl.account_id = account_id.clone();
                            tpl.created_at = now.clone();
                            tpl.updated_at = Some(now.clone());
                            let _ = vault.save_user_template(&tpl);
                        }
                        imported_id
                    };

                    template_id_map.insert(original_id, local_id);
                }
                Err(e) => {
                    tracing::warn!(
                        "[import] 模板反序列化失败，跳过: {}, 错误: {}",
                        tpl_val["id"].as_str().unwrap_or("<unknown>"),
                        e
                    );
                }
            }
        }
    }

    let mut imported = 0usize;
    let mut imported_object_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();
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
                // Soft-deleted objects are not considered existing; import will restore them.
                if existing.is_some_and(|e| !e.is_deleted) {
                    continue;
                }
            }
            ImportStrategy::Overwrite => { /* always overwrite — fall through */ }
            ImportStrategy::Merge => { /* overwrite always — fall through */ }
        }

        // Resolve cross-scope RelationProperty references
        let mut properties = obj_val["properties"].clone();
        resolve_cross_scope_references(&mut properties, &package_ids);

        // ── 解析实际模板 ID ──
        let resolved_template_id = obj_val["template_id"].as_str().map(|tid| {
            template_id_map
                .get(tid)
                .cloned()
                .unwrap_or_else(|| tid.to_string())
        });

        // ── 从模板继承字段敏感度、字段定义和模板名称 ──
        // 即使模板后来被删除，对象仍保留自己的副本
        let mut property_labels = if obj_val["property_labels"].is_null() {
            None
        } else {
            Some(obj_val["property_labels"].clone())
        };
        if let Some(ref tid) = resolved_template_id {
            // 合并 property_labels：payload 原有值优先，模板值作为兜底
            let tpl_labels = crate::commands::object::inherit_property_labels(vault, Some(tid));
            match (tpl_labels, &mut property_labels) {
                (Some(tpl), Some(ref mut existing)) => {
                    // 模板值作为兜底，不覆盖已有值
                    if let Some(tpl_obj) = tpl.as_object() {
                        if let Some(existing_obj) = existing.as_object_mut() {
                            for (k, v) in tpl_obj {
                                existing_obj.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                        }
                    }
                }
                (Some(tpl), None) => {
                    property_labels = Some(tpl);
                }
                _ => {}
            }

            // 注入 __fields（字段名称 + 类型）
            let fields = crate::commands::object::inherit_property_fields(vault, Some(tid));
            crate::commands::object::inject_property_fields(&mut properties, &fields);

            // 注入 __templateName
            crate::commands::object::inject_template_meta(vault, Some(tid), &mut properties);
        }

        let record = solosoul_vault::ObjectRecord {
            contract_type_id: obj_val["contract_type_id"].as_str().map(String::from),
            id: id.to_string(),
            account_id: account_id.clone(),
            type_id: obj_val["type_id"].as_str().unwrap_or("note").to_string(),
            section_type: obj_val["section_type"]
                .as_str()
                .unwrap_or("identity")
                .to_string(),
            name: obj_val["name"].as_str().unwrap_or("Imported").to_string(),
            icon_name: obj_val["icon_name"]
                .as_str()
                .unwrap_or("document")
                .to_string(),
            parent_id: obj_val["parent_id"].as_str().map(String::from),
            children_ids: obj_val["children_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            properties,
            property_labels,
            sensitivity_level: obj_val["sensitivity_level"]
                .as_str()
                .unwrap_or("internal")
                .to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: obj_val["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            template_id: resolved_template_id,
            template_type: obj_val["template_type"].as_str().map(String::from),
            created_at: obj_val["created_at"].as_str().unwrap_or(&now).to_string(),
            updated_at: now.clone(),
            version: obj_val["version"].as_u64().unwrap_or(1) as u32,
        };

        vault
            .save_object(&record)
            .map_err(|e| format!("save: {}", e))?;

        // 为导入对象创建初始 snapshot，使历史记录 badge 正常显示
        let snapshot_data =
            serde_json::to_vec(&record).map_err(|e| format!("snapshot ser: {}", e))?;
        let _ = vault.save_snapshot(id, "import", &snapshot_data, "Imported");

        imported += 1;
        imported_object_ids.insert(id.to_string());
    }

    // 构建选中附件 ID 集合，用于附件过滤
    let sel_att_ids_set: Option<std::collections::HashSet<String>> =
        selected_attachment_ids.map(|ids| ids.into_iter().collect());

    let mut imported_attachments_count = 0usize;

    // ── P1: Import attachments (encrypted) ─────────────────────
    if manifest.has_attachments {
        // Derive attachment key via HKDF
        let att_key =
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .map_err(|e| format!("derive att key: {}", e))?;

        // Build old att_id -> meta map from payload objects
        let mut att_meta_map: std::collections::HashMap<(String, String), AttachmentMeta> =
            std::collections::HashMap::new();
        for obj_val in objects {
            let obj_id = obj_val["id"].as_str().unwrap_or("");
            if obj_id.is_empty() {
                continue;
            }
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
        let mut imported_atts: std::collections::HashMap<String, Vec<AttachmentMeta>> =
            std::collections::HashMap::new();
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
            if validate_export_id(obj_id).is_err() || validate_export_id(old_att_id).is_err() {
                continue;
            }

            // Skip if object was not imported (e.g. SkipExisting)
            if !imported_object_ids.contains(obj_id) {
                continue;
            }

            // 如果指定了附件选择，跳过未选中的附件
            if let Some(ref sel_set) = sel_att_ids_set {
                if !sel_set.contains(old_att_id) {
                    continue;
                }
            }

            let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
                Some(m) => m,
                None => continue,
            };

            // Use streaming decryption to avoid holding the full ciphertext
            // and plaintext in memory simultaneously (P1-024).
            let new_att_id = generate_id();
            let dest = svc
                .base_path()
                .join("attachments")
                .join(obj_id)
                .join(&new_att_id);
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

            // R008: sanitize imported file_name to prevent path traversal.
            let safe_name = std::path::Path::new(&old_meta.file_name)
                .file_name()
                .ok_or("Invalid attachment file name in package")?
                .to_string_lossy()
                .to_string();
            let file_path_dest = dest.join(&safe_name);
            let mut out_file = File::create(&file_path_dest)
                .map_err(|e| format!("create attachment file: {}", e))?;
            solosoul_crypto::cipher::decrypt_chunked_stream(&att_key, &mut f, &mut out_file)
                .map_err(|e| format!("decrypt attachment stream: {}", e))?;
            let file_size = std::fs::metadata(&file_path_dest)
                .map(|m| m.len())
                .unwrap_or(0);

            imported_atts
                .entry(obj_id.to_string())
                .or_default()
                .push(AttachmentMeta {
                    id: new_att_id,
                    object_id: obj_id.to_string(),
                    file_name: old_meta.file_name.clone(),
                    mime_type: old_meta.mime_type.clone(),
                    size_bytes: file_size,
                    created_at: now.clone(),
                    deleted_at: None,
                    src_path: Some(file_path_dest.to_string_lossy().to_string()),
                    vault_path: Some(file_path_dest.to_string_lossy().to_string()),
                });
        }

        // Replace each imported object's __attachments with the newly imported list
        for (obj_id, atts) in &imported_atts {
            let mut obj = vault
                .load_object(obj_id)
                .map_err(|e| format!("get object: {}", e))?
                .ok_or_else(|| format!("object {} not found", obj_id))?;
            let att_json = serde_json::to_value(atts).map_err(|e| e.to_string())?;
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

        imported_attachments_count = imported_atts.values().map(|v| v.len()).sum();
    }

    // ── P2: Import preferences if present ──────────────────────
    if manifest
        .extra_files
        .contains(&"preferences.enc".to_string())
    {
        let prefs_salt = hex::decode(&manifest.salt_hex)
            .map_err(|e| format!("Invalid salt_hex in manifest: {}", e))?;
        let prefs_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(
            &key,
            &prefs_salt,
            b"solosoul:preferences:v1",
        )
        .map_err(|e| format!("derive prefs key: {}", e))?;
        if let Ok(prefs_enc) = read_file_from_zip(&file_path, "preferences.enc") {
            if let Ok(prefs_dec) =
                solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None)
            {
                let profile = solosoul_vault::Profile::new_with_id(
                    &account_id,
                    &account_id,
                    prefs_dec.to_vec(),
                );
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
        Some(&format!(
            "imported {} objects ({} attachments) from {} (strategy: {:?})",
            imported, imported_attachments_count, file_path, strategy
        )),
    );

    Ok(ImportResult {
        object_count: imported,
        attachment_count: imported_attachments_count,
    })
}
