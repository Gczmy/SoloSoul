//! MRZ（机读区）检测、识别与解析。

use super::types::MrzResult;
use image::{imageops::FilterType, Luma, Rgb, RgbImage};

/// 在图像中检测 MRZ 区域。
///
/// 改进算法（v2）：
/// 1. Otsu 阈值二值化（比固定阈值更健壮）
/// 2. 水平形态学膨胀合并断裂笔画
/// 3. 支持 TD-1（3 行）和 TD-3（2 行）
/// 4. 宽度验证过滤非 MRZ 文本
/// 5. 多策略回退（不同搜索区域/阈值）
pub fn detect_mrz_region(image: &RgbImage) -> Option<[(f32, f32); 4]> {
    let (w, h) = (image.width(), image.height());
    if h < 100 || w < 300 {
        return None;
    }

    // 定义搜索策略：(底部区域占比, 阈值偏移)
    // 策略 1: 底部 55%（标准护照/身份证 MRZ 位置）
    // 策略 2: 底部 65%（位置略偏上）
    // 策略 3: 整图搜索（兜底）
    let strategies: [(f32, i32); 5] = [
        (0.55, 0),   // 底部 55%, Otsu
        (0.55, -15), // 底部 55%, Otsu-15
        (0.65, 0),   // 底部 65%, Otsu
        (1.0, 0),    // 整图, Otsu
        (1.0, -15),  // 整图, Otsu-15
    ];

    for &(bottom_ratio, thresh_offset) in &strategies {
        if let Some(region) = try_detect_mrz(image, bottom_ratio, thresh_offset) {
            return Some(region);
        }
    }

    None
}

/// 使用给定参数尝试检测 MRZ 区域。
fn try_detect_mrz(
    image: &RgbImage,
    bottom_ratio: f32,
    thresh_offset: i32,
) -> Option<[(f32, f32); 4]> {
    let (w, h) = (image.width(), image.height());

    // 计算搜索区域
    let search_h = (h as f32 * bottom_ratio) as u32;
    let y_start = h.saturating_sub(search_h);
    if y_start >= h || search_h < 40 {
        return None;
    }
    let cropped = image::imageops::crop_imm(image, 0, y_start, w, search_h).to_image();

    // 灰度 + Otsu 二值化
    let gray = to_grayscale(&cropped);
    let binary = otsu_binarize(&gray, thresh_offset);

    // 水平膨胀：合并断裂的字符笔画
    let dilated = dilate_horizontal(&binary, 5);

    // 计算水平投影（每行白色像素数）
    let projection: Vec<u32> = (0..dilated.height())
        .map(|y| {
            (0..dilated.width())
                .filter(|&x| dilated.get_pixel(x, y).0[0] > 0)
                .count() as u32
        })
        .collect();

    // 宽度验证：检查每行的文本水平跨度是否足够宽
    // 先找所有候选行
    let max_val = *projection.iter().max()?;
    if max_val == 0 {
        return None;
    }

    // 找到所有行带
    let region_threshold = max_val / 4;
    let mut bands: Vec<(usize, usize)> = Vec::new();
    let mut in_region = false;
    let mut start = 0;
    for (i, &val) in projection.iter().enumerate() {
        if val >= region_threshold && !in_region {
            in_region = true;
            start = i;
        } else if val < region_threshold && in_region {
            in_region = false;
            bands.push((start, i));
        }
    }
    if in_region {
        bands.push((start, projection.len()));
    }

    // 过滤掉宽度不足的行带（行带的宽度必须 > 图像宽度的 50%）
    let min_width = (w as f32 * 0.50) as u32;
    let wide_bands: Vec<(usize, usize)> = bands
        .into_iter()
        .filter(|&(s, e)| {
            // 检查该行带内是否有足够宽的水平跨度
            let mid_y = (s + e) / 2;
            if mid_y >= projection.len() {
                return false;
            }
            // 检查该行中白色像素的分布：找最左和最右的白色像素
            let mut leftmost = dilated.width();
            let mut rightmost = 0u32;
            for x in 0..dilated.width() {
                if dilated.get_pixel(x, mid_y as u32).0[0] > 0 {
                    if x < leftmost {
                        leftmost = x;
                    }
                    if x > rightmost {
                        rightmost = x;
                    }
                }
            }
            let span = rightmost.saturating_sub(leftmost);
            span >= min_width
        })
        .collect();

    // 尝试找 2 行或 3 行文本
    let centers = find_text_lines_from_bands(&wide_bands, 2, &projection)
        .or_else(|| find_text_lines_from_bands(&wide_bands, 3, &projection))?;

    // 映射回原图坐标
    let pad_y = 14.0;
    // 计算 MRZ 区域边界：覆盖所有行的宽度 + padding
    // 左边界使用行内最左白色像素（兼顾全宽）
    let region_left: f32 = 0.0; // 使用全宽，确保包含所有 MRZ 字符
    let region_right = (w - 1) as f32;

    let first_center = centers[0];
    let last_center = centers[centers.len() - 1];
    let region_top = (y_start as f32 + first_center - pad_y).max(0.0);
    let region_bottom = (y_start as f32 + last_center + pad_y).min((h - 1) as f32);

    Some([
        (region_left, region_top),
        (region_right, region_top),
        (region_right, region_bottom),
        (region_left, region_bottom),
    ])
}

