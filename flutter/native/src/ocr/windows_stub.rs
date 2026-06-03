//! Windows 平台 OCR Stub 实现
//!
//! ort 2.0.0-rc.12 在 Windows 上存在链接符号冲突（LNK4286/1120），
//! 暂时通过条件编译禁用 Windows 上的 ONNX Runtime 推理。
//! 所有 OCR 函数返回友好的错误信息，不会导致崩溃。

use image::DynamicImage;

use super::error::OcrError;

// =============================================================================
// 类型定义（与通用实现保持 API 兼容）
// =============================================================================

#[derive(Debug, Clone)]
pub struct OcrEngineStatus {
    pub is_loaded: bool,
    pub det_loaded: bool,
    pub cls_loaded: bool,
    pub rec_loaded: bool,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone)]
pub struct TextRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: TextRegion,
}

#[derive(Debug, Clone)]
pub struct GeneralOcrResult {
    pub raw_text: String,
    pub blocks: Vec<OcrBlock>,
    pub confidence: f32,
}

// =============================================================================
// Stub 函数实现
// =============================================================================

pub fn load_models_from_memory(_model_bytes: &[u8]) -> Result<(), OcrError> {
    Err(OcrError::InferenceFailed(
        "OCR is not supported on Windows in this build".to_string(),
    ))
}

pub fn load_models_from_memory_v2(
    _det_bytes: &[u8],
    _cls_bytes: &[u8],
    _rec_bytes: &[u8],
) -> Result<(), OcrError> {
    Err(OcrError::InferenceFailed(
        "OCR is not supported on Windows in this build".to_string(),
    ))
}

pub fn unload_models() {}

pub fn engine_status() -> OcrEngineStatus {
    OcrEngineStatus {
        is_loaded: false,
        det_loaded: false,
        cls_loaded: false,
        rec_loaded: false,
        uptime_secs: 0,
    }
}

pub fn extract_mrz_lines(_img: &DynamicImage) -> Result<Vec<String>, OcrError> {
    Err(OcrError::MrzNotFound {
        reason: "OCR is not supported on Windows in this build".to_string(),
    })
}

pub fn recognize_image(_img: &DynamicImage) -> Result<GeneralOcrResult, OcrError> {
    Err(OcrError::InferenceFailed(
        "OCR is not supported on Windows in this build".to_string(),
    ))
}
