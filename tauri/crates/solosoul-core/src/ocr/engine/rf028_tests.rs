use super::scan_pdf_with;
use crate::ocr::pdf::{extract_pdf_text, has_meaningful_text, render_pdf_pages};
use crate::ocr::types::{OcrBox, OcrResult};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

const COLORS: [[u8; 3]; 3] = [[224, 32, 48], [32, 208, 64], [48, 64, 224]];
const SENTINEL: &[u8] = b"RF028 synthetic parent data must survive";
const BODY_POINTS: [(f32, f32); 4] = [(10.0, 20.0), (30.0, 20.0), (30.0, 40.0), (10.0, 40.0)];
const SEPARATOR_POINTS: [(f32, f32); 4] = [(0.0, 1.0), (0.0, 1.0), (1.0, 1.0), (1.0, 1.0)];

// 原生对象仅在本同步 helper 中存活，Result 返回后才断言或进入生产扫描编排。
fn write_scanned_pdf(path: &Path) -> Result<(), String> {
    let pdfium = crate::pdfium::init_pdfium().map_err(|error| {
        format!("RF028 requires real PDFium; configure PDFIUM_LIBRARY_PATH: {error}")
    })?;
    let mut document = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    for [red, green, blue] in COLORS {
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

struct Fixture {
    input: PathBuf,
    temp_root: PathBuf,
    input_bytes: Vec<u8>,
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new(text_only: bool) -> Self {
        let dir = tempfile::tempdir().expect("create isolated RF028 fixture");
        let input = dir.path().join("synthetic-input.pdf");
        let temp_root = dir.path().join("scan-parent");
        std::fs::write(dir.path().join("parent-sentinel"), SENTINEL).unwrap();
        if text_only {
            std::fs::write(
                &input,
                include_bytes!("../../../tests/fixtures/text_only.pdf"),
            )
            .unwrap();
        } else {
            write_scanned_pdf(&input).expect("create real synthetic three-page PDF");
            std::fs::create_dir(&temp_root).unwrap();
            std::fs::write(temp_root.join("keep.txt"), SENTINEL).unwrap();
        }
        let input_bytes = std::fs::read(&input).unwrap();
        Self {
            input,
            temp_root,
            input_bytes,
            dir,
        }
    }

    fn observe_workspace(&self, output: &Path, seen: &mut Option<PathBuf>) {
        assert!(
            output.is_dir(),
            "worker must create its own output directory"
        );
        assert_eq!(output.parent(), Some(self.temp_root.as_path()));
        assert_ne!(output, self.temp_root.as_path());
        assert!(seen.replace(output.to_path_buf()).is_none());
    }

    fn assert_preserved(&self) {
        assert_eq!(std::fs::read(&self.input).unwrap(), self.input_bytes);
        assert_eq!(
            std::fs::read(self.dir.path().join("parent-sentinel")).unwrap(),
            SENTINEL
        );
    }

    fn assert_cleaned(&self, output: &Path) {
        self.assert_preserved();
        assert!(
            !output.exists(),
            "worker-owned directory remains: {}",
            output.display()
        );
        assert!(self.temp_root.is_dir());
        assert_eq!(
            std::fs::read(self.temp_root.join("keep.txt")).unwrap(),
            SENTINEL
        );
        let remaining: Vec<_> = std::fs::read_dir(&self.temp_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(remaining, vec![std::ffi::OsString::from("keep.txt")]);
    }
}

#[derive(Debug)]
struct PageImage {
    name: String,
    dimensions: (u32, u32),
    center: [u8; 3],
}

fn inspect_png(path: &Path) -> Result<PageImage, String> {
    let image = image::open(path)
        .map_err(|error| error.to_string())?
        .to_rgb8();
    Ok(PageImage {
        name: path
            .file_name()
            .ok_or("PNG filename is missing")?
            .to_string_lossy()
            .into_owned(),
        dimensions: image.dimensions(),
        center: image.get_pixel(image.width() / 2, image.height() / 2).0,
    })
}

fn assert_page(image: &PageImage, index: usize) {
    assert_eq!(image.name, format!("page_{:04}.png", index + 1));
    assert_eq!(image.dimensions, (150, 150));
    for (actual, expected) in image.center.iter().zip(COLORS[index]) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "page {}: {image:?}",
            index + 1
        );
    }
}

fn recognized_page(index: usize) -> OcrResult {
    let (text, entries) = match index {
        0 => ("甲\n乙", vec![("甲", 0.2), ("乙", 0.4)]),
        1 => ("", vec![]),
        2 => ("丙", vec![("丙", 0.6)]),
        _ => panic!("unexpected fourth page"),
    };
    OcrResult {
        text: text.into(),
        // 聚合应保持现有 box 平均值规则，而不是改成页面 confidence 平均值。
        confidence: 0.99,
        boxes: entries
            .into_iter()
            .map(|(text, confidence)| OcrBox {
                text: text.into(),
                confidence,
                points: BODY_POINTS,
            })
            .collect(),
    }
}

fn open_page_handle(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // 允许 READ/WRITE，不共享 DELETE；局部句柄必须先于 worker 的目录 owner 释放。
        options.share_mode(0x0000_0001 | 0x0000_0002);
    }
    options.open(path)
}

