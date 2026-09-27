use super::init_pdfium;
use pdfium_render::prelude::*;
use std::path::Path;
use std::sync::{Arc, Barrier, TryLockError};

#[derive(Debug)]
struct RenderedPage {
    dimensions: (u32, u32),
    text: String,
    samples: Vec<[u8; 3]>,
}

// 本文件所有 PDFium 对象均留在取得 guard 的同步作用域内；错误用 Result 退出，
// 外层再断言，避免测试断言 panic 在持有全局锁时污染其他测试。
fn write_color_pdf(path: &Path, colors: &[[u8; 3]]) -> Result<(), String> {
    let pdfium = init_pdfium().map_err(|error| {
        format!("RF907 requires real PDFium; configure PDFIUM_LIBRARY_PATH: {error}")
    })?;
    let mut document = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    for &[red, green, blue] in colors {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::new_custom(
                PdfPoints::new(72.0),
                PdfPoints::new(72.0),
            ))
            .map_err(|error| error.to_string())?;
        page.objects_mut()
            .create_path_object_rect(
                PdfRect::new_from_values(0.0, 0.0, 72.0, 72.0),
                None,
                None,
                Some(PdfColor::new(red, green, blue, 255)),
            )
            .map_err(|error| error.to_string())?;
    }
    document
        .save_to_file(path)
        .map_err(|error| error.to_string())
}

fn read_color_pdf(path: &Path) -> Result<Vec<RenderedPage>, String> {
    let pdfium = init_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|error| error.to_string())?;
    let mut pages = Vec::new();
    for index in 0..document.pages().len() {
        let page = document
            .pages()
            .get(index)
            .map_err(|error| error.to_string())?;
        let text = page.text().map_err(|error| error.to_string())?.to_string();
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(72)
                    .set_target_height(72),
            )
            .map_err(|error| error.to_string())?;
        let image = bitmap
            .as_image()
            .map_err(|error| error.to_string())?
            .to_rgb8();
        pages.push(RenderedPage {
            dimensions: image.dimensions(),
            text,
            samples: [(8, 8), (36, 36), (63, 63)]
                .into_iter()
                .map(|(x, y)| image.get_pixel(x, y).0)
                .collect(),
        });
    }
    Ok(pages)
}

fn assert_colors(pages: &[RenderedPage], colors: &[[u8; 3]]) {
    assert_eq!(pages.len(), colors.len());
    for (page, expected) in pages.iter().zip(colors) {
        assert_eq!(page.dimensions, (72, 72));
        assert!(
            page.text.trim().is_empty(),
            "unexpected text: {:?}",
            page.text
        );
        for actual in &page.samples {
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 1,
                    "expected {expected:?}, got {actual:?}",
                );
            }
        }
    }
}

fn native_lock_is_held() -> Result<bool, String> {
    // Ok 分支的临时 guard 在返回前释放，旧实现红测不会带锁进入断言。
    match super::PDFIUM.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(true),
        Ok(_temporary_guard) => Ok(false),
        Err(TryLockError::Poisoned(_)) => Err("PDFium lock was already poisoned".to_string()),
    }
}

#[test]
fn rf907_pdfium_guard_covers_live_native_handles() -> Result<(), String> {
    let observations = {
        let pdfium = init_pdfium().map_err(|error| {
            format!("RF907 requires real PDFium; configure PDFIUM_LIBRARY_PATH: {error}")
        })?;
        let after_init = native_lock_is_held()?;
        let mut document = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
        let with_document = native_lock_is_held()?;
        let page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::new_custom(
                PdfPoints::new(72.0),
                PdfPoints::new(72.0),
            ))
            .map_err(|error| error.to_string())?;
        let with_page = native_lock_is_held()?;
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(8)
                    .set_target_height(8),
            )
            .map_err(|error| error.to_string())?;
        let with_bitmap = native_lock_is_held()?;
        // 后续使用确保观测点上 bitmap/page/document 均仍然存活。
        let _pixels = bitmap.as_raw_bytes();
        [after_init, with_document, with_page, with_bitmap]
    };
    {
        // 不使用释放后的 try_lock 成功作断言：其他并行用例此时可以合法取得锁。
        let _next_operation = init_pdfium()?;
    }
    assert_eq!(
        observations, [true; 4],
        "PDFium must stay exclusively locked through all native handle lifetimes",
    );
    Ok(())
}

