//! MRZ 模板匹配：字符切分 + NCC 识别
//!
//! 用 ab_glyph 从系统字体渲染 37 个 MRZ 字符模板（A-Z, 0-9, <），
//! 然后对切分的字符图像做归一化互相关（NCC）匹配。
//!
//! 之所以不依赖 PP-OCR rec 模型，是因为 MRZ 行在原始图像中
//! 只有约 10-15px 高，rec 模型的 48×320 输入会导致文字过度压缩。

use ab_glyph::{point, Font, FontRef, PxScale};
use image::{imageops::FilterType, GrayImage, Luma};

/// MRZ 字符集：A-Z, 0-9, <
const MRZ_CHARS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '<',
];

/// 模板渲染尺寸（像素）
const TEMPLATE_SIZE: u32 = 64;

/// 加载字体文件并返回字节数组
fn load_font_bytes() -> Result<Vec<u8>, String> {
    // 尝试多个常见系统字体路径
    let candidates = [
        // macOS (courier in Supplemental/, monaco/menlo in root)
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/System/Library/Fonts/Courier New.ttf",
        // Windows
        "C:\\Windows\\Fonts\\cour.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    ];

    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            // Menlo.ttc 是字体集合，需要特殊处理
            if path.ends_with(".ttc") {
                if let Ok(single) = extract_first_from_ttc(&data) {
                    return Ok(single);
                }
                continue;
            }
            return Ok(data);
        }
    }

    Err("未找到系统等宽字体".to_string())
}

/// 从 TTC（TrueType Collection）中提取第一个字体
/// TTC 格式：前 4 字节为 "ttcf"，从偏移 12 处读取字体表偏移数组
fn extract_first_from_ttc(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 16 {
        return Err("TTC 文件过短".to_string());
    }
    if &data[0..4] != b"ttcf" {
        return Err("不是 TTC 格式".to_string());
    }

    // TTC header: 4 bytes tag + 2 bytes version + 2 bytes minor + 4 bytes num_fonts
    // + 4 bytes * num_fonts (offset table)
    let num_fonts = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
    if num_fonts == 0 {
        return Err("TTC 中无字体".to_string());
    }

    let offset = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    if offset >= data.len() {
        return Err("TTC 偏移越界".to_string());
    }

    // 第一个字体的长度：到文件末尾或到下一个字体偏移
    let size = if num_fonts > 1 {
        let next_offset = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as usize;
        next_offset.min(data.len()).saturating_sub(offset)
    } else {
        data.len().saturating_sub(offset)
    };

    Ok(data[offset..offset + size].to_vec())
}

/// 用 ab_glyph 渲染单个字符为灰度图
fn render_char(font_data: &[u8], ch: char, px_size: f32) -> Result<GrayImage, String> {
    let font = FontRef::try_from_slice(font_data).map_err(|e| format!("加载字体失败: {e}"))?;

    let scale = PxScale::from(px_size);
    // 在 64x64 画布中央渲染，outline_glyph 需要 PxScaleGlyph
    let glyph = font
        .glyph_id(ch)
        .with_scale_and_position(scale, point(4.0, px_size - 4.0));

    let mut img = GrayImage::new(TEMPLATE_SIZE, TEMPLATE_SIZE);

    if let Some(outline) = font.outline_glyph(glyph) {
        outline.draw(|x, y, coverage| {
            if x < TEMPLATE_SIZE && y < TEMPLATE_SIZE {
                let pixel = (coverage * 255.0) as u8;
                let existing = img.get_pixel(x, y).0[0];
                if pixel > existing {
                    img.put_pixel(x, y, Luma([pixel]));
                }
            }
        });
    }

    Ok(img)
}

/// MRZ 模板集合
pub struct MrzTemplates {
    /// 37 个模板（与 MRZ_CHARS 对应），TEMPLATE_SIZE × TEMPLATE_SIZE
    templates: Vec<GrayImage>,
}

impl MrzTemplates {
    /// 加载系统字体并生成 37 个 MRZ 字符模板
    pub fn load() -> Result<Self, String> {
        let font_data = load_font_bytes()?;
        let mut templates = Vec::with_capacity(37);

        for &ch in MRZ_CHARS {
            if ch == '<' {
                // 特殊处理 '<'：画一个简单的矩形缺口
                let tmpl = render_less_than_template();
                templates.push(tmpl);
            } else {
                let rendered = render_char(&font_data, ch, TEMPLATE_SIZE as f32)?;
                // resize 到统一尺寸 TEMPLATE_SIZE × TEMPLATE_SIZE
                let resized = image::imageops::resize(
                    &rendered,
                    TEMPLATE_SIZE,
                    TEMPLATE_SIZE,
                    FilterType::Lanczos3,
                );
                // 二值化
                let binary = image::GrayImage::from_fn(TEMPLATE_SIZE, TEMPLATE_SIZE, |x, y| {
                    let val = resized.get_pixel(x, y).0[0];
                    Luma([if val > 128 { 255 } else { 0 }])
                });
                templates.push(binary);
            }
        }

        Ok(Self { templates })
    }