pub fn to_grayscale(img: &RgbImage) -> image::GrayImage {
    image::imageops::grayscale(img)
}

/// Otsu 阈值二值化（自动寻找最佳阈值）。
/// `offset` 允许微调阈值（正值 → 更亮，保留更多像素）。
pub fn otsu_binarize(img: &image::GrayImage, offset: i32) -> image::GrayImage {
    let (w, h) = (img.width(), img.height());
    let total = (w * h) as usize;
    if total == 0 {
        return image::GrayImage::new(w.max(1), h.max(1));
    }

    // 计算直方图
    let mut hist = [0u64; 256];
    for y in 0..h {
        for x in 0..w {
            hist[img.get_pixel(x, y).0[0] as usize] += 1;
        }
    }

    // Otsu: 最大化类间方差
    let mut best_t = 128u8;
    let mut max_var = 0f64;
    let total_f = total as f64;
    let mut sum_b = 0f64;
    let mut w_b = 0f64;
    let sum: f64 = hist.iter().enumerate().map(|(i, &c)| (i as f64) * (c as f64)).sum();

    for t in 0..256 {
        w_b += hist[t] as f64;
        if w_b == 0.0 || w_b >= total_f {
            continue;
        }
        let w_f = total_f - w_b;
        sum_b += (t as f64) * (hist[t] as f64);
        let mean_b = sum_b / w_b;
        let mean_f = (sum - sum_b) / w_f;
        let var = w_b * w_f * (mean_b - mean_f).powi(2);
        if var > max_var {
            max_var = var;
            best_t = t as u8;
        }
    }

    // 应用偏移
    let threshold = (best_t as i32 + offset).clamp(0, 255) as u8;

    // 文本行通常为暗底亮字或亮底暗字；我们使文字像素为 255（白色）
    // 计算均值判断背景明暗：均值 < 128 说明背景偏暗，文字亮 → 取 > threshold
    let mean_val = (sum / total_f) as u8;
    let text_is_light = mean_val < 128;

    image::GrayImage::from_fn(w, h, |x, y| {
        let val = img.get_pixel(x, y).0[0];
        let is_text = if text_is_light {
            val > threshold
        } else {
            val < threshold
        };
        Luma([if is_text { 255 } else { 0 }])
    })
}

/// 水平形态学膨胀：将相邻的白色像素合并。
/// `kernel_width` 越大，文本行之间的间隔越小（典型值 3-7）。
fn dilate_horizontal(img: &image::GrayImage, kernel_width: u32) -> image::GrayImage {
    let (w, h) = (img.width(), img.height());
    let half = (kernel_width / 2) as i32;
    image::GrayImage::from_fn(w, h, |x, y| {
        let mut max_val = 0u8;
        for dx in -half..=half {
            let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
            let v = img.get_pixel(nx, y).0[0];
            if v > max_val {
                max_val = v;
            }
        }
        Luma([max_val])
    })
}

/// 从已过滤的宽行带中找最好的 N 行。
fn find_text_lines_from_bands(
    bands: &[(usize, usize)],
    target_count: usize,
    projection: &[u32],
) -> Option<Vec<f32>> {
    if bands.len() < target_count {
        return None;
    }

    let mut best_score = 0u32;
    let mut best_centers: Option<Vec<f32>> = None;

    for i in 0..=bands.len().saturating_sub(target_count) {
        let centers: Vec<f32> = (i..i + target_count)
            .map(|j| (bands[j].0 + bands[j].1) as f32 / 2.0)
            .collect();

        // 检查间距是否合理
        let mut valid = true;
        for j in 1..target_count {
            let gap = centers[j] - centers[j - 1];
            if !(4.0..=70.0).contains(&gap) {
                valid = false;
                break;
            }
        }

        // 如果是 3 行，额外检查间距是否均匀（比例 < 1.5x）
        if valid && target_count == 3 {
            let gap1 = centers[1] - centers[0];
            let gap2 = centers[2] - centers[1];
            let ratio = (gap1 / gap2).max(gap2 / gap1);
            if ratio > 1.8 {
                valid = false;
            }
        }

        if valid {
            let score: u32 = (i..i + target_count)
                .map(|j| projection[bands[j].0..bands[j].1].iter().sum::<u32>())
                .sum();
            if score > best_score {
                best_score = score;
                best_centers = Some(centers);
            }
        }
    }

    best_centers
}

