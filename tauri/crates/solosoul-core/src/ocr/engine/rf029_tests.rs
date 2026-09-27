use super::{recognize_mrz_lines_with, scan_pdf_with_control};
use crate::ocr::control::{OcrCancellation, OCR_CANCELLED};
use crate::ocr::pdf::render_pdf_pages_cancellable;
use crate::ocr::types::OcrResult;
use pdfium_render::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(30);

fn write_scanned_pdf(path: &Path) -> Result<(), String> {
    let pdfium = crate::pdfium::init_pdfium()?;
    let mut document = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    for [red, green, blue] in [[224, 32, 48], [48, 64, 224]] {
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

struct HeldPage {
    file: File,
    alive: Arc<AtomicBool>,
}

impl HeldPage {
    fn open(path: &Path, alive: Arc<AtomicBool>) -> Result<Self, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0x0000_0001 | 0x0000_0002);
        }
        let file = options.open(path).map_err(|error| error.to_string())?;
        alive.store(true, Ordering::SeqCst);
        Ok(Self { file, alive })
    }
}

impl Drop for HeldPage {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }
}

#[test]
fn rf029_cancel_during_recognition_keeps_handles_until_callback_returns() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let input = dir.path().join("input.pdf");
    write_scanned_pdf(&input)?;
    let input_before = std::fs::read(&input).map_err(|error| error.to_string())?;
    let temp_root = dir.path().join("scans");
    std::fs::create_dir(&temp_root).map_err(|error| error.to_string())?;
    std::fs::write(temp_root.join("keep.txt"), b"keep").map_err(|error| error.to_string())?;
    let cancellation = OcrCancellation::default();
    let worker_cancellation = cancellation.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let worker_calls = calls.clone();
    let alive = Arc::new(AtomicBool::new(false));
    let worker_alive = alive.clone();
    let (started_tx, started_rx) = mpsc::channel::<(PathBuf, (u32, u32))>();
    let (release_tx, release_rx) = mpsc::channel();
    let worker_input = input.clone();
    let worker_root = temp_root.clone();
    let worker = std::thread::spawn(move || {
        scan_pdf_with_control(
            &worker_input,
            &worker_root,
            &worker_cancellation,
            |path, dpi, output| {
                render_pdf_pages_cancellable(path, dpi, output, &worker_cancellation)
            },
            |path| {
                let index = worker_calls.fetch_add(1, Ordering::SeqCst);
                let handle = HeldPage::open(path, worker_alive.clone())?;
                let dimensions =
                    image::image_dimensions(path).map_err(|error| error.to_string())?;
                if index == 0 {
                    started_tx
                        .send((path.to_path_buf(), dimensions))
                        .map_err(|error| error.to_string())?;
                    release_rx
                        .recv_timeout(WAIT)
                        .map_err(|error| format!("recognizer release: {error}"))?;
                }
                // 取消信号不能提早关闭正在使用的文件。替身仍正常返回，取消由生产编排检查。
                handle.file.metadata().map_err(|error| error.to_string())?;
                Ok(OcrResult {
                    text: "recognized".into(),
                    confidence: 0.9,
                    boxes: Vec::new(),
                })
            },
        )
    });
    let started = started_rx.recv_timeout(WAIT);
    cancellation.cancel();
    let held_after_cancel = alive.load(Ordering::SeqCst);
    let finished_before_release = worker.is_finished();
    let workspace_present = started
        .as_ref()
        .ok()
        .and_then(|(path, _)| path.parent())
        .is_some_and(Path::is_dir);
    let png_present = started
        .as_ref()
        .ok()
        .is_some_and(|(path, _)| path.is_file());
    let release = release_tx.send(());
    // 先释放并 join，再进行任何断言；失败时也不能遗留持句柄的后台线程。
    let outcome = worker
        .join()
        .map_err(|_| "recognizer worker panicked".to_string())?;
    let (first_path, dimensions) = started.map_err(|error| error.to_string())?;
    release.map_err(|error| error.to_string())?;
    assert_eq!(outcome.unwrap_err(), OCR_CANCELLED);
    assert!(held_after_cancel && workspace_present && png_present);
    assert!(!finished_before_release);
    assert!(!alive.load(Ordering::SeqCst));
    assert_eq!(dimensions, (150, 150));
    assert_eq!(first_path.file_name().unwrap(), "page_0001.png");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "second page must not be recognized"
    );
    assert!(!first_path.parent().unwrap().exists());
    assert_eq!(
        std::fs::read_dir(&temp_root)
            .map_err(|error| error.to_string())?
            .count(),
        1
    );
    assert_eq!(
        std::fs::read(temp_root.join("keep.txt")).map_err(|error| error.to_string())?,
        b"keep"
    );
    assert_eq!(
        std::fs::read(input).map_err(|error| error.to_string())?,
        input_before
    );
    Ok(())
}

#[test]
fn rf029_precancelled_pdf_scan_skips_input_and_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let temp_root = dir.path().join("not-created");
    let cancellation = OcrCancellation::default();
    cancellation.cancel();
    let mut rendered = false;
    let mut recognized = false;
    let outcome = scan_pdf_with_control(
        &dir.path().join("does-not-exist.pdf"),
        &temp_root,
        &cancellation,
        |_, _, _| {
            rendered = true;
            Ok(Vec::new())
        },
        |_| {
            recognized = true;
            Ok(OcrResult {
                text: String::new(),
                confidence: 0.0,
                boxes: Vec::new(),
            })
        },
    );
    assert_eq!(outcome.unwrap_err(), OCR_CANCELLED);
    assert!(!rendered && !recognized && !temp_root.exists());
}

#[test]
fn rf029_mrz_segment_cancellation_is_not_swallowed_by_best_effort() {
    let line = image::GrayImage::from_pixel(80, 24, image::Luma([255]));
    for ordinary_error in [false, true] {
        let cancellation = OcrCancellation::default();
        let mut calls = 0;
        let outcome = recognize_mrz_lines_with(&[&line, &line], &cancellation, |_| {
            calls += 1;
            cancellation.cancel();
            if ordinary_error {
                Err("ordinary recognition failure".into())
            } else {
                Ok(("ABCDEF".into(), 0.8))
            }
        });
        assert_eq!(outcome.unwrap_err(), OCR_CANCELLED);
        assert_eq!(calls, 1, "later segments must not start");
    }
}

#[test]
fn rf029_mrz_without_cancellation_keeps_partial_recognition() {
    let line = image::GrayImage::from_pixel(80, 24, image::Luma([255]));
    let cancellation = OcrCancellation::default();
    let mut calls = 0;
    let (texts, confidence) = recognize_mrz_lines_with(&[&line, &line], &cancellation, |_| {
        calls += 1;
        if calls == 1 {
            Err("ordinary segment failure".into())
        } else {
            Ok(("ABCDEF".into(), 0.8))
        }
    })
    .unwrap();
    assert_eq!(calls, 4);
    assert_eq!(texts, vec!["ABCDEF", "ABCDEFABCDEF"]);
    assert!((confidence - 1.6).abs() < 1e-12);
}
