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

use solosoul_crypto::kdf::KdfConfig;
use solosoul_vault::{ObjectRecord, UserTemplate, VaultStore};
use zeroize::Zeroizing;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// manifest 声明的 KDF 参数；`None` = 旧格式包（未声明），按 balanced 兜底。
    pub kdf: Option<KdfConfig>,
}

impl ManifestData {
    /// 用于解包/加密的 KDF 参数：manifest 声明优先，旧格式包回退 balanced（向后兼容）。
    fn kdf_config(&self) -> KdfConfig {
        self.kdf.unwrap_or_else(KdfConfig::balanced)
    }
}

// ════════════════════════════════════════════════════════════════
// Public API
// ════════════════════════════════════════════════════════════════

/// 单个待写入 ZIP 的附件导出条目（P012：GUI 与 CLI 共用 `write_attachment_entries` 的条目类型）。
pub struct ExportAttachmentEntry {
    pub obj_id: String,
    pub att_id: String,
    pub src: PathBuf,
}

/// 用 HKDF 派生附件密钥并流式加密写入 ZIP（P012 合并后的唯一实现，GUI 与 CLI 共用）。
///
/// `vault_att_key`：P001 vault 附件静态加密密钥——附件源在 vault 内加密落盘（SOLC 头）时
/// 先解密到临时明文再加密进包；旧明文（未加密历史数据）自动跳过解密直接加密进包。
/// 为 None（CLI 无解锁会话密钥）时若遇加密源明确报错（避免双重加密）。
///
/// 返回是否写入过附件。
/// P011：在系统临时目录下创建仅当前用户可访问的私有目录（Unix 0700），
/// 目录名含随机 UUID 不可预测——避免多用户系统上其他用户预占目录或在
/// 明文窗口期读取（对齐 GUI 侧 decrypt_to_temp_dir 的既有模式）。
pub(crate) fn create_private_temp_dir(prefix: &str) -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("solosoul_{}_{}", prefix, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建临时目录失败: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(dir)
}

