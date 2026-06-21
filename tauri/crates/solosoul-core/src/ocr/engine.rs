//! OCR 引擎：加载检测/识别模型并执行端到端扫描。

use super::model::{
    load_det_postprocess_config, load_recognition_dict, resolve_model_bundle, OcrModelBundle,
};
use super::postprocess::{
    build_ocr_result, ctc_decode_enhanced, extract_text_boxes,
};
use super::preprocess::{
    apply_adaptive_threshold, load_rgb_image, perspective_crop, preprocess_for_detection,
    preprocess_for_recognition,
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
    pub fn scan_rgb(&mut self, img: &image::RgbImage) -> Result<OcrResult, String> {
        self.scan_rgb_with_threshold(img, 0.5)
    }

    /// 对已加载的 RGB 图像执行 OCR，使用指定的置信度阈值。
    pub fn scan_rgb_with_threshold(
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
    pub fn recognize_line_rgb(&mut self, line_img: &image::RgbImage) -> Result<(String, f64), String> {
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
    /// 策略：
    /// 1. 优先使用 PP-OCR 在整图上做文本检测 → 过滤底部文本 → 按行分组 → 解析 MRZ
    ///    （利用 PP-OCR 的神经网络检测模型，比投影法更健壮）
    /// 2. 回退到投影法检测区域 → 裁剪 → OCR
    pub fn scan_mrz(&mut self, image_path: &Path) -> Result<Option<MrzResult>, String> {
        use super::mrz::{
            detect_mrz_region, otsu_binarize, split_mrz_lines, to_grayscale,
            verify_checksums_lenient,
        };
        use image::imageops::FilterType;

        let img = load_rgb_image(image_path)?;
        let (img_w, img_h) = (img.width(), img.height());
        tracing::info!("[MRZ] === 扫描开始 === 图像 {}x{}", img_w, img_h);

        // ── 辅助函数：裁剪底部区域 → 放大 → OCR → 行分组 → MRZ 解析 ──
        fn run_mrz_ocr(
            engine: &mut OcrEngine,
            img: &image::RgbImage,
            bottom_ratio: f32,
            upscale_factor: u32,
            label: &str,
        ) -> Result<Option<MrzResult>, String> {
            let (img_w, img_h) = (img.width(), img.height());
            let crop_h = (img_h as f32 * bottom_ratio) as u32;
            let crop_y = img_h.saturating_sub(crop_h);
            if crop_h < 40 || crop_y >= img_h {
                return Ok(None);
            }
            tracing::info!("[MRZ] 策略 A-{}: 裁剪底部 {}% (y={}, h={}), {}x 放大 → {}x{}",
                label, (bottom_ratio * 100.0) as u32, crop_y, crop_h,
                upscale_factor, img_w * upscale_factor, crop_h * upscale_factor);

            let bottom_crop = image::imageops::crop_imm(img, 0, crop_y, img_w, crop_h).to_image();
            let upscaled = image::imageops::resize(
                &bottom_crop,
                img_w * upscale_factor,
                crop_h * upscale_factor,
                FilterType::CatmullRom,
            );

            let ocr_result = engine.scan_rgb_with_threshold(&upscaled, 0.1)?;
            tracing::info!("[MRZ] 策略 A-{}: PP-OCR 检测到 {} 个文本框, 平均置信度 {:.2}",
                label, ocr_result.boxes.len(), ocr_result.confidence);

            if ocr_result.boxes.is_empty() {
                tracing::info!("[MRZ] 策略 A-{}: 未检测到任何文本框", label);
                return Ok(None);
            }

            // 打印每个框
            for (i, b) in ocr_result.boxes.iter().enumerate() {
                let y_c = (b.points[0].1 + b.points[2].1) / 2.0;
                let x_c = (b.points[0].0 + b.points[2].0) / 2.0;
                let preview: String = b.text.chars().take(40).collect();
                tracing::info!("[MRZ]   框[{}] y={:.0} x={:.0} conf={:.2} text={:?}",
                    i, y_c, x_c, b.confidence, preview);
            }

            // 行分组
            let mut boxes_sorted: Vec<&super::types::OcrBox> = ocr_result.boxes.iter().collect();
            boxes_sorted.sort_by(|a, b| {
                let ay = (a.points[0].1 + a.points[2].1) / 2.0;
                let by = (b.points[0].1 + b.points[2].1) / 2.0;
                ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut rows: Vec<Vec<(f32, &String)>> = Vec::new();
            let mut cur: Vec<(f32, &String)> = Vec::new();
            let mut prev_y = 0.0f32;

            for bx in &boxes_sorted {
                let yc = (bx.points[0].1 + bx.points[2].1) / 2.0;
                let h = (bx.points[2].1 - bx.points[0].1).abs();
                let xc = (bx.points[0].0 + bx.points[2].0) / 2.0;

                if cur.is_empty() {
                    cur.push((xc, &bx.text));
                    prev_y = yc;
                } else if (yc - prev_y).abs() < h.max(10.0) * 0.6 {
                    cur.push((xc, &bx.text));
                    prev_y = yc;
                } else {
                    if cur.len() >= 2 {
                        rows.push(std::mem::take(&mut cur));
                    }
                    cur.push((xc, &bx.text));
                    prev_y = yc;
                }
            }
            if cur.len() >= 2 {
                rows.push(cur);
            }

            tracing::info!("[MRZ] 策略 A-{}: 行分组 → {} 行", label, rows.len());
            if rows.len() < 2 {
                return Ok(None);
            }

            let row_texts: Vec<String> = rows
                .iter_mut()
                .map(|row| {
                    row.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                    row.iter().map(|(_, t)| t.as_str()).collect::<Vec<&str>>().concat()
                })
                .collect();

            for (i, rt) in row_texts.iter().enumerate() {
                let preview: String = rt.chars().take(60).collect();
                tracing::info!("[MRZ]   行[{}] ({} chars): {:?}", i, rt.len(), preview);
            }

            // 解析 + checksum 校验（支持宽松校验）
            let try_parse = |cands: &[String]| -> Option<MrzResult> {
                if let Ok(mrz) = direct_mrz_parse(cands, ocr_result.confidence) {
                    if mrz.checksum_valid {
                        return Some(mrz);
                    }
                    tracing::info!("[MRZ]     ✓ 解析成功但 checksum 无效, 尝试 lenient 校验");
                    if verify_checksums_lenient(&mrz) {
                        tracing::info!("[MRZ]     ✓ lenient 校验通过");
                        return Some(mrz);
                    }
                }
                for tgt in [2usize, 3] {
                    if cands.len() >= tgt {
                        let top: Vec<String> = cands[..tgt].to_vec();
                        let merged = greedy_merge_lines(&top, tgt);
                        if let Ok(mrz) = direct_mrz_parse(&merged, ocr_result.confidence) {
                            if mrz.checksum_valid {
                                return Some(mrz);
                            }
                        }
                    }
                }
                None
            };

            // A1: 底部 y 顺序
            let bottom: Vec<String> = row_texts[row_texts.len().saturating_sub(3)..].to_vec();
            let preview: Vec<String> = bottom.iter().map(|l| l.chars().take(30).collect()).collect();
            tracing::info!("[MRZ]   A1(底部y顺序): {:?}", preview);
            if let Some(mrz) = try_parse(&bottom) {
                tracing::info!("[MRZ] ✅ A-{} A1 成功", label);
                return Ok(Some(mrz));
            }

            // A2: 最长行
            let row_sorted = {
                let mut v = row_texts.clone();
                v.sort_by(|a, b| b.len().cmp(&a.len()));
                v
            };
            let longest: Vec<String> = row_sorted[..row_sorted.len().min(3)].to_vec();
            let preview: Vec<String> = longest.iter().map(|l| l.chars().take(30).collect()).collect();
            tracing::info!("[MRZ]   A2(最长行): {:?}", preview);
            if let Some(mrz) = try_parse(&longest) {
                tracing::info!("[MRZ] ✅ A-{} A2 成功", label);
                return Ok(Some(mrz));
            }

            // A3: 暴力所有连续组合
            let row_y_order = row_texts; // still in y-order (not sorted)
            tracing::info!("[MRZ]   A3(暴力尝试 {}-1 个组合)...", row_y_order.len());
            for i in 0..row_y_order.len().saturating_sub(1) {
                for n in [2usize, 3] {
                    if i + n <= row_y_order.len() {
                        let group: Vec<String> = row_y_order[i..i + n].to_vec();
                        let preview: Vec<String> = group.iter().map(|l| l.chars().take(30).collect()).collect();
                        tracing::info!("[MRZ]     A3 [{i}..+{n}]: {:?}", preview);
                        if let Some(mrz) = try_parse(&group) {
                            tracing::info!("[MRZ] ✅ A-{} A3 成功", label);
                            return Ok(Some(mrz));
                        }
                    }
                }
            }

            Ok(None)
        }

        // ── 策略 A：多级裁剪+放大 → PP-OCR 检测 → 分组解析 ──
        // MRZ 字很小（~3mm），在全图上缩小后难以检测。
        // 从紧到松尝试多个裁剪比例和放大倍数：
        //   先试底部 12% + 4x（精确命中 MRZ，避开字段标签）
        //   回退到底部 35% + 2x（包含上下文）
        for &(ratio, upscale) in &[(0.12f32, 4u32), (0.35f32, 2u32), (0.50f32, 2u32)] {
            if let Some(mrz) = run_mrz_ocr(self, &img, ratio, upscale, &format!("{}-{}", (ratio * 100.0) as u32, upscale))? {
                return Ok(Some(mrz));
            }
        }

        // ── 策略 C：投影法定位 → 行切分 → 模板匹配识别 ──
        // 用 NCC 模板匹配替换 PP-OCR rec 模型，避免 rec 模型 48×320 缩放导致文字过小。
        // 模板在原始分辨率下逐字符匹配，精度更高。
        tracing::info!("[MRZ] === 策略 C: 行切分 + 模板匹配 ===");
        if let Some(region) = detect_mrz_region(&img) {
            tracing::info!("[MRZ] 策略 C: 检测到 MRZ 区域 top={:.0} bottom={:.0}", region[0].1, region[2].1);
            let crop = perspective_crop(&img, &region);
            tracing::info!("[MRZ] 策略 C: 裁剪区域 {}x{}", crop.width(), crop.height());

            // 增强 → 灰度 → 二值化 → 行切分（不使用 enhance_mrz_crop 的 2x 放大，
            // 因为 template matching 在原始分辨率工作）
            let gray = to_grayscale(&crop);
            let binary = otsu_binarize(&gray, 0);
            let line_imgs = split_mrz_lines(&binary);
            tracing::info!("[MRZ] 策略 C: 行切分 → {} 行", line_imgs.len());

            if line_imgs.len() >= 2 {
                // 加载模板（lazy static，仅首次加载字体）
                let templates = match super::mrz_templates::MrzTemplates::load() {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[MRZ] 策略 C: 加载模板失败: {e}, 回退到策略 B");
                        return Ok(None); // 降级到策略 B
                    }
                };

                let mut line_texts = Vec::new();
                let mut total_conf = 0.0f64;

                for (i, line_gray) in line_imgs.iter().enumerate() {
                    // MRZ TD-3 每行 44 字符
                    let num_chars = 44usize;
                    let (text, avg_conf) =
                        super::mrz_templates::recognize_mrz_line(&templates, line_gray, num_chars);
                    let preview: String = text.chars().take(50).collect();
                    tracing::info!("[MRZ]   行[{}] conf={:.3} text={:?}", i, avg_conf, preview);
                    line_texts.push(text);
                    total_conf += avg_conf as f64;
                }

                let avg_conf = total_conf / line_texts.len() as f64;

                // 尝试直接解析
                if let Ok(mrz) = direct_mrz_parse(&line_texts, avg_conf) {
                    if mrz.checksum_valid {
                        tracing::info!("[MRZ] ✅ 策略 C 成功");
                        return Ok(Some(mrz));
                    }
                    tracing::info!("[MRZ] 策略 C: strict checksum 无效, 尝试 lenient");
                    if verify_checksums_lenient(&mrz) {
                        tracing::info!("[MRZ] ✅ 策略 C lenient 校验通过");
                        return Ok(Some(mrz));
                    }
                }

                // 尝试贪心合并
                for tgt in [2usize, 3] {
                    if line_texts.len() >= tgt {
                        let merged = greedy_merge_lines(&line_texts, tgt);
                        if let Ok(mrz) = direct_mrz_parse(&merged, avg_conf) {
                            if mrz.checksum_valid || verify_checksums_lenient(&mrz) {
                                tracing::info!("[MRZ] ✅ 策略 C 合并后成功");
                                return Ok(Some(mrz));
                            }
                        }
                    }
                }
            } else {
                tracing::info!("[MRZ] 策略 C: 行不足 2 行, 跳过");
            }
        } else {
            tracing::info!("[MRZ] ❌ 策略 C: 未检测到 MRZ 区域");
        }

        // ── 策略 B：投影法检测区域 → 裁剪 → OCR（回退） ──
        tracing::info!("[MRZ] === 策略 B: 投影法检测 ===");
        let region = match detect_mrz_region(&img) {
            Some(r) => {
                tracing::info!("[MRZ] 策略 B: 检测到 MRZ 区域 top={:.0} bottom={:.0}", r[0].1, r[2].1);
                r
            }
            None => {
                tracing::info!("[MRZ] ❌ 策略 B: 投影法未检测到 MRZ 区域, 返回 None");
                return Ok(None);
            }
        };

        let crop = perspective_crop(&img, &region);
        tracing::info!("[MRZ] 策略 B: 裁剪区域 {}x{}", crop.width(), crop.height());

        let ocr_text: String;
        let ocr_confidence: f64;

        #[cfg(target_os = "macos")]
        {
            // Vision Framework：使用原始裁剪图
            let tmp_dir = std::env::temp_dir().join(format!(
                "solosoul-mrz-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&tmp_dir)
                .map_err(|e| format!("创建临时 MRZ 目录失败: {e}"))?;
            let tmp_path = tmp_dir.join("mrz_crop.png");
            crop
                .save(&tmp_path)
                .map_err(|e| format!("保存裁剪图像失败: {e}"))?;

            match super::macos_vision::scan_image(&tmp_path) {
                Ok((vision_text, vision_conf)) => {
                    ocr_text = vision_text;
                    ocr_confidence = vision_conf;
                }
                Err(e) => {
                    tracing::warn!(
                        "Vision Framework MRZ 失败，回退到 PP-OCR: {e}"
                    );
                    let thresholded = apply_adaptive_threshold(&crop, 31);
                    let fallback = self.scan_rgb_with_threshold(&thresholded, 0.1)?;
                    ocr_text = fallback.text;
                    ocr_confidence = fallback.confidence;
                }
            }

            let _ = std::fs::remove_file(&tmp_path);
            let _ = std::fs::remove_dir(&tmp_dir);
        }

        #[cfg(not(target_os = "macos"))]
        {
            use super::mrz::enhance_mrz_crop;
            let enhanced = enhance_mrz_crop(&crop);
            let pp_result = self.scan_rgb_with_threshold(&enhanced, 0.1)?;
            ocr_text = pp_result.text;
            ocr_confidence = pp_result.confidence;
        }

        let raw_lines: Vec<String> = ocr_text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut last_error = "OCR 未返回任何文本".to_string();

        if !raw_lines.is_empty() {
            match direct_mrz_parse(&raw_lines, ocr_confidence) {
                Ok(mrz) => return Ok(Some(mrz)),
                Err(e) => last_error = e,
            }

            for target_lines in [2usize, 3] {
                let merged = greedy_merge_lines(&raw_lines, target_lines);
                match direct_mrz_parse(&merged, ocr_confidence) {
                    Ok(mrz) => return Ok(Some(mrz)),
                    Err(e) => last_error = e,
                }
            }
        }

        Err(last_error)
    }
}

// ─── MRZ 行重建辅助函数 ────────────────────────────────────────

/// 尝试用给定的行列表直接解析 MRZ。成功时设置 confidence。
fn direct_mrz_parse(lines: &[String], confidence: f64) -> Result<MrzResult, String> {
    use super::mrz::parse_mrz;
    let mut mrz = parse_mrz(lines)?;
    mrz.confidence = confidence;
    Ok(mrz)
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
