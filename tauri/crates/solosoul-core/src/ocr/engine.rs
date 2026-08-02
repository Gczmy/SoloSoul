//! OCR 引擎：加载检测/识别模型并执行端到端扫描。

use super::model::{
    load_det_postprocess_config, load_recognition_dict, resolve_model_bundle, OcrModelBundle,
};
use super::postprocess::{build_ocr_result, ctc_decode_enhanced, extract_text_boxes};
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

    /// 对已加载的 RGB 图像执行 OCR（使用增强型 CTC 解码：置信度过滤 + OCR-B 校正）。
    /// 默认置信度阈值 0.5。
    fn scan_rgb(&mut self, img: &image::RgbImage) -> Result<OcrResult, String> {
        self.scan_rgb_with_threshold(img, 0.5)
    }

    /// 对已加载的 RGB 图像执行 OCR，使用指定的置信度阈值。
    fn scan_rgb_with_threshold(
        &mut self,
        img: &image::RgbImage,
        confidence_threshold: f64,
    ) -> Result<OcrResult, String> {
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
            // 使用增强型解码：置信度过滤 + OCR-B 字符校正
            let (text, conf) =
                ctc_decode_enhanced(&rec_2d.view(), &self.char_list, confidence_threshold);
            texts.push(text);
            confidences.push(conf);
        }

        Ok(build_ocr_result(boxes, texts, confidences))
    }

    /// 对单行文字图直接运行 rec 模型（跳过 det 模型）。
    /// 输入应为包含单行文字的 RGB 图像（如 MRZ 行切分后的图像）。
    fn recognize_line_rgb(&mut self, line_img: &image::RgbImage) -> Result<(String, f64), String> {
        let rec_input = preprocess_for_recognition(line_img);
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
        let rec_2d = rec_array
            .into_dimensionality::<ndarray::Ix3>()
            .map_err(|e| format!("rec output is not 3D: {e}"))?
            .remove_axis(ndarray::Axis(0));
        let (text, conf) = ctc_decode_enhanced(&rec_2d.view(), &self.char_list, 0.1);
        Ok((text, conf))
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
    ///
    /// 使用启发式定位 + rec 模型跳 det：
    /// 1. preprocess_for_mrz: 缩放 → 灰度 → 高斯模糊
    /// 2. locate_mrz_region: 四遍定位（滑窗扫描 → 连通域 → 投影法 → 固定布局）
    /// 3. split_text_lines: 水平投影切分
    /// 4. recognize_line_rgb: 每行单独 rec（跳过 det）
    /// 5. icao_normalize: ICAO 字符标准化
    /// 6. 按长度过滤 + parse_mrz 解析
    ///
    /// 注意：旧策略（A/B/C 多级裁剪+PP-OCR/模板匹配）已在此分支禁用，
    /// 代码保留在 main 分支历史中。
    pub fn scan_mrz(&mut self, image_path: &Path) -> Result<Option<MrzResult>, String> {
        use super::mrz::{icao_normalize, locate_mrz_region, preprocess_for_mrz, split_text_lines};

        let img = load_rgb_image(image_path)?;

        // ── 1. 预处理：缩放 + 灰度 + 高斯模糊 ──
        let gray = preprocess_for_mrz(&img);

        // ── 2. 定位（直接在灰度图上，不用 Sauvola）──
        let region = match locate_mrz_region(&gray, &img) {
            Some(r) => r,
            None => {
                tracing::warn!("[MRZ] ❌ 四遍定位均失败");
                return Ok(None);
            }
        };

        // ── 4. 行切分（在灰度图上做投影，比二值化图更稳定）──
        let line_imgs = split_text_lines(&gray, &region);

        if line_imgs.len() < 2 {
            tracing::warn!("[MRZ] 行数不足 2, 跳过");
            return Ok(None);
        }

        // ── 5. 逐行 rec-only 识别 ──
        // 取最多 4 行（优先底部行）
        let max_lines = 4usize;
        let lines_to_process: Vec<&image::GrayImage> = if line_imgs.len() > max_lines {
            line_imgs[line_imgs.len() - max_lines..].iter().collect()
        } else {
            line_imgs.iter().collect()
        };

        let mut raw_texts = Vec::new();
        let mut total_conf = 0.0f64;

        for (i, &line_gray) in lines_to_process.iter().enumerate() {
            // ── 将 MRZ 行一分为二分别 rec，再拼接结果 ──
            // 行图像约 852×30px，若直接压缩到 320px 则每字符仅 7px 宽，无法识别。
            // 拆为 2 段（各 ~426px），每段压缩到 320px（1.33x），字符 ~14px 宽，可识别。
            let line_w = line_gray.width();
            let line_h = line_gray.height();
            let half_w = line_w / 2;

            let mut recognized = String::new();
            let mut seg_sum_conf = 0.0f64;
            let mut seg_count = 0u32;

            for seg_idx in 0..2 {
                let seg_x = seg_idx * half_w;
                let seg_w = if seg_idx == 0 {
                    half_w
                } else {
                    line_w - half_w
                };
                if seg_w < 10 {
                    continue;
                }

                let seg = image::imageops::crop_imm(line_gray, seg_x, 0, seg_w, line_h).to_image();
                let resized =
                    image::imageops::resize(&seg, 320, 48, image::imageops::FilterType::Triangle);
                let seg_rgb = image::RgbImage::from_fn(320, 48, |x, y| {
                    let val = resized.get_pixel(x, y).0[0];
                    image::Rgb([val, val, val])
                });

                match self.recognize_line_rgb(&seg_rgb) {
                    Ok((text, conf)) => {
                        recognized.push_str(&text);
                        seg_sum_conf += conf;
                        seg_count += 1;
                    }
                    Err(e) => {
                        tracing::warn!("[MRZ]   行[{}] 段[{}] 识别失败: {}", i, seg_idx, e);
                    }
                }
            }

            let avg_conf = if seg_count > 0 {
                seg_sum_conf / seg_count as f64
            } else {
                0.0
            };

            if avg_conf < 0.05 || recognized.trim().len() < 5 {
                continue;
            }

            let normalized = icao_normalize(&recognized);
            raw_texts.push(normalized);
            total_conf += avg_conf;
        }

        if raw_texts.is_empty() {
            tracing::warn!("[MRZ] ❌ 所有行识别均失败");
            return Ok(None);
        }

        let avg_conf = total_conf / raw_texts.len() as f64;

        // ── 6. 宽松过滤 ──
        // 只过滤掉明显非 MRZ 的行（纯填充符或太短），
        // 真正的验证交给 parse_mrz（它会自动 padding + checksum 校验）
        let is_plausible_mrz = |s: &str| -> bool {
            let len = s.len();
            len >= 5 && !s.chars().all(|c| c == '<')
        };

        let valid_texts: Vec<String> = raw_texts
            .into_iter()
            .filter(|t| is_plausible_mrz(t))
            .collect();

        if valid_texts.len() < 2 {
            tracing::warn!("[MRZ] ❌ 有效 MRZ 行不足 2 (仅 {} 行)", valid_texts.len());
            return Ok(None);
        }

        // ── 7. 尝试解析（parse_mrz 内部会 padding + checksum 校验）──
        // 尝试用底部 2 行
        let bottom_2: Vec<String> = valid_texts[valid_texts.len().saturating_sub(2)..].to_vec();
        if let Ok(mrz) = parse_mrz_fallback(&bottom_2, avg_conf) {
            tracing::info!("[MRZ] ✅ 底部 2 行解析成功");
            return Ok(Some(mrz));
        }

        // 尝试底部 3 行（TD-1）
        if valid_texts.len() >= 3 {
            let bottom_3: Vec<String> = valid_texts[valid_texts.len().saturating_sub(3)..].to_vec();
            if let Ok(mrz) = parse_mrz_fallback(&bottom_3, avg_conf) {
                tracing::info!("[MRZ] ✅ 底部 3 行解析成功");
                return Ok(Some(mrz));
            }
        }

        // 贪心合并尝试
        let merged_2 = greedy_merge_lines(&valid_texts, 2);
        if let Ok(mrz) = parse_mrz_fallback(&merged_2, avg_conf) {
            tracing::info!("[MRZ] ✅ 合并 2 行解析成功");
            return Ok(Some(mrz));
        }

        if valid_texts.len() >= 3 {
            let merged_3 = greedy_merge_lines(&valid_texts, 3);
            if let Ok(mrz) = parse_mrz_fallback(&merged_3, avg_conf) {
                tracing::info!("[MRZ] ✅ 合并 3 行解析成功");
                return Ok(Some(mrz));
            }
        }

        tracing::warn!("[MRZ] ❌ 所有解析尝试均失败");
        Err("无法识别 MRZ 格式".to_string())
    }
}

