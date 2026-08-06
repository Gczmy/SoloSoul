//! 附件水印处理
//!
//! 支持对图片（PNG/JPEG/WEBP/BMP/GIF）和 PDF 添加文本水印。
//! 所有操作都在输入文件的副本上进行，不会修改保险库内原始附件。

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use pdfium_render::prelude::*;
use serde::Deserialize;
use std::path::Path;

/// 水印位置
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WatermarkPosition {
    #[default]
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Tile,
}

/// 水印配置
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkConfig {
    #[serde(default = "default_text")]
    pub text: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_color")]
    pub color: [u8; 3],
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_angle")]
    pub angle: f32,
    #[serde(default)]
    pub position: WatermarkPosition,
    #[serde(default)]
    pub tile: bool,
    #[serde(default)]
    pub margin_x: i32,
    #[serde(default)]
    pub margin_y: i32,
}

fn default_text() -> String {
    "SoloSoul".to_string()
}
fn default_font_size() -> f32 {
    72.0
}
fn default_color() -> [u8; 3] {
    [128, 128, 128]
}
fn default_opacity() -> f32 {
    0.3
}
fn default_angle() -> f32 {
    -45.0
}

impl Default for WatermarkConfig {
    fn default() -> Self {
        Self {
            text: default_text(),
            font_size: default_font_size(),
            color: default_color(),
            opacity: default_opacity(),
            angle: default_angle(),
            position: WatermarkPosition::Center,
            tile: false,
            margin_x: 0,
            margin_y: 0,
        }
    }
}

impl WatermarkConfig {
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("解析水印配置失败: {}", e))
    }
}

/// 尝试加载系统等宽字体文件（供图片水印使用）
fn load_font_bytes() -> Result<Vec<u8>, String> {
    let candidates: &[&str] = &[
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/System/Library/Fonts/Courier New.ttf",
        "C:\\Windows\\Fonts\\cour.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    ];

    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            if path.ends_with(".ttc") {
                if let Ok(single) = extract_first_from_ttc(&data) {
                    return Ok(single);
                }
                continue;
            }
            return Ok(data);
        }
    }

    Err("未找到系统字体，无法渲染水印".to_string())
}

/// 从 TTC（TrueType Collection）中提取第一个字体。
/// 当 PDFium / ab_glyph 无法直接读取集合字体时用作 fallback。
fn extract_first_from_ttc(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 16 {
        return Err("TTC 文件过短".to_string());
    }
    if &data[0..4] != b"ttcf" {
        return Err("不是 TTC 格式".to_string());
    }
    let num_fonts = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
    if num_fonts == 0 {
        return Err("TTC 中无字体".to_string());
    }
    let offset = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    if offset >= data.len() {
        return Err("TTC 偏移越界".to_string());
    }
    let size = if num_fonts > 1 {
        let next_offset = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as usize;
        next_offset.min(data.len()).saturating_sub(offset)
    } else {
        data.len().saturating_sub(offset)
    };
    Ok(data[offset..offset + size].to_vec())
}