    /// 对单个字符图像做 NCC 匹配，返回 (字符, 置信度)
    /// segment: 待识别的字符图像（二值化，白字黑底）
    pub fn match_char(&self, segment: &GrayImage) -> (char, f32) {
        let (sw, sh) = (segment.width(), segment.height());
        if sw == 0 || sh == 0 {
            return ('<', 0.0);
        }

        // 将 segment resize 到模板尺寸
        let seg_resized =
            image::imageops::resize(segment, TEMPLATE_SIZE, TEMPLATE_SIZE, FilterType::Nearest);

        let seg_f32: Vec<f32> = seg_resized.pixels().map(|p| p.0[0] as f32).collect();

        let seg_mean = seg_f32.iter().sum::<f32>() / seg_f32.len() as f32;
        let seg_std = (seg_f32.iter().map(|&v| (v - seg_mean).powi(2)).sum::<f32>()
            / seg_f32.len() as f32)
            .sqrt()
            + 1e-6;

        let mut best_idx = 0usize;
        let mut best_ncc = -1.0f32;

        for (i, tmpl) in self.templates.iter().enumerate() {
            let tmpl_f32: Vec<f32> = tmpl.pixels().map(|p| p.0[0] as f32).collect();

            let tmpl_mean = tmpl_f32.iter().sum::<f32>() / tmpl_f32.len() as f32;
            let tmpl_std = (tmpl_f32
                .iter()
                .map(|&v| (v - tmpl_mean).powi(2))
                .sum::<f32>()
                / tmpl_f32.len() as f32)
                .sqrt()
                + 1e-6;

            // 计算 NCC
            let mut cross = 0.0f32;
            for j in 0..seg_f32.len() {
                cross += (seg_f32[j] - seg_mean) * (tmpl_f32[j] - tmpl_mean);
            }
            let ncc = cross / (seg_std * tmpl_std * seg_f32.len() as f32);

            if ncc > best_ncc {
                best_ncc = ncc;
                best_idx = i;
            }
        }

        (MRZ_CHARS[best_idx], best_ncc)
    }
}

/// 生成 `<` 模板：一个矩形左边带小缺口
fn render_less_than_template() -> GrayImage {
    let s = TEMPLATE_SIZE;
    let mut img = GrayImage::new(s, s);

    // 画一个矩形的左边框和下边框（模拟 < 字符）
    let thickness = (s / 6).max(2);
    let margin = s / 8;
    let right = s - margin;

    for y in margin..s - margin {
        for x in margin..=margin + thickness {
            img.put_pixel(x, y, Luma([255]));
        }
    }
    for x in margin..right {
        for y in (s - margin - thickness)..(s - margin) {
            img.put_pixel(x, y, Luma([255]));
        }
    }
    // 左侧小缺口
    let notch_y = s / 2 - s / 12;
    for y in notch_y..notch_y + thickness {
        for x in margin..margin + thickness / 2 {
            img.put_pixel(x, y, Luma([0]));
        }
    }

    img
}

/// 将 MRZ 行图像按等宽切分为单个字符图像
///
/// 利用 MRZ 的等宽特性：一行 N 个字符，每个字符宽度 = line_width / N
pub fn segment_mrz_line(line: &GrayImage, num_chars: usize) -> Vec<GrayImage> {
    let (w, h) = (line.width(), line.height());
    if h == 0 || num_chars == 0 {
        return Vec::new();
    }

    let cell_w = w / num_chars as u32;
    if cell_w == 0 {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(num_chars);

    for i in 0..num_chars {
        let x_start = (i as u32 * cell_w).min(w.saturating_sub(1));
        let actual_w = cell_w.min(w.saturating_sub(x_start));

        let char_img = image::imageops::crop_imm(line, x_start, 0, actual_w, h).to_image();
        segments.push(char_img);
    }

    segments
}

/// 用模板匹配识别一行 MRZ 文本
///
/// 返回 (识别文本, 平均置信度)
pub fn recognize_mrz_line(
    templates: &MrzTemplates,
    line: &GrayImage,
    num_chars: usize,
) -> (String, f32) {
    let segments = segment_mrz_line(line, num_chars);
    let mut text = String::with_capacity(num_chars);
    let mut total_conf = 0.0f32;

    for seg in &segments {
        let (ch, conf) = templates.match_char(seg);
        text.push(ch);
        total_conf += conf;
    }

    let avg_conf = if segments.is_empty() {
        0.0
    } else {
        total_conf / segments.len() as f32
    };

    (text, avg_conf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_mrz_line_44_chars() {
        // 模拟一个 MRZ 行图像
        let line = GrayImage::new(880, 20);
        let segs = segment_mrz_line(&line, 44);
        assert_eq!(segs.len(), 44);
        assert_eq!(segs[0].width(), 20);
    }

    #[test]
    fn test_segment_mrz_line_30_chars() {
        let line = GrayImage::new(600, 15);
        let segs = segment_mrz_line(&line, 30);
        assert_eq!(segs.len(), 30);
        assert_eq!(segs[0].width(), 20);
    }

    #[test]
    fn test_render_less_than_template() {
        let tmpl = render_less_than_template();
        assert_eq!(tmpl.width(), TEMPLATE_SIZE);
        assert_eq!(tmpl.height(), TEMPLATE_SIZE);
        // 至少有一些白色像素
        let mut white_count = 0u32;
        for y in 0..TEMPLATE_SIZE {
            for x in 0..TEMPLATE_SIZE {
                if tmpl.get_pixel(x, y).0[0] > 0 {
                    white_count += 1;
                }
            }
        }
        assert!(white_count > 10, "模板应有至少 10 个白色像素");
    }

    #[test]
    fn test_templates_load() {
        let templates = MrzTemplates::load();
        // 在 CI 或无字体环境可能失败，允许跳过
        if let Ok(tpl) = templates {
            assert_eq!(tpl.templates.len(), 37);
        }
    }
}
