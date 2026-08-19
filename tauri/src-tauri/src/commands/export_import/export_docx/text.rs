//! export_docx 子模块 —— text（P047 拆分）

use super::build_meta_lines;
use super::fields::{attachment_lines, flatten_object_fields};

/// 构造纯文本文档（UTF-8）：封面段 → 每对象（对象名称 / 元信息 / 字段 / 附件清单）。
/// 对象间以横线分隔（连续排版）；多行字段值整体缩进对齐。
pub(crate) fn build_text_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
    account_name: &str,
    account_id: &str,
) -> String {
    let mut out = String::new();
    out.push_str("SoloSoul\n");
    out.push_str(&format!("账户名：{}（{}）\n", account_name, account_id));
    out.push_str(&format!(
        "{} · 导出 {} 个对象\n",
        export_time,
        records.len()
    ));

    for (idx, rec) in records.iter().enumerate() {
        if idx > 0 {
            out.push_str("\n==================================================\n\n");
        }
        out.push_str(&format!("对象名称：{}\n", rec.name));

        // 元信息段
        let meta_lines = build_meta_lines(rec, template_names);
        for line in &meta_lines {
            out.push_str(line);
            out.push('\n');
        }

        // 字段：`标签：值`，多行值后续行缩进对齐
        let fields = flatten_object_fields(rec);
        if !fields.is_empty() {
            out.push('\n');
            for (label, value, _) in &fields {
                let mut lines = value.split('\n');
                if let Some(first) = lines.next() {
                    out.push_str(&format!("{}：{}\n", label, first));
                }
                let indent = " ".repeat(label.chars().count() + 1);
                for rest in lines {
                    out.push_str(&indent);
                    out.push_str(rest);
                    out.push('\n');
                }
            }
        }

        // 附件清单（主行 + 可选描述/标签子行，缩进对齐）
        let attachments = attachment_lines(rec);
        if !attachments.is_empty() {
            out.push_str("\n附件清单：\n");
            for entry in &attachments {
                out.push_str("  - ");
                out.push_str(&entry.main);
                out.push('\n');
                if let Some(desc) = &entry.description {
                    out.push_str("    描述：");
                    out.push_str(desc);
                    out.push('\n');
                }
                if !entry.tags.is_empty() {
                    out.push_str("    标签：");
                    out.push_str(&entry.tags.join("、"));
                    out.push('\n');
                }
            }
        }
        out.push('\n');
    }
    out
}
