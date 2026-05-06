//! 通用 OCR 流水线（Phase 2）
//!
//! 实现 det → rec 两阶段流水线：
//! 1. det 模型检测图像中所有文本区域
//! 2. 对每个检测到的区域进行 rec 识别
//!
//! 注：Phase 2 初始版本跳过 cls（方向分类），假设文本为水平方向。
//! cls 将在后续版本中补充。

use image::DynamicImage;
use ndarray::Array;

use super::error::OcrError;
use super::inference::{classify_orientation, recognize_line, TextOrientation};
use super::model::get_det_session;
use super::preprocess::prepare_det_input;

/// 检测到的文本区域
#[derive(Debug, Clone)]
pub struct TextRegion {
    /// 相对坐标 (0.0~1.0)
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// 区域置信度（来自 det probability map 的平均值）
    pub confidence: f32,
}

/// 通用 OCR 结果
#[derive(Debug, Clone)]
pub struct GeneralOcrResult {
    /// 合并后的原始文本（按 reading order）
    pub raw_text: String,
    /// 每个文本块的识别结果
    pub blocks: Vec<OcrBlock>,
    /// 整体平均置信度
    pub confidence: f32,
}

/// OCR 文本块（含 bbox 和识别结果）
#[derive(Debug, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: TextRegion,
}

/// 对任意图像执行通用 OCR
///
/// 流程：
/// 1. det 预处理 → ONNX 推理 → 文本区域检测
/// 2. 对每个区域：裁剪 → rec 预处理 → ONNX 推理 → CTC 解码
/// 3. 按 reading order 排序 → 组装结果
pub fn recognize_image(img: &DynamicImage) -> Result<GeneralOcrResult, OcrError> {
    let regions = detect_text_regions(img)?;

    if regions.is_empty() {
        return Err(OcrError::TextNotDetected);
    }

    let mut blocks = Vec::with_capacity(regions.len());

    for region in regions {
        // 裁剪文本区域
        let patch = crop_region(img, &region);

        // cls 方向分类
        let rgb_patch = patch.to_rgb8();
        let orientation = classify_orientation(&rgb_patch)?;
        let rotated_patch = match orientation {
            TextOrientation::Normal => patch.clone(),
            TextOrientation::Rotated180 => rotate_180(&patch),
        };

        // rec 识别
        let gray = rotated_patch.to_luma8();
        let rec_result = recognize_line(&gray)?;

        blocks.push(OcrBlock {
            text: rec_result.text,
            confidence: rec_result.confidence,
            bbox: region,
        });
    }

    // 按 reading order 排序
    sort_by_reading_order(&mut blocks);

    // 组装结果
    let raw_text = blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n");
    let avg_confidence = if blocks.is_empty() {
        0.0
    } else {
        blocks.iter().map(|b| b.confidence).sum::<f32>() / blocks.len() as f32
    };

    Ok(GeneralOcrResult {
        raw_text,
        blocks,
        confidence: avg_confidence,
    })
}

// ============================================================================
// 文本区域检测（det 模型）
// ============================================================================

/// 使用 det 模型检测图像中的文本区域
///
/// 简化版 DB 后处理：
/// 1. 对 probability map 应用阈值
/// 2. 查找连通组件
/// 3. 计算每个组件的 bounding box
/// 4. 过滤过小区域
fn detect_text_regions(img: &DynamicImage) -> Result<Vec<TextRegion>, OcrError> {
    let mut session = get_det_session()
        .map_err(|e| OcrError::InferenceFailed(format!("DET session error: {e}")))?;

    let (orig_w, orig_h) = (img.width(), img.height());

    // det 预处理
    let (input_data, _scale, (target_w, target_h)) = prepare_det_input(img);

    // 构造 NCHW tensor
    let input_array = Array::from_shape_vec(
        (1, 3, target_h as usize, target_w as usize),
        input_data,
    )
    .map_err(|e| OcrError::InferenceFailed(format!("DET tensor shape error: {e}")))?;

    let input_value = ort::value::Tensor::from_array(input_array)
        .map_err(|e| OcrError::InferenceFailed(format!("DET tensor creation failed: {e}")))?;

    // ONNX 推理
    let outputs = session
        .run(vec![("x", input_value.into_dyn())])
        .map_err(|e| OcrError::InferenceFailed(format!("DET ONNX run failed: {e}")))?;

    // 提取 probability map
    let output = &outputs[0];
    let (shape, data) = output
        .try_extract_tensor::<f32>()
        .map_err(|e| OcrError::InferenceFailed(format!("DET output extract failed: {e}")))?;

    if shape.len() != 4 || shape[0] != 1 || shape[1] != 1 {
        return Err(OcrError::InferenceFailed(
            format!("Unexpected DET output shape: {:?}", shape)
        ));
    }

    let prob_h = shape[2] as usize;
    let prob_w = shape[3] as usize;

    // 阈值化并查找连通区域
    let threshold = 0.3f32;
    let min_area = 10usize; // 最小区域面积（在 prob_map 坐标系中）

    let regions = extract_regions_from_prob_map(
        data,
        prob_w,
        prob_h,
        threshold,
        min_area,
    );

    // 将坐标从 prob_map 空间映射回原始图像空间
    let scale_x = orig_w as f32 / prob_w as f32;
    let scale_y = orig_h as f32 / prob_h as f32;

    let mut text_regions = Vec::new();
    for (px, py, pw, ph, conf) in regions {
        let x = (px as f32 * scale_x) / orig_w as f32;
        let y = (py as f32 * scale_y) / orig_h as f32;
        let width = (pw as f32 * scale_x) / orig_w as f32;
        let height = (ph as f32 * scale_y) / orig_h as f32;

        // 过滤极端比例的假阳性（文本通常宽 > 高）
        if width > 0.01 && height > 0.005 && width / height.max(0.001) > 0.5 {
            text_regions.push(TextRegion {
                x: x.clamp(0.0, 1.0),
                y: y.clamp(0.0, 1.0),
                width: width.clamp(0.0, 1.0 - x),
                height: height.clamp(0.0, 1.0 - y),
                confidence: conf,
            });
        }
    }

    // NMS：移除重叠度过高的区域（保留置信度高的）
    let text_regions = nms_regions(text_regions, 0.5);

    Ok(text_regions)
}

