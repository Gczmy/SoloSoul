//! 对象级文档导出（Word/docx / PDF / HTML / TXT / Markdown）——
//! 设计文档 docs/next_dev/对象级文档导出功能设计与实现.md（P047 拆分：子模块见 fields/docx/markdown/text/html/pdf）。

use super::*;
use std::io::Write;

pub mod docx;
pub mod fields;
pub mod html;
pub mod markdown;
pub mod pdf;
#[cfg(test)]
mod tests;
pub mod text;

use docx::build_docx;
use html::build_html_document;
use markdown::build_markdown_document;
use pdf::build_pdf_document;
use text::build_text_document;

// ── 类型 ────────────────────────────────────────────────────

/// preflight 返回的最高敏感度等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentSensitivity {
    None,
    Sensitive,
    Critical,
}

/// 文档导出结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDocumentResult {
    pub object_count: u32,
    pub file_size_bytes: u64,
}

/// 字段敏感度等级顺序：public < internal < sensitive < critical。
/// 返回 `Some(rank)`；未知等级视为 internal（默认）。
fn sensitivity_rank(level: &str) -> u8 {
    match level {
        "public" => 0,
        "internal" => 1,
        "sensitive" => 2,
        "critical" => 3,
        _ => 1,
    }
}

/// 依据字段定义来源合并判定对象的最高字段敏感度。
///
/// 来源优先级（与前端 propertyLabels / __fields / 模板定义一致）：
/// 1. 对象 `property_labels`（field_id → level，对象创建时从模板继承的权威快照；
///    即使用户修改模板敏感度，对象仍保留自己的副本——评审 P221 补充）；
/// 2. 对象 `__fields` 内嵌字段定义的 `sensitivityLevel`（模板同步路径会写入）；
/// 3. 模板 `TemplateProperty.sensitivity_level`（对象仍引用模板时）。
///
/// 注意：`inherit_property_fields` 在对象创建时注入的 `__fields` **不含**
/// `sensitivityLevel`（仅 `template_prop_to_field_def` 模板同步路径写入），
/// 因此 `property_labels` 是新建对象的敏感度权威来源，必须纳入判定。
fn object_max_sensitivity(
    record: &solosoul_vault::ObjectRecord,
    tpl: Option<&solosoul_vault::UserTemplate>,
) -> DocumentSensitivity {
    let mut max_rank = 1u8; // internal 兜底

    // 1. property_labels（权威来源）
    if let Some(labels) = record.property_labels.as_ref().and_then(|v| v.as_object()) {
        for level in labels.values() {
            if let Some(level) = level.as_str() {
                max_rank = max_rank.max(sensitivity_rank(level));
            }
        }
    }

    // 2. __fields 内嵌 sensitivityLevel
    if let Some(fields) = record
        .properties
        .get("__fields")
        .and_then(|v| v.as_object())
    {
        for def in fields.values() {
            if let Some(level) = def.get("sensitivityLevel").and_then(|v| v.as_str()) {
                max_rank = max_rank.max(sensitivity_rank(level));
            }
        }

        // 2b. dynamic_group 子项级 sensitivity（DynamicGroupEditor 每子项携带）
        for (k, v) in record.properties.as_object().expect("checked above") {
            if k.starts_with("__") {
                continue;
            }
            let is_dynamic_group = fields
                .get(k)
                .and_then(|def| def.get("type"))
                .and_then(|t| t.as_str())
                == Some("dynamic_group");
            if !is_dynamic_group {
                continue;
            }
            if let serde_json::Value::Array(items) = v {
                for item in items {
                    if let Some(level) = item.get("sensitivity").and_then(|s| s.as_str()) {
                        max_rank = max_rank.max(sensitivity_rank(level));
                    }
                }
            }
        }
    }

    // 3. 模板定义
    if let Some(tpl) = tpl {
        for prop in &tpl.properties {
            if let Some(ref level) = prop.sensitivity_level {
                max_rank = max_rank.max(sensitivity_rank(level));
            }
        }
    }

    match max_rank {
        3 => DocumentSensitivity::Critical,
        2 => DocumentSensitivity::Sensitive,
        _ => DocumentSensitivity::None,
    }
}

