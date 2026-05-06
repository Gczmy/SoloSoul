//! OCR 错误类型定义

use std::fmt;

/// OCR 模块错误枚举
#[derive(Debug, Clone)]
pub enum OcrError {
    /// ONNX 模型未加载或加载失败
    ModelNotLoaded,
    /// 输入图像格式无效或解码失败
    InvalidImage(String),
    /// ONNX 推理失败
    InferenceFailed(String),
    /// 未找到 MRZ 区域
    MrzNotFound { reason: String },
    /// MRZ 识别置信度过低
    MrzLowConfidence { line: String, confidence: f32 },
    /// 图像预处理失败
    PreprocessFailed(String),
    /// 识别超时
    Timeout(u64),
    /// 未检测到文本区域（通用 OCR）
    TextNotDetected,
    /// 方向分类失败
    OrientationClassificationFailed(String),
}

impl fmt::Display for OcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OcrError::ModelNotLoaded => write!(f, "OCR model not loaded"),
            OcrError::InvalidImage(msg) => write!(f, "Invalid image: {}", msg),
            OcrError::InferenceFailed(msg) => write!(f, "Inference failed: {}", msg),
            OcrError::MrzNotFound { reason } => write!(f, "MRZ not found: {}", reason),
            OcrError::MrzLowConfidence { line, confidence } => {
                write!(f, "Low confidence MRZ line: '{}' ({})", line, confidence)
            }
            OcrError::PreprocessFailed(msg) => write!(f, "Preprocess failed: {}", msg),
            OcrError::Timeout(secs) => write!(f, "OCR timeout after {} seconds", secs),
            OcrError::TextNotDetected => write!(f, "No text detected in image"),
            OcrError::OrientationClassificationFailed(msg) => {
                write!(f, "Orientation classification failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for OcrError {}

impl From<image::ImageError> for OcrError {
    fn from(err: image::ImageError) -> Self {
        OcrError::InvalidImage(err.to_string())
    }
}
