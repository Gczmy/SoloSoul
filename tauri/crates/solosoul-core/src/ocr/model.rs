//! OCR 模型路径解析与配置加载。

use super::types::OcrModelTier;
use std::path::{Path, PathBuf};

/// 单个档位的模型文件集合。
#[derive(Debug, Clone)]
pub struct OcrModelBundle {
    pub tier: OcrModelTier,
    pub det_model: PathBuf,
    pub det_config: PathBuf,
    pub rec_model: PathBuf,
    pub rec_config: PathBuf,
}

impl OcrModelBundle {
    /// 验证所有必需文件均存在。
    pub fn validate(&self) -> Result<(), String> {
        for (label, path) in [
            ("det model", &self.det_model),
            ("det config", &self.det_config),
            ("rec model", &self.rec_model),
            ("rec config", &self.rec_config),
        ] {
            if !path.exists() {
                return Err(format!("Missing {label}: {}", path.display()));
            }
        }
        Ok(())
    }
}

/// 检查指定档位的模型是否已安装（所有必需文件均存在）。
pub fn is_model_installed(models_dir: &Path, tier: OcrModelTier) -> bool {
    resolve_model_bundle(models_dir, tier).is_ok()
}

/// 将打包资源中的模型复制到应用数据目录。
///
/// 若目标目录已存在完整模型，则直接返回成功。
pub fn install_model_from_bundled(
    bundled_dir: &Path,
    models_dir: &Path,
    tier: OcrModelTier,
) -> Result<(), String> {
    if is_model_installed(models_dir, tier) {
        return Ok(());
    }

    let src = resolve_model_bundle(bundled_dir, tier)?;
    let dst_base = models_dir.join(tier.dir_name());
    let dst_det_dir = dst_base.join("det");
    let dst_rec_dir = dst_base.join("rec");

    std::fs::create_dir_all(&dst_det_dir).map_err(|e| format!("创建 det 目录失败: {e}"))?;
    std::fs::create_dir_all(&dst_rec_dir).map_err(|e| format!("创建 rec 目录失败: {e}"))?;

    std::fs::copy(&src.det_model, dst_det_dir.join("inference.onnx"))
        .map_err(|e| format!("复制 det 模型失败: {e}"))?;
    std::fs::copy(&src.det_config, dst_det_dir.join("inference.yml"))
        .map_err(|e| format!("复制 det 配置失败: {e}"))?;
    std::fs::copy(&src.rec_model, dst_rec_dir.join("inference.onnx"))
        .map_err(|e| format!("复制 rec 模型失败: {e}"))?;
    std::fs::copy(&src.rec_config, dst_rec_dir.join("inference.yml"))
        .map_err(|e| format!("复制 rec 配置失败: {e}"))?;

    Ok(())
}

/// 在 `models_dir` 下解析指定档位的模型文件路径。
pub fn resolve_model_bundle(
    models_dir: &Path,
    tier: OcrModelTier,
) -> Result<OcrModelBundle, String> {
    let base = models_dir.join(tier.dir_name());
    let det_dir = base.join("det");
    let rec_dir = base.join("rec");

    let bundle = OcrModelBundle {
        tier,
        det_model: det_dir.join("inference.onnx"),
        det_config: det_dir.join("inference.yml"),
        rec_model: rec_dir.join("inference.onnx"),
        rec_config: rec_dir.join("inference.yml"),
    };
    bundle.validate()?;
    Ok(bundle)
}

/// 从识别模型配置中读取字符字典。
///
/// PP-OCRv6 的 `inference.yml` 将字典内嵌在 `PostProcess.character_dict` 列表中。
pub fn load_recognition_dict(config_path: &Path) -> Result<Vec<String>, String> {
    let content =
        std::fs::read_to_string(config_path).map_err(|e| format!("Read rec config: {e}"))?;

    // YAML 解析会引入额外依赖；此处用最小化解析：按行扫描 key。
    // 字典列表位于 `PostProcess:` -> `character_dict:` 之后，以 `- ` 开头。
    let mut in_postprocess = false;
    let mut in_dict = false;
    let mut dict = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "PostProcess:" {
            in_postprocess = true;
            continue;
        }
        if !in_postprocess {
            continue;
        }
        if trimmed.starts_with("character_dict:") {
            in_dict = true;
            continue;
        }
        if in_dict {
            if let Some(val) = trimmed.strip_prefix("- ") {
                // YAML 引号处理
                let val = val.trim().trim_matches('\'').trim_matches('"');
                dict.push(val.to_string());
            } else if trimmed.ends_with(':') || trimmed.is_empty() {
                // 到达下一个 section 或空行
                break;
            }
        }
    }

    if dict.is_empty() {
        return Err(format!(
            "No character dictionary found in {}",
            config_path.display()
        ));
    }
    Ok(dict)
}

/// 从检测模型配置中读取后处理参数。
#[derive(Debug, Clone)]
pub struct DetPostProcessConfig {
    pub thresh: f32,
    pub box_thresh: f32,
    pub unclip_ratio: f32,
    pub max_candidates: usize,
}

