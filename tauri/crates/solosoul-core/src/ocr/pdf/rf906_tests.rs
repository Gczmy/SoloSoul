use super::render_pdf_pages;
use pdfium_render::prelude::*;
use std::path::Path;

/// 相邻页的 R/B 分量相差 4，允许至多 1 级渲染取整误差仍能区分页序。
fn indexed_colors(page_count: usize) -> Vec<[u8; 3]> {
    (0..page_count)
        .map(|index| {
            let value = u8::try_from(index * 4).unwrap();
            [value, 32, 224 - value]
        })
        .collect()
}

/// 仅返回页数与文本；创建、重载产生的所有原生对象及 guard 均在返回前释放。
/// 持锁时使用 Result 传播错误，外层再断言，避免污染全局 PDFium Mutex。
fn write_and_inspect_pdf(path: &Path, colors: &[[u8; 3]]) -> Result<(usize, Vec<String>), String> {
    // 库缺失是测试失败；不在并行测试内修改进程环境。
    let pdfium = crate::pdfium::init_pdfium().map_err(|error| {
        format!("RF906 requires the real PDFium library; configure PDFIUM_LIBRARY_PATH: {error}")
    })?;
    {
        let mut document = pdfium
            .create_new_pdf()
            .map_err(|error| format!("create synthetic PDF: {error}"))?;
        for &[red, green, blue] in colors {
            let mut page = document
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::new_custom(
                    PdfPoints::new(72.0),
                    PdfPoints::new(72.0),
                ))
                .map_err(|error| format!("append synthetic color page: {error}"))?;
            page.objects_mut()
                .create_path_object_rect(
                    PdfRect::new_from_values(0.0, 0.0, 72.0, 72.0),
                    None,
                    None,
                    Some(PdfColor::new(red, green, blue, 255)),
                )
                .map_err(|error| format!("fill synthetic color page: {error}"))?;
        }
        document
            .save_to_file(path)
            .map_err(|error| format!("save synthetic PDF: {error}"))?;
    }

    // 从磁盘逐页重载，证明夹具完整；渲染颜色仍与原始 colors 独立比较。
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|error| format!("reload synthetic PDF: {error}"))?;
    let page_count = document.pages().len();
    let mut page_texts = Vec::with_capacity(page_count as usize);
    for index in 0..page_count {
        let page = document
            .pages()
            .get(index)
            .map_err(|error| format!("reload synthetic page {}: {error}", index + 1))?;
        let text = page
            .text()
            .map_err(|error| format!("read synthetic page {} text: {error}", index + 1))?
            .to_string();
        page_texts.push(text);
    }
    Ok((page_count as usize, page_texts))
}

fn assert_rendered_pages(colors: &[[u8; 3]], dpi: u32) {
    let dir = tempfile::tempdir().expect("create synthetic PDF directory");
    let input = dir.path().join("colored-pages.pdf");
    let output = dir.path().join("rendered");
    std::fs::create_dir(&output).expect("create PNG output directory");
    let (page_count, page_texts) =
        write_and_inspect_pdf(&input, colors).expect("create and inspect real PDF fixture");
    assert_eq!(page_count, colors.len());
    assert_eq!(page_texts.len(), page_count);
    assert!(page_texts.iter().all(|text| text.trim().is_empty()));

    // helper 已释放 guard；生产入口会自行获取同一把锁，不能在持锁时递归调用。
    let paths = render_pdf_pages(&input, dpi, &output).unwrap_or_else(|error| {
        panic!(
            "rendering {} valid color pages must succeed: {error}",
            colors.len()
        )
    });
    let expected_count = colors.len().min(50);
    assert_eq!(paths.len(), expected_count);
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), expected_count);
    assert!(!output.join("page_0000.png").exists());
    assert!(!output.join("page_0051.png").exists());
    for (index, (path, expected)) in paths.iter().zip(colors.iter()).enumerate() {
        assert_eq!(path, &output.join(format!("page_{:04}.png", index + 1)));
        let png = image::open(path)
            .expect("decode actual rendered PNG")
            .to_rgb8();
        assert_eq!(
            png.dimensions(),
            (dpi, dpi),
            "page {} dimensions",
            index + 1
        );
        // 避开页面边缘抗锯齿，检查多个实色区域；不是只检查存在或 PNG 数量。
        for (x, y) in [(8, 8), (dpi / 2, dpi / 2), (dpi - 9, dpi - 9)] {
            let actual = png.get_pixel(x, y).0;
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 1,
                    "page {} pixel ({x},{y}): expected {expected:?}, got {actual:?}",
                    index + 1,
                );
            }
        }
    }
}

#[test]
fn rf906_single_page_renders_first_page_as_page_0001() {
    assert_rendered_pages(&[[255, 0, 0]], 150);
}

#[test]
fn rf906_three_pages_preserve_first_middle_and_last_colors() {
    assert_rendered_pages(&[[255, 0, 0], [0, 255, 0], [0, 0, 255]], 72);
}

#[test]
fn rf906_fifty_pages_render_every_page_in_order() {
    assert_rendered_pages(&indexed_colors(50), 72);
}

#[test]
fn rf906_fifty_one_pages_render_only_the_first_fifty_in_order() {
    assert_rendered_pages(&indexed_colors(51), 72);
}
