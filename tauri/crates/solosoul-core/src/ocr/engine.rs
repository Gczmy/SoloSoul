//! OCR 引擎：加载检测/识别模型并执行端到端扫描。

use super::model::{
    load_det_postprocess_config, load_recognition_dict, resolve_model_bundle, OcrModelBundle,
};
use super::postprocess::{build_ocr_result, ctc_decode, extract_text_boxes};
use super::preprocess::{
    load_rgb_image, perspective_crop, preprocess_for_detection, preprocess_for_recognition,
};
use super::types::{MrzResult, OcrModelTier, OcrResult};
use ort::session::Session;
use std::path::Path;

/// 本地 OCR 引擎。
pub struct OcrEngine {
    det_session: Session,
    rec_session: Session,
    char_list: Vec<String>,
    bundle: OcrModelBundle,
}

impl OcrEngine {
    /// 从模型目录加载指定档位的 OCR 引擎。
    pub fn load(models_dir: &Path, tier: OcrModelTier) -> Result<Self, String> {
        let bundle = resolve_model_bundle(models_dir, tier)?;

        let det_session = Session::builder()
            .map_err(|e| format!("det session builder: {e}"))?
            .commit_from_file(&bundle.det_model)
            .map_err(|e| format!("load det model: {e}"))?;

        let rec_session = Session::builder()
            .map_err(|e| format!("rec session builder: {e}"))?
            .commit_from_file(&bundle.rec_model)
            .map_err(|e| format!("load rec model: {e}"))?;

        let char_list = load_recognition_dict(&bundle.rec_config)?;

        Ok(Self {
            det_session,
            rec_session,
            char_list,
            bundle,
        })
    }

    /// 扫描单张图片并返回所有识别到的文本块。
    pub fn scan_image(&mut self, image_path: &Path) -> Result<OcrResult, String> {
        let img = load_rgb_image(image_path)?;
        self.scan_rgb(&img)
    }

    /// 对已加载的 RGB 图像执行 OCR。
    pub fn scan_rgb(&mut self, img: &image::RgbImage) -> Result<OcrResult, String> {
        let det_cfg = load_det_postprocess_config(&self.bundle.det_config)?;

        // 1. 检测
        let det_input = preprocess_for_detection(img);
        let det_tensor = ndarray_to_ort_tensor(&det_input.tensor.view())?;
        let det_outputs = self
            .det_session
            .run(ort::inputs!("x" => det_tensor))
            .map_err(|e| format!("det inference: {e}"))?;
        let (det_shape, det_data) = det_outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("extract det tensor: {e}"))?;
        let det_shape: Vec<usize> = det_shape.iter().map(|&d| d as usize).collect();
        let det_array =
            ndarray::ArrayD::from_shape_vec(ndarray::IxDyn(&det_shape), det_data.to_vec())
                .map_err(|e| format!("reshape det output: {e}"))?;
        let det_view = det_array
            .into_dimensionality::<ndarray::Ix4>()
            .map_err(|e| format!("det output is not 4D: {e}"))?;

        let det_view = det_view.view();
        let boxes = extract_text_boxes(
            &det_view,
            det_input.scale,
            det_input.original_size,
            &det_cfg,
        );

        if boxes.is_empty() {
            return Ok(OcrResult {
                text: String::new(),
                confidence: 0.0,
                boxes: Vec::new(),
            });
        }