/// 从 probability map 中提取连通区域 bounding boxes
///
/// 使用简单的 BFS/洪水填充算法查找连通组件。
fn extract_regions_from_prob_map(
    data: &[f32],
    width: usize,
    height: usize,
    threshold: f32,
    min_area: usize,
) -> Vec<(usize, usize, usize, usize, f32)> {
    let mut visited = vec![false; width * height];
    let mut regions = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if visited[idx] || data.get(idx).unwrap_or(&0.0) < &threshold {
                continue;
            }

            // BFS 洪水填充
            let mut stack = vec![(x, y)];
            let mut min_x = x;
            let mut min_y = y;
            let mut max_x = x;
            let mut max_y = y;
            let mut sum_prob = 0.0f32;
            let mut count = 0usize;

            visited[idx] = true;

            while let Some((cx, cy)) = stack.pop() {
                let cidx = cy * width + cx;
                sum_prob += data.get(cidx).copied().unwrap_or(0.0);
                count += 1;

                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);

                // 4-邻域
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                        let nidx = ny as usize * width + nx as usize;
                        if !visited[nidx] && data.get(nidx).unwrap_or(&0.0) >= &threshold {
                            visited[nidx] = true;
                            stack.push((nx as usize, ny as usize));
                        }
                    }
                }
            }

            if count >= min_area {
                let avg_prob = sum_prob / count as f32;
                regions.push((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1, avg_prob));
            }
        }
    }

    regions
}

/// 对文本区域执行非极大值抑制（NMS）
///
/// 移除与更高置信度区域 IoU 超过阈值的低置信度区域。
fn nms_regions(mut regions: Vec<TextRegion>, iou_threshold: f32) -> Vec<TextRegion> {
    // 按置信度降序排序
    regions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut keep = Vec::new();
    let mut suppressed = vec![false; regions.len()];

    for i in 0..regions.len() {
        if suppressed[i] {
            continue;
        }
        keep.push(regions[i].clone());

        for j in (i + 1)..regions.len() {
            if suppressed[j] {
                continue;
            }
            if region_iou(&regions[i], &regions[j]) > iou_threshold {
                suppressed[j] = true;
            }
        }
    }

    keep
}

/// 计算两个文本区域的 IoU（交并比）
fn region_iou(a: &TextRegion, b: &TextRegion) -> f32 {
    let a_x1 = a.x;
    let a_y1 = a.y;
    let a_x2 = a.x + a.width;
    let a_y2 = a.y + a.height;

    let b_x1 = b.x;
    let b_y1 = b.y;
    let b_x2 = b.x + b.width;
    let b_y2 = b.y + b.height;

    let inter_x1 = a_x1.max(b_x1);
    let inter_y1 = a_y1.max(b_y1);
    let inter_x2 = a_x2.min(b_x2);
    let inter_y2 = a_y2.min(b_y2);

    if inter_x2 <= inter_x1 || inter_y2 <= inter_y1 {
        return 0.0;
    }

    let inter_area = (inter_x2 - inter_x1) * (inter_y2 - inter_y1);
    let a_area = a.width * a.height;
    let b_area = b.width * b.height;
    let union_area = a_area + b_area - inter_area;

    if union_area <= 0.0 {
        0.0
    } else {
        inter_area / union_area
    }
}

