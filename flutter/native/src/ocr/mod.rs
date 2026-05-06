//! SoloSoul OCR Module
//!
//! 本地优先的 OCR 引擎，基于 ONNX Runtime + PP-OCRv4 模型。
//! Phase 1: MRZ（机器可读区）识别
//! Phase 2: 通用 OCR（det + rec，全本地推理）

pub mod error;
pub mod general_pipeline;
pub mod inference;
pub mod model;
pub mod mrz_pipeline;
pub mod postprocess;
pub mod preprocess;

pub use error::OcrError;
pub use general_pipeline::{recognize_image, GeneralOcrResult, OcrBlock, TextRegion};
pub use model::{
    engine_status, load_models_from_memory, load_models_from_memory_v2, unload_models,
    OcrEngineStatus,
};
pub use mrz_pipeline::extract_mrz_lines;