/// P011：临时明文文件权限收紧为仅当前用户可读写（Unix 0600，best-effort）。
pub(crate) fn tighten_file_perms(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

pub fn write_attachment_entries(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    key: &[u8; 32],
    salt: &[u8],
    entries: &[ExportAttachmentEntry],
    vault_att_key: Option<&[u8; 32]>,
) -> Result<bool, ExportError> {
    if entries.is_empty() {
        return Ok(false);
    }
    let att_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:attachments:v1")
        .map_err(|e| format!("派生附件密钥失败: {}", e))?;
    for entry in entries {
        let zip_name = format!("attachments/{}/{}.enc", entry.obj_id, entry.att_id);
        zip.start_file(&zip_name, options)
            .map_err(|e| format!("写入 ZIP 附件条目失败: {}", e))?;

        // P001: vault 内附件可能为 SOLC 密文——先解密到临时明文再加密进包；
        // 旧明文（无 SOLC 头）直接作为源（与历史行为一致，零迁移兼容）。
        if crate::attachment_crypto::is_encrypted_file(&entry.src) {
            let Some(vault_key) = vault_att_key else {
                return Err(ExportError::Msg(format!(
                    "附件 {}/{} 已加密落盘，请使用最新版 GUI 客户端导出（CLI 缺少附件解密密钥）",
                    entry.obj_id, entry.att_id
                )));
            };
            // P011：私有临时目录（0700 + 随机名）替代共享固定目录
            let temp_dir = create_private_temp_dir("export_att")?;
            let temp_path = temp_dir.join(format!("{}.plain", uuid::Uuid::new_v4()));
            crate::attachment_crypto::copy_decrypt_file(vault_key, &entry.src, &temp_path)
                .map_err(|e| format!("解密附件失败: {}", e))?;
            tighten_file_perms(&temp_path);
            let file_size = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);
            let mut f = File::open(&temp_path).map_err(|e| format!("打开附件失败: {}", e))?;
            let mut reader = std::io::BufReader::new(&mut f);
            let enc_result = solosoul_crypto::cipher::encrypt_chunked_stream(
                &att_key,
                file_size,
                &mut reader,
                zip,
            )
            .map_err(|e| format!("加密附件失败: {}", e));
            let _ = std::fs::remove_file(&temp_path);
            enc_result?;
        } else {
            let file_size = std::fs::metadata(&entry.src).map(|m| m.len()).unwrap_or(0);
            let mut f = File::open(&entry.src).map_err(|e| format!("打开附件失败: {}", e))?;
            let mut reader = std::io::BufReader::new(&mut f);
            solosoul_crypto::cipher::encrypt_chunked_stream(&att_key, file_size, &mut reader, zip)
                .map_err(|e| format!("加密附件失败: {}", e))?;
        }
    }
    Ok(true)
}

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
    // P026: 序列化流式写入临时文件（避免 JSON 树 + 完整字节双份驻留内存），
    // 随后从文件流式加密进包。
    let (payload_tmp, payload_size) = write_payload_to_temp(base_path, &payload)?;

    let salt = solosoul_crypto::kdf::generate_salt();
    let key = derive_export_key(password, &salt)?;

    // 收集附件源文件。
    let attachment_entries = collect_attachment_entries(base_path, &records, scope)?;

    let payload_estimate = payload_size;
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
    // P012: 与 GUI 共用唯一实现——CLI 无附件密钥（未解锁会话）传 None，
    // 检测到 SOLC 密文时由共享实现明确报错而非双重加密。
    if att_key.is_some() {
        let shared_entries: Vec<ExportAttachmentEntry> = attachment_entries
            .iter()
            .map(|(obj_id, att_id, _file_name, src)| ExportAttachmentEntry {
                obj_id: obj_id.clone(),
                att_id: att_id.clone(),
                src: src.clone(),
            })
            .collect();
        write_attachment_entries(&mut zip, options, &key, &salt, &shared_entries, None)?;
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

    // payload.enc（流式加密，源为临时文件）
    zip.start_file("payload.enc", options)
        .map_err(|e| format!("写入 payload 条目失败: {}", e))?;
    {
        let mut payload_reader =
            std::io::BufReader::new(std::fs::File::open(payload_tmp.file.path())?);
        solosoul_crypto::cipher::encrypt_chunked_stream(
            &key,
            payload_size,
            &mut payload_reader,
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
    vault_att_key: Option<&[u8; 32]>,
) -> Result<usize, ExportError> {
    if password.is_empty() {
        return Err(ExportError::Msg("导入密码不能为空".to_string()));
    }

    let manifest = read_manifest(path)?;
    let salt = hex::decode(&manifest.salt_hex).map_err(|e| format!("salt 解码失败: {}", e))?;
    // P202: 按 manifest 声明参数派生（旧格式包无 kdf 字段回退 balanced 兼容）。
    let key = derive_export_key_cfg(password, &salt, &manifest.kdf_config())?;

    // P026: 流式解密主 payload——payload.enc 经 decrypt_chunked_stream 直接写入
    // 0700 临时文件，再从文件解析 JSON；峰值内存由「密文+明文+JSON 树」约 3× 降至约 1×。
    let payload: serde_json::Value = decrypt_payload_stream(path, base_path, &key)?;
    let package_ids = build_package_ids(&payload);

    // ── 模板快照导入（内容哈希隔离） ────────
    let mut template_id_map: HashMap<String, String> = HashMap::new();
    let now = chrono::Utc::now().to_rfc3339();
    import_template_snapshots(vault, account_id, &payload, &mut template_id_map, &now)?;

    // P212: 存在性预查（仅 SkipExisting 需要）——一次 metadata-only 查询收集
    // 现存非删除对象 ID，替代逐对象 load_object 的 N 次解密。VaultStore 按账户
    // 分库，账户内 ID 唯一，集合判定与 load_object+!is_deleted 语义等价。
    let existing_ids: HashSet<String> = match strategy {
        ImportStrategy::SkipExisting => vault
            .list_object_metadata(account_id, None, None, false, false)?
            .into_iter()
            .map(|s| s.id)
            .collect(),
        _ => HashSet::new(),
    };

    // P212: 构建待写对象（借用 payload 迭代，避免整数组克隆）。
    let (records_to_save, imported, imported_object_ids) = build_import_records(
        account_id,
        &payload,
        &template_id_map,
        &package_ids,
        &existing_ids,
        strategy,
        &now,
    );

    // P212: 单事务批量写入（替代逐条 save_object 的 N 次 auto-commit）。
    vault.save_objects_batch(&records_to_save)?;

    // 导入附件（P012：与 GUI 共用唯一实现；CLI 无 KeepBoth/选择性/进度需求，传空值）。
    if manifest.has_attachments {
        import_attachments(
            vault,
            base_path,
            path,
            &key,
            &salt,
            &imported_object_ids,
            &payload,
            vault_att_key,
            &HashMap::new(),
            None,
            &now,
            None,
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

/// 导入模板快照（内容哈希隔离 + 内容去重），填充 `template_id_map`。
///
/// P018: 从 `import_vault` 拆出——原函数 175 行含三个独立阶段，模板阶段与
/// 对象阶段互不依赖，仅通过 `template_id_map` 衔接。
fn import_template_snapshots(
    vault: &VaultStore,
    account_id: &str,
    payload: &serde_json::Value,
    template_id_map: &mut HashMap<String, String>,
    now: &str,
) -> Result<(), ExportError> {
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
                            tpl.created_at = now.to_string();
                            tpl.updated_at = Some(now.to_string());
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
    Ok(())
}

/// 从 payload 构建待写入对象记录（含跨范围引用降级与模板 ID 重映射）。
///
/// P018: 从 `import_vault` 拆出对象阶段；P212 语义保持：借用数组迭代、
/// SkipExisting 存在性预查判定、批量收集交由调用方单事务落库。
#[allow(clippy::too_many_arguments)]
fn build_import_records(
    account_id: &str,
    payload: &serde_json::Value,
    template_id_map: &HashMap<String, String>,
    package_ids: &HashSet<String>,
    existing_ids: &HashSet<String>,
    strategy: ImportStrategy,
    now: &str,
) -> (Vec<ObjectRecord>, usize, HashSet<String>) {
    let mut imported = 0usize;
    let mut imported_object_ids: HashSet<String> = HashSet::new();
    let mut records_to_save: Vec<ObjectRecord> = Vec::new();

    if let Some(objects) = payload["objects"].as_array() {
        for obj_val in objects {
            let id = obj_val["id"].as_str().unwrap_or("");
            if id.is_empty() {
                continue;
            }

            if matches!(strategy, ImportStrategy::SkipExisting) && existing_ids.contains(id) {
                continue;
            }

            let mut properties = obj_val["properties"].clone();
            resolve_cross_scope_references(&mut properties, package_ids);

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
                created_at: obj_val["created_at"].as_str().unwrap_or(now).to_string(),
                updated_at: now.to_string(),
                version: obj_val["version"].as_u64().unwrap_or(1) as u32,
            };

            records_to_save.push(record);
            imported += 1;
            imported_object_ids.insert(id.to_string());
        }
    }

    (records_to_save, imported, imported_object_ids)
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
///
/// 导出端（P202）：与主密钥派生一致走 `KdfConfig::from_env()`——release 为
/// OWASP 推荐 production（64MiB/3iter），debug 为 development（测试快速）。
/// 实际参数随导出写入 manifest 的 `kdf` 字段，导入端按声明参数派生。
fn derive_export_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, ExportError> {
    use solosoul_crypto::kdf::KdfConfig;
    derive_export_key_cfg(password, salt, &KdfConfig::from_env())
}

/// 以指定 KDF 参数派生导出密钥（导入端按 manifest 声明参数调用）。
/// P024: 薄包装 `solosoul-crypto::kdf::derive_export_key` 单一实现，仅映射错误类型。
/// P018: 返回 `Zeroizing<[u8;32]>`，导出密钥不再以裸数组残留在内存。
fn derive_export_key_cfg(
    password: &str,
    salt: &[u8],
    config: &KdfConfig,
) -> Result<Zeroizing<[u8; 32]>, ExportError> {
    solosoul_crypto::kdf::derive_export_key(password, salt, config)
        .map_err(|e| ExportError::Msg(format!("密钥派生失败: {}", e)))
}

/// 将 KDF 参数编码为 manifest 的 `kdf` 字段（自描述 JSON，供导入端复用）。
/// 导出包是最可能的离线攻击目标，参数必须随包携带，导入端按声明派生。
pub fn kdf_to_manifest_value(config: &KdfConfig) -> serde_json::Value {
    serde_json::json!({
        "algo": "argon2id",
        "memory_kb": config.memory_kb,
        "iterations": config.iterations,
        "parallelism": config.parallelism,
    })
}

/// 从 manifest 的 `kdf` 字段解析 KDF 参数。
/// - 字段缺失（`None`）→ 旧格式包，返回 `None`，调用方按 balanced 兜底。
/// - 字段存在但非法/不完整 → `Err`（拒绝静默降级到弱参数，防参数降级攻击）。
pub fn kdf_from_manifest_value(v: Option<&serde_json::Value>) -> Result<Option<KdfConfig>, String> {
    let Some(val) = v else {
        return Ok(None);
    };
    let Some(obj) = val.as_object() else {
        return Err("manifest 的 kdf 字段必须为对象".to_string());
    };
    let algo = obj
        .get("algo")
        .and_then(|x| x.as_str())
        .ok_or("manifest 的 kdf 字段缺少 algo")?;
    if algo != "argon2id" {
        return Err(format!("不支持的 KDF 算法: {}", algo));
    }
    let memory_kb = obj
        .get("memory_kb")
        .and_then(|x| x.as_u64())
        .ok_or("manifest 的 kdf 字段缺少 memory_kb")?;
    let iterations = obj
        .get("iterations")
        .and_then(|x| x.as_u64())
        .ok_or("manifest 的 kdf 字段缺少 iterations")?;
    let parallelism = obj
        .get("parallelism")
        .and_then(|x| x.as_u64())
        .ok_or("manifest 的 kdf 字段缺少 parallelism")?;
    // 上限防御（P202 评审）：manifest 是攻击者可控的，参数无上界会让导入端
    // 按声明跑巨量 Argon2（OOM/挂起）。上限取生产档的合理放大量：
    // memory 1 GiB（production 的 16 倍）、iterations 10、parallelism 64。
    if memory_kb == 0
        || memory_kb > 1_048_576
        || iterations == 0
        || iterations > 10
        || parallelism == 0
        || parallelism > 64
    {
        return Err("manifest 的 kdf 参数非法".to_string());
    }
    Ok(Some(KdfConfig {
        memory_kb: memory_kb as u32,
        iterations: iterations as u32,
        parallelism: parallelism as u32,
    }))
}

/// 从对象属性中读取附件列表。
fn load_attachments(props: &serde_json::Value) -> Vec<AttachmentMeta> {
    props
        .get("__attachments")
        .and_then(|v| serde_json::from_value::<Vec<AttachmentMeta>>(v.clone()).ok())
        .unwrap_or_default()
}

/// P225: 解析附件实际源文件路径（vault 落库副本 > 原始路径 > 附件目录回退）。
/// core 导出与 GUI 导出共用，消除双份实现（字段级参数化以兼容两侧 AttachmentMeta）。
pub fn resolve_attachment_src(
    base_dir: &std::path::Path,
    vault_path: Option<&str>,
    src_path: Option<&str>,
    att_id: &str,
    file_name: &str,
) -> Option<std::path::PathBuf> {
    vault_path
        .or(src_path)
        .map(|p| std::path::Path::new(p).to_path_buf())
        .filter(|p| p.exists())
        .or_else(|| {
            let fallback = base_dir.join(att_id).join(file_name);
            if fallback.exists() {
                Some(fallback)
            } else {
                None
            }
        })
}

/// 根据范围收集对象记录。
fn collect_scope_objects(
    vault: &VaultStore,
    account_id: &str,
    scope: &ExportScope,
) -> Result<Vec<ObjectRecord>, ExportError> {
    // P111: 仅需 id/section_type 做范围收集，随后 load_objects_batch 拉全量，走 metadata-only 查询。
    let all = vault.list_object_metadata(account_id, None, None, false, false)?;

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

            let src = resolve_attachment_src(
                &base_dir,
                att.vault_path.as_deref(),
                att.src_path.as_deref(),
                &att.id,
                &att.file_name,
            );

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
        // P202: 导出包携带实际 KDF 参数，导入端按声明派生（旧包无此字段回退 balanced）。
        "kdf": kdf_to_manifest_value(&KdfConfig::from_env()),
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
        kdf: kdf_from_manifest_value(v.get("kdf"))?,
    })
}

/// 导出 payload 的临时序列化持有者：目录与文件同生命周期，Drop 时文件先删、目录后删
/// （Windows 上打开的文件无法删除，顺序保证目录可被完整移除）。
pub struct PayloadTemp {
    _dir: tempfile::TempDir,
    file: tempfile::NamedTempFile,
}

impl PayloadTemp {
    /// 序列化后的 payload 临时文件路径。
    pub fn path(&self) -> &std::path::Path {
        self.file.path()
    }
}

/// 将 payload JSON 流式序列化到临时文件并返回其大小（P026）：
/// `serde_json::to_writer` 直接写出，避免「JSON 树 + 完整字节」双份驻留内存；
/// 临时明文置于保险库数据目录（0700 同姿态），随持有者 Drop 清理。
/// GUI 导出与 CLI 导出共用。
pub fn write_payload_to_temp(
    base_path: &Path,
    payload: &serde_json::Value,
) -> Result<(PayloadTemp, u64), ExportError> {
    let dir = tempfile::Builder::new()
        .prefix("solosoul-export-tmp-")
        .tempdir_in(base_path)
        .map_err(|e| format!("创建导出临时目录失败: {}", e))?;
    let mut file = tempfile::NamedTempFile::new_in(dir.path())
        .map_err(|e| format!("创建导出临时文件失败: {}", e))?;
    serde_json::to_writer(&mut file, payload).map_err(|e| format!("序列化负载失败: {}", e))?;
    file.flush()
        .map_err(|e| format!("刷新负载文件失败: {}", e))?;
    let size = std::fs::metadata(file.path()).map(|m| m.len()).unwrap_or(0);
    Ok((PayloadTemp { _dir: dir, file }, size))
}

/// 流式解密并解析 `payload.enc`（P026）：密文/明文不整体驻留内存，
/// 解密直接写入临时文件（置于保险库数据目录、0700 同姿态）后由
/// `serde_json::from_reader` 解析，峰值内存约 1× payload。
/// 临时目录随 TempDir Drop 递归删除（含崩溃残留由调用方按前缀清扫）。
fn decrypt_payload_stream(
    path: &Path,
    base_path: &Path,
    key: &[u8; 32],
) -> Result<serde_json::Value, ExportError> {
    let temp_dir = tempfile::Builder::new()
        .prefix("solosoul-import-tmp-")
        .tempdir_in(base_path)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;
    let mut tmp = tempfile::NamedTempFile::new_in(temp_dir.path())
        .map_err(|e| format!("创建临时文件失败: {}", e))?;

    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive
        .by_name("payload.enc")
        .map_err(|_| "ZIP 中缺少: payload.enc".to_string())?;
    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(ExportError::Msg(format!(
            "ZIP 条目 'payload.enc' 过大 ({} 字节, 上限 {} 字节)",
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        )));
    }
    solosoul_crypto::cipher::decrypt_chunked_stream(key, &mut entry, &mut tmp)
        .map_err(|_| ExportError::DecryptionFailed)?;

    let payload: serde_json::Value = {
        let f = std::fs::File::open(tmp.path()).map_err(|e| format!("读取临时文件失败: {}", e))?;
        serde_json::from_reader(f)?
    };
    Ok(payload)
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
/// P010: 由 core 单一实现，GUI 侧 re-export（消除 helpers.rs 逐字副本）。
pub fn build_package_ids(payload: &serde_json::Value) -> HashSet<String> {
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
/// P010: 由 core 单一实现，GUI 侧 re-export。
pub fn resolve_value_references(value: &mut serde_json::Value, package_ids: &HashSet<String>) {
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
/// P010: 由 core 单一实现，GUI 侧 re-export。
pub fn resolve_cross_scope_references(
    properties: &mut serde_json::Value,
    package_ids: &HashSet<String>,
) {
    if let Some(map) = properties.as_object_mut() {
        for (_key, value) in map.iter_mut() {
            resolve_value_references(value, package_ids);
        }
    }
}

/// 导入附件到 vault 存储目录（P012 合并后的唯一实现，GUI 与 CLI 共用）。
///
/// 以 GUI 版为基准补齐的能力（CLI/测试路径自动获得同等行为）：
/// - `id_map`：KeepBoth 策略下旧对象 ID → 新对象 ID 的映射，附件目录与写回均用新 ID；
/// - `sel_att_ids_set`：选择性导入（仅导入选中附件 ID）；
/// - `now`：附件 created_at 时间戳（与对象导入共用同一时间，避免毫秒漂移）；
/// - `progress`：ZIP 条目级进度回调（0-100，调用方负责映射到自己的进度区间）。
///
/// P001-1：`vault_att_key` 为 vault 附件静态加密密钥（CLI 从已解锁会话派生）——
/// 提供时解密 ZIP 条目后以该密钥加密落盘（不再明文写盘，与 GUI 路径一致）；
/// 为 None（测试等无密钥上下文）时保持原明文写盘行为。
///
/// 返回导入的附件数量。
#[allow(clippy::too_many_arguments)]
pub fn import_attachments(
    vault: &VaultStore,
    base_path: &Path,
    path: &Path,
    key: &[u8; 32],
    salt: &[u8],
    imported_object_ids: &HashSet<String>,
    payload: &serde_json::Value,
    vault_att_key: Option<&[u8; 32]>,
    id_map: &HashMap<String, String>,
    sel_att_ids_set: Option<&HashSet<String>>,
    now: &str,
    progress: Option<&(dyn Fn(u8) + Send + Sync)>,
) -> Result<usize, ExportError> {
    let att_key = solosoul_crypto::hkdf_ext::derive_hkdf_key(key, salt, b"solosoul:attachments:v1")
        .map_err(|e| format!("派生附件密钥失败: {}", e))?;

    let att_meta_map = build_attachment_meta_map(payload);

    let zip_file = File::open(path)?;
    let mut archive = ZipArchive::new(zip_file)?;
    let att_prefix = "attachments/";
    let zip_total = archive.len();

    let mut imported_atts: HashMap<String, Vec<AttachmentMeta>> = HashMap::new();

    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        if let Some(cb) = &progress {
            cb(((i * 100) / zip_total.max(1)).min(100) as u8);
        }
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
        // 上 `..\..\evil` 可写出 Vault 目录（路径遍历）。非法条目跳过（与 GUI 一致）。
        if validate_import_id(obj_id).is_err() || validate_import_id(old_att_id).is_err() {
            continue;
        }

        if !imported_object_ids.contains(obj_id) {
            continue;
        }

        // 选择性附件导入：跳过未选中的附件
        if let Some(sel_set) = sel_att_ids_set {
            if !sel_set.contains(old_att_id) {
                continue;
            }
        }

        let old_meta = match att_meta_map.get(&(obj_id.to_string(), old_att_id.to_string())) {
            Some(m) => m,
            None => continue,
        };

        // KeepBoth：附件目录/写回使用新对象 ID（否则后续按新 ID 查找对象会找不到）
        let actual_obj_id = id_map
            .get(obj_id)
            .cloned()
            .unwrap_or_else(|| obj_id.to_string());
        let new_att_id = uuid::Uuid::new_v4().to_string();
        let dest = base_path
            .join("attachments")
            .join(&actual_obj_id)
            .join(&new_att_id);
        std::fs::create_dir_all(&dest)?;

        // P003：落盘文件名取末段安全名，并**写回元数据**——此前元数据保留原始
        // `file_name`（如 `../../evil.txt`），后续插件主机 `copy_attachment_to_workspace`
        // 用原始名 join 目标目录造成存储型路径遍历。
        let safe_name = sanitize_import_file_name(&old_meta.file_name)?;
        let file_path_dest = dest.join(&safe_name);
        let file_size =
            write_imported_attachment(&mut f, &att_key, vault_att_key, &file_path_dest)?;

        imported_atts
            .entry(actual_obj_id.clone())
            .or_default()
            .push(AttachmentMeta {
                id: new_att_id,
                object_id: actual_obj_id.clone(),
                file_name: safe_name.clone(),
                mime_type: old_meta.mime_type.clone(),
                size_bytes: file_size,
                created_at: now.to_string(),
                deleted_at: None,
                src_path: Some(file_path_dest.to_string_lossy().to_string()),
                vault_path: Some(file_path_dest.to_string_lossy().to_string()),
                description: None,
                tags: vec![],
            });
    }

    // 更新已导入对象的 __attachments（按实际对象 ID）
    write_back_imported_attachments(vault, imported_atts)
}

/// P019-①：从 payload 提取「(旧对象ID, 旧附件ID) → 附件元数据」映射。
fn build_attachment_meta_map(
    payload: &serde_json::Value,
) -> HashMap<(String, String), AttachmentMeta> {
    let mut map: HashMap<(String, String), AttachmentMeta> = HashMap::new();
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
                map.insert((obj_id.to_string(), att.id.clone()), att.clone());
            }
        }
    }
    map
}

/// P019-②：单个 ZIP 条目解密落盘。vault_att_key 存在时经私有临时明文中转再加密
/// （P001-1/P011），否则直接以导出密钥解密写入；返回落盘字节数。
fn write_imported_attachment(
    f: &mut impl std::io::Read,
    att_key: &[u8; 32],
    vault_att_key: Option<&[u8; 32]>,
    file_path_dest: &Path,
) -> Result<u64, ExportError> {
    match vault_att_key {
        Some(vault_key) => {
            // P011：私有临时目录（0700 + 随机名）替代共享固定目录
            let temp_dir = create_private_temp_dir("import_att")?;
            let temp_path = temp_dir.join(format!("{}.plain", uuid::Uuid::new_v4()));
            // 解密/加密/清理三阶段，任何失败都必须删除临时明文（不残留）。
            let result = (|| -> Result<u64, String> {
                let mut temp_file =
                    File::create(&temp_path).map_err(|e| format!("创建临时文件失败: {}", e))?;
                solosoul_crypto::cipher::decrypt_chunked_stream(att_key, f, &mut temp_file)
                    .map_err(|e| format!("解密附件流失败: {}", e))?;
                drop(temp_file);
                tighten_file_perms(&temp_path);
                let size = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);
                crate::attachment_crypto::encrypt_file_stream(
                    vault_key,
                    &temp_path,
                    file_path_dest,
                )
                .map_err(|e| format!("附件加密落盘失败: {}", e))?;
                Ok(size)
            })();
            let _ = std::fs::remove_file(&temp_path);
            // P011 核验补修：一次性目录用后即删（remove_dir 仅空目录成功，
            // 防止 0700 UUID 空目录在系统 temp 无限累积）。
            let _ = std::fs::remove_dir(&temp_dir);
            result.map_err(ExportError::Msg)
        }
        None => {
            let mut out_file = File::create(file_path_dest)?;
            solosoul_crypto::cipher::decrypt_chunked_stream(att_key, f, &mut out_file)
                .map_err(|e| format!("解密附件流失败: {}", e))?;
            Ok(std::fs::metadata(file_path_dest)
                .map(|m| m.len())
                .unwrap_or(0))
        }
    }
}