/// XML 转义：`& < > " '` 必须转义。
fn load_records_in_order(
    vault: &solosoul_vault::VaultStore,
    object_ids: &[String],
) -> Result<Vec<solosoul_vault::ObjectRecord>, String> {
    if object_ids.is_empty() {
        return Err(export_err("NO_OBJECTS_SELECTED"));
    }
    let by_id = vault.load_objects_batch(object_ids)?;
    let mut records = Vec::with_capacity(object_ids.len());
    for id in object_ids {
        match by_id.get(id) {
            Some(r) => records.push(r.clone()),
            None => return Err(format!("Object not found: {}", id)),
        }
    }
    Ok(records)
}

/// 加载对象引用的模板名映射（id → name），加载失败静默跳过。
fn load_template_names(
    vault: &solosoul_vault::VaultStore,
    records: &[solosoul_vault::ObjectRecord],
) -> std::collections::HashMap<String, String> {
    let mut names = std::collections::HashMap::new();
    for rec in records {
        if let Some(ref tid) = rec.template_id {
            if let Ok(Some(tpl)) = vault.load_user_template(tid) {
                names.insert(tid.clone(), tpl.name);
            }
        }
    }
    names
}

/// P021: 对象「元信息段」行构建（模板名 + 创建/更新时间 + 标签）。
/// docx / text 两种导出格式共用（逐字相同的构建块，提取避免两处漂移）。
fn build_meta_lines(
    rec: &solosoul_vault::ObjectRecord,
    template_names: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    let tpl_name = rec
        .template_id
        .as_ref()
        .and_then(|tid| template_names.get(tid))
        .cloned()
        .unwrap_or_default();
    let mut meta_lines = Vec::new();
    if !tpl_name.is_empty() {
        meta_lines.push(format!("模板：{}", tpl_name));
    }
    meta_lines.push(format!("创建时间：{}", rec.created_at));
    meta_lines.push(format!("更新时间：{}", rec.updated_at));
    if !rec.tags_json.is_empty() {
        meta_lines.push(format!("标签：{}", rec.tags_json.join(", ")));
    }
    meta_lines
}

/// 导出格式对应的主扩展名（不含点）。
fn format_extension(format: &str) -> Option<&'static str> {
    match format {
        "docx" => Some("docx"),
        "pdf" => Some("pdf"),
        "html" => Some("html"),
        "txt" => Some("txt"),
        "markdown" => Some("md"),
        _ => None,
    }
}

/// 保存路径是否已带目标格式扩展名（html 同时接受 .htm，对应保存对话框过滤器）。
fn path_has_format_ext(save_path: &str, format: &str) -> bool {
    let lower = save_path.to_lowercase();
    match format {
        "html" => lower.ends_with(".html") || lower.ends_with(".htm"),
        _ => lower.ends_with(&format!(".{}", format_extension(format).unwrap_or(""))),
    }
}

/// 解析保存路径：按格式追加扩展名 + 桌面端白名单校验。
/// 移动端前端经 SAF URI 中转（无法传任意路径），跳过校验。
#[allow(unused_variables)]
fn resolve_document_path(
    app: &tauri::AppHandle,
    save_path: &str,
    format: &str,
) -> Result<String, String> {
    let ext = format_extension(format)
        .ok_or_else(|| export_err_with_detail("FORMAT_NOT_SUPPORTED", format))?;
    let path = if path_has_format_ext(save_path, format) {
        save_path.to_string()
    } else {
        format!("{}.{ext}", save_path)
    };

    #[cfg(desktop)]
    validate_export_dest(&path)?;

    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Ok(path)
}

/// 预检：返回所选对象字段的最高敏感度（critical > sensitive > none）。
///
/// 设计 §3.3：逐对象解密与判定移入 `spawn_blocking`（同 `object_get`），
/// 避免多对象全表 AES 解密阻塞 tokio worker。
#[tauri::command]
pub async fn export_document_preflight(
    state: State<'_, AppState>,
    object_ids: Vec<String>,
) -> Result<DocumentSensitivity, String> {
    let vault = vault_handle(&state)?;
    let result = tokio::task::spawn_blocking(move || {
        let records = load_records_in_order(&vault, &object_ids)?;
        let mut max = DocumentSensitivity::None;
        for rec in &records {
            let tpl = rec
                .template_id
                .as_deref()
                .and_then(|tid| vault.load_user_template(tid).ok().flatten());
            let level = object_max_sensitivity(rec, tpl.as_ref());
            if level == DocumentSensitivity::Critical {
                return Ok(DocumentSensitivity::Critical);
            }
            if level == DocumentSensitivity::Sensitive {
                max = DocumentSensitivity::Sensitive;
            }
        }
        Ok::<DocumentSensitivity, String>(max)
    })
    .await
    .map_err(|e| format!("preflight task failed: {e}"))??;
    Ok(result)
}

