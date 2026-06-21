//! MRZ（机读区）检测、识别与解析。

use super::types::MrzResult;
use image::{imageops::FilterType, Luma, Rgb, RgbImage};

/// 在图像中检测 MRZ 区域（多窗口滑窗扫描 + 评分）。
///
/// 算法：
/// 1. CLAHE 对比度增强，提升复杂光照下的二值化效果
/// 2. 从底部 40%~100% 以 15% 步长滑窗扫描
/// 3. 每个窗口：Otsu 二值化 + 水平膨胀 + 投影分析
/// 4. 对每个候选按行数/间距均匀度/行高评分
/// 5. 返回最佳候选（评分 >= 0.5），否则 None 触发下游兜底
pub fn detect_mrz_region(image: &RgbImage) -> Option<[(f32, f32); 4]> {
    let (w, h) = (image.width(), image.height());
    if h < 100 || w < 300 {
        return None;
    }

    // CLAHE 增强：提升暗光/反光场景的二值化效果
    let gray = to_grayscale(image);
    let enhanced = apply_clahe(&gray, 3.0, 4, 12);
    // 转回 RGB（try_detect_mrz_scored 接收 RgbImage）
    let enhanced_rgb = RgbImage::from_fn(enhanced.width(), enhanced.height(), |x, y| {
        let p = enhanced.get_pixel(x, y).0[0];
        Rgb([p, p, p])
    });

    // 滑窗：从底部 40% 到 100%，步长约 15%
    // 覆盖护照底部(85-95%)、BRP(60-90%)、身份证(70-85%) 等不同证件类型
    let windows = [0.40, 0.55, 0.70, 0.85, 1.00];
    let offsets = [0, -15];

    let mut best_region: Option<([(f32, f32); 4], f32)> = None;

    for &bottom_ratio in &windows {
        for &offset in &offsets {
            if let Some((region, score)) = try_detect_mrz_scored(&enhanced_rgb, bottom_ratio, offset)
            {
                if best_region.as_ref().map_or(true, |(_, s)| score > *s) {
                    best_region = Some((region, score));
                }
            }
        }
    }

    if let Some((region, score)) = best_region {
        if score >= 0.5 {
            Some(region)
        } else {
            tracing::debug!("[MRZ-Detect] 最佳评分 {:.3} < 0.5，放弃", score);
            None
        }
    } else {
        None
    }
}

/// 对候选 MRZ 进行评分。
///
/// 评分维度：
/// - 行数：3 行 > 2 行 > 其他（BRP/TD-1 有 3 行，护照/TD-3 有 2 行）
/// - 行间距均匀度：CV 越小越均匀
/// - 行间距大小：在 8-40px 合理范围内得高分
///
/// 注意：2 行时只有 1 个间距，CV=0 会得到完美间距分。为
/// 避免 2 行候选反超正确的 3 行候选，2 行时使用中性间距分。
fn compute_candidate_score(num_lines: usize, centers: &[f32]) -> f32 {
    // 1. 行数得分：3 行 > 2 行 > 其他
    // BRP (TD-1) 有 3 行，护照 (TD-3) 有 2 行。
    let row_score = match num_lines {
        3 => 1.05,  // 3 行略高，优先选择完整 MRZ 块
        2 => 1.0,
        4 => 0.6,
        1 => 0.3,
        _ => 0.0,
    };

    if centers.len() < 2 {
        return row_score * 0.5;
    }

    if centers.len() == 2 {
        // 只有 1 个间距，无法计算均匀度。使用中性间距分。
        let gap = centers[1] - centers[0];
        let gap_ok = if (8.0..=40.0).contains(&gap) { 1.0 } else { 0.3 };
        return row_score * 0.40 + 0.5 * 0.35 + gap_ok * 0.25;
    }

    // 3+ 个中心点：计算间距均匀度
    let gaps: Vec<f32> = centers.windows(2).map(|w| w[1] - w[0]).collect();
    let mean_gap = gaps.iter().sum::<f32>() / gaps.len() as f32;
    let var = gaps
        .iter()
        .map(|&g| (g - mean_gap).powi(2))
        .sum::<f32>()
        / gaps.len() as f32;
    let cv = var.sqrt() / mean_gap.max(1.0);
    let spacing_score = 1.0 / (1.0 + cv * 5.0);

    // 3. 平均行间距在合理范围内
    let gap_ok = if (8.0..=40.0).contains(&mean_gap) {
        1.0
    } else {
        0.3
    };

    row_score * 0.40 + spacing_score * 0.35 + gap_ok * 0.25
}

