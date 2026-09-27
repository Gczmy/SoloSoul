use super::{
    extract_pdf_text_cancellable, extract_pdf_text_controlled, render_pdf_pages_controlled,
};
use crate::ocr::control::{OcrCancellation, OCR_CANCELLED};
use pdfium_render::prelude::*;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

fn write_pdf(path: &Path, text: bool) -> Result<(), String> {
    let pdfium = crate::pdfium::init_pdfium()?;
    let mut document = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    let font = document.fonts_mut().helvetica();
    for index in 0..3 {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::new_custom(
                PdfPoints::new(72.0),
                PdfPoints::new(72.0),
            ))
            .map_err(|error| error.to_string())?;
        if text {
            page.objects_mut()
                .create_text_object(
                    PdfPoints::new(4.0),
                    PdfPoints::new(36.0),
                    format!("RF029 page {index}"),
                    font,
                    PdfPoints::new(6.0),
                )
                .map_err(|error| error.to_string())?;
        } else {
            page.objects_mut()
                .create_path_object_rect(
                    PdfRect::new_from_values(0.0, 0.0, 72.0, 72.0),
                    None,
                    None,
                    Some(PdfColor::new(224, 32, 48, 255)),
                )
                .map_err(|error| error.to_string())?;
        }
    }
    document
        .save_to_file(path)
        .map_err(|error| error.to_string())
}

#[test]
fn rf029_cancel_after_first_real_png_prevents_second_render() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let input = dir.path().join("pages.pdf");
    let output = dir.path().join("pngs");
    std::fs::create_dir(&output).map_err(|error| error.to_string())?;
    write_pdf(&input, false)?;
    let cancellation = OcrCancellation::default();
    let outcome = render_pdf_pages_controlled(
        &input,
        150,
        &output,
        &cancellation,
        || Ok(()),
        |_| {
            cancellation.cancel();
            Ok(())
        },
    );
    // 所有原生句柄已释放；在 guard 外检查真实首张 PNG 和未启动的后续页。
    assert_eq!(outcome.unwrap_err(), OCR_CANCELLED);
    let first = output.join("page_0001.png");
    let png = image::open(first)
        .map_err(|error| error.to_string())?
        .to_rgb8();
    assert_eq!(png.dimensions(), (150, 150));
    for (actual, expected) in png.get_pixel(75, 75).0.into_iter().zip([224, 32, 48]) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "unexpected first-page color"
        );
    }
    assert!(!output.join("page_0002.png").exists());
    assert!(!output.join("page_0003.png").exists());
    assert_eq!(
        std::fs::read_dir(output)
            .map_err(|error| error.to_string())?
            .count(),
        1
    );
    Ok(())
}

#[test]
fn rf029_text_extraction_checks_cancellation_between_real_pages() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let input = dir.path().join("text.pdf");
    write_pdf(&input, true)?;
    let cancellation = OcrCancellation::default();
    let mut reached = Vec::new();
    let outcome = extract_pdf_text_controlled(&input, &cancellation, |index| {
        reached.push(index);
        cancellation.cancel();
        Ok(())
    });
    assert_eq!(outcome.unwrap_err(), OCR_CANCELLED);
    assert_eq!(reached, vec![0]);
    // 已取消操作的 guard 退出后，新的独立操作仍可提取所有真实文字页。
    let pages = extract_pdf_text_cancellable(&input, &OcrCancellation::default())?;
    assert_eq!(pages.len(), 3);
    for (index, text) in pages.iter().enumerate() {
        assert!(
            text.contains(&format!("RF029 page {index}")),
            "unexpected text: {text}"
        );
    }
    Ok(())
}

#[test]
fn rf029_cancel_while_waiting_for_pdfium_skips_opening_document() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let missing_input = dir.path().join("must-not-open.pdf");
    let output = dir.path().to_path_buf();
    let cancellation = OcrCancellation::default();
    let worker_cancellation = cancellation.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (worker, reached_acquire) = {
        let _other_operation = crate::pdfium::init_pdfium()?;
        let worker = std::thread::spawn(move || {
            render_pdf_pages_controlled(
                &missing_input,
                150,
                &output,
                &worker_cancellation,
                || ready_tx.send(()).map_err(|error| error.to_string()),
                |_| Ok(()),
            )
        });
        // seam 位于初次 check 之后、init 之前；此时原生锁仍由当前操作持有。
        // 有界等待只返回 Result，不在 guard 内 assert、panic 或再次调用 PDFium。
        let reached_acquire = ready_rx.recv_timeout(Duration::from_secs(30));
        cancellation.cancel();
        (worker, reached_acquire)
    };
    let outcome = worker
        .join()
        .map_err(|_| "renderer worker panicked".to_string())?;
    reached_acquire.map_err(|error| error.to_string())?;
    assert_eq!(
        outcome.unwrap_err(),
        OCR_CANCELLED,
        "missing path must not be opened after acquiring the guard"
    );
    assert_eq!(
        std::fs::read_dir(dir.path())
            .map_err(|error| error.to_string())?
            .count(),
        0
    );
    Ok(())
}
