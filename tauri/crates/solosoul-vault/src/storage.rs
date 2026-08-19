//! Vault store - SQLite storage with app-layer AES-256-GCM encryption

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use zeroize::Zeroize;

use crate::encryption::{
    decrypt_field, decrypt_text_field, encrypt_field, ensure_encrypted_text, DataEncryptionKey,
};
use crate::migration::run_migrations;
use crate::{VaultConfig, VaultState, VaultStats};

// 表域拆分（P223-②）：objects 域（试点）、snapshots 域、sync_meta 域与 trash 域已抽至子模块。
// 后续 sync_changes / sync_apply / metadata / templates 等域按此模式拆分：
// 方法体逐行搬运到 src/storage/<domain>.rs 的 `impl VaultStore { .. }`，
// 跨域被根模块调用的私有助手提升为 `pub(crate)`，其余保持私有。
mod conversations;
mod metadata;
mod objects;
mod profile;
mod reencrypt;
mod snapshots;
mod sync_apply;
mod sync_changes;
mod sync_meta;
mod trash;

/// 高频查询共享列列表
const OBJECT_COLUMNS: &str = "\
    id, account_id, type_id, section_type, name, icon_name, parent_id, \
    children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version";

/// P213: 对象表 SELECT 前缀常量（load_objects_batch 等动态拼接场景复用，避免每次 format! 分配）。
const OBJECT_SELECT_BASE: &str = "SELECT id, account_id, type_id, section_type, name, icon_name, \
    parent_id, children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version \
    FROM objects";

/// 设备同步「同步设置偏好」勾选框（默认开启）关闭时，**不同步**的 UI 外观偏好键。
///
/// 这些键属于设备外观/界面偏好（主题、主题色、背景、界面语言、侧边栏等），
/// 用户选择不共享后，发送侧从 profile delta 中剥离、接收侧保留本地值不被对端覆盖。
/// preferences 子对象中**其余**键（回收站保留期、自动锁定、LLM 配置等账户级设置）
/// 不受影响，照常随 profiles 表同步。
/// P004: AI 对话已迁出 blob 改存 llm_conversations 行级表（随 SYNC_TABLES 单独同步），
/// 不再出现在 profile delta 中。
pub const UI_PREF_SYNC_EXCLUDED_KEYS: &[&str] = &[
    "theme",
    "accentColor",
    "customAccentHex",
    "backgroundType",
    "backgroundValue",
    "defaultLightTheme",
    "defaultDarkTheme",
    "sidebarPosition",
    "sidebarButtonModes",
    "language",
    "locale",
];

/// P213: load_object 的完整常量 SQL（唯一主键查询，最高频语句）。
const OBJECT_LOAD_SQL: &str = "SELECT id, account_id, type_id, section_type, name, icon_name, \
    parent_id, children_ids, properties, property_labels, sensitivity_level, \
    is_deleted, deleted_at, tags_json, template_id, template_type, \
    contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version \
    FROM objects WHERE id = ?1";

/// P213: save_object 的 UPSERT 常量 SQL（写对象最高频语句）。
const OBJECT_SAVE_SQL: &str = "INSERT INTO objects (id, account_id, type_id, section_type, name, icon_name, parent_id, \
     children_ids, properties, property_labels, sensitivity_level, \
     is_deleted, deleted_at, tags_json, template_id, template_type, \
     contract_type_id, template_hash, ignored_template_hash, created_at, updated_at, version) \
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22) \
     ON CONFLICT(id) DO UPDATE SET \
       type_id=excluded.type_id, section_type=excluded.section_type, name=excluded.name, icon_name=excluded.icon_name, \
       parent_id=excluded.parent_id, children_ids=excluded.children_ids, \
       properties=excluded.properties, property_labels=excluded.property_labels, \
       sensitivity_level=excluded.sensitivity_level, \
       is_deleted=excluded.is_deleted, deleted_at=excluded.deleted_at, \
       tags_json=excluded.tags_json, \
       template_id=excluded.template_id, template_type=excluded.template_type, \
       contract_type_id=excluded.contract_type_id, template_hash=excluded.template_hash, \
       ignored_template_hash=excluded.ignored_template_hash, \
       updated_at=excluded.updated_at, version=excluded.version";

/// P213: load_profile 常量 SQL。
const PROFILE_LOAD_SQL: &str =
    "SELECT id, name, data, created_at, updated_at, version FROM profiles WHERE id = ?1";

/// P213: save_profile 的 UPSERT 常量 SQL。
const PROFILE_SAVE_SQL: &str =
    "INSERT INTO profiles (id, name, data, created_at, updated_at, version) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT(id) DO UPDATE SET \
        name = excluded.name, data = excluded.data, \
        updated_at = excluded.updated_at, version = excluded.version";

/// P213: HLC 读写常量 SQL（同步热路径）。
const HLC_GET_SQL: &str =
    "SELECT wall_time_ms, counter, node_id FROM sync_hlc WHERE table_name = ?1 AND record_id = ?2";