/// 对 MRZ 区域裁剪图做增强：去倾斜 + CLAHE 对比度增强 + 放大。
pub fn enhance_mrz_crop(img: &RgbImage) -> RgbImage {
    // 1. 灰度
    let gray = image::imageops::grayscale(img);

    // 2. 去倾斜（检测文本旋转角并校正）
    let deskewed = deskew(&gray);

    // 3. CLAHE 对比度增强
    let enhanced = apply_clahe(&deskewed, 3.0, 4, 12);

    // 4. 放大 2x（双三次插值更平滑）
    let scaled = image::imageops::resize(
        &enhanced,
        enhanced.width() * 2,
        enhanced.height() * 2,
        FilterType::CatmullRom,
    );

    RgbImage::from_fn(scaled.width(), scaled.height(), |x, y| {
        let p = scaled.get_pixel(x, y).0[0];
        Rgb([p, p, p])
    })
}

// ─── 去倾斜（Deskew）────────────────────────────────────────────

/// 检测文本倾斜角度并校正。
/// 通过水平投影方差最大化找到最佳旋转角（范围 -8° ~ +8°）。
fn deskew(img: &image::GrayImage) -> image::GrayImage {
    let angle = find_skew_angle(img);
    if angle.abs() < 0.3 {
        // 倾斜很小时返回原图避免质量损失
        return image::GrayImage::from_fn(img.width(), img.height(), |x, y| *img.get_pixel(x, y));
    }
    rotate_about_center(img, angle)
}

/// 在 -8° 到 +8° 范围内搜索最优倾斜角（步长 0.5°）。
fn find_skew_angle(img: &image::GrayImage) -> f32 {
    let mut best_angle = 0.0f32;
    let mut best_variance = 0.0f32;

    // 缩小图像加速角度搜索
    let small = image::imageops::resize(img, img.width().min(320), 0, FilterType::Triangle);

    let mut angle = -8.0;
    while angle <= 8.0 {
        let rotated = rotate_about_center(&small, angle);
        let projection: Vec<f32> = (0..rotated.height())
            .map(|y| {
                (0..rotated.width())
                    .map(|x| 255.0 - rotated.get_pixel(x, y).0[0] as f32)
                    .sum()
            })
            .collect();

        let mean = projection.iter().sum::<f32>() / projection.len().max(1) as f32;
        let variance = projection
            .iter()
            .map(|&p| (p - mean).powi(2))
            .sum::<f32>()
            / projection.len().max(1) as f32;

        if variance > best_variance {
            best_variance = variance;
            best_angle = angle;
        }

        angle += 0.5;
    }

    best_angle
}

/// 绕图像中心旋转指定角度（双线性插值）。
fn rotate_about_center(img: &image::GrayImage, angle_deg: f32) -> image::GrayImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let cx = w / 2.0;
    let cy = h / 2.0;
    let rad = angle_deg.to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    // 计算新边界
    let corners = [(-cx, -cy), (w - cx, -cy), (w - cx, h - cy), (-cx, h - cy)];
    let new_corners: Vec<(f32, f32)> = corners
        .iter()
        .map(|&(x, y)| (x * cos_a - y * sin_a, x * sin_a + y * cos_a))
        .collect();

    let min_x = new_corners
        .iter()
        .map(|&(x, _)| x)
        .fold(f32::INFINITY, f32::min);
    let max_x = new_corners
        .iter()
        .map(|&(x, _)| x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = new_corners
        .iter()
        .map(|&(_, y)| y)
        .fold(f32::INFINITY, f32::min);
    let max_y = new_corners
        .iter()
        .map(|&(_, y)| y)
        .fold(f32::NEG_INFINITY, f32::max);

    let new_w = (max_x - min_x).ceil() as u32;
    let new_h = (max_y - min_y).ceil() as u32;
    let new_cx = -min_x;
    let new_cy = -min_y;

    let mut output = image::GrayImage::new(new_w.max(1), new_h.max(1));

    for out_y in 0..output.height() {
        for out_x in 0..output.width() {
            let dx = out_x as f32 - new_cx;
            let dy = out_y as f32 - new_cy;
            let src_x = dx * cos_a + dy * sin_a + cx;
            let src_y = -dx * sin_a + dy * cos_a + cy;

            if src_x >= 0.0 && src_x < w - 1.0 && src_y >= 0.0 && src_y < h - 1.0 {
                let x0 = (src_x.floor() as u32).min(img.width() - 1);
                let y0 = (src_y.floor() as u32).min(img.height() - 1);
                let x1 = (x0 + 1).min(img.width() - 1);
                let y1 = (y0 + 1).min(img.height() - 1);
                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                let p00 = img.get_pixel(x0, y0).0[0] as f32;
                let p10 = img.get_pixel(x1, y0).0[0] as f32;
                let p01 = img.get_pixel(x0, y1).0[0] as f32;
                let p11 = img.get_pixel(x1, y1).0[0] as f32;

                let val = (1.0 - fx) * (1.0 - fy) * p00
                    + fx * (1.0 - fy) * p10
                    + (1.0 - fx) * fy * p01
                    + fx * fy * p11;

                output.put_pixel(out_x, out_y, Luma([(val.round().clamp(0.0, 255.0)) as u8]));
            }
        }
    }

    output
}

