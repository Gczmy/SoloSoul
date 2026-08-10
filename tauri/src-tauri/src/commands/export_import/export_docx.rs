//! 对象级文档导出（Word/docx）— 设计文档 docs/next_dev/对象级文档导出功能设计与实现.md
//!
//! - 将选中对象以多页形式导出为一个 docx（每个对象占一页）。
//! - docx 本质是 ZIP + OOXML，复用 workspace 已有的 `zip` crate，零新依赖。
//! - 附件不嵌入正文，仅以「附件清单」小节列出名称/大小/类型。
//! - 敏感字段确认后全量明文写入（前端先经 preflight 分级确认）。

use super::*;
use std::io::Write;

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
fn escape_xml(s: &str) -> String {
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
fn sanitize_docx_text(s: &str) -> String {
    s.chars()
        .filter(|&c| c >= '\u{20}' || c == '\t' || c == '\n' || c == '\r')
        .collect()
}

/// 将字段值渲染为纯文本。
///
/// - 数组：join(", ")（与前端 flattenProperties 一致）；
/// - dynamic_group：每个子字段独立一行 `名称：值`；
/// - 其他：JSON 字符串化。
fn field_value_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(items) => {
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
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// 字段定义元信息（__fields 中每个字段的 name / type）。
#[derive(Default)]
struct FieldDefMeta {
    name: String,
    ftype: String,
}

/// 拍平对象字段（跳过 `__` 内部键），返回 (字段标签, 字段值文本) 列表。
/// 字段顺序与前端 flattenProperties 一致：properties Map 迭代序（保留 __fields 定义序）。
///
/// dynamic_group 字段：子字段展开为独立条目（label=子字段 name，value=子字段 value），
/// 与前端 objectDetailUtils.flattenProperties 行为一致；组名/占位符不单独成行。
fn flatten_object_fields(record: &solosoul_vault::ObjectRecord) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(props) = record.properties.as_object() else {
        return out;
    };
    // 字段定义：优先 __fields 定义的 name/type，回退键名本身
    let field_meta: std::collections::HashMap<String, FieldDefMeta> = record
        .properties
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
        .unwrap_or_default();

    for (k, v) in props {
        if k.starts_with("__") {
            continue;
        }
        let meta = field_meta.get(k);
        // dynamic_group：子字段展开为独立条目（与前端 flattenProperties 一致）
        if meta.map(|m| m.ftype.as_str()) == Some("dynamic_group") {
            if let serde_json::Value::Array(items) = v {
                for item in items {
                    if let Some(obj) = item.as_object() {
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
                        out.push((name, value));
                    }
                }
            }
            continue;
        }
        let text = field_value_to_text(v);
        if text.is_empty() {
            continue;
        }
        let label = meta.map(|m| m.name.clone()).unwrap_or_else(|| k.clone());
        out.push((label, text));
    }
    out
}

/// 附件清单（名称 / 大小 / 类型），仅未软删附件。
fn collect_attachment_rows(record: &solosoul_vault::ObjectRecord) -> Vec<(String, String, String)> {
    load_attachments(&record.properties)
        .into_iter()
        .filter(|a| a.deleted_at.is_none())
        .map(|a| (a.file_name.clone(), format_bytes(a.size_bytes), a.mime_type))
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

/// 将文本渲染为 run 内多 `<w:t>` + `<w:br/>`：Word 中 `<w:t>` 内的 `\n` 不换行，
/// 需用 `<w:br/>` 显式拆分（多行字段值、dynamic_group 展开值均受益）。
fn text_run(text: &str) -> String {
    // CRLF/CR：XML 解析器会把 \r 规范化为 \n（<w:t> 内不换行），先剔除
    let safe = escape_xml(&sanitize_docx_text(text)).replace('\r', "");
    let mut out = String::from("<w:r>");
    for (i, line) in safe.split('\n').enumerate() {
        if i > 0 {
            out.push_str("<w:br/>");
        }
        out.push_str("<w:t xml:space=\"preserve\">");
        out.push_str(line);
        out.push_str("</w:t>");
    }
    out.push_str("</w:r>");
    out
}

/// 构造 docx 包的最小 OOXML 结构，返回 zip 字节。
///
/// 文档结构（多对象 = 多页）：
/// 1. 封面段：应用名、导出时间、对象总数；
/// 2. 每个对象：分页符 → 对象名(H1) → 元信息段 → 字段表格 → 附件清单小节。
fn build_docx(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
) -> Result<Vec<u8>, String> {
    let mut document = String::new();
    document.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n\
         <w:body>\n",
    );

    // 1. 封面/标题段
    document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>");
    document.push_str(&text_run("SoloSoul"));
    document.push_str("</w:p>\n");
    document.push_str("<w:p>");
    document.push_str(&text_run(&format!("{} · {}", export_time, records.len())));
    document.push_str("</w:p>\n");

    for (idx, rec) in records.iter().enumerate() {
        if idx > 0 {
            // 分页符：第二个对象起每个对象前插入
            document.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>\n");
        }
        // 对象名
        document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>");
        document.push_str(&text_run(&rec.name));
        document.push_str("</w:p>\n");

        // 元信息段
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
        for line in &meta_lines {
            document.push_str("<w:p>");
            document.push_str(&text_run(line));
            document.push_str("</w:p>\n");
        }

        // 字段表格（两列：标签 / 值）
        let fields = flatten_object_fields(rec);
        if !fields.is_empty() {
            document.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
            for (label, value) in &fields {
                document.push_str(
                    "<w:tr><w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr><w:p>",
                );
                document.push_str(&text_run(label));
                document.push_str(
                    "</w:p></w:tc><w:tc><w:tcPr><w:tcW w:w=\"5000\" w:type=\"dxa\"/></w:tcPr><w:p>",
                );
                document.push_str(&text_run(value));
                document.push_str("</w:p></w:tc></w:tr>\n");
            }
            document.push_str("</w:tbl>\n");
        }

        // 附件清单小节
        let attachments = collect_attachment_rows(rec);
        if !attachments.is_empty() {
            document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t>");
            document.push_str("附件清单");
            document.push_str("</w:t></w:r></w:p>\n");
            for (name, size, mime) in &attachments {
                document.push_str("<w:p>");
                document.push_str(&text_run(&format!("{}（{}，{}）", name, size, mime)));
                document.push_str("</w:p>\n");
            }
        }
    }

    document.push_str("</w:body>\n</w:document>\n");

    // styles.xml：最小样式集（Heading1 / Heading2 / 正文）
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:pPr><w:spacing w:before="200" w:after="100"/></w:pPr><w:rPr><w:b/><w:sz w:val="26"/></w:rPr></w:style>
</w:styles>
"#;

    // 组装 zip
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("[Content_Types].xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>
"#
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;

        zip.start_file("_rels/.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;

        zip.start_file("word/document.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(document.as_bytes())
            .map_err(|e| e.to_string())?;

        zip.start_file("word/styles.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(styles.as_bytes())
            .map_err(|e| e.to_string())?;

        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// HTML 转义（`& < > " '`）。
fn escape_html(s: &str) -> String {
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
fn build_html_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
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
         .obj { margin-bottom: 36px; page-break-inside: avoid; }\n\
         .obj + .obj { border-top: 1px solid #d0d7de; padding-top: 20px; }\n\
         table { border-collapse: collapse; width: 100%; margin-top: 10px; }\n\
         th, td { border: 1px solid #d0d7de; padding: 8px 10px; text-align: left; vertical-align: top; font-size: 14px; }\n\
         th { background: #f6f8fa; font-weight: 600; width: 30%; white-space: nowrap; }\n\
         .attach { color: #57606a; font-size: 13px; margin: 4px 0 0; }\n\
         </style></head><body>\n\
         <h1>SoloSoul</h1>\n\
         <div class=\"meta\">",
    );
    html.push_str(&escape_html(&format!(
        "{} · {} 个对象",
        export_time,
        records.len()
    )));
    html.push_str("</div>\n");

    for rec in records {
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
            for (label, value) in &fields {
                html.push_str("<tr><th>");
                html.push_str(&escape_html(label));
                html.push_str("</th><td>");
                html.push_str(&escape_html(value).replace('\n', "<br>"));
                html.push_str("</td></tr>\n");
            }
            html.push_str("</table>\n");
        }

        let attachments = collect_attachment_rows(rec);
        if !attachments.is_empty() {
            for (name, size, mime) in &attachments {
                html.push_str("<p class=\"attach\">");
                html.push_str(&escape_html(&format!(
                    "附件：{}（{}，{}）",
                    name, size, mime
                )));
                html.push_str("</p>\n");
            }
        }
        html.push_str("</div>\n");
    }

    html.push_str("</body></html>\n");
    html
}

/// 构造 PDF（HTML → printpdf from_html，内嵌 Noto Sans SC 中文字体）。
///
/// 字体字节经 `include_bytes!` 嵌入二进制（打包进应用；发布时随主程序分发，
/// 无需额外资源文件）。`PdfSaveOptions` 默认 `subset_fonts=true`，PDF 只嵌用到的字形。
fn build_pdf_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
) -> Result<Vec<u8>, String> {
    use printpdf::Base64OrRaw;

    let html = build_html_document(records, template_names, export_time);
    let mut fonts = std::collections::BTreeMap::new();
    // 字体名需与 build_html_document 的 font-family 列表一致（Noto Sans SC 兜底链中命中）
    fonts.insert(
        "Noto Sans SC".to_string(),
        Base64OrRaw::Raw(
            include_bytes!("../../../resources/fonts/NotoSansSC-Regular.otf").to_vec(),
        ),
    );

    let mut warnings = Vec::new();
    let pdf = printpdf::PdfDocument::from_html(
        &html,
        &std::collections::BTreeMap::new(),
        &fonts,
        &printpdf::GeneratePdfOptions::default(),
        &mut warnings,
    )
    .map_err(|e| format!("PDF render failed: {}", e))?;

    // 渲染警告记录日志（不阻断导出）
    if !warnings.is_empty() {
        tracing::warn!(
            "[export_docx] PDF render warnings: {} item(s)",
            warnings.len()
        );
    }

    let bytes = pdf.save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
    if bytes.is_empty() {
        return Err("PDF render produced empty output".to_string());
    }
    Ok(bytes)
}

/// 加载对象记录并按前端传入顺序返回（空列表报错）。
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

/// 导出格式对应的主扩展名（不含点）。
fn format_extension(format: &str) -> Option<&'static str> {
    match format {
        "docx" => Some("docx"),
        "pdf" => Some("pdf"),
        "html" => Some("html"),
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

/// 导出对象为文档（docx / pdf / html）并落盘。
///
/// - `format` 支持 `"docx"` / `"pdf"` / `"html"`。
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

    // 解析保存路径（白名单校验在 resolve_document_path 内）——提前做，避免把无效路径带进阻塞任务。
    let document_path = resolve_document_path(&app, &save_path, &format)?;

    // 对象解密 + 文档生成 + 写盘（临时文件 + rename；Unix chmod 600）整体移入
    // spawn_blocking，避免大对象集全表 AES 解密与文件写入阻塞 tokio worker（P114 同款）。
    // vault 是 Arc，闭包内 clone 一份；闭包外保留原句柄供审计日志使用。
    let vault_for_task = vault.clone();
    let format_for_task = format.clone();
    let (object_count, file_size_bytes) = tokio::task::spawn_blocking(move || {
        let records = load_records_in_order(&vault_for_task, &object_ids)?;
        let template_names = load_template_names(&vault_for_task, &records);
        let bytes = match format_for_task.as_str() {
            "docx" => build_docx(&records, &template_names, &export_time)?,
            "html" => build_html_document(&records, &template_names, &export_time).into_bytes(),
            "pdf" => build_pdf_document(&records, &template_names, &export_time)?,
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
    let _ = vault.log_structured(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `fields` 仅供调用方语义表达（字段定义实际经 props.__fields 注入），参数保留签名一致性。
    fn make_record(
        id: &str,
        name: &str,
        _fields: serde_json::Value,
        props: serde_json::Value,
    ) -> solosoul_vault::ObjectRecord {
        solosoul_vault::ObjectRecord {
            id: id.to_string(),
            account_id: "acc".to_string(),
            type_id: "identity".to_string(),
            section_type: "identity".to_string(),
            name: name.to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: props,
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            contract_type_id: None,
            template_hash: None,
            ignored_template_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
            version: 1,
        }
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(
            escape_xml("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }

    #[test]
    fn test_sanitize_docx_text() {
        // 保留 \t \n \r 与可打印字符，剔除其他控制字符
        let s = "a\tb\nc\rd\u{01}e";
        assert_eq!(sanitize_docx_text(s), "a\tb\nc\rde");
    }

    #[test]
    fn test_field_value_to_text_dynamic_group() {
        let v = serde_json::json!([
            {"id": "1", "name": "姓", "value": "张"},
            {"id": "2", "name": "名", "value": "三"}
        ]);
        assert_eq!(field_value_to_text(&v), "姓：张\n名：三");
    }

    #[test]
    fn test_flatten_skips_internal_keys() {
        let fields = serde_json::json!({
            "f1": {"name": "姓名"},
            "f2": {"name": "备注"}
        });
        let props = serde_json::json!({
            "f1": "张三",
            "f2": "hello & world",
            "__fields": fields,
            "__attachments": []
        });
        let rec = make_record("o1", "对象", fields, props);
        let flat = flatten_object_fields(&rec);
        assert_eq!(flat.len(), 2);
        assert!(flat.iter().any(|(l, v)| l == "姓名" && v == "张三"));
        // XML 转义在 build 阶段处理，拍平保留原始值
        assert!(flat
            .iter()
            .any(|(l, v)| l == "备注" && v == "hello & world"));
    }

    #[test]
    fn test_flatten_dynamic_group_expands_children() {
        // dynamic_group 字段（id 为 __dynamic_group__ 或自定义 id）：子字段展开为独立条目，
        // 不显示组名占位符（与前端 objectDetailUtils.flattenProperties 一致）。
        let fields = serde_json::json!({
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group"
            }
        });
        let props = serde_json::json!({
            "contactMethods": [
                {"id": "c1", "name": "新字段", "type": "text", "value": "0"},
                {"id": "c2", "name": "新字段2", "type": "phone", "value": "123"}
            ],
            "f1": "普通字段",
            "__fields": fields,
            "__attachments": []
        });
        let rec = make_record("o1", "对象", fields, props);
        let flat = flatten_object_fields(&rec);
        // 两个子字段展开 + 一个普通字段（f1 无 __fields 定义，label 回退键名），
        // 无 __dynamic_group__ 占位符条目
        assert_eq!(flat.len(), 3);
        assert!(flat.iter().any(|(l, v)| l == "新字段" && v == "0"));
        assert!(flat.iter().any(|(l, v)| l == "新字段2" && v == "123"));
        assert!(flat.iter().any(|(l, v)| l == "f1" && v == "普通字段"));
        assert!(!flat
            .iter()
            .any(|(l, _)| l == "__dynamic_group__" || l == "联系方式"));
    }

    #[test]
    fn test_object_max_sensitivity_scans_dynamic_group_children() {
        // 子项级 sensitivity：DynamicGroupEditor 每子项携带，preflight 必须纳入判定，
        // 否则标 critical/sensitive 的子项会被静默明文导出。
        let fields = serde_json::json!({
            "contactMethods": {
                "name": "联系方式",
                "type": "dynamic_group",
                "sensitivityLevel": "internal"
            }
        });
        let props = serde_json::json!({
            "contactMethods": [
                {"id": "c1", "name": "手机", "type": "phone", "sensitivity": "critical", "value": "123"},
                {"id": "c2", "name": "邮箱", "type": "email", "sensitivity": "internal", "value": "a@b.c"}
            ],
            "__fields": fields
        });
        let rec = make_record("o1", "对象", fields.clone(), props);
        assert_eq!(
            object_max_sensitivity(&rec, None),
            DocumentSensitivity::Critical
        );
        // 仅敏感子项 → Sensitive
        let props2 = serde_json::json!({
            "contactMethods": [
                {"id": "c1", "name": "手机", "type": "phone", "sensitivity": "sensitive", "value": "123"}
            ],
            "__fields": fields.clone()
        });
        let rec2 = make_record("o2", "对象2", fields.clone(), props2);
        assert_eq!(
            object_max_sensitivity(&rec2, None),
            DocumentSensitivity::Sensitive
        );
        // 全部 internal/public → None
        let props3 = serde_json::json!({
            "contactMethods": [
                {"id": "c1", "name": "手机", "type": "phone", "sensitivity": "public", "value": "123"}
            ],
            "__fields": fields.clone()
        });
        let rec3 = make_record("o3", "对象3", fields, props3);
        assert_eq!(
            object_max_sensitivity(&rec3, None),
            DocumentSensitivity::None
        );
    }

    #[test]
    fn test_text_run_splits_newlines() {
        // Word 中 <w:t> 内 \n 不换行，text_run 用 <w:br/> 拆分
        let xml = text_run("a\nb");
        assert!(xml.contains("<w:br/>"));
        assert!(xml.contains("<w:t xml:space=\"preserve\">a</w:t>"));
        assert!(xml.contains("<w:t xml:space=\"preserve\">b</w:t>"));
        // 无换行时不含 <w:br/>
        assert!(!text_run("single").contains("<w:br/>"));
        // XML 转义仍然生效
        assert!(text_run("a&b").contains("a&amp;b"));
    }

    #[test]
    fn test_object_max_sensitivity_from_fields() {
        let fields = serde_json::json!({
            "f1": {"name": "a", "sensitivityLevel": "public"},
            "f2": {"name": "b", "sensitivityLevel": "critical"}
        });
        let rec = make_record(
            "o1",
            "对象",
            fields.clone(),
            serde_json::json!({"__fields": fields}),
        );
        assert_eq!(
            object_max_sensitivity(&rec, None),
            DocumentSensitivity::Critical
        );
    }

    #[test]
    fn test_object_max_sensitivity_from_property_labels() {
        // P221: property_labels 是新建对象的敏感度权威来源（__fields 创建时不注入
        // sensitivityLevel），preflight 必须纳入判定，否则敏感字段被静默明文导出。
        let mut rec = make_record("o1", "对象", serde_json::json!({}), serde_json::json!({}));
        rec.property_labels = Some(serde_json::json!({"f1": "sensitive"}));
        assert_eq!(
            object_max_sensitivity(&rec, None),
            DocumentSensitivity::Sensitive
        );
        rec.property_labels = Some(serde_json::json!({"f1": "critical"}));
        assert_eq!(
            object_max_sensitivity(&rec, None),
            DocumentSensitivity::Critical
        );
        // 仅 public/internal → none
        rec.property_labels = Some(serde_json::json!({"f1": "public"}));
        assert_eq!(
            object_max_sensitivity(&rec, None),
            DocumentSensitivity::None
        );
    }

    #[test]
    fn test_object_max_sensitivity_from_template() {
        let rec = make_record("o1", "对象", serde_json::json!({}), serde_json::json!({}));
        let tpl = solosoul_vault::UserTemplate {
            id: "t1".to_string(),
            account_id: "acc".to_string(),
            name: "模板".to_string(),
            icon_id: None,
            properties: vec![solosoul_vault::TemplateProperty {
                id: "f1".to_string(),
                name: "a".to_string(),
                prop_type: solosoul_vault::PropertyType::Text,
                sensitivity_level: Some("sensitive".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            }],
            category: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            contract_type_id: None,
        };
        assert_eq!(
            object_max_sensitivity(&rec, Some(&tpl)),
            DocumentSensitivity::Sensitive
        );
    }

    #[test]
    fn test_build_docx_structure() {
        let fields = serde_json::json!({"f1": {"name": "姓名"}});
        let rec = make_record(
            "o1",
            "张三&档案",
            fields.clone(),
            serde_json::json!({
                "f1": "张三",
                "__fields": fields,
                "__attachments": [
                    {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf"}
                ]
            }),
        );
        let mut tpl_names = std::collections::HashMap::new();
        tpl_names.insert("t1".to_string(), "护照".to_string());
        let bytes = build_docx(&[rec], &tpl_names, "2026-08-10T00:00:00Z").unwrap();

        // zip 可读且包含必需部件
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        assert!(archive.by_name("[Content_Types].xml").is_ok());
        assert!(archive.by_name("_rels/.rels").is_ok());
        assert!(archive.by_name("word/document.xml").is_ok());
        assert!(archive.by_name("word/styles.xml").is_ok());

        let mut doc = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc)
            .unwrap();
        assert!(doc.contains("张三&amp;档案"));
        assert!(doc.contains("证件.pdf"));
        assert!(doc.contains("2.0 KB"));
    }

    #[test]
    fn test_format_extension_and_path_resolution() {
        assert_eq!(format_extension("docx"), Some("docx"));
        assert_eq!(format_extension("pdf"), Some("pdf"));
        assert_eq!(format_extension("html"), Some("html"));
        assert_eq!(format_extension("txt"), None);

        // 已带扩展名 → 原样；否则追加
        assert!(path_has_format_ext("a.PDF", "pdf"));
        assert!(!path_has_format_ext("a", "pdf"));
        assert!(path_has_format_ext("a.html", "html"));
        assert!(path_has_format_ext("a.htm", "html")); // 保存对话框允许 .htm
        assert!(!path_has_format_ext("a.html", "docx"));
    }

    #[test]
    fn test_build_html_document_structure() {
        let fields = serde_json::json!({"f1": {"name": "姓名"}});
        let rec = make_record(
            "o1",
            "张三&档案",
            fields.clone(),
            serde_json::json!({
                "f1": "张三\n第二行",
                "__fields": fields,
                "__attachments": [
                    {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf"}
                ]
            }),
        );
        let mut tpl_names = std::collections::HashMap::new();
        tpl_names.insert("t1".to_string(), "护照".to_string());
        let html = build_html_document(&[rec], &tpl_names, "2026-08-10T00:00:00Z");

        // 自包含：含 DOCTYPE 与内联 style
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<style>"));
        // HTML 转义：& → &amp;
        assert!(html.contains("张三&amp;档案"));
        // 字段表格 + 多行值换行 <br>
        assert!(html.contains("<th>姓名</th>"));
        assert!(html.contains("张三<br>第二行"));
        // 附件清单
        assert!(html.contains("附件：证件.pdf（2.0 KB，application/pdf）"));
    }

    #[test]
    fn test_build_pdf_document_produces_pdf() {
        // PDF 渲染链路（printpdf from_html + 内嵌 Noto Sans SC）：验证产物为合法 PDF。
        // 注意：本机 Windows 测试二进制因预先存在的 0xc0000139 无法启动（设计文档 §9），
        // 本用例在 CI（ubuntu）运行。
        let fields = serde_json::json!({"f1": {"name": "姓名"}});
        let rec = make_record(
            "o1",
            "张三",
            fields.clone(),
            serde_json::json!({"f1": "张三", "__fields": fields}),
        );
        let bytes = build_pdf_document(&[rec], &std::collections::HashMap::new(), "t").unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..4], b"%PDF");
        assert!(bytes.windows(4).any(|w| w == b"%%EOF"));
    }

    #[test]
    fn test_build_docx_page_breaks() {
        let empty = serde_json::json!({});
        let r1 = make_record("o1", "一", empty.clone(), empty.clone());
        let r2 = make_record("o2", "二", empty.clone(), empty.clone());
        let r3 = make_record("o3", "三", empty.clone(), empty.clone());
        let bytes = build_docx(&[r1, r2, r3], &std::collections::HashMap::new(), "t").unwrap();
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc)
            .unwrap();
        // 3 个对象 → 2 个分页符
        assert_eq!(doc.matches("w:type=\"page\"").count(), 2);
    }

    #[test]
    fn test_empty_object_ids_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = solosoul_vault::VaultConfig::new("acc", dir.path().to_path_buf())
            .with_data_key([0x42u8; 32]);
        let vault = solosoul_vault::VaultStore::open(config).unwrap();
        let err = load_records_in_order(&vault, &[]).unwrap_err();
        assert!(err.contains("NO_OBJECTS_SELECTED"));
    }
}