/// 尝试用给定参数检测 MRZ 区域，并返回评分。
/// 与旧的 `try_detect_mrz` 逻辑相同，只是额外计算评分。
fn try_detect_mrz_scored(
    image: &RgbImage,
    bottom_ratio: f32,
    thresh_offset: i32,
) -> Option<([(f32, f32); 4], f32)> {
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

    // 尝试找 3 行或 2 行文本
    // BRP (TD-1) 有 3 行 MRZ；护照 (TD-3) 有 2 行。优先 3 行确保 BRP 完整捕获。
    let centers = find_text_lines_from_bands(&wide_bands, 3, &projection)
        .or_else(|| find_text_lines_from_bands(&wide_bands, 2, &projection))?;

    // 评分
    let score = compute_candidate_score(centers.len(), &centers);

    // 映射回原图坐标
    let pad_y = 14.0;
    // 计算 MRZ 区域边界：覆盖所有行的宽度 + padding
    let region_left: f32 = 0.0; // 使用全宽，确保包含所有 MRZ 字符
    let region_right = (w - 1) as f32;

    let first_center = centers[0];
    let last_center = centers[centers.len() - 1];
    let region_top = (y_start as f32 + first_center - pad_y).max(0.0);
    let region_bottom = (y_start as f32 + last_center + pad_y).min((h - 1) as f32);

    Some((
        [
            (region_left, region_top),
            (region_right, region_top),
            (region_right, region_bottom),
            (region_left, region_bottom),
        ],
        score,
    ))
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
    // CTC 解码可能折叠尾部连续 <（填充符），导致行长度 < 40。
    // parse_td3/parse_td1 内部会 padding 到 44/30 字符，
    // 这里只需确保行有足够的有效内容即可。
    if lines.len() == 2 && lines[0].len() >= 10 && lines[1].len() >= 10 {
        if let Ok(result) = parse_td3(&lines) {
            return Ok(result);
        }
    }
    if lines.len() == 3 && lines.iter().all(|l| l.len() >= 10) {
        if let Ok(result) = parse_td1(&lines) {
            return Ok(result);
        }
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

/// MRZ 预处理：缩放到长边 ≤ 2048 → 灰度 → 高斯模糊
pub fn preprocess_for_mrz(img: &RgbImage) -> image::GrayImage {
    let (w, h) = (img.width(), img.height());

    // 缩放到长边 ≤ 2048
    let max_side = w.max(h);
    let img = if max_side > 2048 {
        let scale = 2048.0 / max_side as f32;
        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;
        let resized = image::imageops::resize(img, new_w.max(1), new_h.max(1), FilterType::Lanczos3);
        resized
    } else {
        // Clone to RgbImage if it's already small enough
        image::imageops::crop_imm(img, 0, 0, w, h).to_image()
    };

    // 灰度
    let gray = image::imageops::grayscale(&img);

    // 高斯模糊（σ=1.0）
    let blurred = image::imageops::blur(&gray, 3.0);

    blurred
}

/// 连通域：BFS 找到所有满足尺寸条件的白色像素区域
/// Sauvola 输出为黑字白底（text=0, bg=255），所以找黑色像素（值 < 128）
fn find_connected_components(
    binary: &image::GrayImage,
    min_width: u32,
    min_height: u32,
) -> Vec<(i32, i32, i32, i32)> {
    let (w, h) = (binary.width(), binary.height());
    let mut visited = vec![false; (w * h) as usize];
    let mut components = Vec::new();

    let dirs = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)];

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if visited[idx] || binary.get_pixel(x, y).0[0] >= 128 {
                // 跳过背景（白色）
                continue;
            }
            visited[idx] = true;

            // BFS
            let mut min_x = x as i32;
            let mut max_x = x as i32;
            let mut min_y = y as i32;
            let mut max_y = y as i32;
            let mut stack = vec![(x as i32, y as i32)];

            while let Some((cx, cy)) = stack.pop() {
                for &(dx, dy) in &dirs {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || nx >= w as i32 || ny < 0 || ny >= h as i32 {
                        continue;
                    }
                    let nidx = (ny as u32 * w + nx as u32) as usize;
                    if visited[nidx] {
                        continue;
                    }
                    let nval = binary.get_pixel(nx as u32, ny as u32).0[0];
                    if nval >= 128 {
                        continue;
                    }
                    visited[nidx] = true;
                    stack.push((nx, ny));
                    min_x = min_x.min(nx);
                    max_x = max_x.max(nx);
                    min_y = min_y.min(ny);
                    max_y = max_y.max(ny);
                }
            }

            let cw = (max_x - min_x + 1) as u32;
            let ch = (max_y - min_y + 1) as u32;
            if cw >= min_width && ch >= min_height {
                components.push((min_x, min_y, max_x, max_y));
            }
        }
    }

    components
}