// ─── CLAHE 对比度增强 ──────────────────────────────────────────

/// 对灰度图应用 CLAHE（Contrast Limited Adaptive Histogram Equalization）。
///
/// - `clip_limit`：对比度裁剪限值（越大对比度越强，典型值 2.0–4.0）
/// - `grid_rows x grid_cols`：分块网格数（越大局部细节越丰富）
pub fn apply_clahe(
    img: &image::GrayImage,
    clip_limit: f32,
    grid_rows: u32,
    grid_cols: u32,
) -> image::GrayImage {
    let (w, h) = (img.width(), img.height());
    let tile_w = (w / grid_cols).max(1);
    let tile_h = (h / grid_rows).max(1);

    // Step 1: 为每个 tile 构建直方图（含裁剪与重分配）
    let tile_cdfs: Vec<[f32; 256]> = (0..grid_rows)
        .flat_map(|ty| {
            (0..grid_cols).map(move |tx| {
                let x_start = tx * tile_w;
                let y_start = ty * tile_h;
                let x_end = if tx == grid_cols - 1 { w } else { (tx + 1) * tile_w };
                let y_end = if ty == grid_rows - 1 { h } else { (ty + 1) * tile_h };
                let tile_pixels = (x_end - x_start) * (y_end - y_start);

                // 计算直方图
                let mut hist = [0u32; 256];
                for y in y_start..y_end {
                    for x in x_start..x_end {
                        let val = img.get_pixel(x, y).0[0] as usize;
                        hist[val] += 1;
                    }
                }

                // 裁剪
                let clip_value =
                    ((clip_limit * tile_pixels as f32) / 256.0).ceil() as u32;
                let mut total_clipped = 0u32;
                for v in hist.iter_mut() {
                    if *v > clip_value {
                        total_clipped += *v - clip_value;
                        *v = clip_value;
                    }
                }

                // 均匀重分配
                let redist_each = total_clipped / 256;
                let remainder = (total_clipped % 256) as usize;
                for (i, v) in hist.iter_mut().enumerate() {
                    *v += redist_each;
                    if i < remainder {
                        *v += 1;
                    }
                }

                // 计算 CDF 并归一化
                let mut cdf = [0f32; 256];
                let mut cum = 0u32;
                for (i, &v) in hist.iter().enumerate() {
                    cum += v;
                    cdf[i] = cum as f32 / tile_pixels.max(1) as f32;
                }

                cdf
            })
        })
        .collect();

    // Step 2: 双线性插值重建每个像素
    let mut output = image::GrayImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let val = img.get_pixel(x, y).0[0] as usize;

            // tile 坐标（以 tile 中心为参考）
            let tx_f = x as f32 / tile_w as f32 - 0.5;
            let ty_f = y as f32 / tile_h as f32 - 0.5;

            let get_cdf = |tx: i32, ty: i32| -> f32 {
                if tx >= 0 && tx < grid_cols as i32 && ty >= 0 && ty < grid_rows as i32 {
                    let idx = (ty * grid_cols as i32 + tx) as usize;
                    tile_cdfs[idx][val] * 255.0
                } else {
                    // 超出边界时用恒等映射
                    val as f32
                }
            };

            let tx1 = tx_f.floor() as i32;
            let ty1 = ty_f.floor() as i32;
            let tx2 = tx1 + 1;
            let ty2 = ty1 + 1;
            let fx = (tx_f - tx1 as f32).clamp(0.0, 1.0);
            let fy = (ty_f - ty1 as f32).clamp(0.0, 1.0);

            let v11 = get_cdf(tx1, ty1);
            let v21 = get_cdf(tx2, ty1);
            let v12 = get_cdf(tx1, ty2);
            let v22 = get_cdf(tx2, ty2);

            let v = (1.0 - fx) * (1.0 - fy) * v11
                + fx * (1.0 - fy) * v21
                + (1.0 - fx) * fy * v12
                + fx * fy * v22;

            let new_val = v.round().clamp(0.0, 255.0) as u8;
            output.put_pixel(x, y, Luma([new_val]));
        }
    }

    output
}