/// 对图片添加文本水印
pub fn apply_to_image(input: &Path, output: &Path, config: &WatermarkConfig) -> Result<(), String> {
    let font_data = load_font_bytes()?;
    let font = FontRef::try_from_slice(&font_data).map_err(|e| format!("加载字体失败: {}", e))?;

    let img = image::open(input).map_err(|e| format!("打开图片失败: {e}"))?;
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);

    // 将原图转换为 RGBA8
    let mut canvas = img.to_rgba8();

    // 手动布局文本（ab_glyph 0.2 未提供 layout 方法）
    let scale = PxScale::from(config.font_size);
    let scaled = font.as_scaled(scale);
    let baseline_y = config.font_size * 0.8;
    let mut glyphs: Vec<ab_glyph::Glyph> = Vec::with_capacity(config.text.chars().count());
    let mut cursor_x = 0.0f32;
    let mut prev_id: Option<ab_glyph::GlyphId> = None;
    for ch in config.text.chars() {
        let id = font.glyph_id(ch);
        if let Some(prev) = prev_id {
            cursor_x += scaled.kern(prev, id);
        }
        glyphs.push(id.with_scale_and_position(scale, point(cursor_x, baseline_y)));
        cursor_x += scaled.h_advance(id);
        prev_id = Some(id);
    }

    let mut max_ink_x = 0.0f32;
    let mut text_h = 0.0f32;
    for glyph in &glyphs {
        if let Some(outline) = font.outline_glyph(glyph.clone()) {
            let bounds = outline.px_bounds();
            max_ink_x = max_ink_x.max(bounds.max.x);
            text_h = text_h.max(bounds.max.y - bounds.min.y);
        }
    }
    // 文本宽度取排版总前进距与最大墨BBox右端的最大值，避免斜体等突出部分被截断。
    let text_w = cursor_x.max(max_ink_x);
    if text_w <= 0.0 || text_h <= 0.0 {
        return Err("水印文本渲染后尺寸为零".to_string());
    }

    // 在临时层上水平绘制文本
    let layer_w = text_w.ceil() as u32 + 4;
    let layer_h = (text_h + config.font_size * 0.3).ceil() as u32 + 4;
    let mut layer = RgbaImage::from_pixel(layer_w, layer_h, Rgba([0, 0, 0, 0]));

    let [r, g, b] = config.color;
    let base_alpha = config.opacity.clamp(0.0, 1.0);

    for glyph in glyphs {
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            let min_x = bounds.min.x as i32;
            let min_y = bounds.min.y as i32;
            outline.draw(|x, y, c| {
                let px = min_x + x as i32 + 2;
                let py = min_y + y as i32 + 2;
                if px >= 0 && px < layer_w as i32 && py >= 0 && py < layer_h as i32 {
                    let alpha = (c * base_alpha * 255.0) as u8;
                    if alpha > 0 {
                        layer.put_pixel(px as u32, py as u32, Rgba([r, g, b, alpha]));
                    }
                }
            });
        }
    }

    // 旋转临时层（0° 时直接复用，避免无意义的重采样）
    let rotated = if config.angle == 0.0 {
        layer
    } else {
        rotate_rgba(&layer, config.angle)
    };

    // 计算绘制位置
    let positions = compute_positions(
        img_w,
        img_h,
        rotated.width() as i32,
        rotated.height() as i32,
        config,
    );

    for (dx, dy) in positions {
        blend_at(&mut canvas, &rotated, dx, dy);
    }

    let fmt = guess_image_format(output).unwrap_or(image::ImageFormat::Png);
    image::DynamicImage::ImageRgba8(canvas)
        .save_with_format(output, fmt)
        .map_err(|e| format!("保存图片失败: {}", e))?;
    Ok(())
}

/// 根据扩展名推断图片保存格式
fn guess_image_format(path: &Path) -> Option<image::ImageFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some(image::ImageFormat::Png),
        "jpg" | "jpeg" => Some(image::ImageFormat::Jpeg),
        "webp" => Some(image::ImageFormat::WebP),
        "bmp" => Some(image::ImageFormat::Bmp),
        "gif" => Some(image::ImageFormat::Gif),
        "tiff" | "tif" => Some(image::ImageFormat::Tiff),
        _ => None,
    }
}

/// 最近邻旋转 RGBA 图像（角度为度，绕中心旋转）
fn rotate_rgba(img: &RgbaImage, angle_deg: f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let rad = angle_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    // 计算旋转后外接矩形
    let corners = [
        (-w / 2.0, -h / 2.0),
        (w / 2.0, -h / 2.0),
        (w / 2.0, h / 2.0),
        (-w / 2.0, h / 2.0),
    ];
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (x, y) in corners {
        let rx = x * cos - y * sin;
        let ry = x * sin + y * cos;
        min_x = min_x.min(rx);
        max_x = max_x.max(rx);
        min_y = min_y.min(ry);
        max_y = max_y.max(ry);
    }
    let out_w = (max_x - min_x).ceil() as u32;
    let out_h = (max_y - min_y).ceil() as u32;
    let mut out = RgbaImage::from_pixel(out_w, out_h, Rgba([0, 0, 0, 0]));

    let cx = out_w as f32 / 2.0;
    let cy = out_h as f32 / 2.0;

    for y in 0..out_h {
        for x in 0..out_w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let sx = dx * cos + dy * sin + w / 2.0;
            let sy = -dx * sin + dy * cos + h / 2.0;
            let sx_i = sx.round() as i32;
            let sy_i = sy.round() as i32;
            if sx_i >= 0 && sx_i < w as i32 && sy_i >= 0 && sy_i < h as i32 {
                out.put_pixel(x, y, *img.get_pixel(sx_i as u32, sy_i as u32));
            }
        }
    }
    out
}

