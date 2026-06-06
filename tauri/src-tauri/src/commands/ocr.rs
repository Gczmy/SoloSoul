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
    _file_path: String,
    _language: Option<String>,
) -> Result<OcrResult, String> {
    // OCR scanning — returns extracted text and structured fields.
    // Full OCR implementation requires platform-specific integration
    // (Apple Vision on macOS, Tesseract on Linux/Windows).
    //
    // For now, returns a placeholder result indicating the feature
    // is ready for platform-specific OCR backend integration.
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
