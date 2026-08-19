//! export_docx 子模块 —— docx（P047 拆分）

use super::fields::{
    collect_attachment_entries, escape_xml, flatten_object_fields, sanitize_docx_text,
};
use super::*;

pub(crate) fn text_run(text: &str) -> String {
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
/// 文档结构（连续排版）：
/// 1. 封面段：应用名、账户名/ID、导出时间、对象总数；
/// 2. 每个对象：分隔横线（第二个对象起，段落顶部边框）→ 对象名称(H1，带「对象名称：」前缀)
///    → 元信息段 → 字段表格 → 附件清单小节。
pub(crate) fn build_docx(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
    account_name: &str,
    account_id: &str,
) -> Result<Vec<u8>, String> {
    let mut document = String::new();
    document.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n\
         <w:body>\n",
    );

    // 1. 封面/标题段
    push_docx_cover(
        &mut document,
        export_time,
        account_name,
        account_id,
        records.len(),
    );

    for (idx, rec) in records.iter().enumerate() {
        push_docx_object_section(&mut document, idx, rec, template_names);
    }

    document.push_str("</w:body>\n</w:document>\n");

    // styles.xml：最小样式集（Heading1 / Heading2 / 正文）
    let styles = docx_styles_xml();

    // 组装 zip
    assemble_docx_zip(&document, styles)
}

/// P017-④: 封面段——应用名、导出账户名/ID、导出时间与对象总数。
fn push_docx_cover(
    document: &mut String,
    export_time: &str,
    account_name: &str,
    account_id: &str,
    object_count: usize,
) {
    document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>");
    document.push_str(&text_run("SoloSoul"));
    document.push_str("</w:p>\n");
    // 第二行：导出账户名 + 账户 ID
    document.push_str("<w:p>");
    document.push_str(&text_run(&format!(
        "账户名：{}（{}）",
        account_name, account_id
    )));
    document.push_str("</w:p>\n");
    document.push_str("<w:p>");
    document.push_str(&text_run(&format!(
        "{} · 导出 {} 个对象",
        export_time, object_count
    )));
    document.push_str("</w:p>\n");
}

/// P017-④: 单个对象节——分隔横线（idx>0）→ 对象名称(H1) → 元信息段 → 字段表格 → 附件清单。
fn push_docx_object_section(
    document: &mut String,
    idx: usize,
    rec: &solosoul_vault::ObjectRecord,
    template_names: &std::collections::HashMap<String, String>,
) {
    // 连续排版：对象间用横线分隔（第二个对象起，每个对象标题段顶部加边框横线）
    if idx > 0 {
        document.push_str(
            "<w:p><w:pPr><w:pBdr><w:top w:val=\"single\" w:sz=\"8\" w:space=\"8\" w:color=\"999999\"/></w:pBdr></w:pPr></w:p>\n",
        );
    }
    // 对象名：左侧标注「对象名称：」明确对象名语义
    document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>");
    document.push_str(&text_run(&format!("对象名称：{}", rec.name)));
    document.push_str("</w:p>\n");

    // 元信息段
    let meta_lines = build_meta_lines(rec, template_names);
    for line in &meta_lines {
        document.push_str("<w:p>");
        document.push_str(&text_run(line));
        document.push_str("</w:p>\n");
    }

    // 字段表格（两列：标签 / 值）
    let fields = flatten_object_fields(rec);
    if !fields.is_empty() {
        document.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
        for (label, value, _) in &fields {
            document
                .push_str("<w:tr><w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr><w:p>");
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
    let attachments = collect_attachment_entries(rec);
    if !attachments.is_empty() {
        document.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t>");
        document.push_str("附件清单");
        document.push_str("</w:t></w:r></w:p>\n");
        for entry in &attachments {
            document.push_str("<w:p>");
            document.push_str(&text_run(&entry.main));
            document.push_str("</w:p>\n");
            if let Some(desc) = &entry.description {
                document.push_str("<w:p>");
                document.push_str(&text_run(&format!("　描述：{}", desc)));
                document.push_str("</w:p>\n");
            }
            if !entry.tags.is_empty() {
                document.push_str("<w:p>");
                document.push_str(&text_run(&format!("　标签：{}", entry.tags.join("、"))));
                document.push_str("</w:p>\n");
            }
        }
    }
}

/// P017-④: styles.xml 最小样式集（Heading1 / Heading2 / 正文）。
fn docx_styles_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:pPr><w:spacing w:before="200" w:after="100"/></w:pPr><w:rPr><w:b/><w:sz w:val="26"/></w:rPr></w:style>
</w:styles>
"#
}

/// P017-④: 组装 docx zip 包（Content_Types / rels / document / styles 四文件）。
fn assemble_docx_zip(document: &str, styles: &str) -> Result<Vec<u8>, String> {
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
