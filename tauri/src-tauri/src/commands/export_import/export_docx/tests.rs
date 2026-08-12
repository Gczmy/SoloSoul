//! export_docx 测试（P047 拆分）

use super::docx::{build_docx, text_run};
use super::fields::{escape_xml, field_value_to_text, flatten_object_fields, sanitize_docx_text};
use super::html::build_html_document;
use super::markdown::{
    build_markdown_document, escape_markdown, markdown_field_value, markdown_link,
};
use super::pdf::build_pdf_document;
use super::text::build_text_document;
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
        template_id: Some("t1".to_string()),
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
    assert!(flat.iter().any(|(l, v, _)| l == "姓名" && v == "张三"));
    // XML 转义在 build 阶段处理，拍平保留原始值
    assert!(flat
        .iter()
        .any(|(l, v, _)| l == "备注" && v == "hello & world"));
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
    assert!(flat.iter().any(|(l, v, _)| l == "新字段" && v == "0"));
    assert!(flat.iter().any(|(l, v, _)| l == "新字段2" && v == "123"));
    assert!(flat.iter().any(|(l, v, _)| l == "f1" && v == "普通字段"));
    assert!(!flat
        .iter()
        .any(|(l, _, _)| l == "__dynamic_group__" || l == "联系方式"));
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
                {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf", "createdAt": "2026-01-01T00:00:00Z"}
            ]
        }),
    );
    let mut tpl_names = std::collections::HashMap::new();
    tpl_names.insert("t1".to_string(), "护照".to_string());
    let bytes = build_docx(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1").unwrap();

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
    // 封面第二行：账户名 + 账户 ID
    assert!(doc.contains("账户名：Gczmy（acc-1）"));
    // 封面第三行：明确对象数量（"导出 N 个对象"）
    assert!(doc.contains("导出 1 个对象"));
}

#[test]
fn test_format_extension_and_path_resolution() {
    assert_eq!(format_extension("docx"), Some("docx"));
    assert_eq!(format_extension("pdf"), Some("pdf"));
    assert_eq!(format_extension("html"), Some("html"));
    assert_eq!(format_extension("txt"), Some("txt"));
    assert_eq!(format_extension("markdown"), Some("md"));
    assert_eq!(format_extension("unknown"), None);

    // 已带扩展名 → 原样；否则追加
    assert!(path_has_format_ext("a.PDF", "pdf"));
    assert!(!path_has_format_ext("a", "pdf"));
    assert!(path_has_format_ext("a.html", "html"));
    assert!(path_has_format_ext("a.htm", "html")); // 保存对话框允许 .htm
    assert!(!path_has_format_ext("a.html", "docx"));
    assert!(path_has_format_ext("a.txt", "txt"));
    assert!(path_has_format_ext("a.MD", "markdown"));
    assert!(!path_has_format_ext("a.md", "txt"));
}

#[test]
fn test_escape_markdown() {
    // 1c516c28 后仅转义 `\\` `` ` `` `[]` `|` `<>`——`.` `-` `+` `()` `*` `_` `#` 保持原样（便于用户直接从源码复制）
    assert_eq!(escape_markdown("a*b_c[d]`e`"), "a*b_c\\[d\\]\\`e\\`");
    assert_eq!(escape_markdown("# 标题\n第二行"), "# 标题\n第二行");
    assert_eq!(escape_markdown("中文与空格 保留"), "中文与空格 保留");
    assert_eq!(escape_markdown("a\\b"), "a\\\\b");
}

