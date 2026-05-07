//! MRZ 专用识别流水线
//!
//! 跳过 det/cls 模型，直接通过启发式定位 + rec 模型完成 MRZ 提取。
//! 三级 ROI 定位策略确保对倾斜、反光、不同尺寸证件的鲁棒性。

use image::{DynamicImage, GrayImage};
use imageproc::rect::Rect;

use super::error::OcrError;
use super::inference::recognize_lines;
use super::postprocess::icao_normalize;
use super::preprocess::{horizontal_projection, preprocess_for_mrz, vertical_projection};

/// 从图像中提取 MRZ 原始文本行
///
/// 返回识别到的原始 MRZ 字符串行（未做 TD1/TD2/TD3 语义解析）。
pub fn extract_mrz_lines(img: &DynamicImage) -> Result<Vec<String>, OcrError> {
    // 步骤 1：预处理（灰度化 + 二值化 + 尺寸归一化）
    let binary = preprocess_for_mrz(img);

    // 步骤 2：三级 ROI 定位
    let roi_regions = locate_mrz_region(&binary)?;

    // 步骤 3：行切分（水平投影）
    let mut text_lines = split_text_lines(&binary, &roi_regions);

    // MRZ 最多 2 行（TD1/TD2/TD3），限制推理数量避免超时
    const MAX_MRZ_LINES: usize = 4;
    if text_lines.len() > MAX_MRZ_LINES {
        // 优先保留图像底部的行（MRZ 通常在证件底部）
        let start = text_lines.len().saturating_sub(MAX_MRZ_LINES);
        text_lines = text_lines.split_off(start);
    }

    if text_lines.is_empty() {
        return Err(OcrError::MrzNotFound {
            reason: "No text lines detected in ROI".to_string(),
        });
    }

    // 步骤 4：逐行识别
    let rec_results = recognize_lines(&text_lines)?;

    // 步骤 5：ICAO 后处理 + 调试日志
    let mut mrz_lines = Vec::new();
    for result in rec_results {
        let normalized = icao_normalize(&result.text);
        if !normalized.is_empty() {
            mrz_lines.push(normalized);
        }
    }

    // 步骤 6：长度校验和过滤
    // 放宽条件：允许 ±2 误差，覆盖模糊/截断情况
    mrz_lines.retain(|line| {
        let len = line.len();
        (28..=32).contains(&len) || (34..=38).contains(&len) || (42..=46).contains(&len)
    });

    if mrz_lines.is_empty() {
        return Err(OcrError::MrzNotFound {
            reason: "No valid MRZ lines found after filtering".to_string(),
        });
    }

    Ok(mrz_lines)
}

// ============================================================================
// 三级 ROI 定位策略
// ============================================================================

/// 三级 ROI 定位：形态学连通域 → 边缘密度投影 → 固定布局兜底
fn locate_mrz_region(binary: &GrayImage) -> Result<Vec<Rect>, OcrError> {
    // 策略 A：形态学连通域过滤（首选）
    if let Some(regions) = locate_by_connected_components(binary) {
        return Ok(regions);
    }

    // 策略 B：边缘密度 + 水平/垂直投影（备用）
    if let Some(regions) = locate_by_projection(binary) {
        return Ok(regions);
    }

    // 策略 C：固定布局假设（兜底）
    if let Some(regions) = locate_by_fixed_layout(binary) {
        return Ok(regions);
    }

    Err(OcrError::MrzNotFound {
        reason: "无法自动定位 MRZ 区域，请尝试调整拍摄角度或手动框选".to_string(),
    })
}

/// 策略 A：形态学连通域过滤
///
/// 1. 直接在二值图像上查找连通域
/// 2. 筛选长宽比 > 5:1 的连通域
/// 3. 验证字符密度（黑点占比 15%~40%）
fn locate_by_connected_components(binary: &GrayImage) -> Option<Vec<Rect>> {
    // 查找连通域（黑色为前景）
    let components = find_connected_components(binary);

    // 筛选候选区域
    let img_area = (binary.width() * binary.height()) as f32;
    let mut candidates: Vec<Rect> = components
        .into_iter()
        .filter(|rect| {
            let w = rect.width() as f32;
            let h = rect.height() as f32;
            let aspect = w / h;
            let area_ratio = (w * h) / img_area;
            let density = compute_char_density(binary, rect);

            aspect > 5.0
                && aspect < 30.0
                && area_ratio > 0.03
                && area_ratio < 0.35
                && density > 0.10
                && density < 0.50
        })
        .collect();

    // 按面积排序，取最大的
    candidates.sort_by_key(|r| r.width() * r.height());
    candidates.last().map(|r| vec![*r])
}

