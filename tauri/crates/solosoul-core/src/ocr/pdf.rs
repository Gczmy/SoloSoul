//! PDF 处理：文本提取 + 无文本时渲染为图片。

use super::control::OcrCancellation;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// 兼容既有默认-token 回归入口。
#[cfg(test)]
pub(crate) fn extract_pdf_text(path: &Path) -> Result<Vec<String>, String> {
    extract_pdf_text_cancellable(path, &OcrCancellation::default())
}

pub(crate) fn extract_pdf_text_cancellable(
    path: &Path,
    cancellation: &OcrCancellation,
) -> Result<Vec<String>, String> {
    extract_pdf_text_controlled(path, cancellation, |_| Ok(()))
}

// 页边界 seam 仅供本模块测试取消时机；生产固定 noop，guard 内只读纯取消 token。
fn extract_pdf_text_controlled(
    path: &Path,
    cancellation: &OcrCancellation,
    mut after_page: impl FnMut(usize) -> Result<(), String>,
) -> Result<Vec<String>, String> {
    cancellation.check()?;
    let pdfium = crate::pdfium::init_pdfium()?;
    // 等待独占 guard 时可能已经取消，不再打开文档。
    cancellation.check()?;
    let document = pdfium.load_pdf_from_file(path, None);
    cancellation.check()?;
    let document = document.map_err(|e| format!("无法加载 PDF: {e}"))?;

    let total_pages = document.pages().len() as usize;
    let mut pages = Vec::with_capacity(total_pages);
    cancellation.check()?;
    for page_index in 0..total_pages {
        cancellation.check()?;
        match document.pages().get(page_index as i32) {
            Ok(page) => {
                let text = page
                    .text()
                    .map(|t| t.to_string())
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                pages.push(text);
            }
            Err(e) => {
                pages.push(String::new());
                tracing::warn!("获取 PDF 第 {} 页文本失败: {e}", page_index + 1);
            }
        }
        after_page(page_index)?;
        cancellation.check()?;
    }
    cancellation.check()?;
    Ok(pages)
}

/// 判断文本层是否"有意义"。
/// 规则：平均每页字符数 ≥ min_chars_per_page（默认 20），且至少有一页非空。
pub(crate) fn has_meaningful_text(pages: &[String], min_chars_per_page: usize) -> bool {
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

/// 将 PDF 最多前 50 页按原顺序渲染为临时 PNG 图片。
/// 返回按页排序的图片路径列表。调用方负责删除临时文件。
#[cfg(test)]
pub(crate) fn render_pdf_pages(
    path: &Path,
    dpi: u32,
    temp_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    render_pdf_pages_cancellable(path, dpi, temp_dir, &OcrCancellation::default())
}

pub(crate) fn render_pdf_pages_cancellable(
    path: &Path,
    dpi: u32,
    temp_dir: &Path,
    cancellation: &OcrCancellation,
) -> Result<Vec<PathBuf>, String> {
    render_pdf_pages_controlled(path, dpi, temp_dir, cancellation, || Ok(()), |_| Ok(()))
}

// 生产回调固定 noop；测试仅允许取消 token 或有界 Result 屏障，不得重入 PDFium。
fn render_pdf_pages_controlled(
    path: &Path,
    dpi: u32,
    temp_dir: &Path,
    cancellation: &OcrCancellation,
    before_acquire: impl FnOnce() -> Result<(), String>,
    mut after_page: impl FnMut(usize) -> Result<(), String>,
) -> Result<Vec<PathBuf>, String> {
    cancellation.check()?;
    before_acquire()?;
    let pdfium = crate::pdfium::init_pdfium()?;
    cancellation.check()?;
    let document = pdfium.load_pdf_from_file(path, None);
    cancellation.check()?;
    let document = document.map_err(|e| format!("无法加载 PDF: {e}"))?;

    cancellation.check()?;
    let page_count = document.pages().len() as usize;
    let max_pages = 50;
    let render_count = page_count.min(max_pages);

    let mut paths = Vec::with_capacity(render_count);

    for page_index in 0..render_count {
        cancellation.check()?;
        let page_number = page_index + 1;
        let page = document
            .pages()
            .get(page_index as i32)
            .map_err(|e| format!("无法获取第 {} 页: {e}", page_number))?;

        let scale = dpi as f32 / 72.0;
        let width_px = (page.width().value * scale) as u32;
        let height_px = (page.height().value * scale) as u32;

        let config = PdfRenderConfig::new()
            .set_target_width(width_px as Pixels)
            .set_target_height(height_px as Pixels);

        cancellation.check()?;
        let bitmap = page.render_with_config(&config);
        cancellation.check()?;
        let bitmap = bitmap.map_err(|e| format!("无法渲染第 {} 页: {e}", page_number))?;

        let img = bitmap
            .as_image()
            .map_err(|e| format!("无法转换第 {} 页位图: {e}", page_number))?;

        cancellation.check()?;
        let out_path = temp_dir.join(format!("page_{:04}.png", page_number));
        img.save(&out_path)
            .map_err(|e| format!("无法保存第 {} 页图片: {e}", page_number))?;

        paths.push(out_path);
        after_page(page_index)?;
        cancellation.check()?;
    }

    cancellation.check()?;
    Ok(paths)
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

    fn pdfium_available() -> bool {
        crate::pdfium::init_pdfium().is_ok()
    }

    #[test]
    fn test_extract_pdf_text() {
        if !pdfium_available() {
            return;
        }
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
        if !pdfium_available() {
            return;
        }
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

#[cfg(test)]
mod rf906_tests;

#[cfg(test)]
mod rf029_tests;
