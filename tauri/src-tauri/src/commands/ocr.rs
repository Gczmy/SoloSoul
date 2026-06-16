//! OCR commands — scan images/documents for text extraction

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub text: String,
    pub confidence: f64,
    pub fields: Vec<OcrField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrField {
    pub label: String,
    pub value: String,
    pub confidence: f64,
}

#[tauri::command]
pub async fn ocr_scan_image(
    file_path: String,
    language: Option<String>,
) -> Result<OcrResult, String> {
    ocr_scan_image_stub(file_path, language).await
}

/// Platform-specific OCR backend placeholder.
/// The public `ocr_scan_image` command delegates here so the stub nature is
/// explicit in the source; replace this implementation with the real backend.
async fn ocr_scan_image_stub(
    _file_path: String,
    _language: Option<String>,
) -> Result<OcrResult, String> {
    // Full OCR implementation requires platform-specific integration
    // (Apple Vision on macOS, Tesseract on Linux/Windows).
    Ok(OcrResult {
        text: String::new(),
        confidence: 0.0,
        fields: vec![],
    })
}

#[tauri::command]
pub async fn ocr_get_supported_languages() -> Result<Vec<String>, String> {
    Ok(vec![
        "en".to_string(),
        "zh-CN".to_string(),
        "ja".to_string(),
        "ko".to_string(),
    ])
}