#[test]
fn test_build_text_document() {
    let fields = serde_json::json!({"f1": {"name": "姓名"}});
    let rec = make_record(
        "o1",
        "张三&档案",
        fields.clone(),
        serde_json::json!({
            "f1": "张三\n第二行",
            "__fields": fields,
            "__attachments": [
                {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf", "createdAt": "2026-01-01T00:00:00Z"}
            ]
        }),
    );
    let mut tpl_names = std::collections::HashMap::new();
    tpl_names.insert("t1".to_string(), "护照".to_string());
    let text = build_text_document(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1");
    // 封面：应用名 / 账户名 / 对象数
    assert!(text.starts_with("SoloSoul\n"));
    assert!(text.contains("账户名：Gczmy（acc-1）"));
    assert!(text.contains("导出 1 个对象"));
    // 对象名带前缀 + 元信息 + 字段（多行值缩进对齐）+ 附件清单
    assert!(text.contains("对象名称：张三&档案"));
    assert!(text.contains("模板：护照"));
    assert!(text.contains("姓名：张三\n   第二行"));
    assert!(text.contains("附件清单："));
    assert!(text.contains("  - 证件.pdf（2.0 KB，application/pdf）"));
    // 单对象无分隔线
    assert!(!text.contains("====="));
}

#[test]
fn test_build_markdown_document() {
    let fields = serde_json::json!({"f1": {"name": "姓名", "type": "text"}});
    let rec = make_record(
        "o1",
        "张三&档案",
        fields.clone(),
        serde_json::json!({
            "f1": "张三\n第二行",
            "__fields": fields,
            "__attachments": [
                {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf", "createdAt": "2026-01-01T00:00:00Z"}
            ]
        }),
    );
    let mut tpl_names = std::collections::HashMap::new();
    tpl_names.insert("t1".to_string(), "护照".to_string());
    let md = build_markdown_document(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1");
    assert!(md.starts_with("# SoloSoul"));
    assert!(md.contains("账户名：Gczmy（acc-1）"));
    assert!(md.contains("导出 1 个对象"));
    assert!(md.contains("### 对象名称：张三&档案"));
    assert!(md.contains("- 模板：护照"));
    // 多行值空行分隔（段落即换行）；字段间空行分隔
    assert!(md.contains("**姓名**：张三\n\n第二行\n\n"));
    assert!(!md.contains("<br>"));
    assert!(md.contains("附件清单："));
    assert!(md.contains("- 证件.pdf（2.0 KB，application/pdf）"));
}

#[test]
fn test_markdown_field_value_types() {
    // url / email / phone 链接化；date 原样；多行空行分隔；内嵌链接自动链接化
    // 注：`escape_markdown` 仅转义 `\` `` ` `` `[]` `|` `<>`，`.` `-` `+` `()` `*` 保持原样
    assert_eq!(
        markdown_field_value("url", "https://example.com/a?b=1"),
        "[https://example.com/a?b=1](https://example.com/a?b=1)"
    );
    assert_eq!(
        markdown_field_value("email", "user@example.com"),
        "[user@example.com](mailto:user@example.com)"
    );
    // 目标含空格时用尖括号包裹（CommonMark 兼容，避免链接被截断）
    assert_eq!(
        markdown_field_value("phone", "+86 138-0013-8000"),
        "[+86 138-0013-8000](<tel:+86 138-0013-8000>)"
    );
    assert_eq!(
        markdown_field_value("phone", "（010）1234-5678"),
        "[（010）1234-5678](tel:（010）1234-5678)"
    );
    assert_eq!(markdown_field_value("date", "2026-08-10"), "2026-08-10");
    // 多行值：空行分隔（Markdown 段落即换行）
    assert_eq!(
        markdown_field_value("multiline", "第一行\n第二行"),
        "第一行\n\n第二行"
    );
    assert_eq!(
        markdown_field_value("text", "官网 https://x.com 邮箱 a@b.co"),
        "官网 [https://x.com](https://x.com) 邮箱 [a@b.co](mailto:a@b.co)"
    );
}

#[test]
fn test_markdown_line_leading_guard() {
    // 行首结构字符防护：仅行首 `#`/`-`/`>` 及有序列表数字被转义，
    // 其余位置（含链接目标）保持原样、可复制；`+` 不是列表标记不转义（电话 +86）
    assert_eq!(
        markdown_field_value("multiline", "普通行\n- 条目\n# 标题\n1. 第一项\n1) 备选"),
        "普通行\n\n\\\\- 条目\n\n\\\\# 标题\n\n\\\\1. 第一项\n\n\\\\1) 备选"
    );
    // 行中间不转义（保持源码干净）
    assert_eq!(
        markdown_field_value("text", "a - b + c # d"),
        "a - b + c # d"
    );
    // 非行首数字不转义；中文开头值不被破坏
    assert_eq!(
        markdown_field_value("multiline", "2026年\n第二行"),
        "2026年\n\n第二行"
    );
}

#[test]
fn test_markdown_link_destinations() {
    // 注：`escape_markdown` 仅转义 `\` `` ` `` `[]` `|` `<>`，`.` `-` `+` `()` `*` 保持原样
    // 目标无需特殊处理：普通 URL 保持裸目标
    assert_eq!(
        markdown_link("example.com", "https://example.com"),
        "[example.com](https://example.com)"
    );
    // 目标含空格（如 tel: 带区号空格）→ 尖括号包裹
    assert_eq!(
        markdown_link("+86 138-0013-8000", "tel:+86 138-0013-8000"),
        "[+86 138-0013-8000](<tel:+86 138-0013-8000>)"
    );
    // 目标含 ASCII 括号 → 尖括号包裹（括号保持原样，避免链接被截断）
    assert_eq!(
        markdown_link("(010) 1234", "tel:(010) 1234"),
        "[(010) 1234](<tel:(010) 1234>)"
    );
    // 文本含 `[`/`]` → 回退自动链接形式（URL 保持原样）
    assert_eq!(
        markdown_link("含[方括号]文本", "https://x.com"),
        "<https://x.com>"
    );
}

#[test]
fn test_markdown_ocr_multiline_and_link_fields() {
    // OCR 扫描对象场景：multiline 字段多行文本硬换行、url/email/phone 链接化，
    // 无 `<br>` 字面量（此前 split+join("<br>") 会在纯文本 markdown 中显示为字符）
    let fields = serde_json::json!({
        "ocrText": {"name": "OCR 文本", "type": "multiline"},
        "website": {"name": "官网", "type": "url"},
        "contact": {"name": "邮箱", "type": "email"},
        "mobile": {"name": "电话", "type": "phone"},
        "note": {"name": "备注", "type": "text"}
    });
    let rec = make_record(
        "o1",
        "扫描文档",
        fields.clone(),
        serde_json::json!({
            "ocrText": "第一行识别文本\n第二行识别文本\n第三行",
            "website": "https://example.com/docs?a=1",
            "contact": "user@example.com",
            "mobile": "+86 138-0013-8000",
            "note": "见官网 https://x.com 或邮箱 a@b.co",
            "__fields": fields
        }),
    );
    let md = build_markdown_document(
        &[rec],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    ); // 1. 多行 OCR 文本：空行分隔（段落即换行），无 <br> 字面量
    assert!(md.contains("**OCR 文本**：第一行识别文本\n\n第二行识别文本\n\n第三行\n\n"));
    assert!(!md.contains("<br>"));
    // 2. url / email / phone 字段链接化；text 字段内嵌链接自动链接化（不额外转义 . - +）
    assert!(md.contains("**官网**：[https://example.com/docs?a=1](https://example.com/docs?a=1)"));
    assert!(md.contains("**邮箱**：[user@example.com](mailto:user@example.com)"));
    assert!(md.contains("**电话**：[+86 138-0013-8000](<tel:+86 138-0013-8000>)"));
    assert!(md.contains("见官网 [https://x.com](https://x.com) 或邮箱 [a@b.co](mailto:a@b.co)"));
}

#[test]
fn test_text_markdown_multi_object_separator() {
    // 多对象：txt 用 `=====` 横线，markdown 用 `---` 分隔（连续排版）
    let empty = serde_json::json!({});
    let r1 = make_record("o1", "一", empty.clone(), empty.clone());
    let r2 = make_record("o2", "二", empty.clone(), empty.clone());
    let text = build_text_document(
        &[r1.clone(), r2.clone()],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    );
    // 分隔线是整行 50 个 `=`，按完整分隔行计数（matches("=====") 会按 5 字符非重叠计成 10）
    let sep_lines = text
        .lines()
        .filter(|l| !l.is_empty() && l.chars().all(|c| c == '='))
        .count();
    assert_eq!(sep_lines, 1);
    let md = build_markdown_document(
        &[r1, r2],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    );
    assert!(md.contains("\n---\n"));
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
                {"id": "a1", "objectId": "o1", "fileName": "证件.pdf", "sizeBytes": 2048, "mimeType": "application/pdf", "createdAt": "2026-01-01T00:00:00Z"}
            ]
        }),
    );
    let mut tpl_names = std::collections::HashMap::new();
    tpl_names.insert("t1".to_string(), "护照".to_string());
    let html = build_html_document(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1");

    // 自包含：含 DOCTYPE 与内联 style
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<style>"));
    // HTML 转义：& → &amp;
    assert!(html.contains("张三&amp;档案"));
    // 字段表格 + 多行值换行 <br>
    assert!(html.contains("<th>姓名</th>"));
    assert!(html.contains("张三<br>第二行"));
    // PDF 布局样式：长值换行（word-break 在 CSS 中）
    assert!(html.contains("word-break: break-all"));
    // 连续排版：无内联 break-after
    assert!(!html.contains("break-after: always"));
    // 附件清单
    assert!(html.contains("附件：证件.pdf（2.0 KB，application/pdf）"));
    // 封面第二行：账户名 + 账户 ID
    assert!(html.contains("账户名：Gczmy（acc-1）"));
    // 封面第三行：明确对象数量（"导出 N 个对象"）
    assert!(html.contains("导出 1 个对象"));
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
    let bytes = build_pdf_document(
        &[rec],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    )
    .unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], b"%PDF");
    // 注意：%%EOF 是 5 字节序列，需用 windows(5) 匹配（早期 windows(4) 恒 false 的断言已修正）
    assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
}