/// 清理 OCR 文本行：转大写 + 非 MRZ 字符替换为 `<`。
///
/// OCR 引擎常返回小写字母和标点符号，这些会被映射为 MRZ 可接受的字符：
/// - 字母转大写
/// - 数字保留
/// - 非字母数字字符（`.` `-` `/` 等）替换为 `<`（MRZ 填充符）
/// - 中文等非 ASCII 字符替换为 `<`（避免后续 byte 切片 panic）
fn sanitize_mrz_line(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' => c,
            'a'..='z' => c.to_ascii_uppercase(),
            '0'..='9' => c,
            '<' | ' ' => c,
            _ => '<', // 标点符号/中文等 → MRZ 填充符
        })
        .collect()
}

/// 解析 MRZ 文本行。
/// 支持 TD-1（3 行 × 30 字符）和 TD-3（护照，2 行 × 44 字符）。
/// 自动处理 Vision Framework 将多行合并或拆分的情况。
pub fn parse_mrz(lines_raw: &[String]) -> Result<MrzResult, String> {
    // 先清理每行：转大写 + 非 MRZ 字符替换为 <
    let lines: Vec<String> = lines_raw.iter().map(|l| sanitize_mrz_line(l)).collect();

    // 检查是否全是非 MRZ 内容（< 占比 > 80%）
    let filler_count: usize = lines.iter().map(|l| l.chars().filter(|&c| c == '<').count()).sum();
    let total_chars: usize = lines.iter().map(|l| l.len()).sum();
    if total_chars > 0 && filler_count as f64 / total_chars as f64 > 0.8 {
        return Err("MRZ 行无效（大部分字符被替换为填充符）".to_string());
    }

    // ---------- 标准路径 ----------
    if lines.len() == 2 && lines[0].len() >= 40 && lines[1].len() >= 40 {
        return parse_td3(&lines);
    }
    if lines.len() == 3 && lines.iter().all(|l| l.len() >= 25) {
        return parse_td1(&lines);
    }

    // ---------- 容错：Vision 将整个 MRZ 合并为 1 个 Observation ----------
    if lines.len() == 1 {
        let line: Vec<char> = lines[0].chars().collect();
        let total = line.len();

        // TD-3: 2 × 44 = 88 字符
        if total >= 78 && total <= 92 {
            let split = if total >= 86 { 44usize } else { total / 2 };
            if split < total {
                let l1: String = line[..split].iter().collect();
                let l2: String = line[split..].iter().collect();
                if l1.len() >= 40 && l2.len() >= 40 {
                    let candidate = vec![l1, l2];
                    if let Ok(result) = parse_td3(&candidate) {
                        return Ok(result);
                    }
                }
            }
        }

        // TD-1: 3 × 30 = 90 字符
        if total >= 85 && total <= 95 {
            let split1 = 30usize.min(total / 3);
            let split2 = 60usize.min(total * 2 / 3);
            if split1 < split2 && split2 < total {
                let l1: String = line[..split1].iter().collect();
                let l2: String = line[split1..split2].iter().collect();
                let l3: String = line[split2..].iter().collect();
                let candidate = vec![l1, l2, l3];
                if candidate.iter().all(|l| l.len() >= 25) {
                    if let Ok(result) = parse_td1(&candidate) {
                        return Ok(result);
                    }
                }
            }
        }
    }

    // ---------- 容错：Vision 按 << 拆分成了多个碎片 ----------
    // 例如 ["P<UTOERIKSSON", "ANNA", "MARIA", "L898902C3..."]
    // 策略：取最长的 2 条（TD-3）或 3 条（TD-1）作为候选
    if lines.len() >= 2 {
        let mut sorted: Vec<&String> = lines.iter().collect();
        sorted.sort_by(|a, b| b.len().cmp(&a.len()));

        // 尝试最长 2 行（TD-3）
        if sorted.len() >= 2 && sorted[0].len() >= 40 && sorted[1].len() >= 40 {
            let candidate = vec![sorted[0].clone(), sorted[1].clone()];
            if let Ok(result) = parse_td3(&candidate) {
                return Ok(result);
            }
        }

        // 尝试最长 3 行（TD-1）
        if sorted.len() >= 3
            && sorted[0].len() >= 25
            && sorted[1].len() >= 25
            && sorted[2].len() >= 25
        {
            let candidate = vec![sorted[0].clone(), sorted[1].clone(), sorted[2].clone()];
            if let Ok(result) = parse_td1(&candidate) {
                return Ok(result);
            }
        }
    }

    Err("无法识别 MRZ 格式".to_string())
}

