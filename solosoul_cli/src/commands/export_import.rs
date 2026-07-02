//! 加密导出/导入命令。
//!
//! 实现 `/export` 与 `/import`，使用与 GUI 相同的 `.solosoul` ZIP/ manifest 格式：
//! - `manifest.json`：明文元数据（版本、对象数、salt、密码提示等）
//! - `payload.enc`：AES-256-GCM 加密的对象负载
//! - `attachments/{obj_id}/{att_id}.enc`：可选附件
//! - `preferences.enc`：可选用户偏好设置

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::app::App;
use crate::commands::CliError;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};
use solosoul_core::{ObjectRecord, Profile, VaultStore};

// 导出包大小限制（与 GUI 一致）
const MAX_ATTACHMENT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB
const MAX_EXPORT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB

/// 附件元数据（与 GUI `attachment::AttachmentMeta` 一致，camelCase 序列化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentMeta {
    pub id: String,
    pub object_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_path: Option<String>,
}

/// 导出范围。
#[derive(Debug, Clone, Default)]
pub(crate) struct ExportScope {
    /// 导出当前账户全部对象。
    pub full: bool,
    /// 按 `section_type` 导出的页面/分类列表。
    pub selected_page_ids: Vec<String>,
    /// 显式指定的对象 ID 列表。
    pub selected_object_ids: Vec<String>,
    /// 是否包含附件。
    pub include_attachments: bool,
}

/// 导入冲突处理策略。
#[derive(Debug, Clone, Copy)]
pub(crate) enum ImportStrategy {
    /// 跳过已存在的对象（保留本地）。
    SkipExisting,
    /// 覆盖本地对象。
    Overwrite,
    /// 合并：CLI 暂按覆盖实现。
    Merge,
}

/// 从 manifest.json 解析出的必要字段。
struct ManifestData {
    pub salt_hex: String,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
    pub version: String,
    pub object_count: usize,
    pub password_hint: Option<String>,
}

/// 确保 Vault 已解锁，返回当前账户 ID（给 `handle` 使用）。
fn require_unlocked(app: &mut App) -> Result<String> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(color_eyre::eyre::eyre!("Vault is locked"));
    }
    app.vault_service
        .get_current_account()
        .ok_or_else(|| color_eyre::eyre::eyre!("No current account"))
}

/// 确保 Vault 已解锁，返回账户 ID（给内部 String 错误函数使用）。
fn require_account_id(app: &mut App) -> Result<String, CliError> {
    if !app.vault_service.is_unlocked() {
        app.error_message = Some("请先使用 /unlock 登录".to_string());
        return Err(CliError::VaultLocked);
    }
    app.vault_service
        .get_current_account()
        .ok_or(CliError::NoAccount)
}

/// 命令入口。`args[0]` 为 `/export` 或 `/import`。
pub fn handle(app: &mut App, args: &[&str]) -> Result<()> {
    let base = args.first().copied().unwrap_or("");
    match base {
        "/export" => handle_export(app, &args[1..]),
        "/import" => handle_import(app, &args[1..]),
        _ => {
            app.error_message = Some(format!("未知的导出/导入子命令: {}", base));
            Ok(())
        }
    }
}

/// 处理 `/export [file] [--full] [--pages a,b] [--objects id1,id2] [--include-attachments]`。
fn handle_export(app: &mut App, args: &[&str]) -> Result<()> {
    require_unlocked(app)?;

    let (file_arg, scope) = match parse_export_args(args) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(e.to_string());
            return Ok(());
        }
    };

    let base = app.vault_service.base_path().to_path_buf();
    let path = match resolve_export_path(&base, file_arg) {
        Ok(p) => p,
        Err(e) => {
            app.error_message = Some(e.to_string());
            return Ok(());
        }
    };

    // 导出密码通过模态提示安全采集。
    let path_for_cb = path;
    let scope_for_cb = scope;
    prompt::open(
        app,
        PromptSpec::Text {
            label: "导出密码".to_string(),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(password) = result {
                if let Err(e) = export_execute(app, &password, &path_for_cb, &scope_for_cb) {
                    app.error_message = Some(format!("导出失败: {}", e));
                }
            }
        }),
    );

    Ok(())
}

/// 处理 `/import [file] [--preview] [--strategy skip|overwrite|merge]`。
fn handle_import(app: &mut App, args: &[&str]) -> Result<()> {
    let (file_arg, preview, strategy) = match parse_import_args(args) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(e.to_string());
            return Ok(());
        }
    };

    let file_arg = match file_arg {
        Some(f) => f,
        None => {
            app.error_message = Some("请提供要导入的文件路径".to_string());
            return Ok(());
        }
    };
    let path = PathBuf::from(file_arg);

    if preview {
        match import_preview(&path) {
            Ok(info) => app.error_message = Some(info),
            Err(e) => app.error_message = Some(format!("导入预览失败: {}", e)),
        }
        return Ok(());
    }

    // 非预览模式需要解锁。
    require_unlocked(app)?;

    let path_for_cb = path;
    prompt::open(
        app,
        PromptSpec::Text {
            label: "导入密码".to_string(),
            initial: String::new(),
            mask: true,
            allow_toggle_mask: true,
        },
        Box::new(move |app, result| {
            if let PromptResult::Text(password) = result {
                if let Err(e) = import_execute(app, &path_for_cb, &password, strategy) {
                    app.error_message = Some(format!("导入失败: {}", e));
                }
            }
        }),
    );

    Ok(())
}

