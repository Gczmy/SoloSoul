//! macOS Vision Framework 原生 OCR 桥接（Swift CLI 封装）。
//!
//! 使用 Apple 的 `VNRecognizeTextRequest` 在设备端进行本地 OCR，精度通常优于
//! PP-OCRv6 small，且通过 ANE 硬件加速速度更快。
//!
//! # 实现方式
//!
//! 将 Swift 源码作为字符串嵌入，在首次调用时编译为临时二进制，缓存到 `{tmp}/solosoul-ocr-vision/`。
//! 后续调用直接使用已编译的二进制，避免每次都重新编译。
//!
//! # 跨平台
//!
//! 本模块仅可在 `cfg(target_os = "macos")` 下编译。非 macOS 平台应使用 PP-OCRv6 回退。

use std::path::{Path, PathBuf};
use std::process::Command;

/// Vision Framework 扫描结果的简化表示。
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VisionOcrResult {
    pub results: Vec<VisionTextBlock>,
}

/// Vision Framework 单条文本块。
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VisionTextBlock {
    pub text: String,
    pub confidence: f64,
    pub bbox: BoundingBox,
}

/// Vision Framework 边界框（归一化坐标，0.0–1.0）。
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 嵌入的 Swift 源码，使用 VNRecognizeTextRequest 扫描图像。
/// v2.1: 添加版本号 stderr 输出 + 强制 RGB 转码。
const VISION_SWIFT_SOURCE: &str = r##"#!/usr/bin/env swift

import AppKit
import Vision
import Accelerate

// === 强制版本号输出到 stderr（用于诊断确认调用的是哪个二进制）===
let version = "SoloSoul-Vision-CLI v2.1-MRZ"
fputs("[VISION-CLI] \(version)\n", stderr)

// 解析参数
guard CommandLine.arguments.count > 1 else {
    print("{\"error\": \"No image path provided\"}")
    exit(1)
}
let imagePath = CommandLine.arguments[1]
let url = URL(fileURLWithPath: imagePath)

// 加载图像
guard let image = NSImage(contentsOf: url) else {
    print("{\"error\": \"Cannot load image at \(imagePath)\"}")
    exit(1)
}

// === 获取 CGImage ===
// 不手动转换颜色空间——VNImageRequestHandler 内部自行处理
var proposedRect = NSRect.zero
guard let cgImage = image.cgImage(forProposedRect: &proposedRect, context: nil, hints: nil) else {
    print("{\"error\": \"Cannot get CGImage from NSImage\"}")
    exit(1)
}

// 创建并配置请求
let request = VNRecognizeTextRequest()
request.recognitionLevel = VNRequestTextRecognitionLevel.accurate
request.usesLanguageCorrection = false
// 设置识别语言为英文 + 数字
request.recognitionLanguages = ["en-US"]

// VNImageRequestHandler.perform() 是同步的，返回后结果已就绪
let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
do {
    try handler.perform([request])
} catch {
    let msg = error.localizedDescription.replacingOccurrences(of: "\"", with: "'")
    print("{\"error\": \"\(msg)\"}")
    exit(1)
}

guard let observations = request.results as? [VNRecognizedTextObservation] else {
    print("{\"error\": \"No results from Vision Framework\"}")
    exit(1)
}

var results: [[String: Any]] = []
for observation in observations {
    guard let topCandidate = observation.topCandidates(1).first else { continue }
    let rect = observation.boundingBox

    // 手动 JSON 转义确保安全
    let text = topCandidate.string
        .replacingOccurrences(of: "\\", with: "\\\\")
        .replacingOccurrences(of: "\"", with: "\\\"")
        .replacingOccurrences(of: "\n", with: "\\n")
        .replacingOccurrences(of: "\r", with: "\\r")
        .replacingOccurrences(of: "\t", with: "\\t")
    results.append([
        "text": text,
        "confidence": Double(topCandidate.confidence),
        "bbox": [
            "x": Double(rect.origin.x),
            "y": Double(rect.origin.y),
            "width": Double(rect.size.width),
            "height": Double(rect.size.height)
        ]
    ])
}

if let jsonData = try? JSONSerialization.data(withJSONObject: ["results": results], options: []),
   let jsonString = String(data: jsonData, encoding: .utf8) {
    print(jsonString)
} else {
    print("{\"error\": \"Failed to serialize results\"}")
    exit(1)
}
"##;