fn parse_td3(lines: &[String]) -> Result<MrzResult, String> {
    let line1 = &lines[0];
    let line2 = &lines[1];

    // 补齐/截断到 44 字符（左对齐，右侧空格替换为 '<'）
    // 使用 chars().take(44) 截断长行，避免 format! 不截断导致索引偏移
    let l1: Vec<char> = {
        let s: String = line1.chars().take(44).collect();
        format!("{:<44}", s).replace(' ', "<").chars().collect()
    };
    let l2: Vec<char> = {
        let s: String = line2.chars().take(44).collect();
        format!("{:<44}", s).replace(' ', "<").chars().collect()
    };

    let document_type: String = l1[0..1].iter().collect();
    let document_type_sub: String = l1[1..2].iter().collect();
    let issuing_country: String = l1[2..5].iter().collect();

    let document_number: String = l2[0..9].iter().collect();
    let check_digit_document_number = *l2.get(9).unwrap_or(&'<');
    let nationality: String = l2[10..13].iter().collect();
    let date_of_birth: String = l2[13..19].iter().collect();
    let check_digit_date_of_birth = *l2.get(19).unwrap_or(&'<');
    let sex: String = l2[20..21].iter().collect();
    let expiry_date: String = l2[21..27].iter().collect();
    let check_digit_expiry = *l2.get(27).unwrap_or(&'<');
    let optional_data: String = l2[28..42].iter().collect();
    let optional_check_digit = *l2.get(42).unwrap_or(&'<');
    let composite_check_digit = *l2.get(43).unwrap_or(&'<');

    let doc_valid = mrz_checksum(&document_number) == check_digit_document_number;
    let dob_valid = mrz_checksum(&date_of_birth) == check_digit_date_of_birth;
    let expiry_valid = mrz_checksum(&expiry_date) == check_digit_expiry;

    let mut composite = String::new();
    composite.push_str(&document_number);
    composite.push(check_digit_document_number);
    composite.push_str(&date_of_birth);
    composite.push(check_digit_date_of_birth);
    composite.push_str(&expiry_date);
    composite.push(check_digit_expiry);
    composite.push_str(&optional_data);
    if optional_check_digit != '<' {
        composite.push(optional_check_digit);
    }
    let composite_valid = mrz_checksum(&composite) == composite_check_digit;

    let checksum_valid = doc_valid && dob_valid && expiry_valid && composite_valid;

    Ok(MrzResult {
        document_type,
        document_type_sub,
        issuing_country,
        document_number,
        check_digit_document_number,
        nationality,
        date_of_birth,
        check_digit_date_of_birth,
        sex,
        expiry_date,
        check_digit_expiry,
        optional_data,
        composite_check_digit: composite_check_digit.to_string(),
        raw_lines: lines.to_vec(),
        confidence: 1.0,
        checksum_valid,
    })
}

fn parse_td1(lines: &[String]) -> Result<MrzResult, String> {
    let line1 = &lines[0];
    let line2 = &lines[1];

    // TD-1: 3 行 × 30 字符
    // 使用 chars().take(30) 截断长行，避免 format! 不截断导致索引偏移
    let l1: Vec<char> = {
        let s: String = line1.chars().take(30).collect();
        format!("{:<30}", s).replace(' ', "<").chars().collect()
    };
    let l2: Vec<char> = {
        let s: String = line2.chars().take(30).collect();
        format!("{:<30}", s).replace(' ', "<").chars().collect()
    };

    let document_type: String = l1[0..1].iter().collect();
    let document_type_sub: String = l1[1..2].iter().collect();
    let issuing_country: String = l1[2..5].iter().collect();

    // TD-1 行 1: document number (5-13=9 chars) + check digit at 14
    let document_number: String = l1[5..14].iter().collect();
    let check_digit_document_number = *l1.get(14).unwrap_or(&'<');
    let optional_data_line1: String = l1[15..18].iter().collect();

    // TD-1 行 2: DOB + check + sex + expiry + check + nationality + optional
    let date_of_birth: String = l2[0..6].iter().collect();
    let check_digit_date_of_birth = *l2.get(6).unwrap_or(&'<');
    let sex: String = l2[7..8].iter().collect();
    let expiry_date: String = l2[8..14].iter().collect();
    let check_digit_expiry = *l2.get(14).unwrap_or(&'<');
    let nationality: String = l2[15..18].iter().collect();
    let optional_data_line2: String = l2[18..28].iter().collect();

    let optional_data = format!("{}{}", optional_data_line1, optional_data_line2);

    let doc_valid = mrz_checksum(&document_number) == check_digit_document_number;
    let dob_valid = mrz_checksum(&date_of_birth) == check_digit_date_of_birth;
    let expiry_valid = mrz_checksum(&expiry_date) == check_digit_expiry;

    // TD-1 没有统一的 composite check digit，以行 1/行 2 各自的 composite 代替
    let composite_check_digit = format!(
        "{}-{}",
        l1.get(18).unwrap_or(&'<'),
        l2.get(28).unwrap_or(&'<')
    );

    let checksum_valid = doc_valid && dob_valid && expiry_valid;

    Ok(MrzResult {
        document_type,
        document_type_sub,
        issuing_country,
        document_number,
        check_digit_document_number,
        nationality,
        date_of_birth,
        check_digit_date_of_birth,
        sex,
        expiry_date,
        check_digit_expiry,
        optional_data,
        composite_check_digit,
        raw_lines: lines.to_vec(),
        confidence: 1.0,
        checksum_valid,
    })
}