/// 根据配置计算水印在画布上的所有锚点（左上角坐标）
fn compute_positions(
    canvas_w: i32,
    canvas_h: i32,
    layer_w: i32,
    layer_h: i32,
    cfg: &WatermarkConfig,
) -> Vec<(i32, i32)> {
    let margin_x = cfg.margin_x;
    let margin_y = cfg.margin_y;

    if cfg.tile || cfg.position == WatermarkPosition::Tile {
        let step_x = layer_w + margin_x.max(20);
        let step_y = layer_h + margin_y.max(20);
        let mut tiled = Vec::new();
        let start_x = margin_x;
        let start_y = margin_y;
        let mut tx = start_x;
        while tx < canvas_w {
            let mut ty = start_y;
            while ty < canvas_h {
                tiled.push((tx - layer_w / 2, ty - layer_h / 2));
                ty += step_y;
            }
            tx += step_x;
        }
        return tiled;
    }

    let (x, y) = match cfg.position {
        WatermarkPosition::Center => ((canvas_w - layer_w) / 2, (canvas_h - layer_h) / 2),
        WatermarkPosition::TopLeft => (margin_x, margin_y),
        WatermarkPosition::TopRight => (canvas_w - layer_w - margin_x, margin_y),
        WatermarkPosition::BottomLeft => (margin_x, canvas_h - layer_h - margin_y),
        WatermarkPosition::BottomRight => {
            (canvas_w - layer_w - margin_x, canvas_h - layer_h - margin_y)
        }
        WatermarkPosition::Tile => unreachable!(),
    };
    vec![(x, y)]
}

/// 将 layer 按 alpha 混合到 canvas 的 (dx, dy) 位置
fn blend_at(canvas: &mut RgbaImage, layer: &RgbaImage, dx: i32, dy: i32) {
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    for y in 0..layer.height() {
        for x in 0..layer.width() {
            let cx = dx + x as i32;
            let cy = dy + y as i32;
            if cx < 0 || cx >= cw || cy < 0 || cy >= ch {
                continue;
            }
            let src = layer.get_pixel(x, y).0;
            let sa = src[3] as f32 / 255.0;
            if sa <= 0.0 {
                continue;
            }
            let dst = canvas.get_pixel(cx as u32, cy as u32).0;
            let inv = 1.0 - sa;
            let r = (src[0] as f32 * sa + dst[0] as f32 * inv) as u8;
            let g = (src[1] as f32 * sa + dst[1] as f32 * inv) as u8;
            let b = (src[2] as f32 * sa + dst[2] as f32 * inv) as u8;
            canvas.put_pixel(cx as u32, cy as u32, Rgba([r, g, b, dst[3]]));
        }
    }
}

// =============================================================================
// PDF 水印（pdfium-render + PDFium）
// =============================================================================

