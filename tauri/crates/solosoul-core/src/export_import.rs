//! Export/Import orchestration — core business logic shared by CLI and GUI.
//!
//! Provides high-level `export_vault()` and `import_vault()` functions
//! that handle the full `.solosoul` package format:
//! - Password derivation via Argon2id → AES-256-GCM
//! - ZIP packaging with manifest.json + payload.enc + optional attachments
//! - Template snapshot management with content-hash dedup
//! - Cross-scope reference resolution
//!
//! ## Architecture
//!
//! The CLI `/export` and `/import` commands as well as the Tauri export/import
//! dialogs share this module's `export_vault()` / `import_vault()` functions.
//! Each host only needs to handle argument parsing and user-facing prompts;
//! all encryption, packaging, and storage logic lives here.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use solosoul_vault::{ObjectRecord, UserTemplate, VaultStore};

// ── Constants ────────────────────────────────────────────

/// 导出包大小限制（与 GUI 一致）。
const MAX_ATTACHMENT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB
const MAX_EXPORT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB
/// ZIP 条目的最大解压大小限制（100 MB），防止 ZIP 炸弹 / OOM。
const MAX_ZIP_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// 附件 ID 与对象 ID 允许使用的字符集，防止路径遍历（P002）。
/// 与 `tauri/src-tauri/src/commands/attachment.rs` 的 `validate_attachment_id` 保持一致。
fn validate_import_id(id: &str) -> Result<(), ExportError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ExportError::Msg(format!(
            "无效的对象/附件 ID（仅允许字母数字与 -_）: {}",
            id
        )));
    }
    Ok(())
}

// ── Existing helper functions ──────────────────────────────

/// 计算 UserTemplate 的内容哈希，用于判断是否是同一份"快照模板"。
/// 忽略 account_id、id、created_at、updated_at 等随账户/时间变化的字段。
/// 重新导出 solosoul-vault 的模板哈希函数（单⼀真理来源）。
pub use solosoul_vault::template_hash::{imported_template_id, user_template_content_hash};

// ── Public types ─────────────────────────────────────────

/// 附件元数据（与 GUI `attachment::AttachmentMeta` 一致，camelCase 序列化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
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
pub struct ExportScope {
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
pub enum ImportStrategy {
    /// 跳过已存在的对象（保留本地）。
    SkipExisting,
    /// 覆盖本地对象。
    Overwrite,
    /// 合并：暂按覆盖实现。
    Merge,
}

/// 导出/导入错误类型。
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// 通用错误消息。
    #[error("{0}")]
    Msg(String),

    /// IO 错误。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化/反序列化错误。
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// ZIP 操作错误。
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// 加密操作错误。
    #[error("Cryptography error: {0}")]
    Crypto(String),

    /// 密码错误/文件损坏。
    #[error("解密失败：密码错误或文件已损坏")]
    DecryptionFailed,
}

impl From<String> for ExportError {
    fn from(s: String) -> Self {
        ExportError::Msg(s)
    }
}

impl From<&str> for ExportError {
    fn from(s: &str) -> Self {
        ExportError::Msg(s.to_string())
    }
}

/// 导入预览信息。
#[derive(Debug, Clone)]
pub struct ImportPreviewData {
    pub version: String,
    pub object_count: usize,
    pub has_attachments: bool,
    pub password_hint: Option<String>,
}

// ── Internal types ───────────────────────────────────────

/// 从 manifest.json 解析出的必要字段。
struct ManifestData {
    pub salt_hex: String,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
    pub version: String,
    pub object_count: usize,
    pub password_hint: Option<String>,
}

// ════════════════════════════════════════════════════════════════
// Public API
// ════════════════════════════════════════════════════════════════