const HLC_SET_SQL: &str =
    "INSERT INTO sync_hlc (table_name, record_id, wall_time_ms, counter, node_id, updated_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT(table_name, record_id) DO UPDATE SET \
        wall_time_ms = excluded.wall_time_ms, \
        counter = excluded.counter, \
        node_id = excluded.node_id, \
        updated_at = excluded.updated_at";

/// P213: 回收站/用户模板读写常量 SQL。
const TRASH_SAVE_SQL: &str =
    "INSERT INTO trash_items (id, item_type, original_id, original_parent_id, \
     original_section_type, original_sort_order, data, deleted_at, expires_at, deleted_by, \
     name_snapshot, icon_snapshot) \
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)";
const USER_TEMPLATE_SAVE_SQL: &str = "INSERT INTO user_templates (id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
     ON CONFLICT(id) DO UPDATE SET \
         name = excluded.name, \
         icon_id = excluded.icon_id, \
         properties_json = excluded.properties_json, \
         category = excluded.category, \
         contract_type_id = excluded.contract_type_id, \
         updated_at = excluded.updated_at";
const USER_TEMPLATE_LOAD_SQL: &str = "SELECT id, account_id, name, icon_id, properties_json, category, contract_type_id, created_at, updated_at \
     FROM user_templates WHERE id = ?1";