/// 计算字符密度：黑色像素占比
fn compute_char_density(binary: &image::GrayImage, x1: i32, y1: i32, x2: i32, y2: i32) -> f32 {
    let x1 = x1.max(0) as u32;
    let y1 = y1.max(0) as u32;
    let x2 = (x2 as u32).min(binary.width().saturating_sub(1));
    let y2 = (y2 as u32).min(binary.height().saturating_sub(1));

    let area = (x2 - x1 + 1) * (y2 - y1 + 1);
    if area == 0 {
        return 0.0;
    }

    let mut black_count = 0u32;
    for y in y1..=y2 {
        for x in x1..=x2 {
            if binary.get_pixel(x, y).0[0] < 128 {
                black_count += 1;
            }
        }
    }

    black_count as f32 / area as f32
}

/// 水平投影
fn horizontal_projection(binary: &image::GrayImage) -> Vec<u32> {
    let (w, h) = (binary.width(), binary.height());
    (0..h)
        .map(|y| {
            (0..w)
                .filter(|&x| binary.get_pixel(x, y).0[0] < 128) // 黑色像素
                .count() as u32
        })
        .collect()
}

/// 检查字符间距是否均匀（CV < 0.5 为均匀）
fn is_uniform_spacing(vertical_proj: &[u32], min_gap: u32) -> bool {
    // 找到投影值的峰值位置（字符间间隙）
    let mut peaks = Vec::new();
    let threshold = vertical_proj.iter().max().copied().unwrap_or(0) / 3;

    for (x, &val) in vertical_proj.iter().enumerate() {
        if val > threshold {
            // 局部峰值
            let left = if x > 0 { vertical_proj[x - 1] } else { 0 };
            let right = vertical_proj.get(x + 1).copied().unwrap_or(0);
            if val >= left && val >= right {
                peaks.push(x as f32);
            }
        }
    }

    if peaks.len() < 5 {
        return false;
    }

    // 计算间距的变异系数
    let gaps: Vec<f32> = peaks.windows(2).map(|w| w[1] - w[0]).collect();
    let mean_gap = gaps.iter().sum::<f32>() / gaps.len() as f32;
    if mean_gap < min_gap as f32 {
        return false;
    }

    let variance = gaps.iter().map(|g| (g - mean_gap).powi(2)).sum::<f32>() / gaps.len() as f32;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean_gap;

    cv < 0.5 // CV < 0.5 表示间距均匀
}

// ─── 三遍定位 ───────────────────────────────────────────────