/// P019-③：把导入的附件元数据写回各对象的 __attachments 属性，返回导入总数。
fn write_back_imported_attachments(
    vault: &VaultStore,
    imported_atts: HashMap<String, Vec<AttachmentMeta>>,
) -> Result<usize, ExportError> {
    let imported_count = imported_atts.values().map(|v| v.len()).sum::<usize>();
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

    Ok(imported_count)
}

/// 净化导入附件的文件名（P003 / P023 收敛到共享实现）。
///
/// P023：语义与 `path_util::sanitize_file_name` 完全一致（平台无关拒绝 `/` `\\`
/// 分隔符 + 取末段兜底 + 拒绝空/`.`/`..`），此处仅做 ExportError 包装转发。
fn sanitize_import_file_name(file_name: &str) -> Result<String, ExportError> {
    crate::path_util::sanitize_file_name(file_name).map_err(ExportError::Msg)
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
    // R2-06: 传播保存失败，避免用户看到"导入成功"但 preferences 未落库。
    vault.save_profile(&profile)?;
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
    fn test_sanitize_import_file_name_accepts_normal_name() {
        let result = sanitize_import_file_name("doc.pdf");
        assert_eq!(result.unwrap(), "doc.pdf");
        // "..." 是合法文件名（非空、非 . 非 ..），应被接受
        assert_eq!(sanitize_import_file_name("...").unwrap(), "...");
    }

    #[test]
    fn test_sanitize_import_file_name_rejects_path_separators() {
        // P003 平台无关拒绝：正斜杠与反斜杠（Windows 反斜杠分隔符）
        assert!(sanitize_import_file_name("../../evil.txt").is_err());
        assert!(sanitize_import_file_name("..\\..\\evil.txt").is_err());
        assert!(sanitize_import_file_name("a/b.txt").is_err());
        assert!(sanitize_import_file_name("a\\b.txt").is_err());
    }

    #[test]
    fn test_sanitize_import_file_name_rejects_dot_and_empty() {
        assert!(sanitize_import_file_name("").is_err());
        assert!(sanitize_import_file_name(".").is_err());
        assert!(sanitize_import_file_name("..").is_err());
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
            None,
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
            None,
        );
        assert!(result.is_err(), "应返回密码错误: {:?}", result);
    }

    #[test]
    fn test_import_skip_existing_semantics() {
        // P212 回归：SkipExisting 存在性判定由逐对象 load_object 改为一次
        // metadata-only 预查，语义必须等价——活动对象跳过、软删对象重新导入。
        let (vault, account_id, dir) = test_setup();
        vault
            .save_object(&make_test_record(&account_id, "skip_me", "Local Name"))
            .unwrap();

        let path = dir.path().join("test_skip_existing.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key = derive_export_key(TEST_EXPORT_PASSWORD, &salt).unwrap();
        let payload = serde_json::json!({
            "objects": [{
                "id": "skip_me", "name": "Imported Name", "account_id": account_id,
                "type_id": "note", "section_type": "identity", "icon_name": "document",
                "properties": {}, "sensitivity_level": "internal", "tags": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z", "version": 1
            }]
        });
        write_test_package(&path, &payload, &key, &salt).unwrap();

        // 活动对象存在 → 跳过，保留本地
        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::SkipExisting,
            dir.path(),
            None,
        )
        .unwrap();
        assert_eq!(imported, 0);
        let local = vault.load_object("skip_me").unwrap().unwrap();
        assert_eq!(local.name, "Local Name");

        // 软删后 → 非活动，重新导入（覆盖软删行）
        vault.delete_object("skip_me", true).unwrap();
        let imported2 = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::SkipExisting,
            dir.path(),
            None,
        )
        .unwrap();
        assert_eq!(imported2, 1);
        let restored = vault.load_object("skip_me").unwrap().unwrap();
        assert_eq!(restored.name, "Imported Name");
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            "extra_files": [],
            // 测试助手与 derive_export_key（from_env）保持一致，导入端按声明派生。
            "kdf": kdf_to_manifest_value(&KdfConfig::from_env()),
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

    /// 手动构建含指定 manifest kdf 字段的包（用于 P202 参数化测试）。
    /// `manifest_kdf`：`None` = 旧格式包（无 kdf 字段）；`Some(v)` = 携带该 kdf 声明。
    fn write_test_package_raw(
        path: &Path,
        payload: &serde_json::Value,
        key: &[u8; 32],
        salt: &[u8; 16],
        manifest_kdf: Option<serde_json::Value>,
    ) -> Result<(), ExportError> {
        let payload_bytes = serde_json::to_vec(payload)?;
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut manifest = serde_json::json!({
            "version": "2.0",
            "salt_hex": hex::encode(salt),
            "has_attachments": false,
            "has_templates": false,
            "extra_files": []
        });
        if let Some(kdf) = manifest_kdf {
            manifest["kdf"] = kdf;
        }

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

    // ── P202：KDF 参数随 manifest 携带 ──────────────────────

    #[test]
    fn test_kdf_manifest_value_roundtrip() {
        for cfg in [
            KdfConfig::production(),
            KdfConfig::balanced(),
            KdfConfig::development(),
        ] {
            let v = kdf_to_manifest_value(&cfg);
            let parsed = kdf_from_manifest_value(Some(&v)).unwrap().unwrap();
            assert_eq!(parsed, cfg, "kdf 往返不一致: {:?}", cfg);
        }
    }

    #[test]
    fn test_kdf_from_manifest_absent_and_invalid() {
        // 字段缺失 → None（旧格式包，调用方按 balanced 兜底）
        assert!(kdf_from_manifest_value(None).unwrap().is_none());
        // 非对象 / 缺字段 / 非法算法 / 非法参数 → Err（拒绝静默降级）
        assert!(kdf_from_manifest_value(Some(&serde_json::json!("argon2id"))).is_err());
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({ "algo": "argon2id" }))).is_err());
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "scrypt",
            "memory_kb": 1,
            "iterations": 1,
            "parallelism": 1
        })))
        .is_err());
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "argon2id",
            "memory_kb": 0,
            "iterations": 1,
            "parallelism": 1
        })))
        .is_err());
        // 超上限参数（攻击者可控 → 导入 DoS 面）：巨量 memory / iterations / parallelism 一律拒绝
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "argon2id",
            "memory_kb": 1_048_577,
            "iterations": 1,
            "parallelism": 1
        })))
        .is_err());
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "argon2id",
            "memory_kb": 65536,
            "iterations": 100_000_000,
            "parallelism": 64
        })))
        .is_err());
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "argon2id",
            "memory_kb": 65536,
            "iterations": 3,
            "parallelism": 1024
        })))
        .is_err());
        // 边界内合法值仍接受
        assert!(kdf_from_manifest_value(Some(&serde_json::json!({
            "algo": "argon2id",
            "memory_kb": 1_048_576,
            "iterations": 10,
            "parallelism": 64
        })))
        .is_ok());
    }

    #[test]
    fn test_import_old_format_falls_back_to_balanced() {
        // 模拟 P202 之前的旧格式包：manifest 无 kdf 字段，payload 用 balanced 加密。
        // 导入端必须回退 balanced 才能解密（向后兼容）。
        let (vault, account_id, dir) = test_setup();
        let path = dir.path().join("test_old_balanced.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key =
            derive_export_key_cfg(TEST_EXPORT_PASSWORD, &salt, &KdfConfig::balanced()).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_old_bal",
                "name": "Old Balanced",
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
        write_test_package_raw(&path, &payload, &key, &salt, None).unwrap();

        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
            None,
        )
        .unwrap();
        assert_eq!(imported, 1);
        let obj = vault.load_object("obj_old_bal").unwrap().unwrap();
        assert_eq!(obj.name, "Old Balanced");
    }

    #[test]
    fn test_import_new_format_uses_declared_kdf() {
        // 新格式包：manifest 声明 balanced，payload 用 balanced 加密。
        // 即使 debug 构建下 from_env()=development，导入端也必须按声明（balanced）派生。
        let (vault, account_id, dir) = test_setup();
        let path = dir.path().join("test_new_declared.solosoul");
        let salt = solosoul_crypto::kdf::generate_salt();
        let key =
            derive_export_key_cfg(TEST_EXPORT_PASSWORD, &salt, &KdfConfig::balanced()).unwrap();

        let payload = serde_json::json!({
            "objects": [{
                "id": "obj_new_decl",
                "name": "Declared Balanced",
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
        write_test_package_raw(
            &path,
            &payload,
            &key,
            &salt,
            Some(kdf_to_manifest_value(&KdfConfig::balanced())),
        )
        .unwrap();

        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
            None,
        )
        .unwrap();
        assert_eq!(imported, 1);
        let obj = vault.load_object("obj_new_decl").unwrap().unwrap();
        assert_eq!(obj.name, "Declared Balanced");
    }

    /// P001-1：CLI 导入（提供 vault 附件静态加密密钥）后附件必须加密落盘——
    /// 不再是明文文件，且可用传入密钥解密还原（回归：原实现直接明文写盘，
    /// 破坏「附件不再明文落盘」不变量）。
    #[test]
    fn test_import_attachments_encrypted_at_rest_with_vault_key() {
        let (vault, account_id, dir) = test_setup();

        // 在 vault attachments 目录放一个明文源附件，并写 __attachments 元数据。
        let att_dir = dir.path().join("attachments").join("obj_1").join("att_1");
        std::fs::create_dir_all(&att_dir).unwrap();
        let src_file = att_dir.join("a.pdf");
        let plain = b"import-attachment-at-rest".repeat(10);
        std::fs::write(&src_file, &plain).unwrap();

        let mut rec = make_test_record(&account_id, "obj_1", "Test Object");
        rec.properties = serde_json::json!({
            "title": "hello",
            "__attachments": [{
                "id": "att_1",
                "objectId": "obj_1",
                "fileName": "a.pdf",
                "mimeType": "application/pdf",
                "sizeBytes": plain.len() as u64,
                "createdAt": "2024-01-01T00:00:00Z",
                "vaultPath": src_file.to_string_lossy().to_string()
            }]
        });
        vault.save_object(&rec).unwrap();

        // 导出（含附件，ZIP 内以导出密钥加密）。
        let path = dir.path().join("test_att_import.solosoul");
        let scope = ExportScope {
            full: true,
            include_attachments: true,
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

        // 删除本地明文源附件，模拟「仅剩导出包」的恢复场景。
        std::fs::remove_file(&src_file).unwrap();
        vault.delete_object("obj_1", false).unwrap();

        // 以 vault 附件静态密钥导入（CLI 从已解锁会话派生）。
        // P011 核验补修：生产已改为 solosoul_import_att_{uuid} 随机目录——
        // 快照改按前缀扫描系统 temp 下全部匹配条目（与其他测试共享，仅对比本次新增）。
        let snapshot_before = scan_import_att_entries();
        let vault_att_key: [u8; 32] = [0x11; 32];
        let imported = import_vault(
            &vault,
            &account_id,
            &path,
            TEST_EXPORT_PASSWORD,
            ImportStrategy::Overwrite,
            dir.path(),
            Some(&vault_att_key),
        )
        .unwrap();
        assert_eq!(imported, 1);

        // 导入后的附件文件必须加密落盘（SOLC magic），且可解密还原。
        let restored = vault.load_object("obj_1").unwrap().unwrap();
        let atts = load_attachments(&restored.properties);
        assert_eq!(atts.len(), 1);
        let new_att = &atts[0];
        let new_file = std::path::Path::new(new_att.vault_path.as_deref().unwrap());
        assert!(
            new_file.exists(),
            "导入附件文件应存在: {}",
            new_file.display()
        );
        assert!(
            crate::attachment_crypto::is_encrypted_file(new_file),
            "CLI 导入附件必须加密落盘（非明文）: {}",
            new_file.display()
        );
        let decrypted =
            crate::attachment_crypto::read_file_decrypted(&vault_att_key, new_file, 1_000_000)
                .expect("以 vault 密钥解密导入附件");
        assert_eq!(decrypted, plain);

        // 本次导入不得在临时目录留下新条目（明文文件或未清理的随机目录）。
        let new_leftovers: Vec<String> = scan_import_att_entries()
            .into_iter()
            .filter(|n| !snapshot_before.contains(n))
            .collect();
        assert!(
            new_leftovers.is_empty(),
            "本次导入不应残留临时明文/临时目录: {:?}",
            new_leftovers
        );
    }

    /// P011 核验补修：扫描系统 temp 下全部 solosoul_import_att_* 条目名
    /// （含随机目录与历史遗留的固定目录），供残留对比断言使用。
    fn scan_import_att_entries() -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        if let Ok(rd) = std::fs::read_dir(std::env::temp_dir()) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("solosoul_import_att") || name == "solosoul_export_att" {
                    out.insert(name);
                }
            }
        }
        out
    }
}