/// 导出对象为文档（docx / pdf / html / txt / markdown）并落盘。
///
/// - `format` 支持 `"docx"` / `"pdf"` / `"html"` / `"txt"` / `"markdown"`。
/// - 写文件用「临时文件 + rename」避免半截文件；Unix 设权限 0600。
/// - 审计日志 `export_document` 仅记录格式与对象数，不记录字段内容（脱敏规范）。
#[tauri::command]
pub async fn export_objects_document(
    #[allow(unused_variables)] app: tauri::AppHandle,
    state: State<'_, AppState>,
    object_ids: Vec<String>,
    save_path: String,
    format: String,
) -> Result<ExportDocumentResult, String> {
    if format_extension(&format).is_none() {
        return Err(export_err_with_detail("FORMAT_NOT_SUPPORTED", &format));
    }

    let vault = vault_handle(&state)?;
    let export_time = chrono::Utc::now().to_rfc3339();

    // 封面第二行：导出账户名 + 账户 ID（current_account 取当前解锁账户，list_accounts 反查 name）。
    // 账户名反查失败（缓存为空等）时兜底显示 account_id，避免封面出现空账户名。
    let account_id = crate::commands::current_account(&state)?;
    let account_name = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.list_accounts()
            .into_iter()
            .find(|a| a.id == account_id)
            .map(|a| a.name)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| account_id.clone())
    };

    // 解析保存路径（白名单校验在 resolve_document_path 内）——提前做，避免把无效路径带进阻塞任务。
    let document_path = resolve_document_path(&app, &save_path, &format)?;

    // 对象解密 + 文档生成 + 写盘（临时文件 + rename；Unix chmod 600）整体移入
    // spawn_blocking，避免大对象集全表 AES 解密与文件写入阻塞 tokio worker（P114 同款）。
    // vault 是 Arc，闭包内 clone 一份；闭包外保留原句柄供审计日志使用。
    let vault_for_task = vault.clone();
    let format_for_task = format.clone();
    let account_name_for_task = account_name.clone();
    let account_id_for_task = account_id.clone();
    let (object_count, file_size_bytes) = tokio::task::spawn_blocking(move || {
        let records = load_records_in_order(&vault_for_task, &object_ids)?;
        let template_names = load_template_names(&vault_for_task, &records);
        let bytes = match format_for_task.as_str() {
            "docx" => build_docx(
                &records,
                &template_names,
                &export_time,
                &account_name_for_task,
                &account_id_for_task,
            )?,
            "html" => build_html_document(
                &records,
                &template_names,
                &export_time,
                &account_name_for_task,
                &account_id_for_task,
            )
            .into_bytes(),
            "pdf" => build_pdf_document(
                &records,
                &template_names,
                &export_time,
                &account_name_for_task,
                &account_id_for_task,
            )?,
            "txt" => build_text_document(
                &records,
                &template_names,
                &export_time,
                &account_name_for_task,
                &account_id_for_task,
            )
            .into_bytes(),
            "markdown" => build_markdown_document(
                &records,
                &template_names,
                &export_time,
                &account_name_for_task,
                &account_id_for_task,
            )
            .into_bytes(),
            other => return Err(export_err_with_detail("FORMAT_NOT_SUPPORTED", other)),
        };
        let count = records.len();

        let tmp_path = format!("{}.tmp{}", document_path, std::process::id());
        {
            let mut f = File::create(&tmp_path).map_err(|e| format!("Create file: {e}"))?;
            f.write_all(&bytes)
                .map_err(|e| format!("Write file: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = f.set_permissions(std::fs::Permissions::from_mode(0o600));
            }
        }
        std::fs::rename(&tmp_path, &document_path).map_err(|e| format!("Finalize file: {e}"))?;

        Ok::<(usize, u64), String>((count, bytes.len() as u64))
    })
    .await
    .map_err(|e| format!("document export task failed: {e}"))??;

    // 第三重：审计日志（脱敏——不含字段内容与对象名明细）
    crate::commands::log_audit_best_effort(
        &vault,
        "export_document",
        "document",
        None,
        None,
        "user",
        Some(&format!("format={} objects={}", format, object_count)),
    );

    Ok(ExportDocumentResult {
        object_count: object_count as u32,
        file_size_bytes,
    })
}
