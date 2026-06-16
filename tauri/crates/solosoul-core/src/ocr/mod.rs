//! 本地 OCR 引擎（PP-OCRv6 ONNX）。
//!
//! 本模块提供不依赖 Tauri 的纯 Rust OCR 实现，供 GUI 与 CLI 复用。

pub mod engine;
pub mod model;
pub mod postprocess;
pub mod preprocess;
pub mod types;

pub use engine::OcrEngine;
pub use model::{resolve_model_bundle, OcrModelBundle};
pub use types::{OcrBox, OcrModelTier, OcrResult};
