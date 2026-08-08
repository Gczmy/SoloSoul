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
        // Skip page-defining objects — they are already represented as section headers
        // and should not appear as duplicate items inside their own page section.
        if obj.collection_type == "page" {
            continue;
        }
        let group_key =
            if !obj.section_type.is_empty() && custom_page_ids.contains(&obj.section_type) {
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
        let (page_name, _icon) = &custom_pages[page_id];
        let objs = groups.remove(page_id.as_str()).unwrap_or_default();
        result.push(PageGroup {
            section_type: page_id.clone(),
            page_name: page_name.clone(),
            object_count: objs.len(),
            objects: objs,
        });
    }

    // 3. Any remaining groups (uncategorized, etc.)
    // Filter out orphan UUID groups that belong to already-deleted custom pages.
    // These appear when a custom page was soft-deleted without its child objects
    // (pre-P0-1 bug), leaving orphan objects with section_type = page UUID.
    let system_sections_set: std::collections::HashSet<&str> = [
        "identity",
        "travel",
        "financial",
        "professional",
        "note",
        "document",
        "uncategorized",
    ]
    .into_iter()
    .collect();

    let mut remaining: Vec<(String, Vec<ObjectSummary>)> = groups
        .into_iter()
        .filter(|(key, _)| {
            // Keep non-UUID keys (e.g. "uncategorized") and UUIDs that match known custom pages
            if system_sections_set.contains(key.as_str()) {
                return true;
            }
            if uuid::Uuid::parse_str(key).is_err() {
                return true;
            }
            // UUID key: only keep if it's a known custom page
            custom_pages.contains_key(key)
        })
        .collect();
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

    // 与导出执行（export_execute）共用同一收集逻辑，
    // 保证「导出前展示的模板清单」与最终包内 templates 一致
    let templates = collect_export_templates(&vault, &account_id, &scope, &records)?;
    let template_count = templates.len();
    let template_names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
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

    // Estimate snapshots payload（历史记录，恢复包保证历史数量一致）
    // 按实际加密后字节数估算（snapshots_size_batch 为 LENGTH(data) 之和），
    // base64 编码后再膨胀约 1/3，此处按 1.4x 折算。
    if !records.is_empty() {
        let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
        if let Ok(bytes) = vault.snapshots_size_batch(&ids) {
            estimated_bytes += (bytes as f64 * 1.4) as u64;
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
        template_count,
        template_names,
    })
}

// ── Export execution helpers ─────────────────────────────────

/// 单个待写入 ZIP 的附件条目。
struct ExportAttachmentEntry {
    obj_id: String,
    att_id: String,
    src: std::path::PathBuf,
}

/// 校验导出密码：非空且与主密码不同。
fn validate_export_password(
    svc: &solosoul_core::vault_service::VaultService,
    account_id: &str,
    password: &str,
) -> Result<(), String> {
    // ── Validate password (any non-empty password is accepted, P0-008) ──
    if password.is_empty() {
        return Err(export_err("PASSWORD_EMPTY"));
    }

    // ── Verify export password is NOT the master password ──────
    match svc.verify_password(account_id, password) {
        Ok(true) => Err(export_err("SAME_AS_MASTER_PASSWORD")),
        Ok(false) => Ok(()), // export password is different from master password — OK
        Err(e) => Err(export_err_with_detail("MASTER_VERIFY_FAILED", &e)),
    }
}

