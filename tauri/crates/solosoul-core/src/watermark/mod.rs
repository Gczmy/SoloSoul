//! 附件水印处理
//!
//! 支持对图片（PNG/JPEG/WEBP/BMP/GIF）和 PDF 添加文本水印。
//! 所有操作都在输入文件的副本上进行，不会修改保险库内原始附件。

use ab_glyph::{point, Font, FontRef, PxScale};
use image::{Rgba, RgbaImage};
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

/// 根据输入文件扩展名判断是否为 PDF
fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// 尝试加载系统等宽字体文件
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

/// 对文件添加水印，根据扩展名自动选择图片或 PDF 路径
pub fn apply_to_file(input: &Path, output: &Path, cfg: &str) -> Result<(), String> {
    let config = WatermarkConfig::from_json(cfg)?;
    if is_pdf(input) {
        apply_to_pdf(input, output, &config)
    } else {
        apply_to_image(input, output, &config)
    }
}

/// 对图片添加文本水印
pub fn apply_to_image(input: &Path, output: &Path, config: &WatermarkConfig) -> Result<(), String> {
    let font_data = load_font_bytes()?;
    let font = FontRef::try_from_slice(&font_data).map_err(|e| format!("加载字体失败: {}", e))?;

    let img = image::open(input).map_err(|e| format!("打开图片失败: {}", e))?;
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);

    // 将原图转换为 RGBA8
    let mut canvas = img.to_rgba8();

    // 手动布局文本（ab_glyph 0.2 未提供 layout 方法）
    let scale = PxScale::from(config.font_size);
    let baseline_y = config.font_size * 0.8;
    let mut glyphs: Vec<ab_glyph::Glyph> = Vec::with_capacity(config.text.chars().count());
    let mut cursor_x = 0.0f32;
    let mut prev_id: Option<ab_glyph::GlyphId> = None;
    for ch in config.text.chars() {
        let id = font.glyph_id(ch);
        if let Some(prev) = prev_id {
            cursor_x += font.kern_unscaled(prev, id) * scale.x;
        }
        glyphs.push(id.with_scale_and_position(scale, point(cursor_x, baseline_y)));
        cursor_x += font.h_advance_unscaled(id) * scale.x;
        prev_id = Some(id);
    }

    let mut text_w = 0.0f32;
    let mut text_h = 0.0f32;
    for glyph in &glyphs {
        if let Some(outline) = font.outline_glyph(glyph.clone()) {
            let bounds = outline.px_bounds();
            text_w = text_w.max(bounds.max.x);
            text_h = text_h.max(bounds.max.y - bounds.min.y);
        }
    }
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

    // 旋转临时层
    let rotated = rotate_rgba(&layer, config.angle);

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
    canvas
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
        let mut pts = Vec::new();
        let mut x = -step_x / 2;
        while x < canvas_w + step_x {
            let mut y = -step_y / 2;
            while y < canvas_h + step_y {
                pts.push((x + (canvas_w - layer_w) / 2 % step_x - step_x, y));
                y += step_y;
            }
            x += step_x;
        }
        // 重新生成平铺网格，以左上角为起点
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

/// 对 PDF 添加文本水印
pub fn apply_to_pdf(input: &Path, output: &Path, config: &WatermarkConfig) -> Result<(), String> {
    let mut doc = lopdf::Document::load(input).map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let pages = doc.get_pages();
    let font_name = "WSHelv";
    let gstate_name = "WSGS";

    for &page_id in pages.values() {
        // 先获取页面尺寸（对 doc 的不可变借用）
        let (page_w, page_h) = page_size(&doc, page_id)?;

        // 构建水印内容流（仍不需要借用 doc）
        let watermark_stream =
            build_pdf_watermark_stream(config, font_name, gstate_name, page_w, page_h)?;
        let watermark_id = doc.add_object(lopdf::Object::Stream(watermark_stream));

        // 再借用页面对象，设置资源并追加内容
        let page_obj = doc
            .get_object_mut(page_id)
            .map_err(|e| format!("获取 PDF 页面对象失败: {}", e))?
            .as_dict_mut()
            .map_err(|e| format!("PDF 页面对象不是字典: {}", e))?;

        let mut resources = page_obj
            .get(b"Resources")
            .ok()
            .and_then(|r| r.as_dict().ok().cloned())
            .unwrap_or_default();

        // 添加字体资源（标准 Helvetica）
        let mut font_dict = lopdf::Dictionary::new();
        font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        font_dict.set("Encoding", lopdf::Object::Name(b"WinAnsiEncoding".to_vec()));
        resources.set(font_name, lopdf::Object::Dictionary(font_dict));

        // 添加透明 ExtGState
        let alpha = config.opacity.clamp(0.0, 1.0);
        let mut gs_dict = lopdf::Dictionary::new();
        gs_dict.set("Type", lopdf::Object::Name(b"ExtGState".to_vec()));
        gs_dict.set("CA", lopdf::Object::Real(alpha));
        gs_dict.set("ca", lopdf::Object::Real(alpha));

        let mut ext_gstates = resources
            .get(b"ExtGState")
            .ok()
            .and_then(|r| r.as_dict().ok().cloned())
            .unwrap_or_default();
        ext_gstates.set(gstate_name, lopdf::Object::Dictionary(gs_dict));
        resources.set("ExtGState", lopdf::Object::Dictionary(ext_gstates));

        page_obj.set("Resources", lopdf::Object::Dictionary(resources));

        // 将新内容追加到页面 Contents
        match page_obj.get_mut(b"Contents") {
            Ok(lopdf::Object::Reference(r)) => {
                let arr = lopdf::Object::Array(vec![
                    lopdf::Object::Reference(*r),
                    lopdf::Object::Reference(watermark_id),
                ]);
                page_obj.set("Contents", arr);
            }
            Ok(lopdf::Object::Array(arr)) => {
                arr.push(lopdf::Object::Reference(watermark_id));
            }
            Ok(_) | Err(_) => {
                page_obj.set(
                    "Contents",
                    lopdf::Object::Array(vec![lopdf::Object::Reference(watermark_id)]),
                );
            }
        }
    }

    doc.save(output)
        .map_err(|e| format!("保存 PDF 失败: {}", e))?;
    Ok(())
}