/// 执行导出操作。
///
/// # 参数
///
/// * `vault` — 已解锁的 VaultStore
/// * `account_id` — 当前账户 ID
/// * `password` — 导出密码（至少 8 位，混合字母数字）
/// * `path` — 输出文件路径
/// * `scope` — 导出范围
/// * `base_path` — Vault 基础路径（用于附件存储解析）
pub fn export_vault(
    vault: &VaultStore,
    account_id: &str,
    password: &str,
    path: &Path,
    scope: &ExportScope,
    base_path: &Path,
) -> Result<usize, ExportError> {
    let records = collect_scope_objects(vault, account_id, scope)?;
    if records.is_empty() {
        return Err(ExportError::Msg("没有选中任何对象".to_string()));
    }

    let payload = build_payload(vault, &records);
    let payload_bytes =
        serde_json::to_vec(&payload).map_err(|e| format!("序列化负载失败: {}", e))?;

    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(password, &salt)?;

    // 收集附件源文件。
    let attachment_entries = collect_attachment_entries(base_path, &records, scope)?;

    let payload_estimate = payload_bytes.len() as u64;
    let total_attachment_bytes: u64 = attachment_entries
        .iter()
        .map(|(_, _, _, src)| std::fs::metadata(src).map(|m| m.len()).unwrap_or(0))
        .sum();
    let total_export_estimate =
        payload_estimate + total_attachment_bytes + (attachment_entries.len() as u64 * 28);
    if total_export_estimate > MAX_EXPORT_TOTAL_BYTES {
        return Err(ExportError::Msg("导出包总大小超过限制".to_string()));
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
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 写入附件（流式加密避免完整明文和密文同时驻留内存）。
    if let Some(ref ak) = att_key {
        for (obj_id, att_id, _file_name, src_path) in &attachment_entries {
            let file_size = std::fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
            let zip_name = format!("attachments/{}/{}.enc", obj_id, att_id);
            zip.start_file(&zip_name, options)
                .map_err(|e| format!("写入 ZIP 附件条目失败: {}", e))?;
            let mut f = File::open(src_path)?;
            let mut reader = std::io::BufReader::new(&mut f);
            solosoul_crypto::cipher::encrypt_chunked_stream(ak, file_size, &mut reader, &mut zip)
                .map_err(|e| format!("加密附件失败: {}", e))?;
        }
    }

    // manifest.json
    let has_templates = payload["templates"]
        .as_array()
        .is_some_and(|a| !a.is_empty());
    let manifest = build_manifest(scope, &records, att_key.is_some(), has_templates, &salt);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    zip.start_file("manifest.json", options)
        .map_err(|e| format!("写入 manifest 条目失败: {}", e))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| format!("写入 manifest 数据失败: {}", e))?;

    // payload.enc（流式加密）
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

    zip.finish()?;

    // 审计日志
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

    Ok(records.len())
}