#[test]
fn rf907_four_workers_round_trip_colored_documents() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let barrier = Arc::new(Barrier::new(4));
    let mut workers = Vec::new();
    for worker in 0..4_u8 {
        let root = dir.path().to_path_buf();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || -> Result<_, String> {
            // 仅在首次取得 PDFium 之前同步，不在持锁区或可能提前返回的轮次之间等待。
            barrier.wait();
            let colors = [[32 + worker * 32, 64, 192], [224, 32 + worker * 32, 48]];
            let mut rounds = Vec::new();
            for round in 0..3 {
                let path = root.join(format!("worker-{worker}-{round}.pdf"));
                write_color_pdf(&path, &colors)?;
                rounds.push(read_color_pdf(&path)?);
            }
            Ok((colors, rounds))
        }));
    }
    // 即使某个 worker 返回错误，也先 join 全部线程，避免失败后遗留后台 FFI。
    let results: Vec<_> = workers.into_iter().map(|worker| worker.join()).collect();
    for result in results {
        let (colors, rounds) = result.map_err(|_| "PDFium worker panicked".to_string())??;
        assert_eq!(rounds.len(), 3);
        for pages in rounds {
            assert_colors(&pages, &colors);
        }
    }
    Ok(())
}

#[test]
fn rf907_error_exits_release_guard_for_following_operations() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let invalid = dir.path().join("invalid.pdf");
    std::fs::write(&invalid, b"not a PDF document").map_err(|error| error.to_string())?;
    let load_failed = {
        let pdfium = init_pdfium()?;
        let result = pdfium.load_pdf_from_file(&invalid, None);
        result.is_err()
    };
    let colors = [[16, 96, 208], [240, 80, 32]];
    let save_failed =
        write_color_pdf(&dir.path().join("missing-parent/output.pdf"), &colors).is_err();
    let valid = dir.path().join("valid.pdf");
    write_color_pdf(&valid, &colors)?;
    let same_thread = read_color_pdf(&valid)?;
    let another_thread = std::thread::spawn(move || read_color_pdf(&valid))
        .join()
        .map_err(|_| "following PDFium worker panicked".to_string())??;
    assert!(load_failed, "invalid PDF must produce a real load error");
    assert!(
        save_failed,
        "missing output parent must produce a real save error"
    );
    assert_colors(&same_thread, &colors);
    assert_colors(&another_thread, &colors);
    Ok(())
}

#[cfg(all(feature = "ocr", feature = "watermark"))]
#[test]
fn rf907_watermark_and_text_readers_share_native_exclusion() -> Result<(), String> {
    use crate::ocr::pdf::extract_pdf_text;
    use crate::watermark::{apply_to_pdf, WatermarkConfig};

    const INPUT: &[u8] = include_bytes!("../watermark/testdata/minimal.pdf");
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let input = dir.path().join("input.pdf");
    std::fs::write(&input, INPUT).map_err(|error| error.to_string())?;
    let barrier = Arc::new(Barrier::new(4));
    let mut workers = Vec::new();
    for worker in 0..4 {
        let input = input.clone();
        let root = dir.path().to_path_buf();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || -> Result<_, String> {
            barrier.wait();
            let writes_watermark = worker % 2 == 0;
            let mut rounds = Vec::new();
            for round in 0..3 {
                let path = if writes_watermark {
                    let output = root.join(format!("watermark-{worker}-{round}.pdf"));
                    let config = WatermarkConfig {
                        text: "RF907".to_string(),
                        font_size: 24.0,
                        angle: 0.0,
                        opacity: 1.0,
                        ..WatermarkConfig::default()
                    };
                    // 生产入口自行获取 guard；此处不能先 init 再递归调用。
                    apply_to_pdf(&input, &output, &config)?;
                    output
                } else {
                    input.clone()
                };
                rounds.push(extract_pdf_text(&path)?);
            }
            Ok((writes_watermark, rounds))
        }));
    }
    let results: Vec<_> = workers.into_iter().map(|worker| worker.join()).collect();
    for result in results {
        let (writes_watermark, rounds) =
            result.map_err(|_| "mixed PDFium worker panicked".to_string())??;
        assert_eq!(rounds.len(), 3);
        for pages in rounds {
            assert_eq!(pages.len(), 1);
            assert!(pages[0].contains("Hello World"), "original text was lost");
            assert_eq!(pages[0].contains("RF907"), writes_watermark);
        }
    }
    assert_eq!(
        std::fs::read(&input).map_err(|error| error.to_string())?,
        INPUT
    );
    Ok(())
}