/// 解析 `/export` 参数。
fn parse_export_args<'a>(args: &[&'a str]) -> Result<(Option<&'a str>, ExportScope), CliError> {
    let mut file_arg: Option<&str> = None;
    let mut scope = ExportScope::default();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            match *arg {
                "--full" => scope.full = true,
                "--include-attachments" => scope.include_attachments = true,
                "--pages" => {
                    let list = iter.next().ok_or(CliError::Msg(
                        "--pages 后需要逗号分隔的页面列表".to_string(),
                    ))?;
                    scope.selected_page_ids = list.split(',').map(String::from).collect();
                }
                "--objects" => {
                    let list = iter.next().ok_or(CliError::Msg(
                        "--objects 后需要逗号分隔的对象 ID 列表".to_string(),
                    ))?;
                    scope.selected_object_ids = list.split(',').map(String::from).collect();
                }
                other => return Err(CliError::Msg(format!("未知导出选项: {}", other))),
            }
        } else if file_arg.is_none() {
            file_arg = Some(*arg);
        } else {
            return Err(CliError::Msg("多余的文件参数".to_string()));
        }
    }

    if !scope.full && scope.selected_page_ids.is_empty() && scope.selected_object_ids.is_empty() {
        return Err(CliError::Msg(
            "请指定 --full、--pages 或 --objects 之一".to_string(),
        ));
    }

    Ok((file_arg, scope))
}

/// 解析 `/import` 参数。
fn parse_import_args<'a>(
    args: &[&'a str],
) -> Result<(Option<&'a str>, bool, ImportStrategy), CliError> {
    let mut file_arg: Option<&str> = None;
    let mut preview = false;
    let mut strategy = ImportStrategy::Overwrite;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            match *arg {
                "--preview" => preview = true,
                "--strategy" => {
                    let value = iter
                        .next()
                        .ok_or(CliError::Msg("--strategy 后需要策略值".to_string()))?;
                    strategy = match *value {
                        "skip" => ImportStrategy::SkipExisting,
                        "overwrite" => ImportStrategy::Overwrite,
                        "merge" => ImportStrategy::Merge,
                        other => return Err(CliError::Msg(format!("未知导入策略: {}", other))),
                    };
                }
                other => return Err(CliError::Msg(format!("未知导入选项: {}", other))),
            }
        } else if file_arg.is_none() {
            file_arg = Some(*arg);
        } else {
            return Err(CliError::Msg("多余的文件参数".to_string()));
        }
    }

    Ok((file_arg, preview, strategy))
}

/// 决定导出包写入路径。
///
/// - 未提供文件名：使用当前工作目录下的 `solosoul_export_{timestamp}.solosoul`。
/// - 提供了文件名：仅使用其文件名字段，并写入 `{base}/exports/` 下，防止越界写入数据目录。
fn resolve_export_path(base: &Path, file_arg: Option<&str>) -> Result<PathBuf, CliError> {
    match file_arg {
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| base.to_path_buf());
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            Ok(cwd.join(format!("solosoul_export_{}.solosoul", ts)))
        }
        Some(arg) => {
            let exports_dir = base.join("exports");
            std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
            let file_name = Path::new(arg)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "export.solosoul".to_string());
            let mut path = exports_dir.join(file_name);
            if path.extension() != Some(OsStr::new("solosoul")) {
                path.set_extension("solosoul");
            }
            Ok(path)
        }
    }
}