#[test]
fn rf028_partial_render_failure_removes_first_png_and_blocking_directory() {
    let f = Fixture::new(false);
    let mut workspace = None;
    let mut original_error = None;
    let mut recognize_calls = 0;
    let result = scan_pdf_with(
        &f.input,
        &f.temp_root,
        |input, dpi, output| {
            f.observe_workspace(output, &mut workspace);
            assert_eq!(dpi, 150);
            std::fs::create_dir(output.join("page_0002.png")).unwrap();
            let rendered = render_pdf_pages(input, dpi, output);
            // 真实 renderer 已返回并释放 PDFium guard，第一页确实写出了有效 PNG。
            assert_page(&inspect_png(&output.join("page_0001.png")).unwrap(), 0);
            assert!(output.join("page_0002.png").is_dir());
            assert!(!output.join("page_0003.png").exists());
            original_error = Some(rendered.as_ref().unwrap_err().clone());
            rendered
        },
        |_| {
            recognize_calls += 1;
            Ok(recognized_page(0))
        },
    );
    let error = result.unwrap_err();
    assert!(error.starts_with("无法保存第 2 页"), "{error}");
    assert_eq!(Some(error), original_error);
    assert_eq!(recognize_calls, 0);
    f.assert_cleaned(workspace.as_deref().expect("renderer must be called"));
}

#[test]
fn rf028_second_page_recognition_error_stops_and_cleans_all_pages() {
    let f = Fixture::new(false);
    let mut workspace = None;
    let mut seen = Vec::new();
    let result = scan_pdf_with(
        &f.input,
        &f.temp_root,
        |input, dpi, output| {
            f.observe_workspace(output, &mut workspace);
            assert_eq!(dpi, 150);
            let paths = render_pdf_pages(input, dpi, output)?;
            assert_eq!(paths.len(), 3);
            assert!(paths.iter().all(|path| path.is_file()));
            Ok(paths)
        },
        |path| {
            let _in_use = open_page_handle(path).map_err(|error| error.to_string())?;
            seen.push(inspect_png(path)?);
            if seen.len() == 2 {
                Err("RF028_SYNTHETIC_RECOGNITION_FAILURE".into())
            } else {
                Ok(recognized_page(seen.len() - 1))
            }
        },
    );
    assert_eq!(result.unwrap_err(), "RF028_SYNTHETIC_RECOGNITION_FAILURE");
    assert_eq!(seen.len(), 2, "third page must not be recognized");
    for (index, image) in seen.iter().enumerate() {
        assert_page(image, index);
    }
    f.assert_cleaned(workspace.as_deref().expect("renderer must be called"));
}

#[test]
fn rf028_success_preserves_aggregation_and_removes_unlisted_partial_file() {
    let mut f = Fixture::new(false);
    let existing_parent = f.temp_root.clone();
    f.temp_root = f.dir.path().join("initially-missing-scan-parent");
    assert!(!f.temp_root.exists());
    let mut workspace = None;
    let mut seen = Vec::new();
    let result = scan_pdf_with(
        &f.input,
        &f.temp_root,
        |input, dpi, output| {
            f.observe_workspace(output, &mut workspace);
            assert_eq!(dpi, 150);
            let paths = render_pdf_pages(input, dpi, output)?;
            assert_eq!(paths.len(), 3);
            std::fs::write(output.join("unlisted.part"), b"synthetic unfinished data").unwrap();
            Ok(paths)
        },
        |path| {
            seen.push(inspect_png(path)?);
            Ok(recognized_page(seen.len() - 1))
        },
    )
    .unwrap();
    assert_eq!(seen.len(), 3);
    for (index, image) in seen.iter().enumerate() {
        assert_page(image, index);
    }
    assert_eq!(result.text, "甲\n乙\n--- Page 2 ---\n\n--- Page 3 ---\n丙");
    assert_eq!(
        result
            .boxes
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        vec!["甲", "乙", "--- Page 2 ---", "--- Page 3 ---", "丙"]
    );
    assert_eq!(
        result
            .boxes
            .iter()
            .map(|item| item.confidence)
            .collect::<Vec<_>>(),
        vec![0.2, 0.4, 1.0, 1.0, 0.6]
    );
    assert!((result.confidence - 0.64).abs() < 1e-12);
    for index in [0, 1, 4] {
        assert_eq!(result.boxes[index].points, BODY_POINTS);
    }
    for index in [2, 3] {
        assert_eq!(result.boxes[index].points, SEPARATOR_POINTS);
    }
    f.assert_preserved();
    let output = workspace.as_deref().expect("renderer must be called");
    assert!(!output.exists(), "worker-owned directory must be removed");
    assert!(
        f.temp_root.is_dir(),
        "missing temp root must be created for scanning"
    );
    assert_eq!(std::fs::read_dir(&f.temp_root).unwrap().count(), 0);
    assert_eq!(
        std::fs::read(existing_parent.join("keep.txt")).unwrap(),
        SENTINEL
    );
}

