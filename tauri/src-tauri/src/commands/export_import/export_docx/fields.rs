//! export_docx 子模块 —— fields（P047 拆分）

use super::*;

pub(crate) fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// 过滤 OOXML 不允许的控制字符（<0x20 除 \t \n \r 外全部剔除）。
pub(crate) fn sanitize_docx_text(s: &str) -> String {
    s.chars()
        .filter(|&c| c >= '\u{20}' || c == '\t' || c == '\n' || c == '\r')
        .collect()
}

/// 将字段值渲染为纯文本。
///
/// - 数组：join(", ")（与前端 flattenProperties 一致）；
/// - dynamic_group：每个子字段独立一行 `名称：值`；
/// - 其他：JSON 字符串化。
pub(crate) fn field_value_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(items) => field_array_to_text(items),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// 数组字段文本化（P014：field_value_to_text 的 Array 分支抽离，消除 match→if→for→if-let 嵌套）：
/// - 全部元素为对象（dynamic_group）：每个子字段独立一行 `名称：值`；
/// - 其他：逐元素递归文本化后用逗号连接。
fn field_array_to_text(items: &[serde_json::Value]) -> String {
    let is_dynamic_group = items.iter().all(|v| v.is_object());
    if is_dynamic_group {
        let mut lines = Vec::new();
        for item in items {
            if let Some(obj) = item.as_object() {
                let name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = obj
                    .get("value")
                    .map(field_value_to_text)
                    .unwrap_or_default();
                if !name.is_empty() {
                    lines.push(format!("{}：{}", name, value));
                }
            }
        }
        lines.join("\n")
    } else {
        items
            .iter()
            .map(field_value_to_text)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// 字段定义元信息（__fields 中每个字段的 name / type）。
#[derive(Default)]
struct FieldDefMeta {
    name: String,
    ftype: String,
}

/// 拍平对象字段（跳过 `__` 内部键），返回 (字段标签, 字段值文本, 字段类型) 列表。
/// 字段顺序与前端 flattenProperties 一致：properties Map 迭代序（保留 __fields 定义序）。
///
/// dynamic_group 字段：子字段展开为独立条目（label=子字段 name，value=子字段 value），
/// 与前端 objectDetailUtils.flattenProperties 行为一致；组名/占位符不单独成行。
/// 从 `properties.__fields` 提取字段定义元信息（name/type），回退键名本身。
fn build_field_meta(
    properties: &serde_json::Value,
) -> std::collections::HashMap<String, FieldDefMeta> {
    properties
        .get("__fields")
        .and_then(|v| v.as_object())
        .map(|fields| {
            fields
                .iter()
                .map(|(k, def)| {
                    let name = def
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or(k)
                        .to_string();
                    let ftype = def
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();
                    (k.clone(), FieldDefMeta { name, ftype })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// dynamic_group 字段：子字段展开为独立条目（与前端 flattenProperties 一致）。
/// 仅收集 name/value 均非空的子项，字段类型固定为 "text"。
fn flatten_dynamic_group(value: &serde_json::Value, out: &mut Vec<(String, String, String)>) {
    let serde_json::Value::Array(items) = value else {
        return;
    };
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let name = obj
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let value = obj
            .get("value")
            .map(field_value_to_text)
            .unwrap_or_default();
        if value.is_empty() {
            continue;
        }
        out.push((name, value, "text".to_string()));
    }
}

/// 展平对象字段为 (label, text, ftype) 三元组：`__` 元数据键跳过，
/// dynamic_group 展开子字段，普通字段按字段定义取展示名。
pub(crate) fn flatten_object_fields(
    record: &solosoul_vault::ObjectRecord,
) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let Some(props) = record.properties.as_object() else {
        return out;
    };
    let field_meta = build_field_meta(&record.properties);

    for (k, v) in props {
        if k.starts_with("__") {
            continue;
        }
        let meta = field_meta.get(k);
        if meta.map(|m| m.ftype.as_str()) == Some("dynamic_group") {
            flatten_dynamic_group(v, &mut out);
            continue;
        }
        let text = field_value_to_text(v);
        if text.is_empty() {
            continue;
        }
        let label = meta.map(|m| m.name.clone()).unwrap_or_else(|| k.clone());
        let ftype = meta.map(|m| m.ftype.clone()).unwrap_or_default();
        out.push((label, text, ftype));
    }
    out
}

/// 附件清单条目（导出用）：主行（名称（大小，类型））+ 可选描述 + 标签。仅未软删附件。
pub(crate) struct AttachmentExportEntry {
    pub(crate) main: String,
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
}

/// 收集附件清单条目（名称 / 大小 / 类型 / 描述 / 标签）。
///
/// 描述与标签为附件级元数据（`attachment_update_meta` 维护），导出时一并列出；
/// 描述内换行折叠为空格（清单行不承担多行排版）。
pub(crate) fn collect_attachment_entries(
    record: &solosoul_vault::ObjectRecord,
) -> Vec<AttachmentExportEntry> {
    load_attachments(&record.properties)
        .into_iter()
        .filter(|a| a.deleted_at.is_none())
        .map(|a| AttachmentExportEntry {
            main: format!(
                "{}（{}，{}）",
                a.file_name,
                format_bytes(a.size_bytes),
                a.mime_type
            ),
            description: a
                .description
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(|d| d.replace('\n', " ")),
            tags: a.tags.clone(),
        })
        .collect()
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 收集附件清单条目（供 txt/markdown 复用）。
pub(crate) fn attachment_lines(
    record: &solosoul_vault::ObjectRecord,
) -> Vec<AttachmentExportEntry> {
    collect_attachment_entries(record)
}