impl Default for DetPostProcessConfig {
    fn default() -> Self {
        Self {
            thresh: 0.2,
            box_thresh: 0.45,
            unclip_ratio: 1.4,
            max_candidates: 3000,
        }
    }
}

pub fn load_det_postprocess_config(config_path: &Path) -> Result<DetPostProcessConfig, String> {
    let content =
        std::fs::read_to_string(config_path).map_err(|e| format!("Read det config: {e}"))?;

    let mut cfg = DetPostProcessConfig::default();
    let mut in_postprocess = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "PostProcess:" {
            in_postprocess = true;
            continue;
        }
        if !in_postprocess {
            continue;
        }
        if trimmed.starts_with("Hpi:") {
            // 下一个 section
            break;
        }
        if trimmed.starts_with("name:") {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once(':') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "thresh" => cfg.thresh = val.parse().map_err(|e| format!("parse thresh: {e}"))?,
                "box_thresh" => {
                    cfg.box_thresh = val.parse().map_err(|e| format!("parse box_thresh: {e}"))?
                }
                "unclip_ratio" => {
                    cfg.unclip_ratio = val
                        .parse()
                        .map_err(|e| format!("parse unclip_ratio: {e}"))?
                }
                "max_candidates" => {
                    cfg.max_candidates = val
                        .parse()
                        .map_err(|e| format!("parse max_candidates: {e}"))?
                }
                _ => {}
            }
        }
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_bundle_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let result = resolve_model_bundle(tmp.path(), OcrModelTier::Small);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Missing"), "got: {err}");
    }

    #[test]
    fn test_load_recognition_dict_minimal() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg_path = tmp.path().join("inference.yml");
        std::fs::write(
            &cfg_path,
            "PostProcess:\n  name: CTCLabelDecode\n  character_dict:\n  - 'a'\n  - 'b'\n  - '1'\n",
        )
        .unwrap();
        let dict = load_recognition_dict(&cfg_path).unwrap();
        assert_eq!(dict, vec!["a", "b", "1"]);
    }

    #[test]
    fn test_is_model_installed() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_model_installed(tmp.path(), OcrModelTier::Small));

        let base = tmp.path().join("pp-ocr-v6-small");
        let det_dir = base.join("det");
        let rec_dir = base.join("rec");
        std::fs::create_dir_all(&det_dir).unwrap();
        std::fs::create_dir_all(&rec_dir).unwrap();
        std::fs::write(det_dir.join("inference.onnx"), b"dummy").unwrap();
        std::fs::write(det_dir.join("inference.yml"), b"dummy").unwrap();
        std::fs::write(rec_dir.join("inference.onnx"), b"dummy").unwrap();
        std::fs::write(
            rec_dir.join("inference.yml"),
            b"PostProcess:\n  character_dict:\n  - 'a'\n",
        )
        .unwrap();

        assert!(is_model_installed(tmp.path(), OcrModelTier::Small));
        assert!(!is_model_installed(tmp.path(), OcrModelTier::Tiny));
    }

    #[test]
    fn test_install_model_from_bundled() {
        let bundled = tempfile::tempdir().unwrap();
        let models = tempfile::tempdir().unwrap();

        let base = bundled.path().join("pp-ocr-v6-small");
        let det_dir = base.join("det");
        let rec_dir = base.join("rec");
        std::fs::create_dir_all(&det_dir).unwrap();
        std::fs::create_dir_all(&rec_dir).unwrap();
        std::fs::write(det_dir.join("inference.onnx"), b"det_model").unwrap();
        std::fs::write(det_dir.join("inference.yml"), b"det_cfg").unwrap();
        std::fs::write(rec_dir.join("inference.onnx"), b"rec_model").unwrap();
        std::fs::write(
            rec_dir.join("inference.yml"),
            b"PostProcess:\n  character_dict:\n  - 'a'\n",
        )
        .unwrap();

        install_model_from_bundled(bundled.path(), models.path(), OcrModelTier::Small).unwrap();

        assert!(models
            .path()
            .join("pp-ocr-v6-small/det/inference.onnx")
            .exists());
        assert_eq!(
            std::fs::read_to_string(models.path().join("pp-ocr-v6-small/det/inference.onnx"))
                .unwrap(),
            "det_model"
        );

        // 再次安装应幂等。
        install_model_from_bundled(bundled.path(), models.path(), OcrModelTier::Small).unwrap();
    }

    #[test]
    fn test_load_det_postprocess_config() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg_path = tmp.path().join("inference.yml");
        std::fs::write(
            &cfg_path,
            "PostProcess:\n  name: DBPostProcess\n  thresh: 0.25\n  box_thresh: 0.50\n  unclip_ratio: 1.5\n  max_candidates: 100\n",
        )
        .unwrap();
        let cfg = load_det_postprocess_config(&cfg_path).unwrap();
        assert!((cfg.thresh - 0.25).abs() < f32::EPSILON);
        assert!((cfg.box_thresh - 0.50).abs() < f32::EPSILON);
        assert!((cfg.unclip_ratio - 1.5).abs() < f32::EPSILON);
        assert_eq!(cfg.max_candidates, 100);
    }
}