/// 策略 B：边缘密度 + 水平/垂直投影
///
/// 1. Canny 边缘检测
/// 2. 水平投影定位文字行密集区
/// 3. 垂直投影验证字符宽度均匀性（MRZ 字符等宽）
fn locate_by_projection(binary: &GrayImage) -> Option<Vec<Rect>> {
    // 水平投影找文字行
    let h_proj = horizontal_projection(binary);
    let text_rows = detect_line_peaks(&h_proj, 10, 5);

    if text_rows.len() < 2 {
        return None;
    }

    // 合并连续的行区域
    let merged = merge_rows(&text_rows, binary.width())?;

    // 垂直投影验证字符间距均匀性
    let v_proj = vertical_projection(binary, &merged);
    if !is_uniform_spacing(&v_proj) {
        return None;
    }

    Some(vec![merged])
}

/// 策略 C：固定布局假设（仅当图像比例接近证件标准时）
fn locate_by_fixed_layout(gray: &GrayImage) -> Option<Vec<Rect>> {
    let (w, h) = gray.dimensions();
    let ratio = w as f32 / h as f32;

    // 护照/身份证常见比例范围
    let is_likely_document = (0.6..0.9).contains(&ratio) || (1.2..1.6).contains(&ratio);

    if !is_likely_document {
        return None;
    }

    // MRZ 通常位于底部 25%~35% 区域
    let y = (h as f32 * 0.65) as i32;
    let height = ((h as f32 * 0.15) as u32).max(1);

    // 确保 y 不超出图像边界，且 height 有效
    let y = y.clamp(0, h.saturating_sub(height) as i32);

    Some(vec![Rect::at(0, y).of_size(w, height)])
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 查找二值图像中的黑色连通域（简单实现）
fn find_connected_components(binary: &GrayImage) -> Vec<Rect> {
    let (width, height) = binary.dimensions();
    let mut visited = vec![vec![false; width as usize]; height as usize];
    let mut components = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if !visited[y as usize][x as usize] && binary.get_pixel(x, y)[0] < 128 {
                // BFS 遍历连通域
                let mut min_x = x;
                let mut max_x = x;
                let mut min_y = y;
                let mut max_y = y;
                let mut stack = vec![(x, y)];
                visited[y as usize][x as usize] = true;

                while let Some((cx, cy)) = stack.pop() {
                    min_x = min_x.min(cx);
                    max_x = max_x.max(cx);
                    min_y = min_y.min(cy);
                    max_y = max_y.max(cy);

                    for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = cx as i32 + dx;
                        let ny = cy as i32 + dy;
                        if nx >= 0
                            && ny >= 0
                            && (nx as u32) < width
                            && (ny as u32) < height
                        {
                            let nx = nx as u32;
                            let ny = ny as u32;
                            if !visited[ny as usize][nx as usize]
                                && binary.get_pixel(nx, ny)[0] < 128
                            {
                                visited[ny as usize][nx as usize] = true;
                                stack.push((nx, ny));
                            }
                        }
                    }
                }

                let rect_width = max_x - min_x + 1;
                let rect_height = max_y - min_y + 1;
                if rect_width > 20 && rect_height > 5 {
                    components.push(Rect::at(min_x as i32, min_y as i32).of_size(
                        rect_width,
                        rect_height,
                    ));
                }
            }
        }
    }

    components
}

/// 计算区域内的字符密度（黑色像素占比）
fn compute_char_density(binary: &GrayImage, rect: &Rect) -> f32 {
    let mut black_count = 0u32;
    let mut total = 0u32;

    for y in 0..rect.height() {
        for x in 0..rect.width() {
            let px = rect.left() + x as i32;
            let py = rect.top() + y as i32;
            if px >= 0
                && py >= 0
                && (px as u32) < binary.width()
                && (py as u32) < binary.height()
            {
                total += 1;
                if binary.get_pixel(px as u32, py as u32)[0] < 128 {
                    black_count += 1;
                }
            }
        }
    }

    if total == 0 {
        0.0
    } else {
        black_count as f32 / total as f32
    }
}