/// 解析保存路径（支持 ~/ 前缀）并追加 .solosoul 后缀，确保父目录存在。
///
/// P001: 桌面端在返回前强制校验落盘位置位于允许基目录（Desktop/Documents/Downloads
/// /SOLOSOUL_FS_BASE）内，与 `attachment_download` 同白名单——防 XSS 后以应用权限向任意
/// 路径写入攻击者可控 zip。移动端前端仅能经 SAF URI/staging 中转（无法传任意路径），跳过校验。
#[allow(unused_variables)]
fn resolve_zip_path(app: &tauri::AppHandle, save_path: &str) -> Result<String, String> {
    let resolved = if save_path.starts_with("~/") {
        #[cfg(mobile)]
        {
            app.path()
                .resolve(&save_path[2..], tauri::path::BaseDirectory::Data)
                .map_err(|e| format!("无法解析应用数据目录: {e}"))?
                .to_string_lossy()
                .to_string()
        }
        #[cfg(desktop)]
        {
            let home = std::env::var("HOME").map_err(|_| {
                "HOME environment variable not set; cannot resolve ~/ in save path".to_string()
            })?;
            home + &save_path[1..]
        }
    } else {
        save_path.to_string()
    };
    let zip_path = if resolved.ends_with(".solosoul") {
        resolved
    } else {
        format!("{resolved}.solosoul")
    };

    // P001: 桌面端落盘位置白名单校验（拒绝 `..` 组件 + 必须在 allowed_fs_bases 内）。
    #[cfg(desktop)]
    validate_export_dest(&zip_path)?;

    if let Some(parent) = std::path::Path::new(&zip_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Ok(zip_path)
}

/// P001: 校验导出落盘路径位于允许基目录内（与 `attachment_download` 的校验语义一致）。
#[cfg(desktop)]
fn validate_export_dest(zip_path: &str) -> Result<(), String> {
    let dest = std::path::Path::new(zip_path);
    // 拒绝路径穿越组件
    if dest
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("导出路径不得包含 '..'".to_string());
    }

    let allowed_bases = crate::commands::attachment::allowed_fs_bases();
    if allowed_bases.is_empty() {
        return Ok(());
    }

    // 目标不存在时 canonicalize 父目录（对齐 attachment_download 的宽容处理）
    let dest_canon = if dest.exists() {
        dest.canonicalize()
            .map_err(|e| format!("无法解析导出路径: {e}"))?
    } else if let Some(parent) = dest.parent() {
        if parent.exists() {
            parent
                .canonicalize()
                .map_err(|_| "无法解析导出路径父目录".to_string())?
        } else {
            return Err("导出路径父目录不存在".to_string());
        }
    } else {
        return Err("无效的导出路径".to_string());
    };

    let in_allowed = allowed_bases.iter().any(|base| {
        if dest_canon.starts_with(base) {
            return true;
        }
        if let Some(parent) = dest_canon.parent() {
            parent.starts_with(base)
        } else {
            false
        }
    });
    if !in_allowed {
        return Err(
            "导出位置必须在 Desktop、Documents、Downloads 或 SOLOSOUL_FS_BASE 内".to_string(),
        );
    }
    Ok(())
}

const MAX_ATTACHMENT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB
const MAX_EXPORT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB

/// 收集选中附件：校验大小上限与路径合法性，返回 (条目列表, 总字节数)。
fn collect_attachment_entries(
    svc: &solosoul_core::vault_service::VaultService,
    records: &[solosoul_vault::ObjectRecord],
    scope: &ExportScope,
) -> Result<(Vec<ExportAttachmentEntry>, u64), String> {
    let selected_attachment_ids: std::collections::HashSet<String> =
        scope.selected_attachment_ids.iter().cloned().collect();
    let mut entries: Vec<ExportAttachmentEntry> = Vec::new();
    let mut total_bytes: u64 = 0;

    for rec in records {
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

            let src = solosoul_core::export_import::resolve_attachment_src(
                &base_dir,
                att.vault_path.as_deref(),
                att.src_path.as_deref(),
                &att.id,
                &att.file_name,
            );

            if let Some(src) = src {
                validate_attachment_path(svc.base_path().join("attachments").as_path(), &src)?;
                total_bytes += att.size_bytes;
                entries.push(ExportAttachmentEntry {
                    obj_id: rec.id.clone(),
                    att_id: att.id.clone(),
                    src,
                });
            }
        }
    }
    Ok((entries, total_bytes))
}

/// 用 HKDF 派生附件密钥并流式加密写入 ZIP。返回是否写入过附件。
fn write_attachment_entries(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    key: &[u8; 32],
    salt: &[u8],
    entries: &[ExportAttachmentEntry],
) -> Result<bool, String> {
    if entries.is_empty() {
        return Ok(false);
    }
    // Derive attachment key via HKDF
    let att_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:attachments:v1")
        .map_err(|e| format!("derive att key: {e}"))?;
    for entry in entries {
        let file_size = std::fs::metadata(&entry.src).map(|m| m.len()).unwrap_or(0);
        let zip_name = format!("attachments/{}/{}.enc", entry.obj_id, entry.att_id);
        zip.start_file(&zip_name, options)
            .map_err(|e| e.to_string())?;

        // Always use streaming chunked encryption for attachments to avoid
        // holding both plaintext and ciphertext in memory (P1-023).
        let mut f = File::open(&entry.src).map_err(|e| format!("open attachment: {e}"))?;
        let mut reader = std::io::BufReader::new(&mut f);
        solosoul_crypto::cipher::encrypt_chunked_stream(&att_key, file_size, &mut reader, zip)
            .map_err(|e| format!("encrypt attachment: {e}"))?;
    }
    Ok(true)
}

