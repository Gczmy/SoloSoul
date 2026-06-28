//! PDF 处理：文本提取 + 无文本时渲染为图片。

use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// 提取 PDF 文本层，返回每页文本。
pub fn extract_pdf_text(path: &Path) -> Result<Vec<String>, String> {
    pdf_extract::extract_text_by_pages(path)
        .map(|pages| pages.into_iter().map(|p| p.trim().to_string()).collect())
        .map_err(|e| format!("PDF 文本提取失败: {e}"))
}

/// 判断文本层是否"有意义"。
/// 规则：平均每页字符数 ≥ min_chars_per_page（默认 20），且至少有一页非空。
pub fn has_meaningful_text(pages: &[String], min_chars_per_page: usize) -> bool {
    if pages.is_empty() {
        return false;
    }
    let non_empty: Vec<&String> = pages.iter().filter(|p| !p.is_empty()).collect();
    if non_empty.is_empty() {
        return false;
    }
    // 按文档要求的“平均每页”计算，空页计入分母。
    let total_chars: usize = pages.iter().map(|p| p.chars().count()).sum();
    let avg = total_chars / pages.len();
    avg >= min_chars_per_page
}

/// 将 PDF 每页渲染为临时 PNG 图片。
/// 返回按页排序的图片路径列表。调用方负责删除临时文件。
pub fn render_pdf_pages(path: &Path, dpi: u32, temp_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let pdfium = crate::pdfium::init_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("无法加载 PDF: {e}"))?;

    let page_count = document.pages().len() as usize;
    let max_pages = 50;
    let render_count = page_count.min(max_pages);

    let mut paths = Vec::with_capacity(render_count);

    for page_index in 1..=render_count {
        let page = document
            .pages()
            .get(page_index as i32)
            .map_err(|e| format!("无法获取第 {} 页: {e}", page_index))?;

        let scale = dpi as f32 / 72.0;
        let width_px = (page.width().value * scale) as u32;
        let height_px = (page.height().value * scale) as u32;

        let config = PdfRenderConfig::new()
            .set_target_width(width_px as Pixels)
            .set_target_height(height_px as Pixels);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| format!("无法渲染第 {} 页: {e}", page_index))?;

        let img = bitmap
            .as_image()
            .map_err(|e| format!("无法转换第 {} 页位图: {e}", page_index))?;

        let out_path = temp_dir.join(format!("page_{:04}.png", page_index));
        img.save(&out_path)
            .map_err(|e| format!("无法保存第 {} 页图片: {e}", page_index))?;

        paths.push(out_path);
    }

    Ok(paths)
}

/// 清理渲染产生的临时图片。
pub fn cleanup_rendered_pages(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_meaningful_text() {
        assert!(!has_meaningful_text(&[], 20));
        assert!(!has_meaningful_text(&["".to_string(), "".to_string()], 20));
        // 平均每页 25 字符，且非空页存在
        assert!(has_meaningful_text(
            &["this page has twenty five chars".to_string()],
            20
        ));
        // 空页计入分母：总字符 30 / 3 页 = 10，不足 20
        assert!(!has_meaningful_text(
            &[
                "short text with thirty chars total".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            20
        ));
    }

    #[test]
    fn test_extract_pdf_text() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("tests/fixtures/text_only.pdf");
        if !path.exists() {
            return;
        }
        let pages = extract_pdf_text(&path).expect("should extract text");
        assert!(!pages.is_empty(), "expected at least one page");
        let full = pages.join(" ");
        assert!(
            full.to_lowercase().contains("solosoul"),
            "expected text layer, got: {full}"
        );
    }

    #[test]
    fn test_extract_pdf_text_scanned_empty() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("tests/fixtures/scanned.pdf");
        if !path.exists() {
            return;
        }
        let pages = extract_pdf_text(&path).expect("should parse scanned pdf");
        assert!(
            pages.iter().all(|p| p.is_empty()),
            "scanned pdf should have no text layer"
        );
        assert!(!has_meaningful_text(&pages, 20));
    }
}