/// 策略 A：连通域定位
/// 过滤条件：宽高比 5:1 ~ 30:1，面积占比 3% ~ 35%，字符密度 10% ~ 50%
fn locate_by_connected_components(binary: &image::GrayImage) -> Option<(u32, u32, u32, u32)> {
    let (bw, bh) = (binary.width(), binary.height());
    let img_area = (bw * bh) as f32;

    let components = find_connected_components(binary, 20, 5);

    let mut candidates: Vec<(i32, i32, i32, i32, f32)> = Vec::new();

    for &(x1, y1, x2, y2) in &components {
        let cw = (x2 - x1 + 1) as u32;
        let ch = (y2 - y1 + 1) as u32;
        let aspect_ratio = cw as f32 / ch.max(1) as f32;

        // 宽高比 5:1 ~ 30:1（MRZ 行特征）
        if !(5.0..=30.0).contains(&aspect_ratio) {
            continue;
        }

        let area = (cw * ch) as f32;
        let area_ratio = area / img_area;
        // 面积占比 3% ~ 35%
        if !(0.03..=0.35).contains(&area_ratio) {
            continue;
        }

        let density = compute_char_density(binary, x1, y1, x2, y2);
        // 字符密度 10% ~ 50%
        if !(0.10..=0.50).contains(&density) {
            continue;
        }

        // 分数：面积大 + 密度高 = 更可能是 MRZ
        let score = area * density;
        candidates.push((x1, y1, x2, y2, score));
    }

    if candidates.is_empty() {
        return None;
    }

    // 选分数最高的候选
    candidates.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    let (x1, y1, x2, y2, _) = candidates[0];

    Some((x1.max(0) as u32, y1.max(0) as u32, x2.max(0) as u32, y2.max(0) as u32))
}

/// 策略 B：投影法定位
/// 用水平投影找高密度文本行，垂直投影验证均匀间距
fn locate_by_projection(binary: &image::GrayImage) -> Option<(u32, u32, u32, u32)> {
    let (bw, bh) = (binary.width(), binary.height());

    let h_proj = horizontal_projection(binary);
    let max_val = *h_proj.iter().max()?;
    if max_val == 0 {
        return None;
    }

    // 找高密度行（投影值 > max/4）
    let threshold = max_val / 4;
    let mut bands: Vec<(u32, u32)> = Vec::new();
    let mut in_band = false;
    let mut start = 0u32;

    for (y, &val) in h_proj.iter().enumerate() {
        let y = y as u32;
        if val > threshold && !in_band {
            in_band = true;
            start = y;
        } else if val <= threshold && in_band {
            in_band = false;
            if y - start >= 8 {
                bands.push((start, y));
            }
        }
    }
    if in_band {
        bands.push((start, bh));
    }

    if bands.is_empty() {
        return None;
    }

    // 合并相近的行带
    let merged = merge_rows(&bands, 4);

    // 对每个合并后的行带，检查垂直投影的均匀间距
    for &(ys, ye) in &merged {
        let roi_h = ye - ys;
        if roi_h < 15 || roi_h > 100 {
            continue;
        }

        // 取该 ROI 的垂直投影
        // 裁剪垂直投影范围（整个宽度，从 ys 到 ye）
        let v_proj = vertical_projection_on_roi(binary, 0, bw, ys, ye);
        if is_uniform_spacing(&v_proj, 5) {
            return Some((0, ys, bw, ye));
        }
    }

    None
}

/// 在 ROI 内计算垂直投影
fn vertical_projection_on_roi(
    binary: &image::GrayImage,
    x_start: u32,
    x_end: u32,
    y_start: u32,
    y_end: u32,
) -> Vec<u32> {
    let x_end = x_end.min(binary.width());
    let y_end = y_end.min(binary.height());

    (x_start..x_end)
        .map(|x| {
            (y_start..y_end)
                .filter(|&y| binary.get_pixel(x, y).0[0] < 128)
                .count() as u32
        })
        .collect()
}

/// 合并相邻行带（间隔 < 4px）
fn merge_rows(bands: &[(u32, u32)], gap_threshold: u32) -> Vec<(u32, u32)> {
    if bands.is_empty() {
        return Vec::new();
    }

    let mut merged = Vec::new();
    let mut cur_start = bands[0].0;
    let mut cur_end = bands[0].1;

    for &(s, e) in &bands[1..] {
        if s - cur_end <= gap_threshold {
            cur_end = e;
        } else {
            merged.push((cur_start, cur_end));
            cur_start = s;
            cur_end = e;
        }
    }
    merged.push((cur_start, cur_end));

    merged
}