/// 宽松校验：对 check digit 应用字符修正后重算 checksum。
///
/// 解决 OCR 将 check digit 本身读错的情况（如 B→8, S→5）。
pub fn verify_checksums_lenient(mrz: &MrzResult) -> bool {
    let correct = |c: char| -> char {
        match c {
            'O' | 'D' | 'Q' => '0',
            'I' | 'l' => '1',
            'S' => '5',
            'B' => '8',
            'Z' => '2',
            _ => c,
        }
    };

    let doc_ok = mrz_checksum(&mrz.document_number) == correct(mrz.check_digit_document_number);
    let dob_ok = mrz_checksum(&mrz.date_of_birth) == correct(mrz.check_digit_date_of_birth);
    let expiry_ok = mrz_checksum(&mrz.expiry_date) == correct(mrz.check_digit_expiry);
    doc_ok && dob_ok && expiry_ok
}

/// 从二值化 MRZ 区域图（白字黑底）中按水平投影切分每一行。
/// 返回 Vec<GrayImage>，每个元素是一行文本的灰度图。
pub fn split_mrz_lines(binary: &image::GrayImage) -> Vec<image::GrayImage> {
    let (w, h) = (binary.width(), binary.height());
    if h == 0 || w == 0 {
        return Vec::new();
    }

    // 水平投影：统计每行白色像素（文字）数
    let projection: Vec<u32> = (0..h)
        .map(|y| {
            (0..w)
                .filter(|&x| binary.get_pixel(x, y).0[0] > 0)
                .count() as u32
        })
        .collect();

    let max_val = *projection.iter().max().unwrap_or(&0);
    if max_val == 0 {
        return Vec::new();
    }

    // 使用较低阈值（max/8）检测行间隙，CLAHE 增强后间隙仍有少量噪声
    let gap_threshold = max_val / 8;
    // 使用较高阈值（max/3）检测真正的文字行，排除微弱噪声
    let line_threshold = max_val / 3;
    let min_line_h = 8u32;

    // 第一遍：用低阈值找所有文本区域（含行间噪声）
    let mut bands: Vec<(u32, u32)> = Vec::new();
    let mut in_region = false;
    let mut start = 0u32;
    for (y, &val) in projection.iter().enumerate() {
        let y = y as u32;
        if val > gap_threshold && !in_region {
            in_region = true;
            start = y;
        } else if val <= gap_threshold && in_region {
            in_region = false;
            bands.push((start, y));
        }
    }
    if in_region {
        bands.push((start, h));
    }

    // 第二遍：在每个 band 内找真正的文字行（高投影值区域），间隙为低投影值区域
    // 如果 band 高度 > 50px，说明可能包含多行，按高投影值聚类
    let mut lines = Vec::new();
    for &(bs, be) in &bands {
        if be - bs < min_line_h {
            continue;
        }

        // 在该 band 内再次扫描：用 line_threshold 分割文字行
        let mut in_line = false;
        let mut ls = 0u32;
        for y in bs..be {
            let val = projection[y as usize];
            if val > line_threshold && !in_line {
                in_line = true;
                ls = y;
            } else if val <= line_threshold && in_line {
                in_line = false;
                if y - ls >= min_line_h {
                    let line_img =
                        image::imageops::crop_imm(binary, 0, ls, w, y - ls).to_image();
                    lines.push(line_img);
                }
            }
        }
        if in_line && be - ls >= min_line_h {
            let line_img =
                image::imageops::crop_imm(binary, 0, ls, w, be - ls).to_image();
            lines.push(line_img);
        }
    }

    // 第三遍：如果仍然只有 1 行但高度 > 50（可能两行粘连未分开），
    // 在 band 内找投影值最低的行作为强制切分点
    if lines.len() == 1 && lines[0].height() > 50 {
        let single_img = lines[0].clone(); // clone 避免后续 borrow 冲突
        let (sw, sh) = (single_img.width(), single_img.height());
        // 对单行重新算投影
        let sub_proj: Vec<u32> = (0..sh)
            .map(|y| {
                (0..sw)
                    .filter(|&x| single_img.get_pixel(x, y).0[0] > 0)
                    .count() as u32
            })
            .collect();

        // 找最低点（行间间隙）
        let min_val = *sub_proj.iter().min().unwrap_or(&0);
        if min_val < max_val / 6 {
            // 找到最低点所在行
            let split_y = sub_proj
                .iter()
                .position(|&v| v == min_val)
                .unwrap_or(sh as usize / 2) as u32;

            // 确保切分点在中间区域，不在顶部或底部
            if split_y > sh / 4 && split_y < sh * 3 / 4 {
                lines.clear();
                let top_line =
                    image::imageops::crop_imm(&single_img, 0, 0, sw, split_y).to_image();
                let bot_line = image::imageops::crop_imm(&single_img, 0, split_y, sw, sh - split_y)
                    .to_image();
                if top_line.height() >= min_line_h && bot_line.height() >= min_line_h {
                    lines.push(top_line);
                    lines.push(bot_line);
                } else {
                    // 切分不合理，恢复原状
                    lines.push(single_img);
                }
            }
        }
    }

    lines
}

