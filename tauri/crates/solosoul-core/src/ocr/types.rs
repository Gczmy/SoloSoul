//! OCR 公共类型定义。
//!
//! 这些类型同时供 Tauri GUI 与 SoloSoul CLI 使用，保持序列化格式稳定。

use serde::{Deserialize, Serialize};

/// 单条文本检测结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrBox {
    pub text: String,
    pub confidence: f64,
    /// 文本框四个角点，顺序为左上、右上、右下、左下（原始图像坐标）。
    pub points: [(f32, f32); 4],
}

/// 单张图片的 OCR 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    /// 按阅读顺序拼接的完整文本。
    pub text: String,
    /// 整体平均置信度。
    pub confidence: f64,
    /// 检测到的文本块列表。
    pub boxes: Vec<OcrBox>,
}

/// OCR 模型档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrModelTier {
    /// 1.5M 参数级，速度最快，适合简单场景。
    Tiny,
    /// 默认档位，约 30MB，速度与精度平衡。
    #[default]
    Small,
    /// 高精度档位，约 132MB，适合复杂文档。
    Medium,
    /// P133：macOS 专用——Apple Vision Framework 原生 OCR（系统内置，无模型文件）。
    /// 仅 macOS 提供该档位（`ocr_list_available_tiers` 在非 macOS 不返回）；
    /// 序列化为 "vision"。
    Vision,
}

impl OcrModelTier {
    /// 返回该档位对应的目录名。
    pub fn dir_name(&self) -> &'static str {
        match self {
            OcrModelTier::Tiny => "pp-ocr-v6-tiny",
            OcrModelTier::Small => "pp-ocr-v6-small",
            OcrModelTier::Medium => "pp-ocr-v6-medium",
            // Vision 为系统内置引擎，无模型目录（此值不会被模型操作使用）。
            OcrModelTier::Vision => "macos-vision",
        }
    }

    /// 返回该档位在远程下载路径中使用的名称。
    pub fn remote_name(&self) -> &'static str {
        match self {
            OcrModelTier::Tiny => "tiny",
            OcrModelTier::Small => "small",
            OcrModelTier::Medium => "medium",
            // Vision 不参与远程下载。
            OcrModelTier::Vision => "vision",
        }
    }
}

impl std::fmt::Display for OcrModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrModelTier::Tiny => write!(f, "tiny"),
            OcrModelTier::Small => write!(f, "small"),
            OcrModelTier::Medium => write!(f, "medium"),
            OcrModelTier::Vision => write!(f, "vision"),
        }
    }
}

impl std::str::FromStr for OcrModelTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tiny" => Ok(OcrModelTier::Tiny),
            "small" => Ok(OcrModelTier::Small),
            "medium" => Ok(OcrModelTier::Medium),
            "vision" => Ok(OcrModelTier::Vision),
            _ => Err(format!("Unknown OCR model tier: {s}")),
        }
    }
}

/// MRZ（机读区）识别结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrzResult {
    pub document_type: String,
    pub document_type_sub: String,
    pub issuing_country: String,
    pub document_number: String,
    pub check_digit_document_number: char,
    pub nationality: String,
    pub date_of_birth: String,
    pub check_digit_date_of_birth: char,
    pub sex: String,
    pub expiry_date: String,
    pub check_digit_expiry: char,
    pub optional_data: String,
    pub composite_check_digit: String,
    pub raw_lines: Vec<String>,
    pub confidence: f64,
    pub checksum_valid: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_tier_parse_display_and_serde() {
        // P133: Vision 档位全链路往返（Display / FromStr / serde lowercase）。
        assert_eq!(OcrModelTier::Vision.to_string(), "vision");
        assert_eq!(
            "vision".parse::<OcrModelTier>().unwrap(),
            OcrModelTier::Vision
        );
        assert_eq!(
            serde_json::to_string(&OcrModelTier::Vision).unwrap(),
            "\"vision\""
        );
        assert_eq!(
            serde_json::from_str::<OcrModelTier>("\"vision\"").unwrap(),
            OcrModelTier::Vision
        );
        // 既有三档不受影响。
        assert_eq!("tiny".parse::<OcrModelTier>().unwrap(), OcrModelTier::Tiny);
    }
}