/// 构建单页 PDF 水印内容流
fn build_pdf_watermark_stream(
    config: &WatermarkConfig,
    font_name: &str,
    gstate_name: &str,
    page_w: f64,
    page_h: f64,
) -> Result<lopdf::Stream, String> {
    let (tx, ty) = pdf_text_position(page_w, page_h, config.font_size, config);

    let [r, g, b] = config.color;
    let rad = config.angle.to_radians();
    let cos_a = rad.cos() as f64;
    let sin_a = rad.sin() as f64;

    let name = |s: &str| lopdf::Object::Name(s.as_bytes().to_vec());
    let real = |v: f64| lopdf::Object::Real(v as f32);

    let ops = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("gs", vec![name(gstate_name)]),
        lopdf::content::Operation::new("Tf", vec![name(font_name), real(config.font_size as f64)]),
        lopdf::content::Operation::new(
            "rg",
            vec![
                real(r as f64 / 255.0),
                real(g as f64 / 255.0),
                real(b as f64 / 255.0),
            ],
        ),
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new(
            "Tm",
            vec![
                real(cos_a),
                real(sin_a),
                real(-sin_a),
                real(cos_a),
                real(tx),
                real(ty),
            ],
        ),
        lopdf::content::Operation::new(
            "Tj",
            vec![lopdf::Object::String(
                escape_pdf_bytes(&config.text),
                lopdf::StringFormat::Literal,
            )],
        ),
        lopdf::content::Operation::new("ET", vec![]),
        lopdf::content::Operation::new("Q", vec![]),
    ];

    let content = lopdf::content::Content { operations: ops };
    let encoded = content
        .encode()
        .map_err(|e| format!("编码 PDF 内容流失败: {}", e))?;
    Ok(lopdf::Stream::new(lopdf::Dictionary::new(), encoded))
}

/// 获取页面尺寸（宽，高）
fn page_size(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<(f64, f64), String> {
    let page_obj = doc
        .get_object(page_id)
        .map_err(|e| format!("获取页面失败: {}", e))?
        .as_dict()
        .map_err(|e| format!("页面对象不是字典: {}", e))?;

    let media_box = page_obj
        .get(b"MediaBox")
        .or_else(|_| page_obj.get(b"CropBox"))
        .map_err(|_| "页面缺少 MediaBox/CropBox".to_string())?;

    let arr = media_box
        .as_array()
        .map_err(|_| "MediaBox 不是数组".to_string())?;
    if arr.len() != 4 {
        return Err("MediaBox 长度不正确".to_string());
    }
    let x1 = as_f64(&arr[0])?;
    let y1 = as_f64(&arr[1])?;
    let x2 = as_f64(&arr[2])?;
    let y2 = as_f64(&arr[3])?;
    Ok((x2 - x1, y2 - y1))
}

fn as_f64(obj: &lopdf::Object) -> Result<f64, String> {
    match obj {
        lopdf::Object::Real(v) => Ok(*v as f64),
        lopdf::Object::Integer(v) => Ok(*v as f64),
        _ => Err("PDF 数值类型不支持".to_string()),
    }
}

fn pdf_text_position(
    page_w: f64,
    page_h: f64,
    font_size: f32,
    cfg: &WatermarkConfig,
) -> (f64, f64) {
    let text_w = cfg.text.len() as f64 * font_size as f64 * 0.55;
    let text_h = font_size as f64;
    let margin_x = cfg.margin_x as f64;
    let margin_y = cfg.margin_y as f64;

    match cfg.position {
        WatermarkPosition::Center => ((page_w - text_w) / 2.0, (page_h + text_h) / 2.0),
        WatermarkPosition::TopLeft => (margin_x, page_h - margin_y - text_h),
        WatermarkPosition::TopRight => (page_w - margin_x - text_w, page_h - margin_y - text_h),
        WatermarkPosition::BottomLeft => (margin_x, margin_y + text_h),
        WatermarkPosition::BottomRight => (page_w - margin_x - text_w, margin_y + text_h),
        WatermarkPosition::Tile => ((page_w - text_w) / 2.0, (page_h + text_h) / 2.0),
    }
}

/// 转义 PDF 字符串中的特殊字符，返回 WinAnsiEncoding 兼容字节。
fn escape_pdf_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push(b'\\');
                out.push(c as u8);
            }
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 128 => out.push(c as u8),
            _ => out.push(b'?'),
        }
    }
    out
}