/// 对 PDF 添加文本水印。
///
/// 使用 PDFium 的页面对象插入能力，以 `PdfPages::watermark()` 在每一页上添加
/// 文本对象。PDFium 会自动处理图形状态隔离，因此不会破坏原有内容流。
pub fn apply_to_pdf(input: &Path, output: &Path, config: &WatermarkConfig) -> Result<(), String> {
    let pdfium = crate::pdfium::init_pdfium()?;
    let mut document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|e| format!("加载 PDF 失败: {e}"))?;

    // 预加载字体；水印闭包无法同时可变借用 document。
    let font = load_pdf_font(&mut document, &config.text)?;

    let text = if config.text.is_empty() {
        " ".to_string()
    } else {
        config.text.clone()
    };
    let alpha = (config.opacity.clamp(0.0, 1.0) * 255.0) as u8;
    let [r, g, b] = config.color;
    let fill_color = PdfColor::new(r, g, b, alpha);
    let angle = config.angle;
    let position = config.position;
    let tile = config.tile || position == WatermarkPosition::Tile;
    let margin_x = config.margin_x;
    let margin_y = config.margin_y;
    let font_size = config.font_size;

    document
        .pages()
        .watermark(|group, _page_index, page_w, page_h| {
            if tile {
                add_tiled_text_watermarks(
                    group,
                    &document,
                    &font,
                    &text,
                    font_size,
                    fill_color,
                    angle,
                    page_w.value,
                    page_h.value,
                    margin_x,
                    margin_y,
                )?;
            } else {
                let mut text_obj =
                    create_watermark_text_object(&document, &font, &text, font_size, fill_color)?;
                let text_w = text_obj.width()?.value;
                let text_h = text_obj.height()?.value;
                let (tx, ty) = pdf_text_position(
                    page_w.value,
                    page_h.value,
                    text_w,
                    text_h,
                    position,
                    margin_x,
                    margin_y,
                );
                // 以文本中心为锚点旋转，再平移到目标位置
                rotate_around_center(&mut text_obj, text_w / 2.0, text_h / 2.0, angle)?;
                text_obj.translate(PdfPoints::new(tx), PdfPoints::new(ty))?;
                group.push(&mut PdfPageObject::from(text_obj))?;
            }
            Ok(())
        })
        .map_err(|e| format!("添加水印失败: {e}"))?;

    document
        .save_to_file(output)
        .map_err(|e| format!("保存 PDF 失败: {e}"))?;
    Ok(())
}

/// 将自定义错误信息转换为 PDFium 错误类型，以便在水印闭包中使用。
fn pdfium_err(msg: String) -> PdfiumError {
    PdfiumError::IoError(std::io::Error::other(msg))
}

/// 创建一个已设置好文本、颜色、透明度的水印文本对象。
fn create_watermark_text_object<'a>(
    document: &PdfDocument<'a>,
    font: &PdfFontToken,
    text: &str,
    font_size: f32,
    fill_color: PdfColor,
) -> Result<PdfPageTextObject<'a>, PdfiumError> {
    let mut text_obj = PdfPageTextObject::new(document, text, *font, PdfPoints::new(font_size))
        .map_err(|e| pdfium_err(format!("创建文本对象失败: {e}")))?;
    text_obj
        .set_fill_color(fill_color)
        .map_err(|e| pdfium_err(format!("设置文本颜色失败: {e}")))?;
    Ok(text_obj)
}

/// 以 (cx, cy) 为中心旋转文本对象。
fn rotate_around_center(
    text_obj: &mut PdfPageTextObject,
    cx: f32,
    cy: f32,
    angle_deg: f32,
) -> Result<(), PdfiumError> {
    if angle_deg == 0.0 {
        return Ok(());
    }
    text_obj
        .translate(PdfPoints::new(-cx), PdfPoints::new(-cy))
        .map_err(|e| pdfium_err(format!("平移失败: {e}")))?;
    text_obj
        .rotate_counter_clockwise_degrees(angle_deg)
        .map_err(|e| pdfium_err(format!("旋转失败: {e}")))?;
    text_obj
        .translate(PdfPoints::new(cx), PdfPoints::new(cy))
        .map_err(|e| pdfium_err(format!("平移失败: {e}")))?;
    Ok(())
}

