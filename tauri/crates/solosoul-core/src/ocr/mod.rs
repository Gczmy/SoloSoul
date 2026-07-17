//! 本地 OCR 引擎（PP-OCRv6 ONNX）。
//!
//! 本模块提供不依赖 Tauri 的纯 Rust OCR 实现，供 GUI 与 CLI 复用。

//! types 和 model 子模块总是可用（移动端需要类型定义和文件操作）。
//! engine 及其依赖的 ONNX/mrz/pdf/图像处理仅在桌面端编译。
pub mod model;
pub mod types;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod engine;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod mrz;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod pdf;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod postprocess;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod preprocess;

#[cfg(target_os = "macos")]
pub mod macos_vision;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use engine::OcrEngine;

pub use model::{resolve_model_bundle, OcrModelBundle};
pub use types::{MrzResult, OcrBox, OcrModelTier, OcrResult};