/// 执行导入操作。
///
/// # 参数
///
/// * `vault` — 已解锁的 VaultStore
/// * `account_id` — 当前账户 ID
/// * `path` — 导入包文件路径
/// * `password` — 导入密码
/// * `strategy` — 冲突处理策略
/// * `base_path` — Vault 基础路径（用于附件存储）
pub fn import_vault(
    vault: &VaultStore,
    account_id: &str,
    path: &Path,
    password: &str,
    strategy: ImportStrategy,
    base_path: &Path,
) -> Result<usize, ExportError> {
    if password.is_empty() {
        return Err(ExportError::Msg("导入密码不能为空".to_string()));
    }

    let manifest = read_manifest(path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("salt 解码失败: {}", e))?;
    let key = derive_export_key(password, &salt)?;

    let enc_bytes = read_file_from_zip(path, "payload.enc")?;
    let decrypted = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc_bytes)
        .map_err(|_| ExportError::DecryptionFailed)?;

    let payload: serde_json::Value = serde_json::from_slice(&decrypted)?;
    let package_ids = build_package_ids(&payload);

    // ── 模板快照导入（内容哈希隔离） ────────
    let mut template_id_map: HashMap<String, String> = HashMap::new();
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(templates) = payload["templates"].as_array() {
        for tpl_val in templates {
            match serde_json::from_value::<UserTemplate>(tpl_val.clone()) {
                Ok(mut tpl) => {
                    let original_id = tpl.id.clone();
                    let hash = user_template_content_hash(&tpl);

                    // 去重：检查是否有完全一致的已有模板（含系统预置模板）
                    let local_id = if let Some(existing) =
                        vault.find_user_template_by_content_hash(account_id, &hash)?
                    {
                        existing.id
                    } else {
                        let imported_id = imported_template_id(&original_id, &hash);
                        if vault
                            .load_user_template(&imported_id)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            tpl.id = imported_id.clone();
                            tpl.account_id = account_id.to_string();
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
            ImportStrategy::Overwrite | ImportStrategy::Merge => { /* 覆盖 */ }
        }

        let mut properties = obj_val["properties"].clone();
        resolve_cross_scope_references(&mut properties, &package_ids);

        let record = ObjectRecord {
            id: id.to_string(),
            account_id: account_id.to_string(),
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
            template_hash: obj_val["template_hash"].as_str().map(String::from),
            ignored_template_hash: obj_val["ignored_template_hash"].as_str().map(String::from),
            created_at: obj_val["created_at"].as_str().unwrap_or(&now).to_string(),
            updated_at: now.clone(),
            version: obj_val["version"].as_u64().unwrap_or(1) as u32,
        };

        vault.save_object(&record)?;
        imported += 1;
        imported_object_ids.insert(id.to_string());
    }

    // 导入附件。
    if manifest.has_attachments {
        import_attachments(
            vault,
            base_path,
            path,
            &key,
            &salt,
            &imported_object_ids,
            &payload,
        )?;
    }

    // 导入偏好设置。
    if manifest
        .extra_files
        .contains(&"preferences.enc".to_string())
    {
        import_preferences(vault, account_id, &key, &salt, path)?;
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

    Ok(imported)
}

/// 读取导入包预览信息（无需解锁）。
pub fn import_preview(path: &Path) -> Result<ImportPreviewData, ExportError> {
    if !path.exists() {
        return Err(ExportError::Msg(format!("文件不存在: {}", path.display())));
    }
    let manifest = read_manifest(path)?;
    Ok(ImportPreviewData {
        version: manifest.version,
        object_count: manifest.object_count,
        has_attachments: manifest.has_attachments,
        password_hint: manifest.password_hint,
    })
}

// ════════════════════════════════════════════════════════════════
// Internal helpers
// ════════════════════════════════════════════════════════════════

/// 使用 Argon2id 从导出密码与 salt 派生 32 字节密钥。
fn derive_export_key(password: &str, salt: &[u8]) -> Result<[u8; 32], ExportError> {
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
) -> Result<Vec<ObjectRecord>, ExportError> {
    let all = vault.list_objects(account_id, None, None, None, false, false)?;

    let ids: Vec<String> = if scope.full {
        all.iter().map(|s| s.id.clone()).collect()
    } else {
        let mut selected_ids: HashSet<String> = scope.selected_object_ids.iter().cloned().collect();
        for summary in &all {
            if scope.selected_page_ids.contains(&summary.section_type) {
                selected_ids.insert(summary.id.clone());
            }
        }
        selected_ids.into_iter().collect()
    };

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
) -> Result<Vec<(String, String, String, PathBuf)>, ExportError> {
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
                return Err(ExportError::Msg(format!("附件过大: {}", att.file_name)));
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
        "has_attachments": has_attachments,
        "has_preferences": false,
        "has_behavioral": false,
        "has_templates": has_templates,
        "extra_files": [],
        "password_hint": "",
        "salt_hex": hex::encode(salt),
    })
}

/// 读取 ZIP 中的 manifest.json。
fn read_manifest(path: &Path) -> Result<ManifestData, ExportError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| "缺少 manifest.json".to_string())?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
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