/// 策略 C：固定布局定位（兜底）
/// 对标准证件比例，假设 MRZ 在底部 25-35%
fn locate_by_fixed_layout(img: &RgbImage) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = (img.width(), img.height());
    let aspect = w as f32 / h.max(1) as f32;

    // 标准证件比例：护照（~1.4），身份证（~0.7）
    let is_portrait = aspect >= 0.6 && aspect <= 0.9;
    let is_landscape = aspect >= 1.2 && aspect <= 1.6;

    if !is_portrait && !is_landscape {
        return None;
    }

    // MRZ 在证件最底部（护照最后 2 行，身份证最后 2-3 行）。
    // 覆盖底部 20%，确保在所有证件上都能捕获 MRZ 区域。
    let y_start = (h as f32 * 0.80) as u32;
    let y_end = h;

    if y_end <= y_start || y_start >= h {
        return None;
    }

    Some((0, y_start, w, y_end))
}

/// 四遍定位主函数
/// 依次尝试：滑窗扫描(推荐) → 连通域 → 投影法 → 固定布局
pub fn locate_mrz_region(binary: &image::GrayImage, img: &RgbImage) -> Option<[(f32, f32); 4]> {
    // 策略 D：滑窗扫描（优先推荐）
    // CLAHE + Otsu + 多窗口评分，覆盖各种证件类型
    if let Some(region) = detect_mrz_region(img) {
        // MRZ 应在图像底部附近。如果滑窗找到的区域远离底部（如非 MRZ 文本），
        // 则跳过滑窗结果，让连通域/投影法/固定布局来捕获真正的底部 MRZ。
        let img_h = img.height() as f32;
        let region_bottom = region[2].1;
        let bottom_ratio = region_bottom / img_h;
        // 阈值 85%：MRZ 通常在底部 85-100% 区域
        if bottom_ratio >= 0.85 {
            tracing::info!("[MRZ] 策略 D (滑窗扫描) 成功, bottom={:.1}/{:.1}", region_bottom, img_h);
            return Some(region);
        }
        tracing::info!("[MRZ] 策略 D 区域不在底部 (bottom={:.1}/{:.1}), 走下一策略", region_bottom, img_h);
    }

    // 策略 A：连通域
    if let Some((x1, y1, x2, y2)) = locate_by_connected_components(binary) {
        tracing::info!("[MRZ] 策略 A (连通域) 成功: ({},{})-({},{})", x1, y1, x2, y2);
        return Some([
            (x1 as f32, y1 as f32),
            (x2 as f32, y1 as f32),
            (x2 as f32, y2 as f32),
            (x1 as f32, y2 as f32),
        ]);
    }

    // 策略 B：投影法
    if let Some((x1, y1, x2, y2)) = locate_by_projection(binary) {
        tracing::info!("[MRZ] 策略 B (投影法) 成功: ({},{})-({},{})", x1, y1, x2, y2);
        return Some([
            (x1 as f32, y1 as f32),
            (x2 as f32, y1 as f32),
            (x2 as f32, y2 as f32),
            (x1 as f32, y2 as f32),
        ]);
    }

    // 策略 C：固定布局（兜底）
    if let Some((x1, y1, x2, y2)) = locate_by_fixed_layout(img) {
        tracing::info!("[MRZ] 策略 C (固定布局) 成功: ({},{})-({},{})", x1, y1, x2, y2);
        return Some([
            (x1 as f32, y1 as f32),
            (x2 as f32, y1 as f32),
            (x2 as f32, y2 as f32),
            (x1 as f32, y2 as f32),
        ]);
    }

    None
}