/// 从水平投影中检测文字行峰值
///
/// `min_peak_height`：峰值最小高度（像素数）
/// `min_gap`：行之间的最小间隔（像素数）
fn detect_line_peaks(projection: &[u32], min_peak_height: u32, min_gap: u32) -> Vec<(u32, u32)> {
    let mut rows = Vec::new();
    let mut in_peak = false;
    let mut peak_start = 0u32;

    for (y, &val) in projection.iter().enumerate() {
        let y = y as u32;
        if val >= min_peak_height {
            if !in_peak {
                in_peak = true;
                peak_start = y;
            }
        } else if in_peak {
            in_peak = false;
            if y - peak_start >= min_gap {
                rows.push((peak_start, y));
            }
        }
    }

    // 处理末尾峰值
    if in_peak {
        let end = projection.len() as u32;
        if end - peak_start >= min_gap {
            rows.push((peak_start, end));
        }
    }

    rows
}

/// 合并多个行区域为一个大的 ROI
/// 返回的 Rect 宽度为图像全宽（调用方通过 split_text_lines 处理）
fn merge_rows(rows: &[(u32, u32)], img_width: u32) -> Option<Rect> {
    if rows.is_empty() {
        return None;
    }
    let min_y = rows.iter().map(|r| r.0).min().unwrap_or(0);
    let max_y = rows.iter().map(|r| r.1).max().unwrap_or(0);
    let height = (max_y - min_y).max(1);

    Some(Rect::at(0, min_y as i32).of_size(img_width.max(1), height))
}

/// 验证垂直投影是否显示字符间距均匀（MRZ 等宽字符特征）
fn is_uniform_spacing(v_proj: &[u32]) -> bool {
    if v_proj.len() < 10 {
        return false;
    }

    // 检测峰值间距的方差：MRZ 字符间距应相对均匀
    let mut peaks = Vec::new();
    for (i, &val) in v_proj.iter().enumerate() {
        if val > 0 {
            peaks.push(i);
        }
    }

    if peaks.len() < 5 {
        return false;
    }

    // 计算相邻峰值间距
    let gaps: Vec<usize> = peaks.windows(2).map(|w| w[1] - w[0]).collect();
    if gaps.len() < 2 {
        return false;
    }

    let mean = gaps.iter().sum::<usize>() as f32 / gaps.len() as f32;
    let variance = gaps
        .iter()
        .map(|&g| {
            let diff = g as f32 - mean;
            diff * diff
        })
        .sum::<f32>()
        / gaps.len() as f32;
    let cv = (variance.sqrt() / mean).max(0.0); // 变异系数

    // 变异系数 < 0.5 认为 spacing 相对均匀
    cv < 0.5
}

/// 在 ROI 区域内按水平投影切分行
fn split_text_lines(binary: &GrayImage, regions: &[Rect]) -> Vec<GrayImage> {
    let mut lines = Vec::new();

    for region in regions {
        // 若 region 宽度为 0，使用图像全宽
        let region_width = if region.width() == 0 {
            binary.width()
        } else {
            region.width()
        };

        let roi = image::imageops::crop_imm(
            binary,
            region.left().max(0) as u32,
            region.top().max(0) as u32,
            region_width,
            region.height(),
        )
        .to_image();

        let proj = horizontal_projection(&roi);
        let row_ranges = detect_line_peaks(&proj, 5, 3);

        for (start, end) in row_ranges {
            if end > start {
                let line_img = image::imageops::crop_imm(&roi, 0, start, roi.width(), end - start)
                    .to_image();
                lines.push(line_img);
            }
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_line_peaks() {
        let proj = vec![0, 0, 5, 5, 5, 0, 0, 3, 3, 3, 0, 0];
        let peaks = detect_line_peaks(&proj, 2, 2);
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[0], (2, 5));
        assert_eq!(peaks[1], (7, 10));
    }

    #[test]
    fn test_icao_normalize() {
        // 在 postprocess.rs 中测试
    }
}