/// 执行导出（可被测试直接调用，跳过密码提示）。
pub(crate) fn export_execute(
    app: &mut App,
    password: &str,
    path: &Path,
    scope: &ExportScope,
) -> Result<(), CliError> {
    validate_export_password(app, password)?;

    let account_id = require_account_id(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| "Vault 未打开".to_string())?;

    let records = collect_scope_objects(&vault, &account_id, scope)?;
    if records.is_empty() {
        return Err(CliError::Msg("没有选中任何对象".to_string()));
    }

    let payload = build_payload(&vault, &records);
    let payload_bytes =
        serde_json::to_vec(&payload).map_err(|e| format!("序列化负载失败: {}", e))?;

    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(password, &salt)?;
    // Payload will be encrypted via streaming chunked cipher to avoid holding the full
    // ciphertext in memory (P1-023).

    // 收集附件源文件。
    let base = app.vault_service.base_path().to_path_buf();
    let attachment_entries = collect_attachment_entries(&base, &records, scope)?;

    let payload_estimate = payload_bytes.len() as u64;
    let total_attachment_bytes: u64 = attachment_entries
        .iter()
        .map(|(_, _, _, src)| std::fs::metadata(src).map(|m| m.len()).unwrap_or(0))
        .sum();
    let total_export_estimate =
        payload_estimate + total_attachment_bytes + (attachment_entries.len() as u64 * 28);
    if total_export_estimate > MAX_EXPORT_TOTAL_BYTES {
        return Err(CliError::Msg("导出包总大小超过限制".to_string()));
    }

    let att_key = if !attachment_entries.is_empty() {
        Some(
            solosoul_crypto::hkdf_ext::derive_hkdf_key(&key, &salt, b"solosoul:attachments:v1")
                .map_err(|e| format!("派生附件密钥失败: {}", e))?,
        )
    } else {
        None
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| format!("创建导出文件失败: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 写入附件（使用流式加密以避免完整明文和密文同时驻留内存 — P1-023）。
    if let Some(ref ak) = att_key {
        for (obj_id, att_id, _file_name, src_path) in &attachment_entries {
            let file_size = std::fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
            let zip_name = format!("attachments/{}/{}.enc", obj_id, att_id);
            zip.start_file(&zip_name, options)
                .map_err(|e| format!("写入 ZIP 附件条目失败: {}", e))?;
            let mut f = File::open(src_path).map_err(|e| format!("打开附件失败: {}", e))?;
            let mut reader = std::io::BufReader::new(&mut f);
            solosoul_crypto::cipher::encrypt_chunked_stream(ak, file_size, &mut reader, &mut zip)
                .map_err(|e| format!("加密附件失败: {}", e))?;
        }
    }

    // manifest.json
    // has_templates 以实际导出 payload 中成功加载的模板为准
    let has_templates = payload["templates"]
        .as_array()
        .is_some_and(|a| !a.is_empty());
    let manifest = build_manifest(scope, &records, att_key.is_some(), has_templates, &salt);
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("序列化 manifest 失败: {}", e))?;
    zip.start_file("manifest.json", options)
        .map_err(|e| format!("写入 manifest 条目失败: {}", e))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| format!("写入 manifest 数据失败: {}", e))?;

    // payload.enc (encrypted via streaming chunked cipher — P1-023)
    zip.start_file("payload.enc", options)
        .map_err(|e| format!("写入 payload 条目失败: {}", e))?;
    {
        let mut cursor = std::io::Cursor::new(&payload_bytes);
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut cursor,
            &mut zip,
        )
        .map_err(|e| format!("加密 payload 流失败: {}", e))?;
    }

    zip.finish().map_err(|e| format!("完成 ZIP 失败: {}", e))?;

    let _ = vault.log_structured(
        "export_execute",
        "export",
        None,
        None,
        "user",
        Some(&format!(
            "exported {} objects to {}",
            records.len(),
            path.display()
        )),
    );

    app.error_message = Some(format!(
        "已导出 {} 个对象到 {}",
        records.len(),
        path.display()
    ));
    Ok(())
}

/// 校验导出密码强度并确认其不是主密码。
fn validate_export_password(app: &App, password: &str) -> Result<(), CliError> {
    if password.len() < 8 {
        return Err(CliError::Msg("导出密码至少需要 8 位".to_string()));
    }
    let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(CliError::Msg("导出密码必须同时包含字母和数字".to_string()));
    }

    let account_id = app
        .vault_service
        .get_current_account()
        .ok_or_else(|| "未找到当前账户".to_string())?;
    match app.vault_service.verify_password(&account_id, password) {
        Ok(true) => Err(CliError::Validation("导出密码不能与主密码相同".to_string())),
        Ok(false) => Ok(()),
        Err(e) => Err(CliError::Msg(format!("校验主密码失败: {}", e))),
    }
}

/// 使用 Argon2id 从导出密码与 salt 派生 32 字节密钥。
fn derive_export_key(password: &str, salt: &[u8]) -> Result<[u8; 32], CliError> {
    use solosoul_crypto::kdf::{derive_key, KdfConfig};
    let key_vec = derive_key(password, salt, &KdfConfig::balanced())
        .map_err(|e| format!("密钥派生失败: {}", e))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_vec);
    Ok(key)
}

