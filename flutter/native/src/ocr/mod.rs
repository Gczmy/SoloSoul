//! SoloSoul OCR Module
//!
//! 本地优先的 OCR 引擎，基于 ONNX Runtime + PP-OCRv4 模型。
//! Phase 1: MRZ（机器可读区）识别
//! Phase 2: 通用 OCR（det + rec，全本地推理）
//!
//! ⚠️ Windows 平台: ort 2.0.0-rc.12 存在链接符号冲突，暂时通过条件编译
//!    禁用 ONNX Runtime 推理。所有 OCR 函数返回友好错误提示。

pub mod error;
pub mod postprocess;
pub mod preprocess;

#[cfg(not(target_os = "windows"))]
pub mod general_pipeline;
#[cfg(not(target_os = "windows"))]
pub mod inference;
#[cfg(not(target_os = "windows"))]
pub mod model;
#[cfg(not(target_os = "windows"))]
pub mod mrz_pipeline;

#[cfg(target_os = "windows")]
mod windows_stub;

pub use error::OcrError;

#[cfg(not(target_os = "windows"))]
pub use general_pipeline::{recognize_image, GeneralOcrResult, OcrBlock, TextRegion};
#[cfg(not(target_os = "windows"))]
pub use model::{
    engine_status, load_models_from_memory, load_models_from_memory_v2, unload_models,
    OcrEngineStatus,
};
#[cfg(not(target_os = "windows"))]
pub use mrz_pipeline::extract_mrz_lines;

#[cfg(target_os = "windows")]
pub use windows_stub::{
    engine_status, extract_mrz_lines, load_models_from_memory, load_models_from_memory_v2,
    recognize_image, unload_models, GeneralOcrResult, OcrBlock, OcrEngineStatus, TextRegion,
};
