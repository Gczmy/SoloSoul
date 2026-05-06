//! ONNX 推理封装
//!
//! 提供 PP-OCRv4 rec 模型的单次/批量推理，输出 CTC 解码后的文本和置信度。

use image::DynamicImage;
use ndarray::Array;
use once_cell::sync::Lazy;

use super::error::OcrError;
use super::model::get_rec_session;
use super::preprocess::{prepare_rec_input, REC_MEAN, REC_STD};

/// PP-OCRv4 字典（编译时包含）
///
/// 字典文件来自 PaddleOCR 官方，共 6622 个字符。
/// CTC blank 索引为 0（不在字典中），字典索引从 0 开始对应模型输出的索引 1。
static PPOCR_DICT: Lazy<Vec<char>> = Lazy::new(|| {
    let dict_str = include_str!("dict.txt");
    dict_str.lines().map(|line| {
        // 字典每行一个字符，可能包含前导/尾随空白
        line.trim().chars().next().unwrap_or('?')
    }).collect()
});

/// 单行识别结果
#[derive(Debug, Clone)]
pub struct RecResult {
    pub text: String,
    pub confidence: f32,
}

/// 对单行灰度文字图进行识别
///
/// 流程：
/// 1. resize 到固定高度 48px
/// 2. 归一化（mean/std）
/// 3. 构造 NCHW tensor
/// 4. ONNX 推理
/// 5. CTC 解码
pub fn recognize_line(gray_line: &image::GrayImage) -> Result<RecResult, OcrError> {
    let mut session = get_rec_session()
        .map_err(|e| OcrError::InferenceFailed(format!("Session error: {e}")))?;

    // 准备 RGB 输入图
    let rgb = prepare_rec_input(gray_line);
    let (width, height) = rgb.dimensions();

    // 构造 NCHW tensor: [batch=1, channels=3, height=48, width=W]
    let mut input_data = Vec::with_capacity((3 * height * width) as usize);

    for c in 0..3 {
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x, y)[c] as f32;
                let normalized = (pixel / 255.0 - REC_MEAN[c]) / REC_STD[c];
                input_data.push(normalized);
            }
        }
    }

    let input_array = Array::from_shape_vec(
        (1, 3, height as usize, width as usize),
        input_data,
    )
    .map_err(|e| OcrError::InferenceFailed(format!("Tensor shape error: {e}")))?;

    // 创建 ONNX Value
    let input_value = ort::value::Tensor::from_array(input_array)
        .map_err(|e| OcrError::InferenceFailed(format!("Tensor creation failed: {e}")))?;

    // ONNX 推理
    let outputs = session
        .run(vec![("x", input_value.into_dyn())])
        .map_err(|e| OcrError::InferenceFailed(format!("ONNX run failed: {e}")))?;

    // 提取第一个输出
    let output = &outputs[0];

    // CTC 解码
    let (text, confidence) = ctc_decode(output);

    Ok(RecResult { text, confidence })
}

