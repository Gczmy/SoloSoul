use std::sync::Arc;

use super::helpers::ManifestData;
use super::*;

// ── Import commands ────────────────────────────────────────────

/// P013: 导入解密明文临时目录前缀。临时目录建于**数据目录内**（0700，与敏感数据
/// 同姿态），不再落系统 temp；前缀固定以便启动时/下次导入前清扫崩溃残留孤儿目录。
/// 目录由 `tempfile::Builder` 生成唯一随机后缀（同前缀并存多个互不冲突）。
const IMPORT_TMP_PREFIX: &str = "solosoul-import-tmp-";

/// P013: 清扫数据目录内崩溃残留的导入明文孤儿临时目录（SIGKILL/断电时
/// `TempDir` 无法 Drop 递归删除）。前缀匹配 + `remove_dir_all` 整目录清除；
/// 单个条目失败仅 warn 不阻断（下次启动/导入仍会重试）。
/// 启动时（lib.rs setup）与每次导入前均调用，保证明文不无限期滞留。
pub(crate) fn cleanup_orphan_import_temps(data_dir: &std::path::Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return Ok(());
    };
    let mut cleaned = 0usize;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(IMPORT_TMP_PREFIX) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        match std::fs::remove_dir_all(entry.path()) {
            Ok(()) => cleaned += 1,
            Err(e) => tracing::warn!("[import] 清扫孤儿导入临时目录失败: {} err={}", name, e),
        }
    }
    if cleaned > 0 {
        tracing::info!("[import] 启动/导入前清扫 {cleaned} 个孤儿导入临时目录");
    }
    Ok(())
}