/// P020: `user_templates` 行解密映射（load/list/sync 三处共用，列序需与
/// USER_TEMPLATE_LOAD_SQL / list_user_templates / sync_changes 的 SELECT 完全一致）。
fn map_user_template_row(
    key: &DataEncryptionKey,
    row: &rusqlite::Row<'_>,
) -> Result<crate::UserTemplate, rusqlite::Error> {
    let props_json: String = row.get(4)?;
    let decrypted = decrypt_text_field(key, &props_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Template properties decryption failed: {}", e),
            )),
        )
    })?;
    let properties: Vec<crate::TemplateProperty> = serde_json::from_str(&decrypted)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(crate::UserTemplate {
        contract_type_id: row.get(6)?,
        id: row.get(0)?,
        account_id: row.get(1)?,
        name: row.get(2)?,
        icon_id: row.get(3)?,
        properties,
        category: row.get(5)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// P213: 对象软删常量 SQL（回收站批量入站/单删共用）。
const OBJECT_SOFT_DELETE_SQL: &str =
    "UPDATE objects SET is_deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2";

/// P213: 手动事务封装——BEGIN/COMMIT/ROLLBACK 三件套。
///
/// rusqlite 的 [`rusqlite::Transaction`] 只实现不可变 Deref（无法获得 `&mut Connection`，
/// 因而无法在其上使用 `prepare_cached`）。本助手改为在调用方持有的 `&mut Connection` 上
/// 手动 BEGIN/COMMIT/ROLLBACK：回调内可直接 `prepare_cached` 复用预编译语句，
/// 失败自动 ROLLBACK，成功 COMMIT。语义与 `conn.transaction()` 等价。
///
/// 注意：与 `Transaction` 不同，回调 panic 时不会自动回滚（无法在 unwind 中持有借用）。
/// 调用方应确保回调内无 panic 操作；本库约定错误一律经 `Result` 返回，故可接受。
/// P007: rusqlite 错误对外消息脱敏。
///
/// `rusqlite::Error::SqliteFailure(_, Some(sql))` 的 Display 会把 SQL 语句文本
/// （表名/列名/查询结构）一并带出——若直接透传，SQL 片段可达前端 UI/toast，
/// 对隐私优先定位属攻击面。本函数把完整错误（含 SQL）落 tracing 供诊断，
/// 对外只保留 ffi 层消息（code + 原因），不携带 SQL 文本。
fn sql_err(context: &str, e: rusqlite::Error) -> String {
    tracing::error!("{context}: sqlite error: {e}");
    match &e {
        rusqlite::Error::SqliteFailure(err, Some(_sql)) => format!("{context}: {err}"),
        _ => format!("{context}: {e}"),
    }
}

fn with_tx<T>(
    conn: &mut Connection,
    begin_err: &'static str,
    commit_err: &'static str,
    f: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    conn.execute_batch("BEGIN")
        .map_err(|e| sql_err(begin_err, e))?;
    let result = f(conn);
    match &result {
        Ok(_) => conn
            .execute_batch("COMMIT")
            .map_err(|e| sql_err(commit_err, e))?,
        Err(_) => {
            let _ = conn.execute_batch("ROLLBACK");
        }
    }
    result
}

/// 动态字段组内部键。对象属性中以该键存动态字段组数据数组，
/// 字段定义存于 `__fields` 中同名键。界面显示名为「动态字段组 / Dynamic Group」。
pub const DYNAMIC_GROUP_KEY: &str = "__dynamic_group__";

/// 动态字段组键在搜索中的用户可见显示名（与前端 locale `editor:field_types.dynamic_group` 同步）。
const DYNAMIC_GROUP_LABELS: [&str; 2] = ["动态字段组", "Dynamic Group"];

/// 查询（须已小写）命中动态字段组显示名时返回命中显示名；否则 None。
///
/// 搜索约定：内部元数据键/占位 token（`__` 前缀）不按原始文本匹配，
/// 仅 `__dynamic_group__` 例外——按用户可见显示名匹配（搜「动态字段组」能命中，
/// 搜内部键名 `_dynamic_group_` 不应命中）。
pub fn dynamic_group_label_match(query_lower: &str) -> Option<&'static str> {
    DYNAMIC_GROUP_LABELS
        .iter()
        .find(|l| l.to_lowercase().contains(query_lower))
        .copied()
}

/// 是否为内部元数据键/占位 token（`__` 前缀）。
/// 此类键不按原始键名参与搜索匹配，值也按内部占位处理。
pub fn is_internal_metadata_key(key: &str) -> bool {
    key.starts_with("__")
}

/// P210: 大小写不敏感子串匹配整个 JSON Value（对象键 + 字符串值 + 数字 + 布尔）。
///
/// 旧实现 `value.to_string().to_lowercase()` 每次对每个对象重新序列化 JSON
/// （含引号/花括号/转义/格式化）并整体复制一份小写字符串，属 Value→String 往返浪费。
/// 本函数递归遍历值树，仅对文本片段做小写匹配：
///   - 对象键与字符串值均参与匹配（保持旧实现“键也可命中”的搜索面）；
///   - 内部元数据键（`__` 前缀，如 `__dynamic_group__`/`__fields`/`__attachments`）
///     不按原始键名匹配——否则搜「_dynamic_group_」会命中内部键；其中
///     `__dynamic_group__` 例外，按用户可见显示名匹配（动态字段组 / Dynamic Group），
///     与前端 locale 同步。内部键的值树仍递归进入（`__fields` 中的字段名、
///     `__attachments` 附件名等用户可见数据继续可搜索，仅键名本身不可命中）；
///   - `__` 前缀的字符串值（如 `__fields` 定义中的 `name: "__dynamic_group__"`）
///     视为内部占位 token，不按原始文本匹配；
///   - 数字/布尔按文本形式匹配（与旧序列化结果一致）；
///   - 字符串值按未转义原文匹配——旧实现匹配的是 JSON 转义形态（如值含真实换行时搜索
///     `\\n` 会命中反斜杠+n），新实现匹配真实文本，更符合用户直觉（转义形态命中是
///     序列化的偶然产物，非产品意图）；
///   - null 不再命中字面 `null`（旧序列化为 `null` 文本可命中，搜索 null 属病态用例，
///     缺失可接受）。
///
/// `needle_lower` 必须已小写（调用方统一处理，避免重复 lowercase）。
pub fn json_contains_ignore_case(value: &serde_json::Value, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    match value {
        serde_json::Value::String(s) => {
            // 内部占位 token（`__` 前缀）不参与原始文本匹配——其搜索面由键路径的显示名匹配覆盖。
            if is_internal_metadata_key(s) {
                return false;
            }
            // 长度快速失败，避免短值无谓的 to_lowercase 分配
            s.len() >= needle_lower.len() && s.to_lowercase().contains(needle_lower)
        }
        serde_json::Value::Number(n) => n.to_string().contains(needle_lower),
        serde_json::Value::Bool(b) => b.to_string().contains(needle_lower),
        serde_json::Value::Null => false,
        serde_json::Value::Array(items) => items
            .iter()
            .any(|v| json_contains_ignore_case(v, needle_lower)),
        serde_json::Value::Object(map) => map.iter().any(|(k, v)| {
            if is_internal_metadata_key(k) {
                // 内部元数据键不按原始键名匹配；`__dynamic_group__` 按用户可见显示名匹配。
                (k == DYNAMIC_GROUP_KEY && dynamic_group_label_match(needle_lower).is_some())
                    || json_contains_ignore_case(v, needle_lower)
            } else {
                k.to_lowercase().contains(needle_lower)
                    || json_contains_ignore_case(v, needle_lower)
            }
        }),
    }
}

/// 判断对象 properties 是否包含未软删的附件（供导出范围树等 UI 按附件存在性
/// 决定是否展示附件展开图标）。
///
/// 口径与 `export_get_attachments` 的过滤一致：`__attachments` 数组存在且至少一条
/// 记录的 `deletedAt` 为空/缺失（camelCase 序列化，None → null → as_str 为 None）。
pub fn object_has_attachments(properties: &serde_json::Value) -> bool {
    properties
        .get("__attachments")
        .and_then(|v| v.as_array())
        .is_some_and(|arr| {
            arr.iter().any(|a| {
                a.get("deletedAt")
                    .and_then(|d| d.as_str())
                    .is_none_or(|s| s.is_empty())
            })
        })
}

/// 敏感度等级排序：public(0) < internal(1) < sensitive(2) < critical(3)。未知等级视为 internal(1)。
pub fn sensitivity_rank(level: &str) -> u8 {
    match level {
        "public" => 0,
        "internal" => 1,
        "sensitive" => 2,
        "critical" => 3,
        _ => 1,
    }
}

/// 收集对象字段级敏感度等级集合（去重、按 public < internal < sensitive < critical 升序）。
///
/// 来源优先级（与导出 preflight 判定口径一致）：
/// 1. `property_labels`（field_id → level，对象创建时从模板继承的权威快照）；
/// 2. `properties.__fields` 内嵌 `sensitivityLevel`（模板同步路径写入）；
/// 3. dynamic_group 子项级 `sensitivity`（DynamicGroupEditor 每子项携带）。
///
/// 仅保留已知等级（public/internal/sensitive/critical），未知等级忽略——前端徽章组件
/// 只认识这四档，避免渲染未知 key。供导出范围树展示字段敏感度徽章。
pub fn object_field_sensitivity_levels(
    property_labels: Option<&serde_json::Value>,
    properties: &serde_json::Value,
) -> Vec<String> {
    let mut levels: Vec<String> = Vec::new();

    // 1. property_labels（权威来源）
    if let Some(labels) = property_labels.and_then(|v| v.as_object()) {
        for lvl in labels.values() {
            if let Some(s) = lvl.as_str() {
                push_sensitivity_level(&mut levels, s);
            }
        }
    }

    // 2. __fields 内嵌 sensitivityLevel
    if let Some(fields) = properties.get("__fields").and_then(|v| v.as_object()) {
        for def in fields.values() {
            if let Some(lvl) = def.get("sensitivityLevel").and_then(|v| v.as_str()) {
                push_sensitivity_level(&mut levels, lvl);
            }
        }

        // 3. dynamic_group 子项级 sensitivity——仅当 __fields 中存在 dynamic_group 字段才
        // 扫描对应 properties 键（避免对每个对象全量遍历 properties 的热路径开销）。
        scan_dynamic_group_levels(fields, properties, &mut levels);
    }

    levels.sort_by_key(|l| sensitivity_rank(l));
    levels
}

/// P045: 收集单个敏感度级别（合法值去重）——从 object_field_sensitivity_levels 拆出。
fn push_sensitivity_level(levels: &mut Vec<String>, lvl: &str) {
    if matches!(lvl, "public" | "internal" | "sensitive" | "critical")
        && !levels.iter().any(|l| l == lvl)
    {
        levels.push(lvl.to_string());
    }
}

/// P045: 扫描 dynamic_group 字段的实际子项级 sensitivity——从 object_field_sensitivity_levels
/// 内层拆出，消除「if fields.any → if let Some(props) → for → if → if → if let Some(items)
/// → for → if let Some」8 层嵌套，改为早退守卫 + 平铺扫描。
fn scan_dynamic_group_levels(
    fields: &serde_json::Map<String, serde_json::Value>,
    properties: &serde_json::Value,
    levels: &mut Vec<String>,
) {
    if !fields
        .values()
        .any(|def| def.get("type").and_then(|t| t.as_str()) == Some("dynamic_group"))
    {
        return;
    }
    let Some(props) = properties.as_object() else {
        return;
    };
    for (k, v) in props {
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
        if let Some(items) = v.as_array() {
            for item in items {
                if let Some(lvl) = item.get("sensitivity").and_then(|s| s.as_str()) {
                    push_sensitivity_level(levels, lvl);
                }
            }
        }
    }
}

/// R-4① 方案 2（probe 判定）：探测给定数据密钥能否解密指定 vault.db 中的现有数据。
///
/// 独立只读连接（`PRAGMA query_only`），**不**走 `VaultStore::open`——后者会触发
/// 迁移/一次性回填（`migrate_to_encrypted_format`/`backfill_*`/`repair_restored_objects`），
/// 用探测密钥执行将造成写副作用；probe 必须纯只读、可安全用错误密钥调用。
///
/// 依次尝试 profiles.data / objects.properties / trash_items.data /
/// user_templates.properties_json 的第一行非空加密字段；任一表有非空数据即用该表
/// 判定（解密成功 → true）；全部为空 → true（无数据可证，任何密钥均可）。
///
/// 用途：reencrypt→config 两阶段交换崩溃后，unlock/verify_password 用旧钥与新钥
/// 各探测一次，判断 reencrypt 事务是否已提交（数据是新钥还是旧钥），从而决定
/// promote（完成交换）还是 discard（丢弃 pending）。全有或全无的 reencrypt 保证
/// 单表探测即确定，无歧义。
pub fn probe_data_key(db_path: &std::path::Path, key: &DataEncryptionKey) -> Result<bool, String> {
    // READ_ONLY：文件不存在时直接报错而非创建——probe 必须是纯只读，
    // 连「误建空 db 文件」这类写副作用都不能有。
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open vault for probe: {}", e))?;
    conn.execute_batch("PRAGMA query_only = ON;")
        .map_err(|e| format!("Failed to set query_only for probe: {}", e))?;

    // profiles.data（AES blob）
    if let Some(row) = conn
        .query_row("SELECT data FROM profiles LIMIT 1", [], |r| {
            r.get::<_, Vec<u8>>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
    {
        if !row.is_empty() {
            return Ok(decrypt_field(key, &row).is_ok());
        }
    }
    // objects.properties（加密文本）
    if let Some(props) = conn
        .query_row("SELECT properties FROM objects LIMIT 1", [], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
    {
        if !props.is_empty() {
            return Ok(decrypt_text_field(key, &props).is_ok());
        }
    }
    // trash_items.data（AES blob）
    if let Some(row) = conn
        .query_row("SELECT data FROM trash_items LIMIT 1", [], |r| {
            r.get::<_, Vec<u8>>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
    {
        if !row.is_empty() {
            return Ok(decrypt_field(key, &row).is_ok());
        }
    }
    // user_templates.properties_json（加密文本）
    if let Some(props) = conn
        .query_row(
            "SELECT properties_json FROM user_templates LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    {
        if !props.is_empty() {
            return Ok(decrypt_text_field(key, &props).is_ok());
        }
    }
    Ok(true)
}

/// Vault store with SQLite backing
pub struct VaultStore {
    conn: Mutex<Option<Connection>>,
    config: VaultConfig, // reserved for future path-based vault operations
    state: Mutex<VaultState>,
    data_key: Mutex<Option<DataEncryptionKey>>,
    /// 设备级「同步设置偏好」开关（默认 true=偏好照常同步）。
    /// profiles 同步发送/接收路径实时读取（&self 方法），无需改动 sync crate。
    ui_prefs_sync_enabled: AtomicBool,
}

/// P016: 表驱动整表重写公共 helper。
/// 供 `migrate_to_encrypted_format`（幂等：仅加密未加密/非空行）与 `reencrypt_all`
/// （换钥：全部行解密→重加密）复用。每张表只需 SELECT / UPDATE 语句、表名与一行
/// 转换闭包：闭包读出该行数据，返回要写回的新列值（不含 id——id 取第 0 列自动
/// 追加为 UPDATE 最后一个参数）；返回 `None` 表示跳过该行（不执行 UPDATE）。
fn rewrite_table<F>(
    tx: &rusqlite::Transaction<'_>,
    select_sql: &str,
    update_sql: &str,
    table_name: &str,
    log_progress: bool,
    mut transform: F,
) -> Result<(), String>
where
    F: FnMut(&rusqlite::Row<'_>) -> Result<Option<Vec<rusqlite::types::Value>>, String>,
{
    // 两阶段（SELECT 整表 → 释放 stmt → 再 UPDATE）：rusqlite 不允许同连接
    // 同时持有两个活动语句，保持与原实现一致。
    let mut stmt = tx.prepare(select_sql).map_err(|e| e.to_string())?;
    let rows: Vec<(rusqlite::types::Value, Option<Vec<rusqlite::types::Value>>)> = {
        let mut q = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = q.next().map_err(|e| e.to_string())? {
            let id = row
                .get::<_, rusqlite::types::Value>(0)
                .map_err(|e| e.to_string())?;
            let new_vals = transform(row)?;
            out.push((id, new_vals));
        }
        out
    };
    drop(stmt);
    let rows_len = rows.len();
    let mut update = tx.prepare(update_sql).map_err(|e| e.to_string())?;
    for (id, new_vals) in rows {
        if let Some(mut params) = new_vals {
            params.push(id);
            update
                .execute(rusqlite::params_from_iter(params.iter()))
                .map_err(|e| e.to_string())?;
        }
    }
    if log_progress {
        tracing::info!(
            "reencrypt_progress: table={}, rows={}",
            table_name,
            rows_len
        );
    }
    Ok(())
}

/// P017-②: 单 blob 列表（profiles/trash_items/object_snapshots）整体加密——
/// 已加密/空值跳过，未加密行用 encrypt_field 加密。
fn rewrite_blob_table_encrypted(
    tx: &rusqlite::Transaction<'_>,
    select_sql: &str,
    update_sql: &str,
    table_name: &str,
    key: &DataEncryptionKey,
) -> Result<(), String> {
    rewrite_table(tx, select_sql, update_sql, table_name, false, |row| {
        let data: Vec<u8> = row.get(1).map_err(|e| e.to_string())?;
        if crate::encryption::is_encrypted_blob(&data) || data.is_empty() {
            Ok(None)
        } else {
            let encrypted = encrypt_field(key, &data)?;
            Ok(Some(vec![rusqlite::types::Value::Blob(encrypted)]))
        }
    })
}

/// P017-②: objects.properties / property_labels 双文本列加密。
fn rewrite_objects_encrypted(
    tx: &rusqlite::Transaction<'_>,
    key: &DataEncryptionKey,
) -> Result<(), String> {
    rewrite_table(
        tx,
        "SELECT id, properties, property_labels FROM objects",
        "UPDATE objects SET properties = ?1, property_labels = ?2 WHERE id = ?3",
        "objects",
        false,
        |row| {
            let properties: String = row.get(1).map_err(|e| e.to_string())?;
            let labels: Option<String> = row.get(2).map_err(|e| e.to_string())?;
            let encrypted_props = ensure_encrypted_text(key, &properties)?;
            let encrypted_labels = labels
                .as_deref()
                .map(|l| ensure_encrypted_text(key, l))
                .transpose()?
                .unwrap_or_default();
            Ok(Some(vec![
                rusqlite::types::Value::Text(encrypted_props),
                rusqlite::types::Value::Text(encrypted_labels),
            ]))
        },
    )
}

/// P017-②: user_templates.properties_json 单文本列加密。
fn rewrite_templates_encrypted(
    tx: &rusqlite::Transaction<'_>,
    key: &DataEncryptionKey,
) -> Result<(), String> {
    rewrite_table(
        tx,
        "SELECT id, properties_json FROM user_templates",
        "UPDATE user_templates SET properties_json = ?1 WHERE id = ?2",
        "user_templates",
        false,
        |row| {
            let props_json: String = row.get(1).map_err(|e| e.to_string())?;
            let encrypted = ensure_encrypted_text(key, &props_json)?;
            Ok(Some(vec![rusqlite::types::Value::Text(encrypted)]))
        },
    )
}

/// P017-②: audit_log.details / entity_name 双可选文本列加密。
fn rewrite_audit_log_encrypted(
    tx: &rusqlite::Transaction<'_>,
    key: &DataEncryptionKey,
) -> Result<(), String> {
    rewrite_table(
        tx,
        "SELECT id, details, entity_name FROM audit_log",
        "UPDATE audit_log SET details = ?1, entity_name = ?2 WHERE id = ?3",
        "audit_log",
        false,
        |row| {
            let details: Option<String> = row.get(1).map_err(|e| e.to_string())?;
            let entity_name: Option<String> = row.get(2).map_err(|e| e.to_string())?;
            let encrypted_details = details
                .as_deref()
                .map(|d| ensure_encrypted_text(key, d))
                .transpose()?
                .unwrap_or_default();
            let encrypted_name = entity_name
                .as_deref()
                .map(|n| ensure_encrypted_text(key, n))
                .transpose()?
                .unwrap_or_default();
            Ok(Some(vec![
                rusqlite::types::Value::Text(encrypted_details),
                rusqlite::types::Value::Text(encrypted_name),
            ]))
        },
    )
}

/// P017-②: 写入 encryption_version=1 与迁移时间标记。
fn write_encryption_version_marker(tx: &rusqlite::Transaction<'_>) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('encryption_version', ?1, ?2)",
        params!["1", now],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR REPLACE INTO sys_config (key, value, updated_at) VALUES ('encryption_migrated_at', ?1, ?2)",
        params![chrono::Utc::now().to_rfc3339(), now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

impl VaultStore {
    /// Open or create a vault at the given path
    pub fn open(config: VaultConfig) -> Result<Self, String> {
        let path = config.path.join("vault.db");
        let mut conn =
            Connection::open(&path).map_err(|e| format!("Failed to open vault: {}", e))?;

        // Set busy timeout
        let _: Result<(), _> = conn.query_row("PRAGMA busy_timeout = 5000;", [], |_| Ok(()));

        // P033: 显式启用 WAL 日志模式（幂等，确认而非假设）——读写并发下避免整库
        // 写放大与读阻塞；lock() 收尾已用 wal_checkpoint(TRUNCATE)，与此模式配套。
        // 只读 probe 连接（probe_data_key）在 WAL 下依赖快照隔离读未提交事务前的旧
        // 数据，reencrypt 崩溃恢复判定不受影响。
        let _: Result<_, _> = conn.query_row("PRAGMA journal_mode = WAL;", [], |_| Ok(()));

        // Initialize schema
        Self::init_schema(&conn)?;
        run_migrations(&mut conn)?;

        let data_key = config.data_key.map(DataEncryptionKey::new);
        let store = Self {
            conn: Mutex::new(Some(conn)),
            config,
            state: Mutex::new(VaultState::Unlocked),
            data_key: Mutex::new(data_key),
            ui_prefs_sync_enabled: AtomicBool::new(true),
        };

        // Migrate plaintext legacy data to encrypted format on first open.
        store.migrate_to_encrypted_format()?;

        // 一次性补齐旧对象缺失的初始 snapshot，使历史 badge 能正常显示。
        // 仅在 Vault 已解锁（有 data_key）时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.backfill_missing_snapshots();
        }

        // 修复旧版 object_restore 因字段名大小写不一致导致的“隐形”对象。
        // 仅在 Vault 已解锁时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.repair_restored_objects();
        }

        // 为旧版恢复对象补齐丢失的字段级敏感度副本 property_labels。
        // 仅在 Vault 已解锁时执行；已标记过的 Vault 会自动跳过。
        if store.data_key().is_ok() {
            let _ = store.backfill_missing_property_labels();
        }

        Ok(store)
    }

    pub fn base_path(&self) -> &std::path::Path {
        &self.config.path
    }

    /// 设置设备级「同步设置偏好」开关（由 src-tauri 层调用）。
    /// profiles 同步发送/接收路径实时读取，开关切换后即刻生效。
    pub fn set_ui_prefs_sync_enabled(&self, enabled: bool) {
        self.ui_prefs_sync_enabled.store(enabled, Ordering::SeqCst);
    }

    /// 读取设备级「同步设置偏好」开关（默认 true=同步偏好）。
    pub fn ui_prefs_sync_enabled(&self) -> bool {
        self.ui_prefs_sync_enabled.load(Ordering::SeqCst)
    }

    fn data_key(&self) -> Result<DataEncryptionKey, String> {
        let guard = self.data_key.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Vault data key not available".to_string())
    }

    /// 判断表中是否存在指定列（R2-15：增量迁移前先探测，不再用 `let _ =` 吞掉全部
    /// ALTER TABLE 错误——重复加列属预期，其余错误须可见）。
    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
        let sql = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        while let Some(col) = rows.next().transpose().map_err(|e| e.to_string())? {
            if col == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn init_schema(conn: &Connection) -> Result<(), String> {
        Self::create_schema_tables(conn)?;
        Self::migrate_missing_columns(conn)?;
        Self::init_data_version(conn)?;
        Ok(())
    }

    /// 建表：一次性执行全部 CREATE TABLE / CREATE INDEX。
    fn create_schema_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_profile_name ON profiles(name);

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                details TEXT,
                entity_type TEXT,
                entity_id TEXT,
                entity_name TEXT,
                performed_by TEXT DEFAULT 'user'
            );

            CREATE TABLE IF NOT EXISTS sync_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_peers (
                peer_node_id TEXT PRIMARY KEY,
                peer_name TEXT,
                trusted INTEGER NOT NULL DEFAULT 0,
                public_key_fingerprint TEXT,
                last_seen INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_watermarks (
                peer_node_id TEXT NOT NULL,
                table_name TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL DEFAULT 0,
                counter INTEGER NOT NULL DEFAULT 0,
                node_id TEXT NOT NULL DEFAULT '',
                cursor_id TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (peer_node_id, table_name)
            );

            CREATE TABLE IF NOT EXISTS sync_hlc (
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL,
                counter INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (table_name, record_id)
            );

            CREATE TABLE IF NOT EXISTS sync_tombstones (
                table_name TEXT NOT NULL,
                record_id TEXT NOT NULL,
                wall_time_ms INTEGER NOT NULL,
                counter INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                deleted_by_node_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (table_name, record_id)
            );

            CREATE TABLE IF NOT EXISTS sys_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                type_id TEXT NOT NULL DEFAULT 'note',
                section_type TEXT NOT NULL DEFAULT 'identity',
                name TEXT NOT NULL,
                icon_name TEXT NOT NULL DEFAULT 'document',
                parent_id TEXT,
                children_ids TEXT NOT NULL DEFAULT '[]',
                properties TEXT NOT NULL DEFAULT '{}',
                property_labels TEXT DEFAULT '{}',
                sensitivity_level TEXT NOT NULL DEFAULT 'internal',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT,
                tags_json TEXT NOT NULL DEFAULT '[]',
                template_id TEXT,
                template_type TEXT CHECK(template_type IN ('system', 'user')),
                contract_type_id TEXT,
                template_hash TEXT,
                ignored_template_hash TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_objects_account ON objects(account_id);
            CREATE INDEX IF NOT EXISTS idx_objects_parent ON objects(parent_id);
            CREATE INDEX IF NOT EXISTS idx_objects_type ON objects(type_id);
            CREATE INDEX IF NOT EXISTS idx_objects_deleted ON objects(is_deleted);

            CREATE TABLE IF NOT EXISTS trash_items (
                id TEXT PRIMARY KEY,
                item_type TEXT NOT NULL,
                original_id TEXT NOT NULL,
                original_parent_id TEXT,
                original_section_type TEXT,
                original_sort_order INTEGER,
                data BLOB NOT NULL,
                deleted_at INTEGER NOT NULL,
                expires_at INTEGER,
                deleted_by TEXT NOT NULL DEFAULT 'user',
                name_snapshot TEXT NOT NULL,
                icon_snapshot TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_trash_expires ON trash_items(expires_at);
            CREATE INDEX IF NOT EXISTS idx_trash_deleted_at ON trash_items(deleted_at);
            CREATE INDEX IF NOT EXISTS idx_trash_type ON trash_items(item_type);

            CREATE TABLE IF NOT EXISTS object_snapshots (
                id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                triggered_by TEXT NOT NULL DEFAULT 'user_edit',
                data BLOB NOT NULL,
                diff_summary TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_object ON object_snapshots(object_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON object_snapshots(timestamp);

            CREATE TABLE IF NOT EXISTS guide_embeddings (
                id TEXT PRIMARY KEY,
                guide_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_guide_embeddings_guide ON guide_embeddings(guide_id);
            "#,
        )
        .map_err(|e| format!("Failed to init schema: {}", e))
    }

    /// 列级迁移：补齐历史版本缺失的列（tags_json / section_type）。
    fn migrate_missing_columns(conn: &Connection) -> Result<(), String> {
        // Migration: add tags_json column if missing (added in schema v2, §24)
        if !Self::column_exists(conn, "objects", "tags_json")? {
            conn.execute(
                "ALTER TABLE objects ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )
            .map_err(|e| format!("Failed to add tags_json column: {}", e))?;
        }
        // Migration: add section_type column if missing (§25.1.3)
        if !Self::column_exists(conn, "objects", "section_type")? {
            conn.execute(
                "ALTER TABLE objects ADD COLUMN section_type TEXT NOT NULL DEFAULT 'identity'",
                [],
            )
            .map_err(|e| format!("Failed to add section_type column: {}", e))?;
        }
        Ok(())
    }

    /// 初始化 data_version（仅首次建库时写入）。
    fn init_data_version(conn: &Connection) -> Result<(), String> {
        let version_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sys_config WHERE key = 'data_version')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !version_exists {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO sys_config (key, value, updated_at) VALUES ('data_version', '1', ?1)",
                params![now],
            )
            .map_err(|e| format!("Failed to init data_version: {}", e))?;
        }
        Ok(())
    }

    pub fn state(&self) -> VaultState {
        *self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stats(&self) -> Result<VaultStats, String> {
        let guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("Vault is locked")?;
        let profile_count: usize = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        // Profiles data
        let profiles_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM profiles",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Objects properties
        let objects_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(properties)), 0) FROM objects",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Trash data
        let trash_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM trash_items",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Snapshots data
        let snapshots_size: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM object_snapshots",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let last_modified: Option<String> = conn
            .query_row("SELECT MAX(updated_at) FROM profiles", [], |r| r.get(0))
            .ok();

        Ok(VaultStats {
            profile_count,
            total_size_bytes: profiles_size + objects_size + trash_size + snapshots_size,
            last_modified,
            profiles_size,
            objects_size,
            trash_size,
            snapshots_size,
            attachments_size: 0, // filled in by get_vault_stats command
            ai_conversations_size: 0,
        })
    }

    pub fn lock(&self) {
        if let Ok(mut guard) = self.conn.lock() {
            if let Some(conn) = guard.take() {
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
            }
        }
        if let Ok(mut key) = self.data_key.lock() {
            if let Some(mut k) = key.take() {
                k.0.zeroize();
            }
        }
        if let Ok(mut s) = self.state.lock() {
            *s = VaultState::Locked;
        }
    }

    /// Migrate legacy plaintext sensitive fields to encrypted format.
    /// Triggered automatically on first open where encryption_version < 1.
    pub fn migrate_to_encrypted_format(&self) -> Result<(), String> {
        let encryption_version: u32 = self
            .get_sys_config("encryption_version")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if encryption_version >= 1 {
            return Ok(());
        }

        let key = self.data_key()?;

        // Backup the database file before migration.
        let db_path = self.config.path.join("vault.db");
        let backup_path = self.config.path.join("vault.db.pre_enc.bak");
        if db_path.exists() {
            if let Err(e) = std::fs::copy(&db_path, &backup_path) {
                tracing::error!(
                    "Failed to backup vault db before encryption migration: {}",
                    e
                );
                return Err(format!("Migration backup failed: {}", e));
            }
        }

        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_mut().ok_or("Vault is locked")?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let result: Result<(), String> = (|| {
            // profiles.data（整行加密；已加密/空行跳过）
            rewrite_blob_table_encrypted(
                &tx,
                "SELECT id, data FROM profiles",
                "UPDATE profiles SET data = ?1 WHERE id = ?2",
                "profiles",
                &key,
            )?;

            // objects.properties / property_labels（ensure_encrypted_text 幂等）
            rewrite_objects_encrypted(&tx, &key)?;

            // trash_items.data
            rewrite_blob_table_encrypted(
                &tx,
                "SELECT id, data FROM trash_items",
                "UPDATE trash_items SET data = ?1 WHERE id = ?2",
                "trash_items",
                &key,
            )?;

            // object_snapshots.data
            rewrite_blob_table_encrypted(
                &tx,
                "SELECT id, data FROM object_snapshots",
                "UPDATE object_snapshots SET data = ?1 WHERE id = ?2",
                "object_snapshots",
                &key,
            )?;

            // user_templates.properties_json
            rewrite_templates_encrypted(&tx, &key)?;

            // audit_log.details / entity_name
            rewrite_audit_log_encrypted(&tx, &key)?;

            write_encryption_version_marker(&tx)?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                tx.commit().map_err(|e| e.to_string())?;
                tracing::info!("Vault encryption migration completed successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Vault encryption migration failed: {}", e);
                // Transaction is dropped here, causing rollback.
                Err(format!("Encryption migration failed: {}", e))
            }
        }
    }

    /// N-2: 替换内存中的数据加密密钥（不改写磁盘，仅影响后续读写）。
    /// 供改密/KDF 升级回滚时先读回再写回；调用方负责保持与磁盘数据一致。
    /// 与 `test_reencrypt_all_roundtrip` 内部手动替换行为一致。
    pub fn set_data_key(&self, key: DataEncryptionKey) {
        if let Ok(mut guard) = self.data_key.lock() {
            *guard = Some(key);
        }
    }

    pub fn get_sync_node_id(&self) -> Result<Option<String>, String> {
        self.read_metadata("node_id", "sync")
            .map(|b| b.and_then(|v| String::from_utf8(v).ok()))
    }

    pub fn set_sync_node_id(&self, node_id: &str) -> Result<(), String> {
        self.write_metadata("node_id", "sync", node_id.as_bytes())
    }

    pub fn get_sync_secret_key(&self) -> Result<Option<[u8; 32]>, String> {
        self.read_metadata("secret_key", "sync").map(|b| {
            b.map(|v| {
                let mut key = [0u8; 32];
                let len = v.len().min(32);
                key[..len].copy_from_slice(&v[..len]);
                key
            })
        })
    }

    pub fn set_sync_secret_key(&self, key: &[u8; 32]) -> Result<(), String> {
        self.write_metadata("secret_key", "sync", key)
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests;