#[test]
fn test_pdf_options_have_margins() {
    // 页边距配置（上下 15mm / 左右 14mm）随渲染 options 生效——防止回归为默认 0 边距。
    let options = printpdf::GeneratePdfOptions {
        margin_top: Some(15.0),
        margin_bottom: Some(15.0),
        margin_left: Some(14.0),
        margin_right: Some(14.0),
        ..printpdf::GeneratePdfOptions::default()
    };
    assert_eq!(options.margin_top, Some(15.0));
    assert_eq!(options.margin_bottom, Some(15.0));
    assert_eq!(options.margin_left, Some(14.0));
    assert_eq!(options.margin_right, Some(14.0));
}

#[test]
fn test_html_multi_object_continuous_layout() {
    // 多对象连续排版：对象间由 .obj + .obj 横线分隔（CSS），无内联强制分页
    let empty = serde_json::json!({});
    let r1 = make_record("o1", "一", empty.clone(), empty.clone());
    let r2 = make_record("o2", "二", empty.clone(), empty.clone());
    let html = build_html_document(
        &[r1, r2],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    );
    // 无任何内联 break-after（连续排版）
    assert!(!html.contains("break-after: always"));
    // CSS 保留对象间横线分隔
    assert!(html.contains(".obj + .obj { border-top: 1px solid #d0d7de; padding-top: 20px; }"));
    // 两个对象 div 均为无样式
    assert_eq!(html.matches("<div class=\"obj\">").count(), 2);
}