/// 从 ZIP 中读取指定名称的文件内容，带大小限制。
fn read_file_from_zip(path: &Path, name: &str) -> Result<Vec<u8>, ExportError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("ZIP 中缺少: {}", name))?;

    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(ExportError::Msg(format!(
            "ZIP 条目 '{}' 过大 ({} 字节, 上限 {} 字节)",
            name,
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        )));
    }

    let mut buf = Vec::new();
    entry.take(MAX_ZIP_ENTRY_SIZE + 1).read_to_end(&mut buf)?;
    Ok(buf)
}

/// 构建包内所有对象 ID 集合。
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

/// 导入附件到 vault 存储目录。
fn import_attachments(
    vault: &VaultStore,
    base_path: &Path,
    path: &Path,
    key: &[u8; 32],
    salt: &[u8],
    imported_object_ids: &HashSet<String>,
    payload: &serde_json::Value,
) -> Result<(), ExportError> {
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

    let zip_file = File::open(path)?;
    let mut archive = ZipArchive::new(zip_file)?;
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

        // P002：对象 ID 与附件 ID 来自攻击者可控的解密 payload，必须做字符集
        // 校验后才能用于 `base_path.join("attachments").join(obj_id)`，否则 Windows
        // 上 `..\..\evil` 可写出 Vault 目录（路径遍历）。
        validate_import_id(obj_id)?;
        validate_import_id(old_att_id)?;

        if !imported_object_ids.contains(obj_id) {
            continue;
        }

        let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
            Some(m) => m,
            None => continue,
        };

        let new_att_id = uuid::Uuid::new_v4().to_string();
        let dest = base_path.join("attachments").join(obj_id).join(&new_att_id);
        std::fs::create_dir_all(&dest)?;

        // P003：落盘文件名取末段安全名，并**写回元数据**——此前元数据保留原始
        // `file_name`（如 `../../evil.txt`），后续插件主机 `copy_attachment_to_workspace`
        // 用原始名 join 目标目录造成存储型路径遍历。
        let safe_name = sanitize_import_file_name(&old_meta.file_name)?;
        let file_path_dest = dest.join(&safe_name);
        let mut out_file = File::create(&file_path_dest)?;
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
                file_name: safe_name.clone(),
                mime_type: old_meta.mime_type.clone(),
                size_bytes: file_size,
                created_at: chrono::Utc::now().to_rfc3339(),
                deleted_at: None,
                src_path: Some(file_path_dest.to_string_lossy().to_string()),
                vault_path: Some(file_path_dest.to_string_lossy().to_string()),
            });
    }

    // 更新已导入对象的 __attachments
    for (obj_id, atts) in imported_atts {
        let mut obj = vault
            .load_object(&obj_id)?
            .ok_or_else(|| format!("找不到对象 {}", obj_id))?;
        let att_json = serde_json::to_value(&atts)?;
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

/// 净化导入附件的文件名（P003）。
///
/// 显式拒绝包含路径分隔符的名字（`/` 与 `\\`）——Unix 上 `\\` 不是分隔符，
/// 仅靠 `Path::file_name()` 无法剥离 `..\\..\\evil.txt` 中的反斜杠，
/// 因此必须平台无关地拒绝，再取末段组件作为兜底。
fn sanitize_import_file_name(file_name: &str) -> Result<String, ExportError> {
    if file_name.contains('/') || file_name.contains('\\') {
        return Err(ExportError::Msg(format!("附件文件名无效: {}", file_name)));
    }
    let base = Path::new(file_name)
        .file_name()
        .ok_or("附件文件名无效")?
        .to_string_lossy()
        .to_string();
    if base.is_empty() || base == "." || base == ".." {
        return Err(ExportError::Msg(format!("附件文件名无效: {}", file_name)));
    }
    Ok(base)
}

/// 导入偏好设置。
fn import_preferences(
    vault: &VaultStore,
    account_id: &str,
    key: &[u8; 32],
    salt: &[u8],
    path: &Path,
) -> Result<(), ExportError> {
    let prefs_key =
        solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:preferences:v1")
            .map_err(|e| format!("派生偏好设置密钥失败: {}", e))?;
    let prefs_enc = read_file_from_zip(path, "preferences.enc")?;
    let prefs_dec = solosoul_crypto::cipher::decrypt_from_bytes(&prefs_key, &prefs_enc, None)
        .map_err(|_| "解密偏好设置失败".to_string())?;
    let profile = solosoul_vault::Profile::new_with_id(account_id, account_id, prefs_dec.to_vec());
    let _ = vault.save_profile(&profile);
    Ok(())
}

// ════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultService;

    /// 测试范围的全局锁，串行化涉及同一数据目录的测试。
    static CORE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const TEST_PASSWORD: &str = "password123";
    const TEST_EXPORT_PASSWORD: &str = "ExportPass1";

    fn test_setup() -> (std::sync::Arc<VaultStore>, String, tempfile::TempDir) {
        let _guard = CORE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        let vault = VaultService::with_base_path(dir.path().to_path_buf());
        let account = vault.create_account("Test", TEST_PASSWORD, None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let vault_store: std::sync::Arc<VaultStore> = vault.get_vault_store().unwrap();
        (vault_store, account_id, dir)
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
            template_hash: None,
            ignored_template_hash: None,
        }
    }

    #[test]
    fn test_export_full_creates_valid_package() {
        let (vault, account_id, dir) = test_setup();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = dir.path().join("test_export_full.solosoul");
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        let count = export_vault(
            &vault,
            &account_id,
            TEST_EXPORT_PASSWORD,
            &path,
            &scope,
            dir.path(),
        )
        .unwrap();
        assert_eq!(count, 1);
        assert!(path.exists());

        let preview = import_preview(&path).unwrap();
        assert_eq!(preview.object_count, 1);
        assert_eq!(preview.version, "2.0");
    }

    #[test]
    fn test_import_with_correct_password_restores_objects() {
        let (vault, account_id, dir) = test_setup();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = dir.path().join("test_import_restore.solosoul");
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_vault(
            &vault,
            &account_id,
            TEST_EXPORT_PASSWORD,
            &path,
            &scope,
            dir.path(),
        )
        .unwrap();

        // 删除本地对象后重新导入
        vault.delete_object("obj_1", false).unwrap();
        assert!(vault.load_object("obj_1").unwrap().is_none());

        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();
        assert_eq!(imported, 1);

        let restored = vault.load_object("obj_1").unwrap().unwrap();
        assert_eq!(restored.name, "Test Object");
        assert_eq!(restored.properties["title"].as_str().unwrap(), "hello");
    }

    #[test]
    fn test_import_with_wrong_password_fails() {
        let (vault, account_id, dir) = test_setup();
        vault
            .save_object(&make_test_record(&account_id, "obj_1", "Test Object"))
            .unwrap();

        let path = dir.path().join("test_import_wrong.solosoul");
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_vault(
            &vault,
            &account_id,
            TEST_EXPORT_PASSWORD,
            &path,
            &scope,
            dir.path(),
        )
        .unwrap();

        let result = import_vault(
            &vault,
            &account_id,
            &path,
            "WrongPass1",
            ImportStrategy::Overwrite,
            dir.path(),
        );
        assert!(result.is_err(), "应返回密码错误: {:?}", result);
    }

    #[test]
    fn test_export_includes_templates_in_payload() {
        let (vault, account_id, dir) = test_setup();

        // 创建模板
        let tpl = UserTemplate {
            contract_type_id: None,
            id: "passport_tpl".to_string(),
            account_id: account_id.clone(),
            name: "护照信息".to_string(),
            icon_id: Some("passport".to_string()),
            properties: vec![solosoul_vault::TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: "姓名".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
                allowed_types: None,
                max_items: None,
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

        let path = dir.path().join("test_export_tmpl.solosoul");
        let scope = ExportScope {
            full: true,
            include_attachments: false,
            ..Default::default()
        };
        export_vault(
            &vault,
            &account_id,
            TEST_EXPORT_PASSWORD,
            &path,
            &scope,
            dir.path(),
        )
        .unwrap();

        // 验证导出的 payload 包含 templates
        let manifest = read_manifest(&path).unwrap();
        let enc = read_file_from_zip(&path, "payload.enc").unwrap();
        let salt = hex::decode(&manifest.salt_hex).unwrap();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();
        let dec = solosoul_crypto::cipher::decrypt_chunked_from_bytes(&key, &enc).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&dec).unwrap();

        let tmpls = payload["templates"].as_array().unwrap();
        assert!(!tmpls.is_empty(), "payload 中应包含 templates");
        let has_passport = tmpls.iter().any(|t| t["name"].as_str() == Some("护照信息"));
        assert!(has_passport, "应包含护照模板");

        let objs = payload["objects"].as_array().unwrap();
        assert!(objs.iter().any(|o| o["template_id"] == "passport_tpl"));
    }

    #[test]
    fn test_import_remaps_template_id() {
        let (vault, account_id, dir) = test_setup();

        // 先创建目标账户自己的同名模板（英文版）
        let existing_tpl = UserTemplate {
            contract_type_id: None,
            id: "passport_tpl".to_string(),
            account_id: account_id.clone(),
            name: "Passport Info".to_string(),
            icon_id: Some("passport".to_string()),
            properties: vec![solosoul_vault::TemplateProperty {
                contract_field: None,
                contract_bindings: None,
                id: "fullName".to_string(),
                name: "Full Name".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitivity_level: Some("internal".to_string()),
                options: None,
                sensitive: None,
                deprecated_at: None,
                allowed_types: None,
                max_items: None,
            }],
            category: Some("travel".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
        };
        vault.save_user_template(&existing_tpl).unwrap();

        // 手动构建含中文模板的导出包
        let path = dir.path().join("test_import_remap.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();

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

        write_test_package(&path, &payload, &key, &salt).unwrap();

        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();
        assert_eq!(imported, 1);

        let imported_obj = vault.load_object("obj_cn").unwrap().unwrap();
        let tid = imported_obj.template_id.unwrap();
        assert!(
            tid.starts_with("imported:"),
            "template_id 应被重定向到 imported:..., 实际为: {}",
            tid
        );
        assert!(tid.ends_with(":passport_tpl"), "应保留原始 ID 后缀");

        let snapshot_tpl = vault.load_user_template(&tid).unwrap().unwrap();
        assert_eq!(snapshot_tpl.name, "护照信息");

        let orig_tpl = vault.load_user_template("passport_tpl").unwrap().unwrap();
        assert_eq!(orig_tpl.name, "Passport Info");
    }

    #[test]
    fn test_import_old_package_without_templates() {
        let (vault, account_id, dir) = test_setup();

        let path = dir.path().join("test_import_old.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();

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
        });

        write_test_package(&path, &payload, &key, &salt).unwrap();

        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();
        assert_eq!(imported, 1);

        let imported_obj = vault.load_object("obj_old").unwrap().unwrap();
        assert_eq!(imported_obj.name, "Old Object");
    }

    #[test]
    fn test_import_reuses_snapshot_template() {
        let (vault, account_id, dir) = test_setup();

        // 第一次导入
        let path = dir.path().join("test_dedup_1.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_1", "name": "对象一", "account_id": account_id,
                "type_id": "note", "section_type": "identity", "icon_name": "document",
                "properties": {}, "sensitivity_level": "internal", "tags": [],
                "template_id": "chinese_tpl",
                "created_at": "2024-01-01T00:00:00Z", "updated_at": "2024-06-01T00:00:00Z", "version": 1
            }],
            "templates": [{
                "id": "chinese_tpl", "accountId": "acc_export", "name": "中文模板", "iconId": null,
                "properties": [{"id": "f1", "name": "字段一", "type": "text", "sensitivityLevel": "internal"}],
                "category": null, "createdAt": "2024-01-01T00:00:00Z", "updatedAt": null
            }]
        });

        write_test_package(&path, &payload, &key, &salt).unwrap();
        import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();

        let imported_1 = vault.load_object("obj_1").unwrap().unwrap();
        let snapshot_id = imported_1.template_id.unwrap();
        assert!(snapshot_id.starts_with("imported:"));

        let all_before = vault.list_user_templates(&account_id).unwrap();
        let snapshot_count_before = all_before
            .iter()
            .filter(|t| t.id.starts_with("imported:"))
            .count();

        // 第二次导入 — 同一模板内容，不同对象
        let path2 = dir.path().join("test_dedup_2.solosoul");
        let salt2 = solosoul_crypto::kdf::generate_salt();
        let key2 = derive_export_key(TEST_EXPORT_PASSWORD, &salt2).unwrap();

        let payload2 = serde_json::json!({
            "objects": [{
                "id": "obj_2", "name": "对象二", "account_id": account_id,
                "type_id": "note", "section_type": "identity", "icon_name": "document",
                "properties": {}, "sensitivity_level": "internal", "tags": [],
                "template_id": "chinese_tpl",
                "created_at": "2024-01-01T00:00:00Z", "updated_at": "2024-06-01T00:00:00Z", "version": 1
            }],
            "templates": [{
                "id": "chinese_tpl", "accountId": "acc_export", "name": "中文模板", "iconId": null,
                "properties": [{"id": "f1", "name": "字段一", "type": "text", "sensitivityLevel": "internal"}],
                "category": null, "createdAt": "2024-01-01T00:00:00Z", "updatedAt": null
            }]
        });

        write_test_package(&path2, &payload2, &key2, &salt2).unwrap();
        import_vault(
            &vault,
            &account_id,
            &path2,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();

        let imported_2 = vault.load_object("obj_2").unwrap().unwrap();
        assert_eq!(
            imported_2.template_id.unwrap(),
            snapshot_id,
            "同一模板内容应复用同一个快照模板 ID"
        );

        let all_after = vault.list_user_templates(&account_id).unwrap();
        let snapshot_count_after = all_after
            .iter()
            .filter(|t| t.id.starts_with("imported:"))
            .count();
        assert_eq!(
            snapshot_count_after, snapshot_count_before,
            "同样内容的模板不应产生新的快照模板"
        );
    }

    #[test]
    fn test_import_custom_page_name_preserved() {
        let (vault, account_id, dir) = test_setup();

        let path = dir.path().join("test_custom_page.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();

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

        write_test_package(&path, &payload, &key, &salt).unwrap();
        import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
        )
        .unwrap();

        let imported = vault.load_object("custom_page_1").unwrap().unwrap();
        assert_eq!(imported.name, "我的中文页面");
        assert_eq!(imported.type_id, "page");
        assert_eq!(imported.section_type, "custom_page_1");
    }

    // ── 测试辅助 ──

    /// 手动构建一个导出的 .solosoul 包（用于测试导入各种场景）。
    fn write_test_package(
        path: &Path,
        payload: &serde_json::Value,
        key: &[u8; 32],
        salt: &[u8; 16],
    ) -> Result<(), ExportError> {
        let payload_bytes = serde_json::to_vec(payload)?;
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let manifest = serde_json::json!({
            "version": "2.0",
            "salt_hex": hex::encode(salt),
            "has_attachments": false,
            "has_templates": true,
            "extra_files": []
        });

        zip.start_file("manifest.json", options)
            .map_err(|e| format!("写入 manifest 条目失败: {}", e))?;
        zip.write_all(manifest.to_string().as_bytes())
            .map_err(|e| format!("写入 manifest 失败: {}", e))?;

        zip.start_file("payload.enc", options)
            .map_err(|e| format!("写入 payload 条目失败: {}", e))?;
        solosoul_crypto::cipher::encrypt_chunked_stream(
            key,
            payload_bytes.len() as u64,
            &mut std::io::Cursor::new(&payload_bytes),
            &mut zip,
        )
        .map_err(|e| format!("加密 payload 流失败: {}", e))?;

        zip.finish()?;
        Ok(())
    }
}