/// 在 MRZ 区域内按水平投影切分为单行文本图像
pub fn split_text_lines(binary: &image::GrayImage, region: &[(f32, f32); 4]) -> Vec<image::GrayImage> {
    let (bw, bh) = (binary.width(), binary.height());

    // 计算 ROI 边界
    let x1 = (region[0].0.max(0.0) as u32).min(bw.saturating_sub(1));
    let y1 = (region[0].1.max(0.0) as u32).min(bh.saturating_sub(1));
    let x2 = (region[2].0 as u32).min(bw.saturating_sub(1));
    let y2 = (region[2].1 as u32).min(bh.saturating_sub(1));

    if x2 <= x1 || y2 <= y1 {
        return Vec::new();
    }

    // 在 ROI 内计算水平投影（黑色像素）
    let h_proj: Vec<u32> = (y1..y2)
        .map(|y| {
            (x1..x2)
                .filter(|&x| binary.get_pixel(x, y).0[0] < 128)
                .count() as u32
        })
        .collect();

    let max_val = *h_proj.iter().max().unwrap_or(&0);
    if max_val == 0 {
        return Vec::new();
    }

    // 检测行峰值（固定阈值 5，灰度图文字行投影值远高于此）
    let line_peaks = detect_line_peaks(&h_proj, 5, 3);

    // 裁剪每行
    let mut lines = Vec::new();
    for &(ys, ye) in &line_peaks {
        let roi = image::imageops::crop_imm(binary, x1, y1 + ys, x2 - x1, ye - ys).to_image();
        lines.push(roi);
    }

    lines
}

/// 检测行峰值：在投影数组中找连续的文字行区域。
///
/// 使用固定阈值（5），灰度图中文字行投影值远高于 5，
/// 而行间间隙（高斯模糊后背景平滑）投影值接近 0，可清晰分离。
/// `min_peak_height`: 峰值最小高度（像素），默认 5
/// `min_gap`: 行间最小间隔（像素），默认 3
fn detect_line_peaks(proj: &[u32], min_peak_height: u32, min_gap: u32) -> Vec<(u32, u32)> {
    let mut peaks = Vec::new();
    let mut in_line = false;
    let mut start = 0u32;

    for (y, &val) in proj.iter().enumerate() {
        let y = y as u32;
        if val > min_peak_height && !in_line {
            in_line = true;
            start = y;
        } else if val <= min_peak_height && in_line {
            in_line = false;
            if y - start >= min_gap {
                peaks.push((start, y));
            }
        }
    }
    if in_line && proj.len() as u32 - start >= min_gap {
        peaks.push((start, proj.len() as u32));
    }

    // 合并间隙小的相邻行
    if peaks.len() >= 2 {
        let mut merged = Vec::new();
        let mut cur = peaks[0];
        for &(s, e) in &peaks[1..] {
            if s - cur.1 <= min_gap {
                cur.1 = e;
            } else {
                merged.push(cur);
                cur = (s, e);
            }
        }
        merged.push(cur);
        merged
    } else {
        peaks
    }
}

/// ICAO 字符标准化
/// - O → 0, I → 1, 空格 → <
/// - 小写转大写，非法字符 → <
pub fn icao_normalize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            // O 不是有效 MRZ 字符（ICAO 9303），常见 OCR 混淆 → 转 0
            'O' => '0',
            // I 在某些证件姓名中有效，保留原样
            'A'..='Z' => c,
            '0'..='9' => c,
            '<' => c,
            'a'..='z' => c.to_ascii_uppercase(),
            ' ' => '<',
            _ => '<',
        })
        .collect()
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

    #[test]
    fn test_icao_normalize() {
        // O→0（ICAO 标准）
        assert_eq!(icao_normalize("O"), "0");
        // I 是有效 MRZ 字符，保留
        assert_eq!(icao_normalize("I"), "I");
        // 小写→大写, 空格→<, 非法字符→<
        assert_eq!(icao_normalize("hello"), "HELLO");
        assert_eq!(icao_normalize("abc 123"), "ABC<123");
        assert_eq!(icao_normalize("P<UT0ER1KSS0N<<ANNA"), "P<UT0ER1KSS0N<<ANNA");
        assert_eq!(icao_normalize(" "), "<");
    }



    #[test]
    fn test_split_text_lines_empty() {
        // 全白图像（无黑色像素）应返回空
        let binary = image::GrayImage::from_pixel(100, 50, Luma([255]));
        let region = [(0.0, 0.0), (99.0, 0.0), (99.0, 49.0), (0.0, 49.0)];
        let lines = split_text_lines(&binary, &region);
        assert!(lines.is_empty(), "no text should produce no lines");
    }
}