/// MRZ 校验位算法。
fn mrz_checksum(s: &str) -> char {
    let weights = [7, 3, 1];
    let mut sum = 0u32;

    for (i, c) in s.chars().enumerate() {
        let val = mrz_char_value(c);
        sum += val * weights[i % 3];
    }

    let digit = sum % 10;
    std::char::from_digit(digit, 10).unwrap_or('0')
}

fn mrz_char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c.to_digit(10).unwrap_or(0),
        'A'..='Z' => (c as u32 - 'A' as u32) + 10,
        '<' => 0,
        'a'..='z' => (c as u32 - 'a' as u32) + 10,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mrz_checksum_known() {
        // L898902C3 -> 6 (from ICAO sample)
        assert_eq!(mrz_checksum("L898902C3"), '6');
        // 740812 -> 2
        assert_eq!(mrz_checksum("740812"), '2');
        // 120415 -> 9
        assert_eq!(mrz_checksum("120415"), '9');
    }

    #[test]
    fn test_parse_td3_valid() {
        let lines = vec![
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10".to_string(),
        ];
        let result = parse_td3(&lines).unwrap();
        assert_eq!(result.document_type, "P");
        assert_eq!(result.document_type_sub, "<");
        assert_eq!(result.issuing_country, "UTO");
        assert_eq!(result.document_number, "L898902C3");
        assert_eq!(result.check_digit_document_number, '6');
        assert_eq!(result.nationality, "UTO");
        assert_eq!(result.date_of_birth, "740812");
        assert_eq!(result.check_digit_date_of_birth, '2');
        assert_eq!(result.sex, "F");
        assert_eq!(result.expiry_date, "120415");
        assert_eq!(result.check_digit_expiry, '9');
        assert!(result.checksum_valid);
    }

    #[test]
    fn test_parse_td3_invalid_checksum() {
        let lines = vec![
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
            "L898902C30UTO7408122F1204159ZE184226B<<<<<10".to_string(),
        ];
        let result = parse_td3(&lines).unwrap();
        assert!(!result.checksum_valid);
    }

    #[test]
    fn test_parse_td1_valid() {
        let lines = vec![
            "I<UTOD231458907<<<<<<<<<<<<<<<".to_string(),
            "7408122F1204159UTO<<<<<<<<<<<<".to_string(),
            "ERIKSSON<<ANNA<MARIA<<<<<<<<<<".to_string(),
        ];
        let result = parse_td1(&lines).unwrap();
        assert_eq!(result.document_type, "I");
        assert_eq!(result.document_type_sub, "<");
        assert_eq!(result.issuing_country, "UTO");
        assert_eq!(result.document_number, "D23145890");
        assert_eq!(result.check_digit_document_number, '7');
        assert_eq!(result.nationality, "UTO");
        assert_eq!(result.date_of_birth, "740812");
        assert_eq!(result.check_digit_date_of_birth, '2');
        assert_eq!(result.sex, "F");
        assert_eq!(result.expiry_date, "120415");
        assert_eq!(result.check_digit_expiry, '9');
        assert!(result.checksum_valid);
    }
}