        // 2. 识别每个文本块
        let mut texts = Vec::with_capacity(boxes.len());
        let mut confidences = Vec::with_capacity(boxes.len());
        for pts in &boxes {
            let crop = perspective_crop(img, pts);
            let rec_input = preprocess_for_recognition(&crop);
            let rec_tensor = ndarray_to_ort_tensor(&rec_input.tensor.view())?;
            let rec_outputs = self
                .rec_session
                .run(ort::inputs!("x" => rec_tensor))
                .map_err(|e| format!("rec inference: {e}"))?;
            let (rec_shape, rec_data) = rec_outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("extract rec tensor: {e}"))?;
            let rec_shape: Vec<usize> = rec_shape.iter().map(|&d| d as usize).collect();
            let rec_array =
                ndarray::ArrayD::from_shape_vec(ndarray::IxDyn(&rec_shape), rec_data.to_vec())
                    .map_err(|e| format!("reshape rec output: {e}"))?;
            // rec 输出形状通常为 [1, T, C]，squeeze batch dim
            let rec_2d = rec_array
                .into_dimensionality::<ndarray::Ix3>()
                .map_err(|e| format!("rec output is not 3D: {e}"))?
                .remove_axis(ndarray::Axis(0));
            let (text, conf) = ctc_decode(&rec_2d.view(), &self.char_list);
            texts.push(text);
            confidences.push(conf);
        }

        Ok(build_ocr_result(boxes, texts, confidences))
    }

    /// 扫描 PDF 文件。
    /// 优先提取文本层；无文本时渲染为图片再 OCR。
    pub fn scan_pdf(&mut self, pdf_path: &Path) -> Result<OcrResult, String> {
        use super::pdf::{
            cleanup_rendered_pages, extract_pdf_text, has_meaningful_text, render_pdf_pages,
        };

        // 1. 提取文本层
        let pages = extract_pdf_text(pdf_path)?;

        // 2. 若文本有意义，直接返回
        if has_meaningful_text(&pages, 20) {
            let mut all_text = String::new();
            let mut all_boxes = Vec::new();
            for (i, page_text) in pages.iter().enumerate() {
                if i > 0 {
                    all_text.push_str(&format!("\n--- Page {} ---\n", i + 1));
                    all_boxes.push(super::types::OcrBox {
                        text: format!("--- Page {} ---", i + 1),
                        confidence: 1.0,
                        points: [(0.0, 1.0), (0.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
                    });
                }
                all_text.push_str(page_text);
                all_boxes.push(super::types::OcrBox {
                    text: page_text.clone(),
                    confidence: 1.0,
                    points: [(0.0, 1.0), (0.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
                });
            }
            return Ok(OcrResult {
                text: all_text,
                confidence: 1.0,
                boxes: all_boxes,
            });
        }

        // 3. 渲染为图片并 OCR
        let temp_dir =
            std::env::temp_dir().join(format!("solosoul-pdf-{}-pages", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

        let image_paths = render_pdf_pages(pdf_path, 150, &temp_dir)?;

        let mut all_text_parts = Vec::new();
        let mut all_boxes = Vec::new();

        for (i, path) in image_paths.iter().enumerate() {
            let page_result = self.scan_image(path)?;
            if i > 0 {
                all_text_parts.push(format!("\n--- Page {} ---\n", i + 1));
                all_boxes.push(super::types::OcrBox {
                    text: format!("--- Page {} ---", i + 1),
                    confidence: 1.0,
                    points: [(0.0, 1.0), (0.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
                });
            }
            all_text_parts.push(page_result.text.clone());
            all_boxes.extend(page_result.boxes);
        }

        cleanup_rendered_pages(&image_paths);
        let _ = std::fs::remove_dir(&temp_dir);

        let text = all_text_parts.join("");
        let confidence = if !all_boxes.is_empty() {
            all_boxes.iter().map(|b| b.confidence).sum::<f64>() / all_boxes.len() as f64
        } else {
            0.0
        };

        Ok(OcrResult {
            text,
            confidence,
            boxes: all_boxes,
        })
    }

    /// 扫描图片中的 MRZ 区域并解析。
    /// 若未检测到 MRZ 或解析失败，返回 Ok(None)。
    pub fn scan_mrz(&mut self, image_path: &Path) -> Result<Option<MrzResult>, String> {
        use super::mrz::{detect_mrz_region, enhance_mrz_crop, parse_mrz};

        let img = load_rgb_image(image_path)?;

        // 1. 检测 MRZ 区域
        let region = match detect_mrz_region(&img) {
            Some(r) => r,
            None => return Ok(None),
        };

        // 2. 裁剪 MRZ 区域
        let crop = perspective_crop(&img, &region);

        // 3. 增强
        let enhanced = enhance_mrz_crop(&crop);

        // 4. OCR
        let ocr_result = self.scan_rgb(&enhanced)?;

        // 5. 提取文本行并解析
        let lines: Vec<String> = ocr_result
            .text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if lines.len() < 2 {
            return Ok(None);
        }

        let mut mrz = parse_mrz(&lines)?;
        mrz.confidence = ocr_result.confidence;
        Ok(Some(mrz))
    }
}

/// 将 ndarray 转换为 ort 输入张量。
fn ndarray_to_ort_tensor(
    arr: &ndarray::ArrayView<f32, ndarray::Ix4>,
) -> Result<ort::value::Tensor<f32>, String> {
    let data = arr.as_standard_layout().into_owned();
    ort::value::Tensor::from_array(data).map_err(|e| format!("create tensor: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_load_missing_models() {
        let tmp = tempfile::tempdir().unwrap();
        let result = OcrEngine::load(tmp.path(), OcrModelTier::Small);
        assert!(result.is_err());
    }

    #[test]
    fn test_model_bundle_path_resolution() {
        // 仅验证路径构建，不加载实际模型
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("pp-ocr-v6-small");
        std::fs::create_dir_all(base.join("det")).unwrap();
        std::fs::create_dir_all(base.join("rec")).unwrap();
        std::fs::write(base.join("det/inference.onnx"), b"dummy").unwrap();
        std::fs::write(base.join("det/inference.yml"), b"dummy").unwrap();
        std::fs::write(base.join("rec/inference.onnx"), b"dummy").unwrap();
        std::fs::write(
            base.join("rec/inference.yml"),
            b"PostProcess:\n  character_dict:\n  - 'a'\n",
        )
        .unwrap();

        let bundle = resolve_model_bundle(tmp.path(), OcrModelTier::Small).unwrap();
        assert!(bundle.det_model.exists());
        assert!(bundle.rec_model.exists());
    }

    /// 端到端集成测试：若项目内已放置 PP-OCRv6 small 模型则运行。
    #[test]
    fn test_ocr_end_to_end() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let models_dir = manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("src-tauri/resources/models");
        if !models_dir
            .join("pp-ocr-v6-small/det/inference.onnx")
            .exists()
        {
            return;
        }

        let image_path = manifest_dir.join("tests/fixtures/ocr_test.png");
        let mut engine = OcrEngine::load(&models_dir, OcrModelTier::Small).unwrap();
        let result = engine.scan_image(&image_path).unwrap();

        assert!(!result.boxes.is_empty(), "Expected at least one text box");
        let full = result.text.to_lowercase();
        assert!(
            full.contains("hello") || full.contains("solosoul") || full.contains("1234567890"),
            "Expected recognizable text, got: {full}"
        );
    }
}
