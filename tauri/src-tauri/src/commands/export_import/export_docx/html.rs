//! export_docx 子模块 —— html（P047 拆分）

use super::fields::{collect_attachment_entries, flatten_object_fields};

/// HTML 转义（`& < > " '`）。
pub(crate) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 构造自包含 HTML 文档（内联 CSS，零外部资源）。
///
/// 与 docx 相同的文档结构：封面段 → 每对象（标题/元信息/字段表格/附件清单）。
/// 单文件、可用系统默认浏览器直接打开；打印另存为 PDF 亦可用。
pub(crate) fn build_html_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
    account_name: &str,
    account_id: &str,
) -> String {
    let mut html = String::from(
        "<!DOCTYPE html>\n<html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n\
         <title>SoloSoul 导出</title>\n\
         <style>\n\
         body { font-family: -apple-system, 'PingFang SC', 'Microsoft YaHei', 'Noto Sans SC', sans-serif; \
           max-width: 800px; margin: 0 auto; padding: 32px 24px; color: #1f2328; line-height: 1.6; }\n\
         h1 { font-size: 24px; border-bottom: 2px solid #d0d7de; padding-bottom: 8px; }\n\
         h2 { font-size: 19px; margin-top: 28px; }\n\
         .meta { color: #57606a; font-size: 13px; margin-bottom: 20px; }\n\
         /* PDF：对象间用横线分隔连续排版（不再强制每对象独立页）；azul 固定高度分页 */\n\
         .obj { margin-bottom: 36px; }\n\
         .obj + .obj { border-top: 1px solid #d0d7de; padding-top: 20px; }\n\
         table { border-collapse: collapse; width: 100%; margin-top: 10px; table-layout: fixed; }\n\
         /* 长值在页边距处换行（azul-layout 支持 word-break / overflow-wrap） */\n\
         th, td { border: 1px solid #d0d7de; padding: 8px 10px; text-align: left; vertical-align: top; font-size: 14px; word-break: break-all; overflow-wrap: anywhere; }\n\
         th { background: #f6f8fa; font-weight: 600; width: 30%; white-space: nowrap; }\n\
         .attach { color: #57606a; font-size: 13px; margin: 4px 0 0; word-break: break-all; }\n\
         </style></head><body>\n\
         <h1>SoloSoul</h1>\n\
         <p class=\"meta\">",
    );
    // 第二行：导出账户名 + 账户 ID
    html.push_str(&escape_html(&format!(
        "账户名：{}（{}）",
        account_name, account_id
    )));
    html.push_str("</p>\n");
    html.push_str("<div class=\"meta\">");
    html.push_str(&escape_html(&format!(
        "{} · 导出 {} 个对象",
        export_time,
        records.len()
    )));
    html.push_str("</div>\n");

    for rec in records {
        // 连续排版：对象间由 .obj + .obj 横线分隔（不再强制分页）
        html.push_str("<div class=\"obj\">\n<h2>");
        html.push_str(&escape_html(&rec.name));
        html.push_str("</h2>\n<div class=\"meta\">");
        let tpl_name = rec
            .template_id
            .as_ref()
            .and_then(|tid| template_names.get(tid))
            .cloned()
            .unwrap_or_default();
        let mut meta_parts = Vec::new();
        if !tpl_name.is_empty() {
            meta_parts.push(format!("模板：{}", tpl_name));
        }
        meta_parts.push(format!("创建时间：{}", rec.created_at));
        meta_parts.push(format!("更新时间：{}", rec.updated_at));
        if !rec.tags_json.is_empty() {
            meta_parts.push(format!("标签：{}", rec.tags_json.join(", ")));
        }
        html.push_str(&escape_html(&meta_parts.join("　·　")));
        html.push_str("</div>\n");

        let fields = flatten_object_fields(rec);
        if !fields.is_empty() {
            html.push_str("<table>\n");
            for (label, value, _) in &fields {
                html.push_str("<tr><th>");
                html.push_str(&escape_html(label));
                html.push_str("</th><td>");
                html.push_str(&escape_html(value).replace('\n', "<br>"));
                html.push_str("</td></tr>\n");
            }
            html.push_str("</table>\n");
        }

        let attachments = collect_attachment_entries(rec);
        if !attachments.is_empty() {
            for entry in &attachments {
                html.push_str("<p class=\"attach\">");
                html.push_str(&escape_html(&format!("附件：{}", entry.main)));
                html.push_str("</p>\n");
                if let Some(desc) = &entry.description {
                    html.push_str("<p class=\"attach\">");
                    html.push_str(&escape_html(&format!("　描述：{}", desc)));
                    html.push_str("</p>\n");
                }
                if !entry.tags.is_empty() {
                    html.push_str("<p class=\"attach\">");
                    html.push_str(&escape_html(&format!("　标签：{}", entry.tags.join("、"))));
                    html.push_str("</p>\n");
                }
            }
        }
        html.push_str("</div>\n");
    }

    html.push_str("</body></html>\n");
    html
}