/// 从对象属性中读取附件列表。
fn load_attachments(props: &serde_json::Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

/// 根据范围收集对象记录。
fn collect_scope_objects(
    vault: &VaultStore,
    account_id: &str,
    scope: &ExportScope,
) -> Result<Vec<ObjectRecord>, CliError> {
    let all = vault.list_objects(account_id, None, None, None, false, false)?;

    if scope.full {
        let ids: Vec<String> = all.iter().map(|s| s.id.clone()).collect();
        let loaded = vault.load_objects_batch(&ids)?;
        return Ok(loaded.into_values().collect());
    }

    let mut selected_ids: HashSet<String> = scope.selected_object_ids.iter().cloned().collect();
    for summary in &all {
        if scope.selected_page_ids.contains(&summary.section_type) {
            selected_ids.insert(summary.id.clone());
        }
    }

    let ids: Vec<String> = selected_ids.into_iter().collect();
    let loaded = vault.load_objects_batch(&ids)?;
    Ok(loaded.into_values().collect())
}

/// 构建加密前的 payload JSON（包含模板快照）。
fn build_payload(vault: &VaultStore, records: &[ObjectRecord]) -> serde_json::Value {
    let template_ids: HashSet<String> = records
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

    serde_json::json!({
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
    })
}

/// 收集需要导出的附件源路径。
fn collect_attachment_entries(
    base: &Path,
    records: &[ObjectRecord],
    scope: &ExportScope,
) -> Result<Vec<(String, String, String, PathBuf)>, CliError> {
    if !scope.include_attachments {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for rec in records {
        let atts = load_attachments(&rec.properties);
        if atts.is_empty() {
            continue;
        }
        let base_dir = base.join("attachments").join(&rec.id);
        for att in &atts {
            if att.deleted_at.is_some() {
                continue;
            }
            if att.size_bytes > MAX_ATTACHMENT_BYTES {
                return Err(CliError::Msg(format!("附件过大: {}", att.file_name)));
            }

            let src = att
                .vault_path
                .as_ref()
                .or(att.src_path.as_ref())
                .map(|p| Path::new(p).to_path_buf())
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
                entries.push((rec.id.clone(), att.id.clone(), att.file_name.clone(), src));
            }
        }
    }

    // 按对象 ID 排序，便于测试与排查。
    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Ok(entries)
}

/// 构建 manifest.json 内容。
fn build_manifest(
    scope: &ExportScope,
    records: &[ObjectRecord],
    has_attachments: bool,
    has_templates: bool,
    salt: &[u8; 16],
) -> serde_json::Value {
    serde_json::json!({
        "version": "2.0",
        "export_scope": if scope.full { "full" } else { "partial" },
        "selected_pages": scope.selected_page_ids,
        "selected_objects": scope.selected_object_ids,
        "object_count": records.len(),
        "export_time": chrono::Utc::now().to_rfc3339(),
        "export_platform": std::env::consts::OS,
        "export_app_version": env!("CARGO_PKG_VERSION"),
        "has_attachments": has_attachments,
        "has_preferences": false,
        "has_behavioral": false,
        "has_templates": has_templates,
        "extra_files": [],
        "password_hint": "",
        "salt_hex": hex::encode(salt),
    })
}

/// 读取导入包预览信息。
pub(crate) fn import_preview(path: &Path) -> Result<String, CliError> {
    let manifest = read_manifest(path)?;
    Ok(format!(
        "导出包预览: 版本 {}, 对象数 {}, 包含附件: {}, 密码提示: {}",
        manifest.version,
        manifest.object_count,
        if manifest.has_attachments {
            "是"
        } else {
            "否"
        },
        manifest.password_hint.unwrap_or_else(|| "无".to_string())
    ))
}

/// 执行导入（可被测试直接调用，跳过密码提示）。
pub(crate) fn import_execute(
    app: &mut App,
    path: &Path,
    password: &str,
    strategy: ImportStrategy,
) -> Result<(), CliError> {
    if password.is_empty() {
        return Err(CliError::Msg("导入密码不能为空".to_string()));
    }

    let account_id = require_account_id(app)?;
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| "Vault 未打开".to_string())?;

    let manifest = read_manifest(path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("salt 解码失败: {}", e))?;
    let key = derive_export_key(password, &salt)?;

    let enc_bytes = read_file_from_zip(path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc_bytes)
        .map_err(|_| "解密失败：密码错误或文件已损坏".to_string())?;

    let payload: serde_json::Value =
        serde_json::from_slice(&decrypted).map_err(|e| format!("解析负载失败: {}", e))?;

    let package_ids = build_package_ids(&payload);

    // ── 模板快照导入（内容哈希隔离） ──────────────────────────
    let mut template_id_map: HashMap<String, String> = HashMap::new();
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(templates) = payload["templates"].as_array() {
        for tpl_val in templates {
            match serde_json::from_value::<solosoul_core::UserTemplate>(tpl_val.clone()) {
                Ok(mut tpl) => {
                    let original_id = tpl.id.clone();
                    let hash = solosoul_core::export_import::user_template_content_hash(&tpl);
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

                    template_id_map.insert(original_id, imported_id);
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

    let objects = payload["objects"].as_array().cloned().unwrap_or_default();
    let mut imported = 0usize;
    let mut imported_object_ids: HashSet<String> = HashSet::new();

    for obj_val in &objects {
        let id = obj_val["id"].as_str().unwrap_or("");
        if id.is_empty() {
            continue;
        }

        let existing = vault.load_object(id).ok().flatten();
        match strategy {
            ImportStrategy::SkipExisting => {
                if existing.is_some_and(|e| !e.is_deleted) {
                    continue;
                }
            }
            ImportStrategy::Overwrite | ImportStrategy::Merge => { /* 覆盖/合并均按覆盖处理 */
            }
        }

        let mut properties = obj_val["properties"].clone();
        resolve_cross_scope_references(&mut properties, &package_ids);

        let record = ObjectRecord {
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
            property_labels: if obj_val["property_labels"].is_null() {
                None
            } else {
                Some(obj_val["property_labels"].clone())
            },
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
            contract_type_id: obj_val["contract_type_id"].as_str().map(String::from),
            template_id: obj_val["template_id"].as_str().map(|tid| {
                template_id_map
                    .get(tid)
                    .cloned()
                    .unwrap_or_else(|| tid.to_string())
            }),
            template_type: obj_val["template_type"].as_str().map(String::from),
            created_at: obj_val["created_at"].as_str().unwrap_or(&now).to_string(),
            updated_at: now.clone(),
            version: obj_val["version"].as_u64().unwrap_or(1) as u32,
        };

        vault
            .save_object(&record)
            .map_err(|e| format!("保存对象失败: {}", e))?;
        imported += 1;
        imported_object_ids.insert(id.to_string());
    }

    // 导入附件（P118: 传入已解密的 payload 避免二次解密）。
    if manifest.has_attachments {
        import_attachments(app, path, &key, &salt, &imported_object_ids, &payload)?;
    }

    // 导入偏好设置。
    if manifest
        .extra_files
        .contains(&"preferences.enc".to_string())
    {
        import_preferences(&vault, &account_id, &key, &salt, path)?;
    }

    let _ = vault.log_structured(
        "import_execute",
        "import",
        None,
        None,
        "user",
        Some(&format!(
            "imported {} objects from {} (strategy: {:?})",
            imported,
            path.display(),
            strategy
        )),
    );

    app.error_message = Some(format!("成功导入 {} 个对象", imported));
    Ok(())
}

/// 导入附件到 vault 存储目录。
///
/// P118: 接受已解密的 payload，避免二次解密。
fn import_attachments(
    app: &mut App,
    path: &Path,
    key: &[u8; 32],
    salt: &[u8],
    imported_object_ids: &HashSet<String>,
    payload: &serde_json::Value,
) -> Result<(), CliError> {
    let vault = app
        .vault_service
        .get_vault_store()
        .ok_or_else(|| "Vault 未打开".to_string())?;
    let base = app.vault_service.base_path().to_path_buf();

    let att_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:attachments:v1")
        .map_err(|e| format!("派生附件密钥失败: {}", e))?;

    let mut att_meta_map: HashMap<(String, String), AttachmentMeta> = HashMap::new();
    if let Some(arr) = payload["objects"].as_array() {
        for obj_val in arr {
            let obj_id = obj_val["id"].as_str().unwrap_or("");
            if obj_id.is_empty() {
                continue;
            }
            let props = obj_val["properties"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let atts = load_attachments(&serde_json::Value::Object(props));
            for att in &atts {
                att_meta_map.insert((obj_id.to_string(), att.id.clone()), att.clone());
            }
        }
    }

    let zip_file = File::open(path).map_err(|e| format!("打开导入包失败: {}", e))?;
    let mut archive = ZipArchive::new(zip_file).map_err(|_| "无效的导入包".to_string())?;
    let att_prefix = "attachments/";

    let mut imported_atts: HashMap<String, Vec<AttachmentMeta>> = HashMap::new();

    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_string();
        if !name.starts_with(att_prefix) || name.ends_with('/') {
            continue;
        }
        let rel = &name[att_prefix.len()..];
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

        if !imported_object_ids.contains(obj_id) {
            continue;
        }

        let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
            Some(m) => m,
            None => continue,
        };

        // 使用流式解密避免完整密文和明文同时驻留内存 — P1-024。
        let new_att_id = generate_id();
        let dest = base.join("attachments").join(obj_id).join(&new_att_id);
        std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

        let safe_name = Path::new(&old_meta.file_name)
            .file_name()
            .ok_or("附件文件名无效")?
            .to_string_lossy()
            .to_string();
        let file_path_dest = dest.join(&safe_name);
        let mut out_file =
            File::create(&file_path_dest).map_err(|e| format!("创建附件文件失败: {}", e))?;
        solosoul_crypto::cipher::decrypt_chunked_stream(&att_key, &mut f, &mut out_file)
            .map_err(|e| format!("解密附件流失败: {}", e))?;
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
                created_at: chrono::Utc::now().to_rfc3339(),
                deleted_at: None,
                src_path: Some(file_path_dest.to_string_lossy().to_string()),
                vault_path: Some(file_path_dest.to_string_lossy().to_string()),
            });
    }

    // 更新已导入对象的 __attachments。
    for (obj_id, atts) in imported_atts {
        let mut obj = vault
            .load_object(&obj_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("找不到对象 {}", obj_id))?;
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

    Ok(())
}

/// 导入偏好设置。
fn import_preferences(
    vault: &VaultStore,
    account_id: &str,
    key: &[u8; 32],
    salt: &[u8],
    path: &Path,
) -> Result<(), CliError> {
    let prefs_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:preferences:v1")
            .map_err(|e| format!("派生偏好设置密钥失败: {}", e))?;
    let prefs_enc = read_file_from_zip(path, "preferences.enc")?;
    let prefs_dec = solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None)
        .map_err(|_| "解密偏好设置失败".to_string())?;
    let profile = Profile::new_with_id(account_id, account_id, prefs_dec.to_vec());
    let _ = vault.save_profile(&profile);
    Ok(())
}

/// 构建包内所有对象 ID 集合，用于跨范围关系引用降级。
fn build_package_ids(payload: &serde_json::Value) -> HashSet<String> {
    payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// 递归扫描 JSON 值，将指向包外对象的关系引用降级为文本备注。
fn resolve_value_references(value: &mut serde_json::Value, package_ids: &HashSet<String>) {
    match value {
        serde_json::Value::Object(obj) => {
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
                        *value = serde_json::Value::String(format!("[引用对象未导出: {}]", tid));
                        return;
                    }
                }
            }
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

/// 扫描对象属性并降级跨范围关系引用。
fn resolve_cross_scope_references(
    properties: &mut serde_json::Value,
    package_ids: &HashSet<String>,
) {
    if let Some(map) = properties.as_object_mut() {
        for (_key, value) in map.iter_mut() {
            resolve_value_references(value, package_ids);
        }
    }
}

/// 读取 ZIP 中的 manifest.json。
fn read_manifest(path: &Path) -> Result<ManifestData, CliError> {
    if !path.exists() {
        return Err(CliError::Msg(format!("文件不存在: {}", path.display())));
    }
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "无效的 ZIP 包".to_string())?;

    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| "缺少 manifest.json".to_string())?;
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取 manifest 失败: {}", e))?;
    let s = String::from_utf8_lossy(&buf);
    let v: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| format!("manifest JSON 无效: {}", e))?;

    let extra_files: Vec<String> = v
        .get("extra_files")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ManifestData {
        salt_hex: v["salt_hex"].as_str().ok_or("缺少 salt_hex")?.to_string(),
        has_attachments: v["has_attachments"].as_bool().unwrap_or(false),
        extra_files,
        version: v["version"].as_str().unwrap_or("1.0").to_string(),
        object_count: v["object_count"].as_u64().unwrap_or(0) as usize,
        password_hint: v
            .get("password_hint")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    })
}