/// 写入一个加密的附加文件（preferences.enc / behavioral.enc），返回写入的 ZIP 条目名。
fn write_encrypted_extra(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    key: &[u8; 32],
    salt: &[u8],
    label: &[u8],
    file_name: &str,
    content: &[u8],
) -> Result<String, String> {
    let extra_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, label)
        .map_err(|e| format!("derive {file_name} key: {e}"))?;
    let enc = solosoul_crypto::cipher::encrypt_to_bytes(&extra_key, content, None)
        .map_err(|e| format!("encrypt {file_name}: {e}"))?;
    zip.start_file(file_name, options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&enc).map_err(|e| e.to_string())?;
    Ok(file_name.to_string())
}

/// 构建明文 manifest.json（export_scope 由选中 ID 是否为空推导）。
#[allow(clippy::too_many_arguments)]
fn build_manifest_json(
    scope: &ExportScope,
    object_count: usize,
    has_attachments: bool,
    has_preferences: bool,
    has_behavioral: bool,
    has_templates: bool,
    extra_files: &[String],
    password_hint: &Option<String>,
    salt: &[u8],
) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0",
        "export_scope": if scope.selected_page_ids.is_empty() && scope.selected_object_ids.is_empty() { "full" } else { "partial" },
        "selected_pages": scope.selected_page_ids,
        "selected_objects": scope.selected_object_ids,
        "selected_tags": scope.selected_tags,
        "object_count": object_count,
        "export_time": chrono::Utc::now().to_rfc3339(),
        "export_platform": std::env::consts::OS,
        "export_app_version": env!("CARGO_PKG_VERSION"),
        "has_attachments": has_attachments,
        "has_preferences": has_preferences,
        "has_behavioral": has_behavioral,
        "has_templates": has_templates,
        "extra_files": extra_files,
        "password_hint": password_hint.clone().unwrap_or_default(),
        "salt_hex": hex::encode(salt),
        // P202: 导出包携带实际 KDF 参数，导入端按声明派生（旧包无此字段回退 balanced）。
        // from_env()：release 为 production（OWASP 推荐 64MiB/3iter），debug 为 development。
        "kdf": solosoul_core::export_import::kdf_to_manifest_value(
            &solosoul_crypto::kdf::KdfConfig::from_env(),
        ),
    })
}

