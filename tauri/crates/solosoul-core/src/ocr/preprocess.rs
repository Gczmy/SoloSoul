//! 图像预处理：检测与识别模型输入构造。

use image::{imageops::FilterType, Rgb, RgbImage};
use ndarray::Array4;

/// 预处理参数常量。
const DET_TARGET_SIZE: u32 = 736;
const REC_HEIGHT: u32 = 48;
const REC_WIDTH: u32 = 320;

/// 检测模型预处理结果。
pub struct DetInput {
    /// NCHW float32 tensor，已归一化。
    pub tensor: Array4<f32>,
    /// 图像缩放比例（相对长边）。
    pub scale: f32,
    /// 原始图像尺寸 (height, width)。
    pub original_size: (u32, u32),
}

/// 对整张图片进行检测模型预处理。
pub fn preprocess_for_detection(img: &RgbImage) -> DetInput {
    let (orig_h, orig_w) = (img.height(), img.width());
    let max_side = orig_h.max(orig_w);
    let scale = DET_TARGET_SIZE as f32 / max_side as f32;
    let new_h = (orig_h as f32 * scale) as u32;
    let new_w = (orig_w as f32 * scale) as u32;

    let resized = image::imageops::resize(img, new_w, new_h, FilterType::Triangle);
    let mut pad = RgbImage::from_pixel(DET_TARGET_SIZE, DET_TARGET_SIZE, Rgb([127, 127, 127]));
    image::imageops::replace(&mut pad, &resized, 0, 0);

    // Normalize: mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225], scale=1/255
    let mut tensor =
        Array4::<f32>::zeros([1, 3, DET_TARGET_SIZE as usize, DET_TARGET_SIZE as usize]);
    for y in 0..DET_TARGET_SIZE {
        for x in 0..DET_TARGET_SIZE {
            let Rgb([r, g, b]) = pad.get_pixel(x, y);
            let r = *r as f32 / 255.0;
            let g = *g as f32 / 255.0;
            let b = *b as f32 / 255.0;
            // PP-OCRv6 检测模型期望 BGR 输入（image crate 读取为 RGB，需交换通道）。
            tensor[[0, 0, y as usize, x as usize]] = (b - 0.406) / 0.225;
            tensor[[0, 1, y as usize, x as usize]] = (g - 0.456) / 0.224;
            tensor[[0, 2, y as usize, x as usize]] = (r - 0.485) / 0.229;
        }
    }

    DetInput {
        tensor,
        scale,
        original_size: (orig_h, orig_w),
    }
}

/// 识别模型预处理结果。
pub struct RecInput {
    /// NCHW float32 tensor，值域约 [0, 1]。
    pub tensor: Array4<f32>,
}

/// 对裁剪后的文字块进行识别模型预处理。
pub fn preprocess_for_recognition(crop: &RgbImage) -> RecInput {
    let (h, w) = (crop.height(), crop.width());
    let scale_h = REC_HEIGHT as f32 / h as f32;
    let scale_w = REC_WIDTH as f32 / w.max(1) as f32;
    let scale = scale_h.min(scale_w);

    let new_h = (h as f32 * scale) as u32;
    let new_w = (w as f32 * scale) as u32;
    let resized = image::imageops::resize(crop, new_w.max(1), new_h.max(1), FilterType::Triangle);

    let mut pad = RgbImage::from_pixel(REC_WIDTH, REC_HEIGHT, Rgb([127, 127, 127]));
    image::imageops::replace(&mut pad, &resized, 0, 0);

    let mut tensor = Array4::<f32>::zeros([1, 3, REC_HEIGHT as usize, REC_WIDTH as usize]);
    for y in 0..REC_HEIGHT {
        for x in 0..REC_WIDTH {
            let Rgb([r, g, b]) = pad.get_pixel(x, y);
            // PP-OCRv6 识别模型期望 BGR 输入（image crate 读取为 RGB，需交换通道）。
            tensor[[0, 0, y as usize, x as usize]] = *b as f32 / 255.0;
            tensor[[0, 1, y as usize, x as usize]] = *g as f32 / 255.0;
            tensor[[0, 2, y as usize, x as usize]] = *r as f32 / 255.0;
        }
    }

    RecInput { tensor }
}

/// 从文件路径读取 RGB 图像。
pub fn load_rgb_image(path: &std::path::Path) -> Result<RgbImage, String> {
    let img = image::open(path).map_err(|e| format!("Open image {}: {e}", path.display()))?;
    Ok(img.to_rgb8())
}

/// 使用给定角点从原图中裁剪出文字块。
///
/// 当前实现基于所有点的外接矩形做轴对齐裁剪。OCR 检测阶段返回的框
/// 已经是 AABB，因此直接裁剪即可避免透视变换的数值误差。
pub fn perspective_crop(img: &RgbImage, points: &[(f32, f32); 4]) -> RgbImage {
    let min_x = points
        .iter()
        .map(|p| p.0)
        .fold(f32::INFINITY, f32::min)
        .max(0.0);
    let min_y = points
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .max(0.0);
    let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

    let x = min_x as u32;
    let y = min_y as u32;
    let w = ((max_x - min_x) as u32 + 1).min(img.width() - x).max(1);
    let h = ((max_y - min_y) as u32 + 1).min(img.height() - y).max(1);

    image::imageops::crop_imm(img, x, y, w, h).to_image()
}

/// 对 MRZ 裁剪区域做增强：灰度、放大 2x。
pub fn enhance_mrz_crop(img: &RgbImage) -> RgbImage {
    let gray = image::imageops::grayscale(img);
    let scaled = image::imageops::resize(
        &gray,
        gray.width() * 2,
        gray.height() * 2,
        FilterType::Triangle,
    );
    RgbImage::from_fn(scaled.width(), scaled.height(), |x, y| {
        let p = scaled.get_pixel(x, y).0[0];
        Rgb([p, p, p])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_for_detection_shape() {
        let img = RgbImage::new(200, 400);
        let input = preprocess_for_detection(&img);
        assert_eq!(input.tensor.shape(), &[1, 3, 736, 736]);
        assert!(input.scale > 0.0);
    }

    #[test]
    fn test_preprocess_for_recognition_shape() {
        let img = RgbImage::new(100, 30);
        let input = preprocess_for_recognition(&img);
        assert_eq!(input.tensor.shape(), &[1, 3, 48, 320]);
    }

    #[test]
    fn test_perspective_crop_identity() {
        let img = RgbImage::from_pixel(10, 10, Rgb([255, 0, 0]));
        let points = [(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (0.0, 9.0)];
        let crop = perspective_crop(&img, &points);
        assert_eq!(crop.width(), 10);
        assert_eq!(crop.height(), 10);
        assert_eq!(crop.get_pixel(5, 5).0, [255, 0, 0]);
    }
}