/// ZIP 条目的最大解压大小限制（100 MB），防止 ZIP 炸弹 / OOM。
const MAX_ZIP_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// 从 ZIP 中读取指定名称的文件内容，带大小限制。
///
/// # 安全
/// - 读取前检查 `entry.size()`，超过限制直接拒绝。
/// - 使用 `.take()` 做第二道防线。
fn read_file_from_zip(path: &Path, name: &str) -> Result<Vec<u8>, CliError> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "无效的 ZIP 包".to_string())?;
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("ZIP 中缺少: {}", name))?;

    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(CliError::Msg(format!(
            "ZIP 条目 '{}' 过大 ({} 字节, 上限 {} 字节)",
            name,
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        )));
    }

    let mut buf = Vec::new();
    entry
        .take(MAX_ZIP_ENTRY_SIZE + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取 {} 失败: {}", name, e))?;
    Ok(buf)
}

/// 生成新的唯一 ID。
fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::VaultService;
    use std::sync::Arc;

    fn unlocked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault
            .create_account("Test", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    fn make_test_record(account_id: &str, id: &str, name: &str) -> ObjectRecord {
        ObjectRecord {
            id: id.to_string(),
            account_id: account_id.to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: name.to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"title": "hello"}),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        }
    }

    #[test]
    fn test_export_full_creates_valid_package() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = std::env::temp_dir().join("test_export_full.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_execute(&mut app, crate::TEST_EXPORT_PASSWORD, &path, &scope).unwrap();

        assert!(path.exists());
        let manifest = read_manifest(&path).unwrap();
        assert_eq!(manifest.object_count, 1);
        assert_eq!(manifest.version, "2.0");
    }

    #[test]
    fn test_import_preview_shows_count_and_hint() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = std::env::temp_dir().join("test_import_preview.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_execute(&mut app, crate::TEST_EXPORT_PASSWORD, &path, &scope).unwrap();

        let preview = import_preview(&path).unwrap();
        assert!(preview.contains("对象数 1"), "preview: {}", preview);
        assert!(preview.contains("版本 2.0"), "preview: {}", preview);
    }

    #[test]
    fn test_import_with_correct_password_restores_objects() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = std::env::temp_dir().join("test_import_restore.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_execute(&mut app, crate::TEST_EXPORT_PASSWORD, &path, &scope).unwrap();

        // 删除本地对象后重新导入。
        vault.delete_object("obj_1", false).unwrap();
        assert!(vault.load_object("obj_1").unwrap().is_none());

        import_execute(
            &mut app,
            &path,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();
        let restored = vault.load_object("obj_1").unwrap().unwrap();
        assert_eq!(restored.name, "Test Object");
        assert_eq!(restored.properties["title"].as_str().unwrap(), "hello");
    }

    #[test]
    fn test_import_with_wrong_password_fails() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = std::env::temp_dir().join("test_import_wrong.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_execute(&mut app, crate::TEST_EXPORT_PASSWORD, &path, &scope).unwrap();

        let result = import_execute(&mut app, &path, "WrongPass1", ImportStrategy::Overwrite);
        assert!(result.is_err(), "应返回密码错误: {:?}", result);
    }

    #[test]
    fn test_cli_export_includes_templates_in_payload() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 创建模板
        let tpl = solosoul_core::UserTemplate {
            contract_type_id: None,
            id: "passport_tpl".to_string(),
            account_id: account_id.clone(),
            name: "护照信息".to_string(),
            icon_id: Some("passport".to_string()),
            properties: vec![solosoul_core::TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: "姓名".to_string(),
                prop_type: solosoul_core::PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
            }],
            category: Some("travel".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        };
        vault.save_user_template(&tpl).unwrap();

        // 创建引用模板的对象
        let mut rec = make_test_record(&account_id, "obj_tpl", "My Passport");
        rec.template_id = Some("passport_tpl".to_string());
        vault.save_object(&rec).unwrap();

        let path = std::env::temp_dir().join("test_cli_export_tmpl.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_execute(&mut app, crate::TEST_EXPORT_PASSWORD, &path, &scope).unwrap();

        // 验证导出的 payload 包含 templates
        let manifest = read_manifest(&path).unwrap();
        let enc = read_file_from_zip(&path, "payload.enc").unwrap();
        let salt = hex::decode(&manifest.salt_hex).unwrap();
        let key = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt).unwrap();
        let dec = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&dec).unwrap();

        let tmpls = payload["templates"].as_array().unwrap();
        assert!(!tmpls.is_empty(), "payload 中应包含 templates");
        let has_passport = tmpls.iter().any(|t| t["name"].as_str() == Some("护照信息"));
        assert!(has_passport, "应包含护照模板");

        // 验证对象携带 template_id
        let objs = payload["objects"].as_array().unwrap();
        assert!(objs.iter().any(|o| o["template_id"] == "passport_tpl"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cli_import_remaps_template_id() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 先创建目标账户自己的同名模板（英文版 — 模拟跨语言场景）
        let existing_tpl = solosoul_core::UserTemplate {
            contract_type_id: None,
            id: "passport_tpl".to_string(),
            account_id: account_id.clone(),
            name: "Passport Info".to_string(),
            icon_id: Some("passport".to_string()),
            properties: vec![solosoul_core::TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: "Full Name".to_string(),
                prop_type: solosoul_core::PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
            }],
            category: Some("travel".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        };
        vault.save_user_template(&existing_tpl).unwrap();

        // 导出含中文模板的包
        let path = std::env::temp_dir().join("test_cli_import_remap.solosoul");
        let _ = std::fs::remove_file(&path);
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_cn",
                "name": "我的护照",
                "account_id": account_id,
                "type_id": "note",
                "section_type": "travel",
                "icon_name": "passport",
                "properties": {"fullName": "张三"},
                "sensitivity_level": "internal",
                "tags": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z",
                "version": 1,
                "template_id": "passport_tpl"
            }],
            "templates": [{
                "id": "passport_tpl",
                "accountId": "acc_export",
                "name": "护照信息",
                "iconId": "passport",
                "properties": [{
                    "id": "fullName",
                    "name": "姓名",
                    "type": "text",
                    "sensitivityLevel": "internal"
                }],
                "category": "travel",
                "createdAt": "2024-01-01T00:00:00Z",
                "updatedAt": null
            }]
        });

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let manifest = serde_json::json!({
            "version": "2.0",
            "salt_hex": hex::encode(salt),
            "has_attachments": false,
            "has_templates": true,
            "extra_files": []
        });
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();

        zip.start_file("payload.enc", options).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload_bytes),
            &mut zip,
        )
        .unwrap();
        zip.finish().unwrap();

        // 导入到英文账户
        import_execute(
            &mut app,
            &path,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();

        // 验证对象 template_id 被重定向（不再是原始 "passport_tpl"）
        let imported = vault.load_object("obj_cn").unwrap().unwrap();
        let tid = imported.template_id.unwrap();
        assert!(
            tid.starts_with("imported:"),
            "template_id 应被重定向到 imported:..., 实际为: {}",
            tid
        );
        assert!(tid.ends_with(":passport_tpl"), "应保留原始 ID 后缀");

        // 验证快照模板数据导入且名称为中文
        let snapshot_tpl = vault.load_user_template(&tid).unwrap().unwrap();
        assert_eq!(snapshot_tpl.name, "护照信息");
        assert_eq!(snapshot_tpl.properties[0].name, "姓名");

        // 验证原始英文模板未被覆盖
        let orig_tpl = vault.load_user_template("passport_tpl").unwrap().unwrap();
        assert_eq!(orig_tpl.name, "Passport Info");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cli_import_old_package_without_templates() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 创建旧格式包（没有 templates 数组）
        let path = std::env::temp_dir().join("test_cli_import_old.solosoul");
        let _ = std::fs::remove_file(&path);
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_old",
                "name": "Old Object",
                "account_id": account_id,
                "type_id": "note",
                "section_type": "identity",
                "icon_name": "document",
                "properties": {},
                "sensitivity_level": "internal",
                "tags": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z",
                "version": 1
            }]
            // 没有 "templates" 字段
        });

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let manifest = serde_json::json!({
            "version": "1.0",
            "salt_hex": hex::encode(salt),
            "has_attachments": false,
            "extra_files": []
        });
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();

        zip.start_file("payload.enc", options).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload_bytes),
            &mut zip,
        )
        .unwrap();
        zip.finish().unwrap();

        // 导入 — 应正常工作（无 templates 的旧包兼容）
        import_execute(
            &mut app,
            &path,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();
        let imported = vault.load_object("obj_old").unwrap().unwrap();
        assert_eq!(imported.name, "Old Object");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cli_import_reuses_snapshot_template() {
        // 同一模板内容导入两次，应只创建一个快照模板
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        // 构造含中文模板的包
        let path = std::env::temp_dir().join("test_cli_import_dedup.solosoul");
        let _ = std::fs::remove_file(&path);
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_1",
                "name": "对象一",
                "account_id": account_id,
                "type_id": "note",
                "section_type": "identity",
                "icon_name": "document",
                "properties": {},
                "sensitivity_level": "internal",
                "tags": [],
                "template_id": "chinese_tpl",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z",
                "version": 1
            }],
            "templates": [{
                "id": "chinese_tpl",
                "accountId": "acc_export",
                "name": "中文模板",
                "iconId": null,
                "properties": [{
                    "id": "f1",
                    "name": "字段一",
                    "type": "text",
                    "sensitivityLevel": "internal"
                }],
                "category": null,
                "createdAt": "2024-01-01T00:00:00Z",
                "updatedAt": null
            }]
        });

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let manifest = serde_json::json!({
            "version": "2.0", "salt_hex": hex::encode(salt),
            "has_attachments": false, "has_templates": true, "extra_files": []
        });
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file("payload.enc", options).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload_bytes),
            &mut zip,
        )
        .unwrap();
        zip.finish().unwrap();

        // 第一次导入 — 创建快照模板
        import_execute(
            &mut app,
            &path,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();
        let imported_1 = vault.load_object("obj_1").unwrap().unwrap();
        let snapshot_id = imported_1.template_id.unwrap();
        assert!(snapshot_id.starts_with("imported:"));

        // 统计当前快照模板数量
        let all_templates = vault.list_user_templates(&account_id).unwrap();
        let snapshot_count_before = all_templates
            .iter()
            .filter(|t| t.id.starts_with("imported:"))
            .count();

        // 第二次导入 — 使用不同的对象 ID 但同一模板内容
        let payload2 = serde_json::json!({
            "objects": [{
                "id": "obj_2",
                "name": "对象二",
                "account_id": account_id,
                "type_id": "note",
                "section_type": "identity",
                "icon_name": "document",
                "properties": {},
                "sensitivity_level": "internal",
                "tags": [],
                "template_id": "chinese_tpl",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z",
                "version": 1
            }],
            "templates": [{
                "id": "chinese_tpl",
                "accountId": "acc_export",
                "name": "中文模板",
                "iconId": null,
                "properties": [{
                    "id": "f1",
                    "name": "字段一",
                    "type": "text",
                    "sensitivityLevel": "internal"
                }],
                "category": null,
                "createdAt": "2024-01-01T00:00:00Z",
                "updatedAt": null
            }]
        });
        let salt2 = solosoul_crypto::kdf::generate_salt();
        let key2 = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt2).unwrap();
        let path2 = std::env::temp_dir().join("test_cli_import_dedup_2.solosoul");
        let _ = std::fs::remove_file(&path2);
        let payload2_bytes = serde_json::to_vec(&payload2).unwrap();
        let f2 = File::create(&path2).unwrap();
        let mut z2 = zip::ZipWriter::new(f2);
        let m2 = serde_json::json!({
            "version": "2.0", "salt_hex": hex::encode(salt2),
            "has_attachments": false, "has_templates": true, "extra_files": []
        });
        z2.start_file("manifest.json", options).unwrap();
        z2.write_all(m2.to_string().as_bytes()).unwrap();
        z2.start_file("payload.enc", options).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key2,
            payload2_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload2_bytes),
            &mut z2,
        )
        .unwrap();
        z2.finish().unwrap();

        import_execute(
            &mut app,
            &path2,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();

        // 验证第二个对象指向同一个快照模板 ID
        let imported_2 = vault.load_object("obj_2").unwrap().unwrap();
        assert_eq!(
            imported_2.template_id.unwrap(),
            snapshot_id,
            "同一模板内容的两次导入应复用同一个快照模板 ID"
        );

        // 验证快照模板数量没有增加
        let all_templates_after = vault.list_user_templates(&account_id).unwrap();
        let snapshot_count_after = all_templates_after
            .iter()
            .filter(|t| t.id.starts_with("imported:"))
            .count();
        assert_eq!(
            snapshot_count_after, snapshot_count_before,
            "同样内容的模板不应产生新的快照模板: 之前={}, 之后={}",
            snapshot_count_before, snapshot_count_after
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&path2);
    }

    #[test]
    fn test_cli_import_custom_page_name_preserved() {
        // 自定义页面对象名称应在导入后保持原样
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();

        let path = std::env::temp_dir().join("test_cli_import_custom_page.solosoul");
        let _ = std::fs::remove_file(&path);
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(crate::TEST_EXPORT_PASSWORD, &salt).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "custom_page_1",
                "name": "我的中文页面",
                "account_id": account_id,
                "type_id": "page",
                "section_type": "custom_page_1",
                "icon_name": "folder",
                "properties": {},
                "sensitivity_level": "internal",
                "tags": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z",
                "version": 1
            }]
        });

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let manifest = serde_json::json!({
            "version": "2.0", "salt_hex": hex::encode(salt),
            "has_attachments": false, "has_templates": false, "extra_files": []
        });
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file("payload.enc", options).unwrap();
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload_bytes),
            &mut zip,
        )
        .unwrap();
        zip.finish().unwrap();

        import_execute(
            &mut app,
            &path,
            crate::TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
        )
        .unwrap();

        let imported = vault.load_object("custom_page_1").unwrap().unwrap();
        assert_eq!(imported.name, "我的中文页面", "自定义页面名称应保持原样");
        assert_eq!(imported.type_id, "page");
        assert_eq!(imported.section_type, "custom_page_1");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_password_same_as_master_rejected() {
        let (mut app, account_id, _dir) = unlocked_app();
        let vault = app.vault_service.get_vault_store().unwrap();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = std::env::temp_dir().join("test_export_same_master.solosoul");
        let _ = std::fs::remove_file(&path);
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        let result = export_execute(&mut app, crate::TEST_PASSWORD, &path, &scope);
        assert!(
            result.is_err(),
            "应拒绝与主密码相同的导出密码: {:?}",
            result
        );
        assert!(!path.exists());
    }
}