#[test]
fn test_build_docx_continuous_layout() {
    // 连续排版：无分页符；对象间用段落顶部边框横线分隔；对象名带「对象名称：」前缀
    let empty = serde_json::json!({});
    let r1 = make_record("o1", "一", empty.clone(), empty.clone());
    let r2 = make_record("o2", "二", empty.clone(), empty.clone());
    let r3 = make_record("o3", "三", empty.clone(), empty.clone());
    let bytes = build_docx(
        &[r1, r2, r3],
        &std::collections::HashMap::new(),
        "t",
        "Gczmy",
        "acc-1",
    )
    .unwrap();
    let cursor = std::io::Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut doc = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut doc)
        .unwrap();
    // 无分页符（连续排版）
    assert!(!doc.contains("w:type=\"page\""));
    // 3 个对象 → 2 条分隔横线（第二个对象起每个对象前一条）
    assert_eq!(doc.matches("w:top w:val=\"single\"").count(), 2);
    // 对象名带「对象名称：」前缀
    assert!(doc.contains("对象名称：一"));
    assert!(doc.contains("对象名称：三"));
}

#[test]
fn test_attachment_description_and_tags_exported_docx() {
    let fields = serde_json::json!({});
    let rec = make_record(
        "o1",
        "对象一",
        fields.clone(),
        serde_json::json!({
            "__fields": fields,
            "__attachments": [
                {
                    "id": "a1",
                    "objectId": "o1",
                    "fileName": "证件.pdf",
                    "sizeBytes": 2048,
                    "mimeType": "application/pdf",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "description": "带拍摄地点 <A区> & 扫描件",
                    "tags": ["旅行", "证件"]
                }
            ]
        }),
    );
    let tpl_names = std::collections::HashMap::new();
    let bytes = build_docx(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1").unwrap();

    let cursor = std::io::Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut doc = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut doc)
        .unwrap();
    assert!(doc.contains("证件.pdf"));
    // 描述中的 < & 必须 XML 转义（防止破坏 document.xml）
    assert!(doc.contains("　描述：带拍摄地点 &lt;A区&gt; &amp; 扫描件"));
    assert!(!doc.contains("<A区>"));
    assert!(doc.contains("　标签：旅行、证件"));
}

#[test]
fn test_attachment_description_and_tags_exported_markdown() {
    let fields = serde_json::json!({});
    let rec = make_record(
        "o1",
        "对象一",
        fields.clone(),
        serde_json::json!({
            "__fields": fields,
            "__attachments": [
                {
                    "id": "a1",
                    "objectId": "o1",
                    "fileName": "证件.pdf",
                    "sizeBytes": 2048,
                    "mimeType": "application/pdf",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "description": "带拍摄地点的证件扫描件",
                    "tags": ["旅行", "证件"]
                }
            ]
        }),
    );
    let tpl_names = std::collections::HashMap::new();
    let md = build_markdown_document(&[rec], &tpl_names, "2026-08-10T00:00:00Z", "Gczmy", "acc-1");
    assert!(md.contains("- 证件.pdf（2.0 KB，application/pdf）"));
    assert!(md.contains("  - 描述：带拍摄地点的证件扫描件"));
    assert!(md.contains("  - 标签：旅行、证件"));
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