/// 在页面上平铺多个水印文本对象。
#[allow(clippy::too_many_arguments)]
fn add_tiled_text_watermarks<'a>(
    group: &mut PdfPageGroupObject<'a>,
    document: &PdfDocument<'a>,
    font: &PdfFontToken,
    text: &str,
    font_size: f32,
    fill_color: PdfColor,
    angle: f32,
    page_w: f32,
    page_h: f32,
    margin_x: i32,
    margin_y: i32,
) -> Result<(), PdfiumError> {
    // 先建一个原型对象用于测量尺寸
    let prototype = create_watermark_text_object(document, font, text, font_size, fill_color)?;
    let text_w = prototype
        .width()
        .map_err(|e| pdfium_err(format!("测量宽度失败: {e}")))?
        .value;
    let text_h = prototype
        .height()
        .map_err(|e| pdfium_err(format!("测量高度失败: {e}")))?
        .value;
    drop(prototype);

    let step_x = text_w + margin_x.max(20) as f32;
    let step_y = text_h + margin_y.max(20) as f32;

    let mut x = margin_x as f32;
    while x < page_w {
        let mut y = margin_y as f32;
        while y < page_h {
            let mut text_obj =
                create_watermark_text_object(document, font, text, font_size, fill_color)?;
            rotate_around_center(&mut text_obj, text_w / 2.0, text_h / 2.0, angle)?;
            text_obj
                .translate(PdfPoints::new(x), PdfPoints::new(y))
                .map_err(|e| pdfium_err(format!("平铺平移失败: {e}")))?;
            group
                .push(&mut PdfPageObject::from(text_obj))
                .map_err(|e| pdfium_err(format!("加入水印对象失败: {e}")))?;
            y += step_y;
        }
        x += step_x;
    }
    Ok(())
}

/// 根据位置参数计算单条水印的左下角锚点。
fn pdf_text_position(
    page_w: f32,
    page_h: f32,
    text_w: f32,
    text_h: f32,
    position: WatermarkPosition,
    margin_x: i32,
    margin_y: i32,
) -> (f32, f32) {
    let mx = margin_x as f32;
    let my = margin_y as f32;
    match position {
        WatermarkPosition::Center => ((page_w - text_w) / 2.0, (page_h - text_h) / 2.0),
        WatermarkPosition::TopLeft => (mx, page_h - my - text_h),
        WatermarkPosition::TopRight => (page_w - mx - text_w, page_h - my - text_h),
        WatermarkPosition::BottomLeft => (mx, my),
        WatermarkPosition::BottomRight => (page_w - mx - text_w, my),
        WatermarkPosition::Tile => ((page_w - text_w) / 2.0, (page_h - text_h) / 2.0),
    }
}

/// 为 PDF 水印加载字体。
///
/// 如果水印文本包含非 ASCII 字符，优先尝试加载系统 CJK 字体；
/// 失败时回退到 PDFium 内置 Helvetica。
fn load_pdf_font<'a>(document: &mut PdfDocument<'a>, text: &str) -> Result<PdfFontToken, String> {
    let needs_cjk = !text.is_ascii();
    if needs_cjk {
        for path in cjk_font_candidates() {
            if let Ok(token) = try_load_font(document, path) {
                return Ok(token);
            }
        }
    }
    Ok(document.fonts_mut().helvetica())
}

/// 各平台 CJK 字体候选路径。
fn cjk_font_candidates() -> Vec<&'static str> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "macos")]
    {
        candidates.push("/System/Library/Fonts/PingFang.ttc");
        candidates.push("/System/Library/Fonts/Hiragino Sans GB.ttc");
        candidates.push("/Library/Fonts/Arial Unicode.ttf");
    }
    #[cfg(target_os = "windows")]
    {
        candidates.push(r"C:\Windows\Fonts\msyh.ttc");
        candidates.push(r"C:\Windows\Fonts\simsun.ttc");
        candidates.push(r"C:\Windows\Fonts\simhei.ttf");
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc");
        candidates.push("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc");
        candidates.push("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc");
    }
    candidates
}