#[test]
fn rf028_real_text_layer_returns_without_creating_temp_root() {
    let f = Fixture::new(true);
    let pages = extract_pdf_text(&f.input).expect("real PDFium must extract the text fixture");
    assert!(has_meaningful_text(&pages, 20));
    assert!(!f.temp_root.exists());
    let mut rendered = false;
    let mut recognized = false;
    let result = scan_pdf_with(
        &f.input,
        &f.temp_root,
        |_, _, _| {
            rendered = true;
            Err("text PDF must not render".into())
        },
        |_| {
            recognized = true;
            Err("text PDF must not invoke OCR".into())
        },
    )
    .unwrap();
    assert!(!rendered && !recognized);
    assert!(
        !f.temp_root.exists(),
        "text shortcut must never create the supplied temp root"
    );
    assert_eq!(result.confidence, 1.0);
    assert_eq!(result.boxes.len(), pages.len() * 2 - 1);
    let expected_text = pages
        .iter()
        .enumerate()
        .map(|(index, text)| {
            if index == 0 {
                text.clone()
            } else {
                format!("\n--- Page {} ---\n{text}", index + 1)
            }
        })
        .collect::<String>();
    assert_eq!(result.text, expected_text);
    for (index, text) in pages.iter().enumerate() {
        let item = &result.boxes[index * 2];
        assert_eq!(&item.text, text);
        assert_eq!(item.confidence, 1.0);
        assert_eq!(item.points, SEPARATOR_POINTS);
        if index > 0 {
            let separator = &result.boxes[index * 2 - 1];
            assert_eq!(separator.text, format!("--- Page {} ---", index + 1));
            assert_eq!(separator.confidence, 1.0);
            assert_eq!(separator.points, SEPARATOR_POINTS);
        }
    }
    f.assert_preserved();
}

#[test]
fn rf028_recognizer_unwind_releases_page_handle_and_cleans_workspace() {
    let f = Fixture::new(false);
    let mut workspace = None;
    let mut renderer_returned = false;
    let mut seen = Vec::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scan_pdf_with(
            &f.input,
            &f.temp_root,
            |input, dpi, output| {
                f.observe_workspace(output, &mut workspace);
                let paths = render_pdf_pages(input, dpi, output)?;
                assert_eq!(paths.len(), 3);
                renderer_returned = true;
                Ok(paths)
            },
            |path| {
                let _in_use = open_page_handle(path).map_err(|error| error.to_string())?;
                seen.push(inspect_png(path)?);
                if seen.len() == 2 {
                    panic!("RF028_SYNTHETIC_RECOGNIZER_PANIC");
                }
                Ok(recognized_page(seen.len() - 1))
            },
        )
    }));
    let payload = outcome.expect_err("recognizer must reach its deliberate unwind");
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(message, Some("RF028_SYNTHETIC_RECOGNIZER_PANIC"));
    assert!(renderer_returned);
    assert_eq!(seen.len(), 2);
    for (index, image) in seen.iter().enumerate() {
        assert_page(image, index);
    }
    // 故意 panic 在识别替身中发生；仍可再次使用真实 PDFium，不能污染其全局锁。
    let pages =
        extract_pdf_text(&f.input).expect("PDFium guard must remain usable after OCR unwind");
    assert_eq!(pages.len(), 3);
    assert!(pages.iter().all(|text| text.trim().is_empty()));
    f.assert_cleaned(workspace.as_deref().expect("renderer must be called"));
}