/// 获取或编译 Vision Framework CLI 二进制路径。
fn ensure_vision_cli() -> Result<PathBuf, String> {
    let tmp_dir = std::env::temp_dir().join("solosoul-ocr-vision");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("创建 Vision CLI 缓存目录失败: {e}"))?;

    let binary_path = tmp_dir.join("ocr_vision_cli");
    let source_path = tmp_dir.join("ocr_vision_cli.swift");

    // 始终写入最新源码
    std::fs::write(&source_path, VISION_SWIFT_SOURCE)
        .map_err(|e| format!("写入 Swift 源码失败: {e}"))?;

    // 诊断：验证源码文件已落地
    let src_meta =
        std::fs::metadata(&source_path).map_err(|e| format!("验证 Swift 源码写入失败: {e}"))?;
    let src_size = src_meta.len();
    if src_size == 0 {
        return Err("Swift 源码写入成功但文件大小为 0".to_string());
    }
    tracing::debug!(
        "Vision CLI 源码已写入: {} ({} bytes)",
        source_path.display(),
        src_size
    );

    // 判断是否需要重新编译
    let needs_compile = if !binary_path.exists() {
        true
    } else if let (Ok(src_meta), Ok(bin_meta)) = (
        std::fs::metadata(&source_path),
        std::fs::metadata(&binary_path),
    ) {
        src_meta
            .modified()
            .ok()
            .zip(bin_meta.modified().ok())
            .map(|(s, b)| s > b)
            .unwrap_or(true)
    } else {
        true
    };

    if needs_compile {
        tracing::debug!("编译 Vision CLI...");
        let output = Command::new("swiftc")
            .args([
                "-O",
                "-o",
                &binary_path.to_string_lossy(),
                &source_path.to_string_lossy(),
            ])
            .output()
            .map_err(|e| format!("启动 swiftc 编译失败: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("swiftc 编译 Vision CLI 失败: {stderr}"));
        }

        // 诊断：验证编译产物
        let bin_meta =
            std::fs::metadata(&binary_path).map_err(|e| format!("编译成功但找不到二进制: {e}"))?;
        tracing::debug!(
            "Vision CLI 编译成功: {} ({} bytes)",
            binary_path.display(),
            bin_meta.len()
        );
    }

    // 诊断：输出二进制绝对路径
    let canonical =
        std::fs::canonicalize(&binary_path).map_err(|e| format!("获取二进制绝对路径失败: {e}"))?;
    tracing::info!("Vision CLI 路径: {}", canonical.display());

    Ok(binary_path)
}

/// 使用 macOS Vision Framework 扫描图像并返回识别文本。
///
/// 返回 (完整文本, 平均置信度)。失败时返回错误信息。
pub fn scan_image(image_path: &Path) -> Result<(String, f64), String> {
    let binary_path = ensure_vision_cli()?;

    tracing::debug!(
        "Vision CLI 执行: {} {}",
        binary_path.display(),
        image_path.display()
    );

    let output = Command::new(&binary_path)
        .arg(image_path)
        .output()
        .map_err(|e| format!("执行 Vision CLI 失败: {e}"))?;

    // 诊断：捕获并记录 stderr（含 CLI 版本号）
    let stderr_out = String::from_utf8_lossy(&output.stderr);
    if !stderr_out.is_empty() {
        tracing::info!("Vision CLI stderr: {}", stderr_out.trim());
    }

    if !output.status.success() {
        let stdout_out = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Vision CLI 异常退出 (stdout: {}, stderr: {})",
            stdout_out.trim(),
            stderr_out.trim()
        ));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("解析 Vision CLI 输出: {e}"))?;

    if stdout.trim().is_empty() {
        return Err(format!(
            "Vision CLI stdout 为空 (stderr: {})",
            stderr_out.trim()
        ));
    }

    let vision_result: VisionOcrResult = serde_json::from_str(&stdout).map_err(|e| {
        format!(
            "解析 Vision JSON 失败: {e}, raw (first 500): {}",
            &stdout[..stdout.len().min(500)]
        )
    })?;

    if vision_result.results.is_empty() {
        return Err(format!(
            "Vision Framework 未检测到任何文本 (stderr: {})",
            stderr_out.trim()
        ));
    }

    // 按从上到下、从左到右合并文本
    let mut sorted_blocks: Vec<&VisionTextBlock> = vision_result.results.iter().collect();
    sorted_blocks.sort_by(|a, b| {
        let ay = (1.0 - a.bbox.y - a.bbox.height) as f32;
        let by = (1.0 - b.bbox.y - b.bbox.height) as f32;
        ay.partial_cmp(&by)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .x
                    .partial_cmp(&b.bbox.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut lines = Vec::new();
    let mut total_conf = 0.0;
    for block in &sorted_blocks {
        lines.push(block.text.clone());
        total_conf += block.confidence;
    }

    let text = lines.join("\n");
    let avg_conf = total_conf / sorted_blocks.len() as f64;

    tracing::debug!(
        "Vision CLI 返回 {} 个文本块, 平均置信度 {:.2}",
        sorted_blocks.len(),
        avg_conf
    );

    Ok((text, avg_conf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_not_available_in_test() {
        // 在 CI/无 macOS 环境跳过；若本地有 swiftc 则验证二进制可编译
        let result = ensure_vision_cli();
        match result {
            Ok(path) => {
                assert!(
                    path.exists(),
                    "Vision CLI binary should exist at: {}",
                    path.display()
                );
                // 用空路径调用应返回错误（不是崩溃）
                let scan = scan_image(Path::new("/nonexistent/image.png"));
                assert!(
                    scan.is_err(),
                    "Vision scan with nonexistent path should fail"
                );
            }
            Err(e) => {
                // macOS 上没有 swiftc 的可能性很低，但 Ci 服务器可能没有
                if cfg!(target_os = "macos") {
                    panic!("Failed to compile Vision CLI on macOS: {e}");
                }
            }
        }
    }
}