// ─── MRZ 行重建辅助函数 ────────────────────────────────────────

/// 尝试用给定的行列表直接解析 MRZ。成功时设置 confidence。
fn parse_mrz_fallback(lines: &[String], confidence: f64) -> Result<MrzResult, String> {
    use super::mrz::{parse_mrz, verify_checksums_lenient};
    match parse_mrz(lines) {
        Ok(mut mrz) => {
            mrz.confidence = confidence;
            if mrz.checksum_valid {
                return Ok(mrz);
            }
            tracing::info!("[MRZ]     strict checksum 无效, 尝试 lenient");
            if verify_checksums_lenient(&mrz) {
                tracing::info!("[MRZ]     ✓ lenient 校验通过");
                return Ok(mrz);
            }
            tracing::info!("[MRZ]     lenient 也无效, 返回 None");
            Err("checksum 校验失败".to_string())
        }
        Err(e) => {
            tracing::info!("[MRZ]     parse_mrz 失败: {e}");
            Err(e)
        }
    }
}

/// 贪心合并相邻行，直到只剩 `target_count` 行。
///
/// 用于处理 Vision Framework 将 MRZ 文本按 `<<` 分隔符拆成多个碎片的情况。
/// 例如 `["P<UTOERIKSSON", "ANNA", "MARIA", "L898902C3..."]` 合并为 2 行：
/// `["P<UTOERIKSSONANNAMARIA", "L898902C3..."]`
fn greedy_merge_lines(lines: &[String], target_count: usize) -> Vec<String> {
    if lines.len() <= target_count {
        return lines.to_vec();
    }

    let mut result: Vec<String> = Vec::with_capacity(target_count);
    let merge_count = lines.len() - target_count + 1;

    // 前 merge_count 行合并为第一行
    let first: String = lines[..merge_count].concat();
    result.push(first);

    // 剩余行保持原样
    for line in lines[merge_count..].iter() {
        result.push(line.clone());
    }

    debug_assert_eq!(result.len(), target_count);
    result
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
