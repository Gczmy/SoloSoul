//! 图像预处理
//!
//! 为 MRZ 识别提供：灰度化、去噪、Sauvola 自适应二值化、尺寸标准化。
//! 所有处理基于 `image` 和 `imageproc` crate。

use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgb, RgbImage};

/// PP-OCRv4 rec 模型输入高度（固定）
pub const REC_INPUT_HEIGHT: u32 = 48;

/// PP-OCRv4 rec 模型归一化均值
pub const REC_MEAN: [f32; 3] = [0.485, 0.456, 0.406];

/// PP-OCRv4 rec 模型归一化标准差
pub const REC_STD: [f32; 3] = [0.229, 0.224, 0.225];

/// PP-OCRv4 det 模型输入尺寸
pub const DET_INPUT_SIZE: u32 = 640;

/// PP-OCRv4 det 模型归一化均值
pub const DET_MEAN: [f32; 3] = [0.5, 0.5, 0.5];

/// PP-OCRv4 det 模型归一化标准差
pub const DET_STD: [f32; 3] = [0.5, 0.5, 0.5];

/// PP-OCR cls 模型输入高度
pub const CLS_INPUT_HEIGHT: u32 = 48;

/// PP-OCR cls 模型输入宽度
pub const CLS_INPUT_WIDTH: u32 = 192;

/// PP-OCR cls 模型归一化均值
pub const CLS_MEAN: [f32; 3] = [0.5, 0.5, 0.5];

/// PP-OCR cls 模型归一化标准差
pub const CLS_STD: [f32; 3] = [0.5, 0.5, 0.5];