/// 尝试从路径加载 TrueType 字体；对 TTC 先提取首字体再经内存加载。
fn try_load_font<'a>(document: &mut PdfDocument<'a>, path: &str) -> Result<PdfFontToken, String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".ttc") {
        let data = std::fs::read(path).map_err(|e| format!("读取字体失败: {e}"))?;
        let single = extract_first_from_ttc(&data)?;
        // R2-05: 直接经内存加载 TTC 首字体——pdfium-render 的 load_true_type_from_bytes
        // 内部经 FPDFText_LoadFont 复制字体数据到 PDFium 内存，字体数据无需（也不能依赖）
        // 临时文件存活期。原实现写 NamedTempFile 后立即 `let _ = temp;` drop（文件随即删除），
        // 仅因 Pdfium 急切读入内存才"靠运气正确"，且注释与实现自相矛盾；
        // 现彻底消除临时文件生命周期隐患。
        document
            .fonts_mut()
            .load_true_type_from_bytes(&single, true)
            .map_err(|e| format!("加载 TTC 字体失败: {e}"))
    } else {
        document
            .fonts_mut()
            .load_true_type_from_file(path, true)
            .map_err(|e| format!("加载字体失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_positions_center() {
        let cfg = WatermarkConfig {
            position: WatermarkPosition::Center,
            ..Default::default()
        };
        let pts = compute_positions(1000, 800, 200, 50, &cfg);
        assert_eq!(pts, vec![(400, 375)]);
    }

    #[test]
    fn test_pdf_text_position_center() {
        let (x, y) = pdf_text_position(
            1000.0_f32,
            800.0_f32,
            200.0_f32,
            50.0_f32,
            WatermarkPosition::Center,
            0,
            0,
        );
        assert_eq!(x, 400.0_f32);
        assert_eq!(y, 375.0_f32);
    }

    #[test]
    fn test_extract_first_from_ttc_invalid() {
        assert!(extract_first_from_ttc(b"not ttc").is_err());
    }

    /// 使用 fixtures/testdata/minimal.pdf 验证 pdfium-render 水印不会崩溃。
    #[test]
    fn test_apply_to_pdf_smoke() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("output.pdf");

        const MINIMAL_PDF: &[u8] = include_bytes!("testdata/minimal.pdf");
        std::fs::write(&input, MINIMAL_PDF).expect("write input pdf");

        // 测试环境下指向仓库中的 PDFium 动态库。
        let dylib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../src-tauri/resources/pdfium/libpdfium.dylib");
        if dylib_path.exists() {
            std::env::set_var("PDFIUM_LIBRARY_PATH", &dylib_path);
        }

        let cfg = WatermarkConfig {
            text: "SoloSoul".to_string(),
            font_size: 48.0,
            color: [128, 128, 128],
            opacity: 0.3,
            angle: -45.0,
            position: WatermarkPosition::Center,
            tile: false,
            margin_x: 0,
            margin_y: 0,
        };

        let result = apply_to_pdf(&input, &output, &cfg);
        assert!(result.is_ok(), "PDF 水印应成功: {:?}", result);
        assert!(output.exists(), "输出文件应存在");
        assert!(output.metadata().unwrap().len() > 0, "输出文件不应为空");

        // 进一步确认 PDF 仍能被 PDFium 加载，并且至少保留 1 页。
        let pdfium = crate::pdfium::init_pdfium().expect("init pdfium");
        let doc = pdfium
            .load_pdf_from_file(&output, None)
            .expect("reload output pdf");
        assert_eq!(doc.pages().len(), 1, "页数应保持不变");
    }

    /// 验证图片水印不会陷入无限大内存/超时，并正确生成输出文件。
    #[test]
    fn test_apply_to_image_smoke() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("input.jpg");
        let output = dir.path().join("output.jpg");

        let rgb = image::RgbImage::from_pixel(200, 150, image::Rgb([255, 255, 255]));
        rgb.save_with_format(&input, image::ImageFormat::Jpeg)
            .expect("create test jpeg");

        let cfg = WatermarkConfig {
            text: "SoloSoul".to_string(),
            font_size: 48.0,
            color: [128, 128, 128],
            opacity: 0.5,
            angle: -30.0,
            position: WatermarkPosition::Center,
            tile: false,
            margin_x: 0,
            margin_y: 0,
        };

        let start = std::time::Instant::now();
        let result = apply_to_image(&input, &output, &cfg);
        let elapsed = start.elapsed();
        assert!(result.is_ok(), "图片水印应成功: {:?}", result);
        assert!(output.exists(), "输出文件应存在");
        assert!(output.metadata().unwrap().len() > 0, "输出文件不应为空");
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "图片水印应在合理时间内完成，实际耗时 {:?}",
            elapsed
        );
    }
}