/// P013: 桌面端导入文件路径白名单校验（Desktop/Documents/Downloads + SOLOSOUL_FS_BASE），
/// 拒绝越界路径；移动端文件来自 SAF 选择/应用内路径，不做此校验。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn validate_import_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    file_path: &str,
) -> Result<(), String> {
    crate::commands::fs::resolve_allowed_path(app, file_path).map(|_| ())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn validate_import_path<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    _file_path: &str,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn import_parse_package<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    file_path: String,
) -> Result<ImportPreview, String> {
    validate_import_path(&app, &file_path)?;
    let fp = file_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        // P201: 统一经 read_manifest_json 读取（含 100MB 大小上限，防 ZIP 炸弹 OOM）
        let v = read_manifest_json(&fp)?;

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
pub async fn import_decrypt_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    file_path: String,
    password: String,
) -> Result<DecryptedImportPreview, String> {
    validate_import_path(&app, &file_path)?;
    let vault = vault_handle(&state)?;

    let manifest = read_manifest(&file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    // P202: 按 manifest 声明参数派生（旧格式包无 kdf 字段回退 balanced 兼容）。
    let key = derive_export_key_cfg(&password, &salt, &manifest.kdf_config())?;
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
                        template_hash: o["template_hash"].as_str().map(String::from),
                        ignored_template_hash: o["ignored_template_hash"]
                            .as_str()
                            .map(String::from),
                        icon_name: o["icon_name"].as_str().unwrap_or("document").to_string(),
                        parent_id: o["parent_id"].as_str().map(String::from),
                        properties: o["properties"].clone(),
                        property_labels: None,
                        // 与导出范围树同一口径（solosoul_vault::object_has_attachments）：
                        // 未软删附件存在性，供导入侧对象行按附件展开。
                        has_attachments: solosoul_vault::object_has_attachments(&o["properties"]),
                        // 字段敏感度集合（导入包无 property_labels，由 __fields/dynamic_group 推导）
                        sensitivity_levels: solosoul_vault::object_field_sensitivity_levels(
                            None,
                            &o["properties"],
                        ),
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
                // 比较名称判断冲突类型：名称相同为 Identical，否则为 RenamedLocal
                //（只能区分名称是否相同，无法判断是本地改名还是导入包名称被修改）
                let kind = if obj.name == existing.name {
                    ConflictKind::Identical
                } else {
                    ConflictKind::RenamedLocal
                };
                conflicts.push(ConflictInfo {
                    object_id: obj.id.clone(),
                    imported_name: obj.name.clone(),
                    existing_name: existing.name.clone(),
                    kind,
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

/// P2: Advanced import with object selection and strategy
#[tauri::command]
pub async fn import_execute_advanced<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    account_id: String,
    req: AdvancedImportRequest,
) -> Result<ImportResult, String> {
    validate_import_path(&app, &req.source_path)?;
    import_execute_internal(
        state,
        account_id,
        req.source_path,
        // P015: IPC 边界立即 Zeroizing 包装
        zeroize::Zeroizing::new(req.password),
        req.strategy,
        Some(req.selections),
        req.selected_attachment_ids,
        req.object_strategies,
        &req.locale,
        None,
    )
    .await
}

/// 导入执行核心。`progress` 可选进度回调：供恢复等长耗时导入场景展示进度条；常规导入传 `None`。
/// 进度语义：对象阶段按循环下标报告 0-80（跳过对象也推进，保证阶段可到达 80），
/// 附件阶段续接报告 80-100（`import_attachments` 内部换算），整体单调不回落。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn import_execute_internal(
    state: State<'_, AppState>,
    account_id: String,
    file_path: String,
    password: zeroize::Zeroizing<String>,
    strategy: ImportStrategy,
    selections: Option<Vec<ImportSelection>>,
    selected_attachment_ids: Option<Vec<String>>,
    object_strategies: HashMap<String, ImportStrategy>,
    locale: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
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

    // ── 阶段 1：解密包读取（password 为 Zeroizing，自动 Deref 为 &str）──
    // P013: 导入前清扫上次崩溃残留的明文孤儿临时目录（数据目录内）。
    let _ = cleanup_orphan_import_temps(svc.base_path());
    let (manifest, payload, key) = decrypt_package(&file_path, &password, svc.base_path())?;

    // Build selection set if provided
    let selected_ids = build_selected_ids(selections);

    let objects = payload["objects"]
        .as_array()
        .ok_or("No objects array in payload")?;
    let package_ids = build_package_ids(&payload);

    // ── 阶段 1.5：解析包内对象历史快照（object_id → 快照列表）──
    // 导出端携带每个对象的全部历史快照（含原时间戳），导入时按原时间线恢复，
    // 保证跨设备恢复后历史记录数量与旧设备一致。
    let package_snapshots = build_package_snapshots(&payload);

    // ── 阶段 2：重建包内引用模板（快照隔离，按内容哈希去重）──
    let template_id_map = rebuild_imported_templates(vault, &account_id, &payload)?;

    let now = chrono::Utc::now().to_rfc3339();

    // ── 阶段 3：预构建 KeepBoth ID 映射表（解决前向引用问题）──
    let id_map = build_keepboth_id_map(objects, &object_strategies);

    // ── 阶段 4：对象导入主循环（策略/模板/KeepBoth/快照已抽至 import_one_object）──
    let (imported, imported_object_ids) = import_objects_loop(
        vault,
        objects,
        &account_id,
        strategy,
        &object_strategies,
        selected_ids.as_ref(),
        &package_ids,
        &template_id_map,
        &id_map,
        &package_snapshots,
        &now,
        locale,
        progress.as_deref(),
    )?;

    // 构建选中附件 ID 集合，用于附件过滤
    let sel_att_ids_set: Option<std::collections::HashSet<String>> =
        selected_attachment_ids.map(|ids| ids.into_iter().collect());

    // ── 阶段 5+6：导入附件与偏好设置（附件进度续接 80-100）──
    let imported_attachments_count = import_attachments_and_preferences(
        vault,
        svc.base_path(),
        &file_path,
        &key,
        &manifest,
        objects,
        &id_map,
        &imported_object_ids,
        sel_att_ids_set.as_ref(),
        &now,
        progress.clone(),
        &account_id,
    )?;

    let details = build_import_details(imported, imported_attachments_count, &file_path, strategy);
    crate::commands::log_audit_best_effort(
        vault,
        "import_execute",
        "import",
        None,
        None,
        "user",
        Some(&details.to_string()),
    );
    state.auto_sync.trigger_debounce();

    Ok(ImportResult {
        object_count: imported,
        attachment_count: imported_attachments_count,
    })
}
/// 构建选中附件/对象 ID 集合（selections 中 selected=true 的 object_id）。
fn build_selected_ids(selections: Option<Vec<ImportSelection>>) -> Option<BTreeSet<String>> {
    selections.map(|sels| {
        sels.into_iter()
            .filter(|s| s.selected)
            .map(|s| s.object_id)
            .collect()
    })
}

/// 阶段 4：对象导入主循环（策略解析/模板继承/KeepBoth 重写/快照恢复已抽至 import_one_object）。
/// 返回 (imported 计数, 已导入对象 ID 集合)。
#[allow(clippy::too_many_arguments)]
fn import_objects_loop(
    vault: &solosoul_vault::VaultStore,
    objects: &[serde_json::Value],
    account_id: &str,
    strategy: ImportStrategy,
    object_strategies: &HashMap<String, ImportStrategy>,
    selected_ids: Option<&BTreeSet<String>>,
    package_ids: &std::collections::HashSet<String>,
    template_id_map: &std::collections::HashMap<String, String>,
    id_map: &HashMap<String, String>,
    package_snapshots: &HashMap<String, Vec<serde_json::Value>>,
    now: &str,
    locale: &str,
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
) -> Result<(usize, std::collections::HashSet<String>), String> {
    let mut imported = 0usize;
    let mut imported_object_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let objects_len = objects.len();
    for (obj_index, obj_val) in objects.iter().enumerate() {
        // 阶段 4 主体抽至 import_one_object：策略解析/模板继承/KeepBoth 重写/快照恢复
        let outcome = import_one_object(
            vault,
            obj_val,
            account_id,
            strategy,
            object_strategies,
            selected_ids,
            package_ids,
            template_id_map,
            id_map,
            package_snapshots,
            now,
            locale,
            progress,
            obj_index,
            objects_len,
        )?;
        let Some((final_id, is_keepboth)) = outcome else {
            continue;
        };
        imported += 1;
        imported_object_ids.insert(final_id);
        // 也记录旧 ID 以便附件查找（KeepBoth 场景）
        if is_keepboth {
            imported_object_ids.insert(obj_val["id"].as_str().unwrap_or("").to_string());
        }
    }
    Ok((imported, imported_object_ids))
}

/// 阶段 5+6：导入附件（加密，流式解密）与偏好设置。
/// 附件阶段进度续接对象阶段末尾（80-100），避免进度条回落。返回附件导入数量。
#[allow(clippy::too_many_arguments)]
fn import_attachments_and_preferences(
    vault: &solosoul_vault::VaultStore,
    base_path: &std::path::Path,
    file_path: &str,
    key: &[u8; 32],
    manifest: &ManifestData,
    objects: &[serde_json::Value],
    id_map: &HashMap<String, String>,
    imported_object_ids: &std::collections::HashSet<String>,
    sel_att_ids_set: Option<&std::collections::HashSet<String>>,
    now: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
    account_id: &str,
) -> Result<usize, String> {
    let att_progress = progress.map(wrap_attachment_progress);
    let imported_attachments_count = if manifest.has_attachments {
        import_attachments(
            vault,
            base_path,
            file_path,
            key,
            manifest,
            objects,
            id_map,
            imported_object_ids,
            sel_att_ids_set,
            now,
            att_progress.as_deref(),
        )?
    } else {
        0
    };
    import_preferences(vault, file_path, key, manifest, account_id)?;
    Ok(imported_attachments_count)
}

/// 组装导入审计详情（count / attachmentCount / fileName / strategy）。
fn build_import_details(
    imported: usize,
    imported_attachments_count: usize,
    file_path: &str,
    strategy: ImportStrategy,
) -> serde_json::Value {
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    serde_json::json!({
        "count": imported,
        "attachmentCount": imported_attachments_count,
        "fileName": file_name,
        "strategy": match strategy {
            ImportStrategy::SkipExisting => "skipExisting",
            ImportStrategy::Overwrite => "overwrite",
            ImportStrategy::KeepBoth => "keepBoth",
        },
    })
}

/// 阶段 1.5：解析包内对象历史快照（object_id → 快照列表）。
/// 导出端携带每个对象的全部历史快照（含原时间戳），导入时按原时间线恢复。
fn build_package_snapshots(payload: &serde_json::Value) -> HashMap<String, Vec<serde_json::Value>> {
    payload["snapshots"]
        .as_array()
        .map(|arr| {
            let mut map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            for snap in arr {
                if let Some(oid) = snap["object_id"].as_str() {
                    map.entry(oid.to_string()).or_default().push(snap.clone());
                }
            }
            map
        })
        .unwrap_or_default()
}

/// 阶段 3：预构建 KeepBoth ID 映射表（解决前向引用问题）。
fn build_keepboth_id_map(
    objects: &[serde_json::Value],
    object_strategies: &HashMap<String, ImportStrategy>,
) -> HashMap<String, String> {
    let mut id_map: HashMap<String, String> = HashMap::new();
    for obj_val in objects {
        let id = obj_val["id"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }
        if object_strategies.get(id).copied() == Some(ImportStrategy::KeepBoth) {
            id_map.insert(id.to_string(), generate_id());
        }
    }
    id_map
}

/// 阶段 5：附件进度续接对象阶段末尾（80-100），避免进度条回落。
fn wrap_attachment_progress(cb: Arc<dyn Fn(u8) + Send + Sync>) -> Arc<dyn Fn(u8) + Send + Sync> {
    Arc::new(move |pct: u8| {
        cb((80 + u16::from(pct) * 20 / 100) as u8);
    })
}

/// 阶段 4：导入单个对象（策略解析、模板继承、KeepBoth 重写、快照恢复）。
/// 返回 `(final_id, imported)`：`imported=false` 表示该对象被过滤/跳过。
/// 副作用：写对象行、恢复/创建快照、更新进度回调。
#[allow(clippy::too_many_arguments)]
fn import_one_object(
    vault: &solosoul_vault::VaultStore,
    obj_val: &serde_json::Value,
    account_id: &str,
    strategy: ImportStrategy,
    object_strategies: &HashMap<String, ImportStrategy>,
    selected_ids: Option<&BTreeSet<String>>,
    package_ids: &std::collections::HashSet<String>,
    template_id_map: &std::collections::HashMap<String, String>,
    id_map: &HashMap<String, String>,
    package_snapshots: &HashMap<String, Vec<serde_json::Value>>,
    now: &str,
    locale: &str,
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
    obj_index: usize,
    objects_len: usize,
) -> Result<Option<(String, bool)>, String> {
    let id = obj_val["id"].as_str().unwrap_or("");
    if id.is_empty() {
        return Ok(None);
    }

    // Apply selection filter
    if let Some(sel_ids) = selected_ids {
        if !sel_ids.contains(id) {
            return Ok(None);
        }
    }

    // Check conflict & apply strategy (per-object override first, then global)
    let effective_strategy = object_strategies.get(id).copied().unwrap_or(strategy);
    // KeepBoth 不需要冲突判断（永远继续往下走）；SkipExisting 遇非软删既有对象则跳过
    if effective_strategy != ImportStrategy::KeepBoth {
        let existing = vault.load_object(id).ok().flatten();
        if effective_strategy == ImportStrategy::SkipExisting
            && existing.is_some_and(|e| !e.is_deleted)
        {
            return Ok(None);
        }
    }

    // Resolve cross-scope RelationProperty references
    let mut properties = obj_val["properties"].clone();
    resolve_cross_scope_references(&mut properties, package_ids);

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
            (Some(tpl), Some(existing)) => merge_labels_into(&tpl, existing),
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

    // ── 重写 KeepBoth ID 引用（所有对象都需要，不仅仅是 KeepBoth 对象）
    // 这样如果 Object A（overwrite）引用 Object B（KeepBoth），A 的引用也会被更新
    if !id_map.is_empty() {
        rewrite_id_references(&mut properties, id_map);
    }

    // ── KeepBoth: 使用预先生成的新 ID + 新名称 ────────────────────
    let (final_id, final_name): (String, String) = if effective_strategy == ImportStrategy::KeepBoth
    {
        let new_id = id_map.get(id).cloned().unwrap_or_else(generate_id);
        let new_name = unique_object_name(
            vault,
            account_id,
            obj_val["name"].as_str().unwrap_or("Imported"),
            locale,
        )?;
        (new_id, new_name)
    } else {
        (
            id.to_string(),
            obj_val["name"].as_str().unwrap_or("Imported").to_string(),
        )
    };

    // ── 阶段 4.1：构建导入对象记录（含 KeepBoth ID 重写）──
    let record = build_import_record(
        obj_val,
        account_id,
        id_map,
        resolved_template_id,
        &final_id,
        &final_name,
        properties,
        property_labels,
        now,
    );

    vault
        .save_object(&record)
        .map_err(|e| format!("save: {}", e))?;

    // 恢复包内历史快照（若有），否则创建初始 snapshot 使历史 badge 正常显示。
    // KeepBoth 场景下对象获得新 ID，快照随之挂到新 ID 上。
    let snapshot_key = if effective_strategy == ImportStrategy::KeepBoth {
        &final_id
    } else {
        id
    };
    let restored = if let Some(snaps) = package_snapshots.get(id) {
        // P1: Overwrite 覆盖导入时，仅在包内确实携带【可恢复】的快照时先清空本地旧历史，
        // 防止包内快照叠加导致历史数量翻倍；损坏包（快照 base64 全部解码失败）保留本地
        // 历史，避免本地历史被误删后仅剩一条 diff_imported 的数据丢失。
        // SkipExisting 遇既有对象会跳过；KeepBoth 使用新 ID 天然无旧历史，均不受影响。
        if effective_strategy == ImportStrategy::Overwrite && snapshots_any_restorable(snaps) {
            if let Err(e) = vault.delete_snapshots(snapshot_key) {
                tracing::warn!(
                    "[import] 覆盖导入清空旧快照失败: object={} err={}",
                    snapshot_key,
                    e
                );
            }
        }
        restore_package_snapshots(vault, snapshot_key, snaps)
    } else {
        0
    };
    if restored == 0 {
        // 旧包或对象无历史时，保持既有行为：创建 diff_imported 初始快照
        let snapshot_data =
            serde_json::to_vec(&record).map_err(|e| format!("snapshot ser: {}", e))?;
        crate::commands::save_snapshot_best_effort(
            vault,
            snapshot_key,
            "import",
            &snapshot_data,
            "diff_imported",
        );
    }

    if let Some(cb) = progress {
        let total = objects_len.max(1);
        // 对象阶段 0-80（按循环下标推进，跳过对象也前进，保证单调到达 80）
        cb(((obj_index + 1) * 80 / total).min(80) as u8);
    }

    Ok(Some((
        final_id,
        effective_strategy == ImportStrategy::KeepBoth,
    )))
}

// ── 阶段化辅助函数（P023 拆分）──────────────────────────────────

/// 阶段 1：读取并解密导入包，返回 (manifest, payload, 派生密钥)。
///
/// P013: 明文临时目录建于 `temp_base`（保险库数据目录，0700）内而非系统 temp——
/// 进程 SIGKILL/崩溃时残留明文仍位于受保护的数据目录，且前缀固定可被
/// `cleanup_orphan_import_temps` 清扫；正常路径 `TempDir` Drop 递归删除整个目录。
fn decrypt_package(
    file_path: &str,
    password: &str,
    temp_base: &std::path::Path,
) -> Result<(ManifestData, serde_json::Value, [u8; 32]), String> {
    let manifest = read_manifest(file_path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    // P202: 按 manifest 声明参数派生（旧格式包无 kdf 字段回退 balanced 兼容）。
    let key = derive_export_key_cfg(password, &salt, &manifest.kdf_config())?;
    // R2-15: 主 payload 流式解密——`payload.enc` 经 decrypt_chunked_stream 直接写入临时文件，
    // 再从文件流式解析 JSON；峰值内存由「密文 + 明文 + JSON 树」约 3× 降至约 1× payload。
    let tmp_dir = tempfile::Builder::new()
        .prefix(IMPORT_TMP_PREFIX)
        .tempdir_in(temp_base)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;
    let mut tmp = tempfile::NamedTempFile::new_in(tmp_dir.path())
        .map_err(|e| format!("创建临时文件失败: {}", e))?;
    decrypt_zip_entry_streaming(file_path, "payload.enc", &key, &mut tmp)?;
    let payload: serde_json::Value = {
        let f = std::fs::File::open(tmp.path()).map_err(|e| format!("读取临时文件失败: {}", e))?;
        serde_json::from_reader(f).map_err(|e| format!("Invalid payload: {}", e))?
    };
    // tmp（NamedTempFile）先于 tmp_dir Drop；tmp_dir 随后递归删除整个临时目录。
    Ok((manifest, payload, key))
}

/// 阶段 1.5 helper：按原时间戳恢复对象历史快照（base64 解码 → 加密写入）。
/// 返回成功恢复的快照条数；快照为空/解码失败返回 0（调用方回退到 diff_imported 初始快照）。
/// P1 辅助：判断包内快照列表中是否存在至少一条可恢复的快照（base64 可解码且非空）。
/// 覆盖导入仅在确有可恢复快照时才清空本地旧历史，防止损坏包（快照全部解码失败）
/// 误删本地历史后仅回退为一条 diff_imported 快照。
pub(crate) fn snapshots_any_restorable(snaps: &[serde_json::Value]) -> bool {
    snaps.iter().any(|snap| match snap["data"].as_str() {
        Some(b64) => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map(|d| !d.is_empty())
            .unwrap_or(false),
        None => false,
    })
}

pub(crate) fn restore_package_snapshots(
    vault: &solosoul_vault::VaultStore,
    object_id: &str,
    snaps: &[serde_json::Value],
) -> usize {
    let mut restored = 0usize;
    for snap in snaps {
        // 原时间戳缺失/非法时回退到当前时间，避免 0 时间戳破坏历史排序
        let timestamp = snap["timestamp"]
            .as_i64()
            .filter(|t| *t > 0)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let triggered_by = snap["triggered_by"].as_str().unwrap_or("import");
        let diff_summary = snap["diff_summary"].as_str().unwrap_or("diff_imported");
        let data = match snap["data"].as_str() {
            Some(b64) => {
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(
                            "[import] 快照 base64 解码失败，跳过: object={} err={}",
                            object_id,
                            e
                        );
                        continue;
                    }
                }
            }
            None => continue,
        };
        if data.is_empty() {
            continue;
        }
        if vault
            .save_snapshot_at(object_id, triggered_by, &data, diff_summary, timestamp)
            .is_ok()
        {
            restored += 1;
        }
    }
    restored
}

/// 阶段 2：重建包内引用的模板（快照隔离，按内容哈希去重），返回 原模板 ID → 本地模板 ID 映射。
pub(crate) fn rebuild_imported_templates(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    payload: &serde_json::Value,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut template_id_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if let Some(templates) = payload["templates"].as_array() {
        let now = chrono::Utc::now().to_rfc3339();
        for tpl_val in templates {
            match serde_json::from_value::<solosoul_vault::UserTemplate>(tpl_val.clone()) {
                Ok(mut tpl) => {
                    let original_id = tpl.id.clone();
                    let hash = solosoul_core::export_import::user_template_content_hash(&tpl);

                    // P035: 三分支去重逻辑抽纯函数。
                    let local_id = resolve_template_id(vault, account_id, &mut tpl, &hash, &now)?;

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
    Ok(template_id_map)
}

/// P035: 解析单个模板的本地 ID（去重三分支）：
/// 1. 内容哈希已存在（含系统预置模板）→ 复用已有模板 ID；
/// 2. 本地无同 ID 模板 → 保留原始 ID（预置种子模板 key 如 passport 得以保留，
///    恢复后模板 ID 与旧设备一致）；
/// 3. 本地已有同 ID 但内容不同 → 派生 ID（快照隔离，避免覆盖本地模板）。
///
/// 需要保存新模板时统一改写归属字段并写入。
fn resolve_template_id(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    tpl: &mut solosoul_vault::UserTemplate,
    hash: &str,
    now: &str,
) -> Result<String, String> {
    if let Some(existing) = vault.find_user_template_by_content_hash(account_id, hash)? {
        return Ok(existing.id);
    }

    let original_id = tpl.id.clone();
    if vault
        .load_user_template(&original_id)
        .ok()
        .flatten()
        .is_none()
    {
        // 本地无同 ID 模板 → 保留原始 ID
        tpl.id = original_id.clone();
        tpl.account_id = account_id.to_string();
        tpl.created_at = now.to_string();
        tpl.updated_at = Some(now.to_string());
        let _ = vault.save_user_template(tpl);
        return Ok(original_id);
    }

    // 本地已有同 ID 但内容不同 → 派生 ID
    let imported_id = solosoul_core::export_import::imported_template_id(&original_id, hash);
    if vault
        .load_user_template(&imported_id)
        .ok()
        .flatten()
        .is_none()
    {
        tpl.id = imported_id.clone();
        tpl.account_id = account_id.to_string();
        tpl.created_at = now.to_string();
        tpl.updated_at = Some(now.to_string());
        let _ = vault.save_user_template(tpl);
    }
    Ok(imported_id)
}

/// 阶段 4.1：构建导入对象记录（含 KeepBoth ID 引用重写）。
#[allow(clippy::too_many_arguments)]
fn build_import_record(
    obj_val: &serde_json::Value,
    account_id: &str,
    id_map: &HashMap<String, String>,
    resolved_template_id: Option<String>,
    final_id: &str,
    final_name: &str,
    properties: serde_json::Value,
    property_labels: Option<serde_json::Value>,
    now: &str,
) -> solosoul_vault::ObjectRecord {
    solosoul_vault::ObjectRecord {
        contract_type_id: obj_val["contract_type_id"].as_str().map(String::from),
        id: final_id.to_string(),
        account_id: account_id.to_string(),
        type_id: obj_val["type_id"].as_str().unwrap_or("note").to_string(),
        section_type: obj_val["section_type"]
            .as_str()
            .unwrap_or("identity")
            .to_string(),
        name: final_name.to_string(),
        icon_name: obj_val["icon_name"]
            .as_str()
            .unwrap_or("document")
            .to_string(),
        parent_id: obj_val["parent_id"].as_str().map(|pid| {
            // 无条件重写引用：如果父对象被 KeepBoth 重写了 ID，使用新 ID
            id_map.get(pid).cloned().unwrap_or_else(|| pid.to_string())
        }),
        children_ids: {
            // 无条件重写 children_ids 引用
            let mut cids: Vec<String> = obj_val["children_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            for cid in &mut cids {
                if let Some(new_cid) = id_map.get(cid) {
                    *cid = new_cid.clone();
                }
            }
            cids
        },
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
        template_hash: obj_val["template_hash"].as_str().map(String::from),
        ignored_template_hash: obj_val["ignored_template_hash"].as_str().map(String::from),
        created_at: obj_val["created_at"].as_str().unwrap_or(now).to_string(),
        updated_at: now.to_string(),
        version: obj_val["version"].as_u64().unwrap_or(1) as u32,
    }
}

/// 合并模板 property_labels 进现有 labels：模板值作为兜底，不覆盖已有值。
fn merge_labels_into(tpl: &serde_json::Value, existing: &mut serde_json::Value) {
    if let (Some(tpl_obj), Some(existing_obj)) = (tpl.as_object(), existing.as_object_mut()) {
        for (k, v) in tpl_obj {
            existing_obj.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

/// 阶段 5：导入附件（加密）。流式解密避免明文/密文同时驻留内存（P1-024）。
/// `progress` 的调用契约：传入 0-100 的附件条目进度，本函数不再换算；调用方需负责把
/// 回调映射到自己的剩余区间（当前唯一调用方 import_execute_internal 映射到 80-100，
/// 保证整体进度单调不回落）。
#[allow(clippy::too_many_arguments)]
fn import_attachments(
    vault: &solosoul_vault::VaultStore,
    base_path: &std::path::Path,
    file_path: &str,
    key: &[u8; 32],
    manifest: &ManifestData,
    objects: &[serde_json::Value],
    id_map: &HashMap<String, String>,
    imported_object_ids: &std::collections::HashSet<String>,
    sel_att_ids_set: Option<&std::collections::HashSet<String>>,
    now: &str,
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
) -> Result<usize, String> {
    // Derive attachment key via HKDF
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("Invalid salt: {}", e))?;
    let att_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(key, &salt, b"solosoul:attachments:v1")
            .map_err(|e| format!("derive att key: {}", e))?;

    // Build old att_id -> meta map from payload objects
    let att_meta_map = build_att_meta_map(objects);

    // Open ZIP and iterate attachments
    let zip_file = File::open(file_path).map_err(|e| format!("open zip: {}", e))?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| format!("invalid zip: {}", e))?;
    let att_prefix = "attachments/";
    let mut imported_atts: std::collections::HashMap<String, Vec<AttachmentMeta>> =
        std::collections::HashMap::new();
    let zip_total = archive.len();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_string();
        if let Some(cb) = &progress {
            cb(((i * 100) / zip_total.max(1)).min(100) as u8);
        }
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
        if let Some(sel_set) = sel_att_ids_set {
            if !sel_set.contains(old_att_id) {
                continue;
            }
        }

        let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
            Some(m) => m,
            None => continue,
        };
        let new_att = extract_att_meta_for_object(
            base_path, &att_key, &mut f, obj_id, old_meta, id_map, now,
        )?;
        imported_atts
            .entry(obj_id.to_string())
            .or_default()
            .push(new_att);
    }

    // 对于 KeepBoth 对象，将附件 key 从旧 ID 映射到新 ID
    let mut remapped_atts: std::collections::HashMap<String, Vec<AttachmentMeta>> =
        std::collections::HashMap::new();
    for (old_obj_id, atts) in &imported_atts {
        let actual_obj_id = id_map
            .get(old_obj_id)
            .cloned()
            .unwrap_or_else(|| old_obj_id.clone());
        let mut new_atts = atts.clone();
        for att in &mut new_atts {
            att.object_id = actual_obj_id.clone();
        }
        remapped_atts
            .entry(actual_obj_id)
            .or_default()
            .append(&mut new_atts);
    }

    // Replace each imported object's __attachments with the newly imported list
    write_attachments_back(vault, &remapped_atts)?;

    Ok(imported_atts.values().map(|v| v.len()).sum())
}

/// 从 payload objects 构建「旧附件 ID → 元数据」映射。
fn build_att_meta_map(
    objects: &[serde_json::Value],
) -> std::collections::HashMap<(String, String), AttachmentMeta> {
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
    att_meta_map
}

/// 解码并落盘单个 ZIP 附件条目：净化文件名 → 流式解密 → 返回新 AttachmentMeta。
/// Use streaming decryption to avoid holding the full ciphertext
/// and plaintext in memory simultaneously (P1-024)。
/// 参数多因含路径上下文/密钥/元数据/映射/时间戳，语义独立不合并。
#[allow(clippy::too_many_arguments)]
fn extract_att_meta_for_object(
    base_path: &std::path::Path,
    att_key: &[u8; 32],
    f: &mut zip::read::ZipFile<'_>,
    obj_id: &str,
    old_meta: &AttachmentMeta,
    id_map: &HashMap<String, String>,
    now: &str,
) -> Result<AttachmentMeta, String> {
    let new_att_id = generate_id();
    // KeepBoth 场景下附件目录应使用新对象 ID，否则后续 load_object 基于新 ID 查找会找不到
    let att_obj_id = id_map
        .get(obj_id)
        .cloned()
        .unwrap_or_else(|| obj_id.to_string());
    let dest = base_path
        .join("attachments")
        .join(&att_obj_id)
        .join(&new_att_id);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    // R008/P003: 净化导入文件名——显式拒绝路径分隔符（Unix 上 `\\` 不是分隔符，
    // 仅靠 Path::file_name() 无法剥离 `..\\..\\evil.txt` 中的反斜杠），再取末段组件兜底。
    let raw_name = &old_meta.file_name;
    if raw_name.contains('/') || raw_name.contains('\\') {
        return Err("Invalid attachment file name in package".to_string());
    }
    let safe_name = std::path::Path::new(raw_name)
        .file_name()
        .ok_or("Invalid attachment file name in package")?
        .to_string_lossy()
        .to_string();
    if safe_name.is_empty() || safe_name == "." || safe_name == ".." {
        return Err("Invalid attachment file name in package".to_string());
    }
    let file_path_dest = dest.join(&safe_name);
    let mut out_file =
        File::create(&file_path_dest).map_err(|e| format!("create attachment file: {}", e))?;
    solosoul_crypto::cipher::decrypt_chunked_stream(att_key, f, &mut out_file)
        .map_err(|e| format!("decrypt attachment stream: {}", e))?;
    let file_size = std::fs::metadata(&file_path_dest)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(AttachmentMeta {
        id: new_att_id,
        object_id: obj_id.to_string(),
        // P003: 元数据写回净化后的 safe_name，防止后续插件主机 join 时存储型路径遍历
        file_name: safe_name.clone(),
        mime_type: old_meta.mime_type.clone(),
        size_bytes: file_size,
        created_at: now.to_string(),
        deleted_at: None,
        src_path: Some(file_path_dest.to_string_lossy().to_string()),
        vault_path: Some(file_path_dest.to_string_lossy().to_string()),
        description: None,
        tags: vec![],
    })
}

/// 将新附件列表写回各对象的 __attachments 元数据。
fn write_attachments_back(
    vault: &solosoul_vault::VaultStore,
    remapped_atts: &std::collections::HashMap<String, Vec<AttachmentMeta>>,
) -> Result<(), String> {
    for (obj_id, atts) in remapped_atts {
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
        vault.save_object(&obj)?;
    }
    Ok(())
}

/// 阶段 6：导入偏好设置（如包内含 preferences.enc）。
fn import_preferences(
    vault: &solosoul_vault::VaultStore,
    file_path: &str,
    key: &[u8; 32],
    manifest: &ManifestData,
    account_id: &str,
) -> Result<(), String> {
    if !manifest
        .extra_files
        .contains(&"preferences.enc".to_string())
    {
        return Ok(());
    }
    let prefs_salt = hex::decode(&manifest.salt_hex)
        .map_err(|e| format!("Invalid salt_hex in manifest: {}", e))?;
    let prefs_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(key, &prefs_salt, b"solosoul:preferences:v1")
            .map_err(|e| format!("derive prefs key: {}", e))?;
    if let Ok(prefs_enc) = read_file_from_zip(file_path, "preferences.enc") {
        if let Ok(prefs_dec) =
            solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None)
        {
            let profile =
                solosoul_vault::Profile::new_with_id(account_id, account_id, prefs_dec.to_vec());
            let _ = vault.save_profile(&profile);
        }
    }
    Ok(())
}