/// 图像预处理管道：输入任意图像 → 输出灰度二值化图
///
/// 步骤：
/// 1. 转为灰度图
/// 2. 高斯模糊去噪
/// 3. Sauvola 局部自适应二值化（对反光/阴影鲁棒）
/// 4. 尺寸归一化（长边 ≤ 2048，保持比例）
pub fn preprocess_for_mrz(img: &DynamicImage) -> GrayImage {
    // 步骤 1：尺寸归一化（长边限制 2048，减少后续计算量）
    let (width, height) = (img.width(), img.height());
    let max_dim = width.max(height);
    let scale = if max_dim > 2048 {
        2048.0 / max_dim as f32
    } else {
        1.0
    };

    let resized = if scale < 1.0 {
        img.resize(
            (width as f32 * scale) as u32,
            (height as f32 * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img.clone()
    };

    // 步骤 2：转为灰度图
    let gray = resized.to_luma8();

    // 步骤 3：轻度高斯模糊去噪（sigma = 1.0）
    let blurred = imageproc::filter::gaussian_blur_f32(&gray, 1.0);

    // 步骤 4：Sauvola 局部自适应二值化
    // 窗口大小 25x25，k=0.2，对反光/渐变背景鲁棒
    sauvola_binarize(&blurred, 25, 0.2)
}

/// Sauvola 局部自适应二值化
///
/// 比 Otsu 更适合 MRZ 场景（反光证件、阴影、不均匀光照）。
/// 参考：Sauvola, J. et al. "Adaptive document image binarization"
fn sauvola_binarize(img: &GrayImage, window_size: u32, k: f32) -> GrayImage {
    let (width, height) = img.dimensions();
    let half_window = (window_size / 2) as i32;
    let mut result = GrayImage::new(width, height);

    // 预计算积分图（均值和方差）加速
    // 注意：integral_squared_image 使用 u64 避免溢出
    // 2048x2048 图像的平方积分最大值 ≈ 2.7e11，远超 u32 上限
    let integral = imageproc::integral_image::integral_image(img);
    let integral_sq: ImageBuffer<Luma<u64>, Vec<u64>> =
        imageproc::integral_image::integral_squared_image(img);

    let _window_area = (window_size * window_size) as f32;
    let r = 128.0; // 标准动态范围的一半

    for y in 0..height {
        for x in 0..width {
            // 计算窗口边界
            let x0 = x.saturating_sub(half_window as u32);
            let y0 = y.saturating_sub(half_window as u32);
            let x1 = (x + half_window as u32 + 1).min(width);
            let y1 = (y + half_window as u32 + 1).min(height);

            // 从积分图快速计算窗口内均值和方差
            let sum = integral_sum(&integral, x0, y0, x1, y1);
            let sum_sq = integral_sum_u64(&integral_sq, x0, y0, x1, y1);
            let area = ((x1 - x0) * (y1 - y0)) as f32;

            let mean = sum / area;
            let variance = (sum_sq / area) - (mean * mean);
            let std_dev = variance.sqrt().max(1.0);

            // Sauvola 阈值：T = mean * (1 + k * (std_dev / r - 1))
            let threshold = mean * (1.0 + k * (std_dev / r - 1.0));

            let pixel = img.get_pixel(x, y)[0] as f32;
            let bin_val = if pixel > threshold { 255 } else { 0 };
            result.put_pixel(x, y, Luma([bin_val]));
        }
    }

    result
}

/// 从积分图计算矩形区域和（u32 版本）
fn integral_sum(
    integral: &ImageBuffer<Luma<u32>, Vec<u32>>,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
) -> f32 {
    let get = |x: u32, y: u32| integral.get_pixel(x, y)[0] as f32;

    get(x1, y1) - get(x0, y1) - get(x1, y0) + get(x0, y0)
}

/// 从积分图计算矩形区域和（u64 版本，用于平方积分图）
fn integral_sum_u64(
    integral: &ImageBuffer<Luma<u64>, Vec<u64>>,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
) -> f32 {
    let get = |x: u32, y: u32| integral.get_pixel(x, y)[0] as f64;

    (get(x1, y1) - get(x0, y1) - get(x1, y0) + get(x0, y0)) as f32
}

/// 为 rec 模型准备输入：将单行文字图 resize 到固定高度 48，宽度按比例或 padding
///
/// 输出：RGB 图像（NCHW 布局前的前置处理）
pub fn prepare_rec_input(gray_line: &GrayImage) -> RgbImage {
    let (w, h) = gray_line.dimensions();
    let target_height = REC_INPUT_HEIGHT;

    // 保持比例 resize
    let target_width = ((w as f32 / h as f32) * target_height as f32) as u32;
    let target_width = target_width.max(10); // 最小宽度 10

    let resized = image::imageops::resize(
        gray_line,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    // 转为 RGB（模型期望 3 通道输入）
    let mut rgb = RgbImage::new(target_width, target_height);
    for y in 0..target_height {
        for x in 0..target_width {
            let val = resized.get_pixel(x, y)[0];
            rgb.put_pixel(x, y, Rgb([val, val, val]));
        }
    }

    rgb
}

/// 水平投影：统计每行的黑色像素数（用于行切分）
pub fn horizontal_projection(binary: &GrayImage) -> Vec<u32> {
    let (width, height) = binary.dimensions();
    let mut projection = vec![0u32; height as usize];

    for y in 0..height {
        let mut count = 0;
        for x in 0..width {
            if binary.get_pixel(x, y)[0] < 128 {
                count += 1;
            }
        }
        projection[y as usize] = count;
    }

    projection
}

/// 垂直投影：统计每列的黑色像素数（用于字符间距分析）
pub fn vertical_projection(binary: &GrayImage, region: &imageproc::rect::Rect) -> Vec<u32> {
    let mut projection = vec![0u32; region.width() as usize];

    for x in 0..region.width() {
        let mut count = 0;
        for y in 0..region.height() {
            let px = region.left() + x as i32;
            let py = region.top() + y as i32;
            if px >= 0
                && py >= 0
                && (px as u32) < binary.width()
                && (py as u32) < binary.height()
                && binary.get_pixel(px as u32, py as u32)[0] < 128
            {
                count += 1;
            }
        }
        projection[x as usize] = count;
    }

    projection
}

/// 为 det 模型准备输入：resize 到固定尺寸，归一化
///
/// PP-OCRv4 det 模型期望输入：
/// - 尺寸：640×640（正方形，简化处理）
/// - 布局：NCHW
/// - 归一化：(pixel/255 - mean) / std，mean=0.5, std=0.5
///
/// 返回：(NCHW 数据 Vec<f32>, 原始图像缩放比例, 目标尺寸)
pub fn prepare_det_input(img: &DynamicImage) -> (Vec<f32>, f32, (u32, u32)) {
    let (orig_w, orig_h) = (img.width(), img.height());

    // resize 到 640×640（保持简单，正方形填充）
    let target_size = DET_INPUT_SIZE;
    let resized = img.resize_exact(target_size, target_size, image::imageops::FilterType::Triangle);

    // 计算缩放比例（用于后处理时将 bbox 坐标映射回原始图像）
    let scale_x = orig_w as f32 / target_size as f32;
    let scale_y = orig_h as f32 / target_size as f32;
    let scale = (scale_x + scale_y) / 2.0; // 取平均简化

    // 构造 NCHW 数据
    let mut input_data = Vec::with_capacity((3 * target_size * target_size) as usize);
    let rgb = resized.to_rgb8();

    for c in 0..3 {
        for y in 0..target_size {
            for x in 0..target_size {
                let pixel = rgb.get_pixel(x, y)[c] as f32;
                let normalized = (pixel / 255.0 - DET_MEAN[c]) / DET_STD[c];
                input_data.push(normalized);
            }
        }
    }

    (input_data, scale, (target_size, target_size))
}

/// 为 cls 模型准备输入：resize 到固定尺寸 48×192，归一化
///
/// PP-OCR cls 模型期望输入：
/// - 尺寸：48×192（H=48, W=192）
/// - 布局：NCHW
/// - 归一化：(pixel/255 - mean) / std，mean=0.5, std=0.5
pub fn prepare_cls_input(img: &DynamicImage) -> Vec<f32> {
    let target_h = CLS_INPUT_HEIGHT;
    let target_w = CLS_INPUT_WIDTH;

    let resized = img.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();

    let mut input_data = Vec::with_capacity((3 * target_h * target_w) as usize);

    for c in 0..3 {
        for y in 0..target_h {
            for x in 0..target_w {
                let pixel = rgb.get_pixel(x, y)[c] as f32;
                let normalized = (pixel / 255.0 - CLS_MEAN[c]) / CLS_STD[c];
                input_data.push(normalized);
            }
        }
    }

    input_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sauvola_binarize() {
        // 创建一个简单的渐变灰度图
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let val = ((x + y) / 2) as u8;
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let binary = sauvola_binarize(&img, 25, 0.2);
        // 验证输出为纯黑白
        for y in 0..100 {
            for x in 0..100 {
                let val = binary.get_pixel(x, y)[0];
                assert!(val == 0 || val == 255, "Pixel should be binary");
            }
        }
    }

    #[test]
    fn test_horizontal_projection() {
        // 创建白色背景图像
        let mut img = GrayImage::from_pixel(50, 10, Luma([255]));
        // 第 5 行全黑
        for x in 0..50 {
            img.put_pixel(x, 5, Luma([0]));
        }

        let proj = horizontal_projection(&img);
        assert_eq!(proj.len(), 10);
        assert_eq!(proj[5], 50);
        assert_eq!(proj[0], 0);
    }
}
