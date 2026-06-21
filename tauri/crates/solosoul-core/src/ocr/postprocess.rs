//! OCR 后处理：DBNet 文本框提取与 CTC 解码。

use super::model::DetPostProcessConfig;
use super::types::OcrBox;
use ndarray::ArrayView4;

/// 从检测模型输出中提取文本框。
///
/// 输入 `scores` 形状为 `[1, 1, H, W]`，输出为按阅读顺序排序的 4 角点。
pub fn extract_text_boxes(
    scores: &ArrayView4<f32>,
    scale: f32,
    original_size: (u32, u32),
    cfg: &DetPostProcessConfig,
) -> Vec<[(f32, f32); 4]> {
    let seg = binary_segmentation(scores, cfg.thresh);
    let contours = find_contours(&seg);

    let mut boxes = Vec::new();
    for contour in contours.iter().take(cfg.max_candidates) {
        let area = contour_area(contour);
        if area < 10.0 {
            continue;
        }

        let contour_f: Vec<(f32, f32)> =
            contour.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
        // 先使用轴对齐包围盒保证稳定；后续可升级为最小旋转矩形。
        let rect = axis_aligned_bounding_box(&contour_f);
        let mut pts = order_points(rect);

        // unclip：以中心为基准按 unclip_ratio 缩放
        let center = centroid(&pts);
        for p in &mut pts {
            p.0 = center.0 + (p.0 - center.0) * cfg.unclip_ratio;
            p.1 = center.1 + (p.1 - center.1) * cfg.unclip_ratio;
        }

        // 映射回原图坐标并裁剪
        let (orig_h, orig_w) = original_size;
        for p in &mut pts {
            p.0 = (p.0 / scale).clamp(0.0, orig_w as f32);
            p.1 = (p.1 / scale).clamp(0.0, orig_h as f32);
        }

        // 增加少量上下 padding，避免文字被截断导致识别失败。
        let pad_y = 8.0;
        pts[0].1 -= pad_y;
        pts[1].1 -= pad_y;
        pts[2].1 += pad_y;
        pts[3].1 += pad_y;
        for p in &mut pts {
            p.0 = p.0.clamp(0.0, orig_w as f32);
            p.1 = p.1.clamp(0.0, orig_h as f32);
        }

        // 过滤过小的框
        let (cw, ch) = box_size(&pts);
        if cw < 3.0 || ch < 3.0 {
            continue;
        }

        boxes.push(pts);
    }

    // 按从上到下、从左到右排序
    boxes.sort_by(|a, b| {
        let ay = a.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let by = b.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        if (ay - by).abs() < 10.0 {
            let ax = a.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
            let bx = b.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
            ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    boxes
}

/// 对概率图做阈值分割。
#[allow(clippy::needless_range_loop)]
fn binary_segmentation(scores: &ArrayView4<f32>, thresh: f32) -> Vec<Vec<u8>> {
    let (_, _, h, w) = scores.dim();
    let mut seg = vec![vec![0u8; w]; h];
    for y in 0..h {
        for x in 0..w {
            if scores[[0, 0, y, x]] > thresh {
                seg[y][x] = 1;
            }
        }
    }
    seg
}

/// 4 邻域轮廓查找（简化实现，基于 Moore-Neighbor 跟踪）。
fn find_contours(seg: &[Vec<u8>]) -> Vec<Vec<(i32, i32)>> {
    let h = seg.len();
    if h == 0 {
        return Vec::new();
    }
    let w = seg[0].len();
    let mut visited = vec![vec![false; w]; h];
    let mut contours = Vec::new();

    for y in 0..h {
        for x in 0..w {
            if seg[y][x] == 1 && !visited[y][x] {
                if let Some(c) = trace_contour(seg, x as i32, y as i32, &mut visited) {
                    contours.push(c);
                }
            }
        }
    }

    contours
}

fn trace_contour(
    seg: &[Vec<u8>],
    start_x: i32,
    start_y: i32,
    visited: &mut [Vec<bool>],
) -> Option<Vec<(i32, i32)>> {
    let h = seg.len() as i32;
    let w = seg[0].len() as i32;
    let dirs4 = [(1, 0), (0, 1), (-1, 0), (0, -1)];

    let mut contour = Vec::new();

    // BFS 收集边界点：值为 1 且至少有一个 4-邻域值为 0 的点
    let mut stack = vec![(start_x, start_y)];
    visited[start_y as usize][start_x as usize] = true;

    while let Some((cx, cy)) = stack.pop() {
        let mut is_boundary = false;
        for (dx, dy) in dirs4 {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || nx >= w || ny < 0 || ny >= h {
                is_boundary = true;
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            if seg[uy][ux] == 0 {
                is_boundary = true;
            } else if !visited[uy][ux] {
                visited[uy][ux] = true;
                stack.push((nx, ny));
            }
        }
        if is_boundary {
            contour.push((cx, cy));
        }
    }

    if contour.len() < 4 {
        None
    } else {
        Some(contour)
    }
}

fn contour_area(contour: &[(i32, i32)]) -> f32 {
    if contour.len() < 3 {
        return 0.0;
    }
    // BFS 采集的边界点不是顺时针/逆时针顺序，shoelace 公式会严重低估面积。
    // 直接使用轴对齐包围盒面积作为过滤依据，足够剔除细小噪声。
    let min_x = contour.iter().map(|p| p.0).min().unwrap_or(0);
    let max_x = contour.iter().map(|p| p.0).max().unwrap_or(0);
    let min_y = contour.iter().map(|p| p.1).min().unwrap_or(0);
    let max_y = contour.iter().map(|p| p.1).max().unwrap_or(0);
    ((max_x - min_x) as f32 + 1.0) * ((max_y - min_y) as f32 + 1.0)
}

/// 轴对齐包围盒（左上、右上、右下、左下）。
fn axis_aligned_bounding_box(points: &[(f32, f32)]) -> [(f32, f32); 4] {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    [
        (min_x, min_y),
        (max_x, min_y),
        (max_x, max_y),
        (min_x, max_y),
    ]
}

fn centroid(points: &[(f32, f32)]) -> (f32, f32) {
    let (sum_x, sum_y) = points
        .iter()
        .fold((0.0, 0.0), |acc, p| (acc.0 + p.0, acc.1 + p.1));
    (sum_x / points.len() as f32, sum_y / points.len() as f32)
}

/// 将矩形 4 点按左上、右上、右下、左下排序。
fn order_points(rect: [(f32, f32); 4]) -> [(f32, f32); 4] {
    let mut pts = rect.to_vec();
    pts.sort_by(|a, b| {
        (a.1 as i32)
            .cmp(&(b.1 as i32))
            .then((a.0 as i32).cmp(&(b.0 as i32)))
    });

    // 处理 y 相同导致排序不稳定的情况：分别取 x 最小/最大。
    let (top_left, top_right) = if (pts[0].1 - pts[1].1).abs() < f32::EPSILON {
        if pts[0].0 <= pts[1].0 {
            (pts[0], pts[1])
        } else {
            (pts[1], pts[0])
        }
    } else if pts[0].1 < pts[1].1 {
        (pts[0], pts[1])
    } else {
        (pts[1], pts[0])
    };

    let (bottom_left, bottom_right) = if (pts[2].1 - pts[3].1).abs() < f32::EPSILON {
        if pts[2].0 <= pts[3].0 {
            (pts[2], pts[3])
        } else {
            (pts[3], pts[2])
        }
    } else if pts[2].1 > pts[3].1 {
        (pts[3], pts[2])
    } else {
        (pts[2], pts[3])
    };

    [top_left, top_right, bottom_right, bottom_left]
}

fn box_size(points: &[(f32, f32); 4]) -> (f32, f32) {
    let w = ((points[0].0 - points[1].0).powi(2) + (points[0].1 - points[1].1).powi(2)).sqrt();
    let h = ((points[0].0 - points[3].0).powi(2) + (points[0].1 - points[3].1).powi(2)).sqrt();
    (w, h)
}

/// CTC 解码识别模型输出。
///
/// `pred` 形状为 `[T, C]`。索引 0 为 blank，最后一个索引通常忽略。
/// 返回 (text, 平均置信度)。
pub fn ctc_decode(pred: &ndarray::ArrayView2<f32>, char_list: &[String]) -> (String, f64) {
    let (text, _, avg) = ctc_decode_detailed(pred, char_list);
    (text, avg)
}

/// CTC 解码，返回每个字符的置信度。
///
/// 返回 (decoded_text, per_char_confidences, average_confidence)。
pub fn ctc_decode_detailed(
    pred: &ndarray::ArrayView2<f32>,
    char_list: &[String],
) -> (String, Vec<f64>, f64) {
    let idxs: Vec<usize> = pred
        .rows()
        .into_iter()
        .map(|row| {
            let mut max_idx = 0;
            let mut max_val = row[0];
            for (i, &v) in row.iter().enumerate().skip(1) {
                if v > max_val {
                    max_val = v;
                    max_idx = i;
                }
            }
            max_idx
        })
        .collect();

    let mut text = String::new();
    let mut char_confs = Vec::new();
    let mut prev = 0usize;

    for (t, &idx) in idxs.iter().enumerate() {
        if idx != prev && idx != 0 && idx <= char_list.len() {
            text.push_str(&char_list[idx - 1]);
            char_confs.push(pred[[t, idx]] as f64);
        }
        prev = idx;
    }

    let avg = if char_confs.is_empty() {
        0.0
    } else {
        char_confs.iter().sum::<f64>() / char_confs.len() as f64
    };
    (text, char_confs, avg)
}

/// 置信度阈值过滤：低于 `threshold` 的字符替换为 `?`。
pub fn filter_low_confidence_chars(text: &str, confidences: &[f64], threshold: f64) -> String {
    text.chars()
        .zip(confidences.iter().copied())
        .map(|(c, conf)| if conf < threshold { '?' } else { c })
        .collect()
}

/// OCR-B 字符集校正（MRZ 上下文）。
///
/// 修正规则：
/// - O → 0（O 不是有效 MRZ 字符，仅用 0）
/// - l → 1（小写 l 在 MRZ 中视为 1）
/// - 连续 3+ 个 C 或 E → 对应量的 <（PP-OCRv6 常将 MRZ 填充符 `<` 识别为 C/E）
///
/// 注意：I、Z、S、B 等字母都是有效的 MRZ 字符（姓名、证件号），
/// 不做全局替换。校验位级别的修正由 verify_checksums_lenient 处理。
pub fn correct_ocr_b_mrz(text: &str) -> String {
    // 第一遍：字符级修正
    let corrected: String = text
        .chars()
        .map(|c| match c {
            'O' => '0',
            'l' => '1',
            _ => c,
        })
        .collect();

    // 第二遍：连续 3+ 个 C/E（混合也可） → <（MRZ 填充符误识）
    // PP-OCRv6 常将 `<`（左尖括号）误识别为 C 或 E。
    // 在有效 MRZ 文本中，连续 3+ 个 C/E 不会出现
    // （CHN 只有 1 个 C，MCCARTHY 有 2 个 C），所以整个 C/E 块都可安全转换。
    let chars: Vec<char> = corrected.chars().collect();
    let mut result = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == 'C' || chars[i] == 'E' {
            // 找连续 C/E 块（不要求相同字符，C 和 E 可以交替出现）
            let block_start = i;
            while i < chars.len() && (chars[i] == 'C' || chars[i] == 'E') {
                i += 1;
            }
            let block_len = i - block_start;
            if block_len >= 3 {
                // 连续 3+ 个 C/E → 替换为同等数量的 <
                for _ in 0..block_len {
                    result.push('<');
                }
            } else {
                // 单个或两个 C/E：保留原样（可能是有效 MRZ 内容）
                result.extend(chars[block_start..i].iter());
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// 增强型 CTC 解码：解码 + 置信度过滤 + OCR-B 校正。
pub fn ctc_decode_enhanced(
    pred: &ndarray::ArrayView2<f32>,
    char_list: &[String],
    confidence_threshold: f64,
) -> (String, f64) {
    let (mut text, char_confs, avg) = ctc_decode_detailed(pred, char_list);
    text = filter_low_confidence_chars(&text, &char_confs, confidence_threshold);
    text = correct_ocr_b_mrz(&text);
    (text, avg)
}

/// 构建最终的 `OcrResult`。
pub fn build_ocr_result(
    boxes: Vec<[(f32, f32); 4]>,
    texts: Vec<String>,
    confidences: Vec<f64>,
) -> super::types::OcrResult {
    let mut ocr_boxes = Vec::with_capacity(boxes.len());
    let mut full_text_parts = Vec::with_capacity(texts.len());
    let mut total_conf = 0.0;

    for ((pts, text), conf) in boxes.into_iter().zip(texts).zip(confidences) {
        if !text.is_empty() {
            full_text_parts.push(text.clone());
        }
        total_conf += conf;
        ocr_boxes.push(OcrBox {
            text,
            confidence: conf,
            points: pts,
        });
    }

    let text = full_text_parts.join("\n");
    let confidence = if ocr_boxes.is_empty() {
        0.0
    } else {
        total_conf / ocr_boxes.len() as f64
    };

    super::types::OcrResult {
        text,
        confidence,
        boxes: ocr_boxes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array2;

    #[test]
    fn test_ctc_decode_basic() {
        // char_list: ["a", "b", "c"], blank=0
        let char_list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        // 时间步：blank, a, a, blank, b, b, c, blank
        let mut pred = Array2::<f32>::zeros([8, 4]);
        pred[[1, 1]] = 1.0;
        pred[[2, 1]] = 1.0;
        pred[[4, 2]] = 1.0;
        pred[[5, 2]] = 1.0;
        pred[[6, 3]] = 1.0;

        let (text, conf) = ctc_decode(&pred.view(), &char_list);
        assert_eq!(text, "abc");
        assert_eq!(conf, 1.0);
    }

    #[test]
    fn test_build_ocr_result() {
        let boxes = vec![[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]];
        let texts = vec!["hello".to_string()];
        let confs = vec![0.95];
        let result = build_ocr_result(boxes, texts, confs);
        assert_eq!(result.text, "hello");
        assert_eq!(result.boxes.len(), 1);
        assert!((result.confidence - 0.95).abs() < 1e-9);
    }

    #[test]
    fn test_extract_text_boxes_basic() {
        // 构造一个 10x10 的概率图，中间 6x6 区域高于阈值。
        let mut data = vec![0.0f32; 10 * 10];
        for y in 2..8 {
            for x in 2..8 {
                data[y * 10 + x] = 0.9;
            }
        }
        let scores = ndarray::Array4::from_shape_vec([1, 1, 10, 10], data).unwrap();
        let cfg = DetPostProcessConfig {
            thresh: 0.2,
            box_thresh: 0.45,
            unclip_ratio: 1.0,
            max_candidates: 3000,
        };
        let boxes = extract_text_boxes(&scores.view(), 1.0, (10, 10), &cfg);
        assert_eq!(
            boxes.len(),
            1,
            "expected one text box from a single high-probability region"
        );

        let pts = boxes[0];
        let min_x = pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = pts.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = pts.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        assert!(min_x >= 0.0 && max_x <= 10.0);
        assert!(min_y >= 0.0 && max_y <= 10.0);
        assert!(max_x - min_x >= 3.0);
        assert!(max_y - min_y >= 3.0);
    }

    #[test]
    fn test_extract_text_boxes_empty() {
        let scores = ndarray::Array4::zeros([1, 1, 4, 4]);
        let cfg = DetPostProcessConfig::default();
        let boxes = extract_text_boxes(&scores.view(), 1.0, (4, 4), &cfg);
        assert!(boxes.is_empty());
    }

    #[test]
    fn test_correct_ocr_b_mrz_ce_block() {
        // BRP 行: 连续 14 个 C/E → 全部转为 <
        assert_eq!(
            correct_ocr_b_mrz("IRGBRRU01146795CCCCCCCCCCCCCE"),
            "IRGBRRU01146795<<<<<<<<<<<<<<"
        );
        // 混合 CECE 块 (5个) → 全部转为 <
        assert_eq!(correct_ocr_b_mrz("CECEC"), "<<<<<");
        // 单个 C 保持不变（有效 MRZ 字符）
        assert_eq!(correct_ocr_b_mrz("CHN"), "CHN");
        // 双 C 保持不变（可能出现在姓名中）
        assert_eq!(correct_ocr_b_mrz("MCCARTHY"), "MCCARTHY");
        // 三个 E 连续 → 转为 <
        assert_eq!(correct_ocr_b_mrz("EEENAME"), "<<<NAME");
        // 空输入
        assert_eq!(correct_ocr_b_mrz(""), "");
        // O→0, l→1 仍然生效
        assert_eq!(correct_ocr_b_mrz("Ol"), "01");
    }
}