#[tauri::command]
pub async fn export_execute(
    #[allow(unused_variables)] app: tauri::AppHandle,
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

    // ── 密码校验（非空 + 不得等于主密码）────────────────────
    validate_export_password(&svc, &account_id, &req.password)?;

    // ── Collect objects ────────────────────────────────────────
    let records = collect_scope_objects(vault, &account_id, &req.scope)?;
    // 全量导出（如恢复主机）时允许空对象列表，普通导出仍要求至少选择一个对象。
    if records.is_empty() && !req.scope.include_all {
        return Err(export_err("NO_OBJECTS_SELECTED"));
    }

    // ── Collect templates ───────────────────────────────────────
    // 全量导出（include_all，如恢复主机）打包账户全部模板（含预置种子模板）；
    // 部分导出仅打包被对象引用的模板（快照隔离）。
    let templates: Vec<serde_json::Value> =
        collect_export_templates(vault, &account_id, &req.scope, &records)?
            .iter()
            .filter_map(|tpl| serde_json::to_value(tpl).ok())
            .collect();

    // ── Collect object snapshots（历史记录）──────────────────────
    // 携带每个对象的全部历史快照（含原时间戳），恢复后历史数量与旧设备一致。
    let snapshots = collect_object_snapshots(vault, &records)?;

    // ── Serialise payload ──────────────────────────────────────
    let payload_bytes = serialize_export_payload(&records, &templates, &snapshots)?;

    // ── Derive key & encrypt ──────────────────────────────────
    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(&req.password, &salt)?;

    // ── Build ZIP ──────────────────────────────────────────────
    let zip_path = resolve_zip_path(&app, &req.save_path)?;
    let file = File::create(&zip_path).map_err(|e| format!("Create ZIP: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // ── P1: Attachments ────────────────────────────────────────
    let (attachment_entries, total_attachment_bytes) = if req.scope.include_attachments {
        collect_attachment_entries(&svc, &records, &req.scope)?
    } else {
        (Vec::new(), 0)
    };

    // Total export size limit (payload + attachments + ~28 bytes overhead per attachment for nonce/chunk_count)
    let payload_estimate = payload_bytes.len() as u64;
    let total_export_estimate =
        payload_estimate + total_attachment_bytes + (attachment_entries.len() as u64 * 28);
    if total_export_estimate > MAX_EXPORT_TOTAL_BYTES {
        return Err(export_err("TOTAL_SIZE_EXCEEDED"));
    }

    let has_attachments =
        write_attachment_entries(&mut zip, options, &key, &salt, &attachment_entries)?;

    // ── P2: Preferences ────────────────────────────────────────
    let mut extra_files: Vec<String> = Vec::new();
    let mut preferences_encrypted = false;
    if req.scope.include_preferences {
        if let Ok(Some(profile)) = vault.load_profile(&account_id) {
            extra_files.push(write_encrypted_extra(
                &mut zip,
                options,
                &key,
                &salt,
                b"solosoul:preferences:v1",
                "preferences.enc",
                &profile.data,
            )?);
            preferences_encrypted = true;
        }
    }

    // ── P2: Behavioral data (audit log) ────────────────────────
    let mut behavioral_encrypted = false;
    if req.scope.include_behavioral {
        if let Ok(logs) = vault.list_audit_log(MAX_AUDIT_LOG_EXPORT) {
            let logs_json = serde_json::to_vec(&logs).unwrap_or_default();
            if !logs_json.is_empty() {
                extra_files.push(write_encrypted_extra(
                    &mut zip,
                    options,
                    &key,
                    &salt,
                    b"solosoul:behavioral:v1",
                    "behavioral.enc",
                    &logs_json,
                )?);
                behavioral_encrypted = true;
            }
        }
    }

    // ── manifest.json (plaintext) ─────────────────────────────
    let has_templates = !templates.is_empty();
    let manifest = build_manifest_json(
        &req.scope,
        records.len(),
        has_attachments,
        preferences_encrypted,
        behavioral_encrypted,
        has_templates,
        &extra_files,
        &req.password_hint,
        &salt,
    );
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    // ── payload.enc (encrypted via streaming chunked cipher — P1-023) ──
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
        .map_err(|e| format!("encrypt payload stream: {e}"))?;
    }

    zip.finish().map_err(|e| format!("ZIP finish: {e}"))?;

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

/// 收集各对象的全部历史快照（含原时间戳），恢复后历史数量与旧设备一致。
fn collect_object_snapshots(
    vault: &solosoul_vault::VaultStore,
    records: &[solosoul_vault::ObjectRecord],
) -> Result<Vec<serde_json::Value>, String> {
    let mut snapshots: Vec<serde_json::Value> = Vec::new();
    for r in records {
        let object_id = r.id.clone();
        for meta in vault.list_snapshots(&object_id).unwrap_or_default() {
            let snap_id = match meta["id"].as_str() {
                Some(id) => id.to_string(),
                None => continue,
            };
            let data = match vault.get_snapshot(&snap_id).ok().flatten() {
                Some(d) => d,
                None => continue,
            };
            snapshots.push(serde_json::json!({
                "object_id": object_id,
                "timestamp": meta["timestamp"],
                "triggered_by": meta["triggeredBy"],
                "diff_summary": meta["diffSummary"],
                "data": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &data
                ),
            }));
        }
    }
    Ok(snapshots)
}

/// 序列化导出 payload（对象 + 模板 + 快照）。
fn serialize_export_payload(
    records: &[solosoul_vault::ObjectRecord],
    templates: &[serde_json::Value],
    snapshots: &[serde_json::Value],
) -> Result<Vec<u8>, String> {
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
            "contract_type_id": r.contract_type_id,
            "tags": r.tags_json,
            "created_at": r.created_at,
            "updated_at": r.updated_at,
            "version": r.version,
            "template_id": r.template_id,
            "template_type": r.template_type,
        })).collect::<Vec<_>>(),
        "templates": templates,
        "snapshots": snapshots,
    });
    serde_json::to_vec(&payload).map_err(|e| format!("serialize: {e}"))
}

// ── Attachment info for export UI ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub id: String,
    pub file_name: String,
    pub size_bytes: u64,
}

/// P005: 批量读取对象附件（N+1 优化）——一次 `load_objects_batch` 解密多个对象，
/// 消除前端全选时逐对象 IPC + 逐对象整解密的 N+1 放大。返回 object_id → 附件列表。
#[tauri::command]
pub async fn export_get_attachments_batch(
    state: State<'_, AppState>,
    _account_id: String,
    object_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<AttachmentInfo>>, String> {
    let vault = vault_handle(&state)?;

    let records = vault.load_objects_batch(&object_ids)?;
    let mut result = std::collections::HashMap::with_capacity(object_ids.len());
    for id in &object_ids {
        let atts = records
            .get(id)
            .map(|r| load_attachments(&r.properties))
            .unwrap_or_default();
        let infos: Vec<AttachmentInfo> = atts
            .into_iter()
            .filter(|a| a.deleted_at.is_none())
            .map(|a| AttachmentInfo {
                id: a.id,
                file_name: a.file_name,
                size_bytes: a.size_bytes,
            })
            .collect();
        result.insert(id.clone(), infos);
    }
    Ok(result)
}