/// CTC 贪心解码
///
/// PP-OCRv4 rec 模型输出为 softmax 后的字符概率分布，
/// 每时间步取 argmax，然后去重去空白（blank=0）。
fn ctc_decode(output: &ort::value::DynValue) -> (String, f32) {
    // 提取 tensor 数据
    let (shape, data) = match output.try_extract_tensor::<f32>() {
        Ok(r) => r,
        Err(_) => return (String::new(), 0.0),
    };

    if shape.len() < 2 {
        return (String::new(), 0.0);
    }

    // 假设 shape 为 [T, C] 或 [T, 1, C]
    let time_steps = shape[0] as usize;
    let num_classes = (*shape.last().unwrap_or(&0)) as usize;

    if num_classes == 0 || data.is_empty() {
        return (String::new(), 0.0);
    }

    let mut decoded = String::new();
    let mut total_confidence = 0.0f32;
    let mut valid_steps = 0usize;
    let mut prev_label = 0usize;

    for t in 0..time_steps {
        // 找到该时间步概率最高的字符
        let mut max_prob = f32::MIN;
        let mut max_idx = 0usize;

        for c in 0..num_classes {
            let idx = if shape.len() == 3 {
                let mid_dim = shape[1] as usize;
                t * mid_dim * num_classes + c
            } else {
                t * num_classes + c
            };
            if idx < data.len() {
                let prob = data[idx];
                if prob > max_prob {
                    max_prob = prob;
                    max_idx = c;
                }
            }
        }

        // CTC 规则：跳过 blank（索引 0）和重复字符
        if max_idx != 0 && max_idx != prev_label {
            if let Some(ch) = ppocr_dict_char(max_idx) {
                decoded.push(ch);
            }
            total_confidence += max_prob.max(0.0).min(1.0);
            valid_steps += 1;
        }
        prev_label = max_idx;
    }

    let confidence = if valid_steps > 0 {
        (total_confidence / valid_steps as f32).min(1.0)
    } else {
        0.0
    };

    (decoded, confidence)
}

/// 根据模型输出索引查找对应字符
///
/// 注意：模型输出索引 0 为 CTC blank，字典索引从 0 开始对应模型输出索引 1。
fn ppocr_dict_char(idx: usize) -> Option<char> {
    if idx == 0 {
        None // CTC blank
    } else {
        PPOCR_DICT.get(idx - 1).copied()
    }
}

/// 批量识别多行文字
pub fn recognize_lines(lines: &[image::GrayImage]) -> Result<Vec<RecResult>, OcrError> {
    lines.iter().map(|line| recognize_line(line)).collect()
}

// ============================================================================
// 方向分类（cls 模型）
// ============================================================================

/// cls 模型输出：2 分类（0° / 180°）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOrientation {
    /// 无需旋转（0°）
    Normal,
    /// 需要旋转 180°
    Rotated180,
}

/// 使用 cls 模型判断文本行方向
///
/// 返回该文本行是否需要旋转 180°。
/// PP-OCR cls 模型输出 2 维概率：[0°概率, 180°概率]。
pub fn classify_orientation(img: &image::RgbImage) -> Result<TextOrientation, OcrError> {
    use super::model::get_cls_session;
    use super::preprocess::{prepare_cls_input, CLS_INPUT_HEIGHT, CLS_INPUT_WIDTH};
    use ndarray::Array;

    let mut session = get_cls_session()
        .map_err(|e| OcrError::InferenceFailed(format!("CLS session error: {e}")))?;

    let input_data = prepare_cls_input(&DynamicImage::ImageRgb8(img.clone()));

    let input_array = Array::from_shape_vec(
        (1, 3, CLS_INPUT_HEIGHT as usize, CLS_INPUT_WIDTH as usize),
        input_data,
    )
    .map_err(|e| OcrError::InferenceFailed(format!("CLS tensor shape error: {e}")))?;

    let input_value = ort::value::Tensor::from_array(input_array)
        .map_err(|e| OcrError::InferenceFailed(format!("CLS tensor creation failed: {e}")))?;

    let outputs = session
        .run(vec![("x", input_value.into_dyn())])
        .map_err(|e| OcrError::InferenceFailed(format!("CLS ONNX run failed: {e}")))?;

    let output = &outputs[0];
    let (shape, data) = output
        .try_extract_tensor::<f32>()
        .map_err(|e| OcrError::InferenceFailed(format!("CLS output extract failed: {e}")))?;

    if shape.len() != 2 || shape[1] != 2 {
        return Err(OcrError::InferenceFailed(
            format!("Unexpected CLS output shape: {:?}", shape)
        ));
    }

    let prob_0 = data[0];
    let prob_180 = data[1];

    Ok(if prob_180 > prob_0 {
        TextOrientation::Rotated180
    } else {
        TextOrientation::Normal
    })
}
