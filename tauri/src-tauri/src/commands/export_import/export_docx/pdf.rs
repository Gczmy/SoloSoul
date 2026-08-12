//! export_docx 子模块 —— pdf（P047 拆分）

use super::html::build_html_document;

/// 构造 PDF（HTML → printpdf from_html，内嵌 Noto Sans SC 中文字体）。
///
/// 字体字节经 `include_bytes!` 嵌入二进制（打包进应用；发布时随主程序分发，
/// 无需额外资源文件）。`PdfSaveOptions` 默认 `subset_fonts=true`，PDF 只嵌用到的字形。
pub(crate) fn build_pdf_document(
    records: &[solosoul_vault::ObjectRecord],
    template_names: &std::collections::HashMap<String, String>,
    export_time: &str,
    account_name: &str,
    account_id: &str,
) -> Result<Vec<u8>, String> {
    use printpdf::Base64OrRaw;

    let html = build_html_document(
        records,
        template_names,
        export_time,
        account_name,
        account_id,
    );
    let mut fonts = std::collections::BTreeMap::new();
    // 字体名需与 build_html_document 的 font-family 列表一致（Noto Sans SC 兜底链中命中）
    fonts.insert(
        "Noto Sans SC".to_string(),
        Base64OrRaw::Raw(
            include_bytes!("../../../../resources/fonts/NotoSansSC-Regular.otf").to_vec(),
        ),
    );

    let mut warnings = Vec::new();
    // 页边距（mm）：上下 15 / 左右 14。printpdf 的 margin 作用于每个分页页面的内容区，
    // 解决「第一页有边距、第二页无上下边距」与左右边距过窄问题。
    let pdf_options = printpdf::GeneratePdfOptions {
        margin_top: Some(15.0),
        margin_bottom: Some(15.0),
        margin_left: Some(14.0),
        margin_right: Some(14.0),
        ..printpdf::GeneratePdfOptions::default()
    };
    let pdf = printpdf::PdfDocument::from_html(
        &html,
        &std::collections::BTreeMap::new(),
        &fonts,
        &pdf_options,
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
