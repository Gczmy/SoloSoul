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
}

impl OcrModelTier {
    /// 返回该档位对应的目录名。
    pub fn dir_name(&self) -> &'static str {
        match self {
            OcrModelTier::Tiny => "pp-ocr-v6-tiny",
            OcrModelTier::Small => "pp-ocr-v6-small",
            OcrModelTier::Medium => "pp-ocr-v6-medium",
        }
    }

    /// 返回该档位在远程下载路径中使用的名称。
    pub fn remote_name(&self) -> &'static str {
        match self {
            OcrModelTier::Tiny => "tiny",
            OcrModelTier::Small => "small",
            OcrModelTier::Medium => "medium",
        }
    }
}

impl std::fmt::Display for OcrModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrModelTier::Tiny => write!(f, "tiny"),
            OcrModelTier::Small => write!(f, "small"),
            OcrModelTier::Medium => write!(f, "medium"),
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
            _ => Err(format!("Unknown OCR model tier: {s}")),
        }
    }
}