// ============================================================================
// 区域裁剪与 Reading Order
// ============================================================================

/// 将图像旋转 180°
fn rotate_180(img: &DynamicImage) -> DynamicImage {
    img.rotate180()
}

/// 根据相对坐标从原图中裁剪区域
fn crop_region(img: &DynamicImage, region: &TextRegion) -> DynamicImage {
    let (width, height) = (img.width(), img.height());

    let x = (region.x * width as f32) as u32;
    let y = (region.y * height as f32) as u32;
    let w = (region.width * width as f32) as u32;
    let h = (region.height * height as f32) as u32;

    // 确保不越界
    let x = x.min(width - 1);
    let y = y.min(height - 1);
    let w = w.min(width - x);
    let h = h.min(height - y);

    // 裁剪并稍微放大（给 rec 模型留边距）
    let pad_x = (w as f32 * 0.05) as u32;
    let pad_y = (h as f32 * 0.05) as u32;

    let x0 = x.saturating_sub(pad_x);
    let y0 = y.saturating_sub(pad_y);
    let x1 = (x + w + pad_x).min(width);
    let y1 = (y + h + pad_y).min(height);

    img.crop_imm(x0, y0, x1 - x0, y1 - y0)
}

/// 按 reading order 排序（从上到下，从左到右）
///
/// 使用简单的行分组策略：
/// 1. 按 y 中心坐标排序
/// 2. 将 y 接近的框分到同一行
/// 3. 每行内按 x 排序
fn sort_by_reading_order(blocks: &mut [OcrBlock]) {
    if blocks.len() <= 1 {
        return;
    }

    // 简单的 reading order：先按 y 排序，y 接近的按 x 排序
    // 使用 y 中心坐标
    blocks.sort_by(|a, b| {
        let a_cy = a.bbox.y + a.bbox.height / 2.0;
        let b_cy = b.bbox.y + b.bbox.height / 2.0;

        // 如果 y 中心差距小于高度的一半，认为是同一行，按 x 排序
        let y_diff = (a_cy - b_cy).abs();
        let min_h = a.bbox.height.min(b.bbox.height);

        if y_diff < min_h * 0.5 {
            a.bbox.x.partial_cmp(&b.bbox.x).unwrap()
        } else {
            a_cy.partial_cmp(&b_cy).unwrap()
        }
    });
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_iou() {
        let a = TextRegion { x: 0.0, y: 0.0, width: 0.5, height: 0.5, confidence: 1.0 };
        let b = TextRegion { x: 0.25, y: 0.25, width: 0.5, height: 0.5, confidence: 1.0 };
        let iou = region_iou(&a, &b);
        // 交集面积 = 0.25 * 0.25 = 0.0625
        // 并集面积 = 0.25 + 0.25 - 0.0625 = 0.4375
        // IoU = 0.0625 / 0.4375 ≈ 0.143
        assert!(iou > 0.14 && iou < 0.15, "IoU should be ~0.143, got {}", iou);
    }

    #[test]
    fn test_nms_regions() {
        let regions = vec![
            TextRegion { x: 0.0, y: 0.0, width: 0.5, height: 0.5, confidence: 0.9 },
            TextRegion { x: 0.05, y: 0.05, width: 0.5, height: 0.5, confidence: 0.8 }, // 与第一个高度重叠 (IoU ~0.64)
            TextRegion { x: 0.6, y: 0.6, width: 0.3, height: 0.3, confidence: 0.7 }, // 不重叠
        ];
        let result = nms_regions(regions, 0.5);
        assert_eq!(result.len(), 2, "NMS should remove the overlapping low-confidence region");
    }

    #[test]
    fn test_sort_by_reading_order() {
        let mut blocks = vec![
            OcrBlock { text: "C".into(), confidence: 1.0, bbox: TextRegion { x: 0.5, y: 0.0, width: 0.1, height: 0.1, confidence: 1.0 } },
            OcrBlock { text: "A".into(), confidence: 1.0, bbox: TextRegion { x: 0.0, y: 0.0, width: 0.1, height: 0.1, confidence: 1.0 } },
            OcrBlock { text: "B".into(), confidence: 1.0, bbox: TextRegion { x: 0.2, y: 0.0, width: 0.1, height: 0.1, confidence: 1.0 } },
        ];
        sort_by_reading_order(&mut blocks);
        assert_eq!(blocks[0].text, "A");
        assert_eq!(blocks[1].text, "B");
        assert_eq!(blocks[2].text, "C");
    }
}
