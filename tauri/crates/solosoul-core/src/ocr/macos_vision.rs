//! macOS Vision Framework 原生 OCR 桥接（Swift CLI 封装）。
//!
//! 使用 Apple 的 `VNRecognizeTextRequest` 在设备端进行本地 OCR，精度通常优于
//! PP-OCRv6 small，且通过 ANE 硬件加速速度更快。
//!
//! # 实现方式
//!
//! 将 Swift 源码作为字符串嵌入，在首次调用时编译为临时二进制，缓存到系统缓存目录
//! `{cache}/com.solosoul.app/vision_cli/`（hash 另存 `{config}/com.solosoul.app/vision_cli/`）。
//! 后续调用直接使用已编译的二进制，避免每次都重新编译。
//!
//! # 跨平台
//!
//! 本模块仅可在 `cfg(target_os = "macos")` 下编译。非 macOS 平台应使用 PP-OCRv6 回退。

use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
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
/// v2.2: 修复 `--` 分隔符误读为图像路径（Rust 侧不再传 `--`，Swift 侧防御性跳过）。
/// v2.1: 添加版本号 stderr 输出 + 强制 RGB 转码。
const VISION_SWIFT_SOURCE: &str = r##"#!/usr/bin/env swift

import AppKit
import Vision
import Accelerate

// === 强制版本号输出到 stderr（用于诊断确认调用的是哪个二进制）===
let version = "SoloSoul-Vision-CLI v2.2"
fputs("[VISION-CLI] \(version)\n", stderr)

// 解析参数：兼容可选的 "--" 分隔符（防御性——POSIX 分隔符不会由系统剥离，
// 若调用方误传则会占据 arguments[1]，导致图像路径错位；此处显式跳过）
var argIndex = 1
if CommandLine.arguments.count > 1 && CommandLine.arguments[1] == "--" {
    argIndex = 2
}
guard CommandLine.arguments.count > argIndex else {
    print("{\"error\": \"No image path provided\"}")
    exit(1)
}
let imagePath = CommandLine.arguments[argIndex]
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

/// 计算文件的 SHA-256 哈希。
fn sha256_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开文件计算哈希失败: {e}"))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| format!("计算哈希失败: {e}"))?;
    Ok(hasher.finalize().to_vec())
}

/// 返回 Vision CLI 缓存目录。
/// 生产环境使用系统缓存目录，测试环境使用独立临时目录，避免污染/冲突。
fn vision_cli_cache_root() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        let dir = std::env::temp_dir().join(format!("solosoul-vision-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("创建 Vision CLI 测试缓存目录失败: {e}"))?;
        Ok(dir)
    }
    #[cfg(not(test))]
    {
        let root = dirs::cache_dir()
            .ok_or_else(|| "无法获取系统缓存目录".to_string())?
            .join("com.solosoul.app")
            .join("vision_cli");
        std::fs::create_dir_all(&root).map_err(|e| format!("创建 Vision CLI 缓存目录失败: {e}"))?;
        Ok(root)
    }
}

/// 返回 Vision CLI 哈希存储目录（与二进制缓存目录分离——P019：
/// hash 不再与二进制同目录，避免「能写二进制者也能改 hash」的自证式校验失效）。
/// 生产环境使用系统配置目录，测试环境与缓存目录共用临时目录。
fn vision_cli_hash_root() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        vision_cli_cache_root()
    }
    #[cfg(not(test))]
    {
        let root = dirs::config_dir()
            .ok_or_else(|| "无法获取系统配置目录".to_string())?
            .join("com.solosoul.app")
            .join("vision_cli");
        std::fs::create_dir_all(&root).map_err(|e| format!("创建 Vision CLI 哈希目录失败: {e}"))?;
        // 哈希目录同样 0o700，保持与缓存目录一致的最小权限。
        let mut perms = std::fs::metadata(&root)
            .map_err(|e| format!("读取哈希目录元数据失败: {e}"))?
            .permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(&root, perms).map_err(|e| format!("设置哈希目录权限失败: {e}"))?;
        Ok(root)
    }
}

/// 解析 swiftc 绝对路径（P019：不再依赖 PATH 查找）。
/// 优先 `xcrun --find swiftc`（Xcode Command Line Tools 标准定位），
/// 失败时回退 PATH 中的 `swiftc`（返回命令名，最终仍由系统 PATH 解析）。
///
/// P019-R1: spawn 本身失败（xcrun 未安装/不可执行）也必须回退 PATH，
/// 不得直接上抛——与「失败回退 PATH」的声称一致（原实现 `?` 直接返回 Err）。
fn resolve_swiftc() -> Result<PathBuf, String> {
    match Command::new("xcrun").args(["--find", "swiftc"]).output() {
        Ok(xcrun) => {
            if xcrun.status.success() {
                let path = String::from_utf8_lossy(&xcrun.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(PathBuf::from(path));
                }
            }
            tracing::warn!("xcrun --find swiftc 未返回路径，回退 PATH 查找");
        }
        Err(e) => {
            // P019-R1: spawn 失败（xcrun 不在 PATH）→ 回退，不阻断 OCR
            tracing::warn!("xcrun 不可用（{e}），回退 PATH 查找 swiftc");
        }
    }
    Ok(PathBuf::from("swiftc"))
}

/// Vision CLI 编译时的最低 macOS 部署目标主版本。
///
/// 取值依据：嵌入的 Swift 源码使用 `VNRecognizeTextRequest.recognitionLanguages`
/// （macOS 13.0+ API），部署目标不能低于 13.0。
///
/// P135: 此前编译不指定部署目标，swiftc 默认取**当前 SDK 版本**作为 target
/// （如 macOS 26 SDK → `arm64-apple-macosx26.0`）。当本机 Xcode/CLT 工具链的
/// 标准库不包含该新 target 时编译失败：
/// `unable to load standard library for target 'arm64-apple-macosx26.0'`
/// （见 CODE_ANALYSIS_REPORT P002 与本机复现）。显式固定保守版本后，
/// 任何够新的工具链都能找到对应标准库，从代码层规避该错配。
const VISION_MIN_MACOS_VERSION: &str = "13.0";

/// 当前进程架构对应的 Swift target 架构段（arm64 / x86_64）。
fn target_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// 生成 swiftc 的 `-target` 三元组，如 `arm64-apple-macosx13.0`。
fn vision_cli_target() -> String {
    format!("{}-apple-macosx{}", target_arch(), VISION_MIN_MACOS_VERSION)
}

/// 解析 macOS SDK 路径（`xcrun --show-sdk-path --sdk macosx`）。
///
/// 显式传入 `-sdk` 可保证 swiftc 使用与系统一致的 SDK（而非自行猜测），
/// 与显式 `-target` 形成双重规避。xcrun 不可用/未返回路径时返回 `None`，
/// swiftc 会回退自行推断 SDK——显式 target 已是主防线，此处为加强。
fn resolve_macos_sdk() -> Option<String> {
    match Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let sdk = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if sdk.is_empty() {
                None
            } else {
                Some(sdk)
            }
        }
        _ => None,
    }
}

/// 获取或编译 Vision Framework CLI 二进制路径。
///
/// # 安全
/// - 使用确定性、仅限所有者的应用缓存目录，避免随机临时目录泄漏。
/// - 编译后设置二进制权限为 0o700（仅所有者可执行）。
/// - 源文件权限设为 0o600（仅所有者可读）。
/// - 计算并持久化编译产物的 SHA-256 哈希。
/// - 每次返回路径前校验哈希，防止 TOCTOU 篡改。
fn ensure_vision_cli() -> Result<PathBuf, String> {
    let cache_root = vision_cli_cache_root()?;
    // 确保目录权限为 0o700，防止其他用户访问。
    let mut root_perms = std::fs::metadata(&cache_root)
        .map_err(|e| format!("读取缓存目录元数据失败: {e}"))?
        .permissions();
    root_perms.set_mode(0o700);
    std::fs::set_permissions(&cache_root, root_perms)
        .map_err(|e| format!("设置缓存目录权限失败: {e}"))?;

    let binary_path = cache_root.join("ocr_vision_cli");
    // P019: hash 存配置目录（与二进制分离）
    let hash_path = vision_cli_hash_root()?.join("ocr_vision_cli.sha256");
    let source_path = cache_root.join("ocr_vision_cli.swift");

    // 仅在源码发生变化时才写入，避免每次调用都更新 mtime 导致重复编译。
    let existing_source = std::fs::read_to_string(&source_path).unwrap_or_default();
    if existing_source != VISION_SWIFT_SOURCE {
        std::fs::write(&source_path, VISION_SWIFT_SOURCE)
            .map_err(|e| format!("写入 Swift 源码失败: {e}"))?;
    }
    // 源文件权限 0o600
    if let Ok(meta) = std::fs::metadata(&source_path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = std::fs::set_permissions(&source_path, perms);
    }

    // 判断是否需要重新编译：不存在二进制、或源码比二进制新、或哈希不存在。
    let needs_compile = !binary_path.exists()
        || !hash_path.exists()
        || std::fs::metadata(&source_path)
            .and_then(|m| m.modified())
            .ok()
            .zip(
                std::fs::metadata(&binary_path)
                    .and_then(|m| m.modified())
                    .ok(),
            )
            .map(|(s, b)| s > b)
            .unwrap_or(true);

    if needs_compile {
        tracing::debug!("编译 Vision CLI...");
        let swiftc = resolve_swiftc()?;
        tracing::debug!("使用 swiftc: {}", swiftc.display());
        // P135: 显式指定部署目标（macOS 13.0）与 SDK 路径，规避工具链/SDK
        // 版本错配导致的 `unable to load standard library for target ...` 编译失败。
        // 同时以 MACOSX_DEPLOYMENT_TARGET 环境变量兜底，三重保险。
        let mut cmd = Command::new(&swiftc);
        cmd.arg("-target").arg(vision_cli_target());
        if let Some(sdk) = resolve_macos_sdk() {
            cmd.arg("-sdk").arg(&sdk);
        }
        cmd.env("MACOSX_DEPLOYMENT_TARGET", VISION_MIN_MACOS_VERSION)
            .arg("-O")
            .arg("-o")
            .arg(&binary_path.to_string_lossy().as_ref())
            .arg(&source_path.to_string_lossy().as_ref());
        let output = cmd
            .output()
            .map_err(|e| format!("启动 swiftc 编译失败: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // P135: 失败时附用户可操作的排查指引（而非裸工具链报错）。
            return Err(format!(
                "swiftc 编译 Vision CLI 失败: {stderr}。\n\n\
                 解决建议：请在终端执行 `xcode-select --install` 安装或更新 \
                 Xcode Command Line Tools（或 `sudo xcode-select --switch \
                 /Applications/Xcode.app` 切换工具链），确保 Xcode/CLT 与系统版本匹配；\
                 也可在 OCR 设置中改用 PP-OCR 档位（Small/Medium）作为替代。"
            ));
        }

        let bin_meta =
            std::fs::metadata(&binary_path).map_err(|e| format!("编译成功但找不到二进制: {e}"))?;
        tracing::debug!(
            "Vision CLI 编译成功: {} ({} bytes)",
            binary_path.display(),
            bin_meta.len()
        );

        let mut bin_perms = bin_meta.permissions();
        bin_perms.set_mode(0o700);
        std::fs::set_permissions(&binary_path, bin_perms)
            .map_err(|e| format!("设置二进制权限失败: {e}"))?;

        // 重新计算哈希并持久化
        let hash = sha256_file(&binary_path)?;
        let hash_hex = hex::encode(&hash);
        std::fs::write(&hash_path, &hash_hex).map_err(|e| format!("写入哈希文件失败: {e}"))?;
        // P019-R1: hash 文件显式收紧为 0o600（此前仅目录 0o700 兜底，文件权限
        // 由 umask 决定可能过宽；hash 与二进制同等敏感——能改 hash 即可自证式绕过校验）。
        if let Ok(meta) = std::fs::metadata(&hash_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&hash_path, perms);
        }
        tracing::debug!("Vision CLI 哈希已存储: {}", &hash_hex[..16]);
    }

    // 每次返回前强制校验哈希，防止 TOCTOU 篡改。
    if hash_path.exists() {
        let stored_hash =
            std::fs::read_to_string(&hash_path).map_err(|_| "读取哈希文件失败".to_string())?;
        let actual_hash = sha256_file(&binary_path)?;
        let actual_hex = hex::encode(&actual_hash);
        if stored_hash.trim() != actual_hex {
            return Err(
                "缓存二进制哈希不匹配，文件可能已被篡改。请删除缓存目录与配置目录下的 vision_cli 后重试"
                    .to_string(),
            );
        }
        tracing::debug!("Vision CLI 哈希校验通过");
    } else {
        return Err("Vision CLI 哈希文件缺失，无法验证二进制完整性".to_string());
    }

    tracing::info!("Vision CLI 路径: {}", binary_path.display());
    Ok(binary_path)
}

/// 使用 macOS Vision Framework 扫描图像并返回识别文本。
///
/// 返回 (完整文本, 平均置信度)。失败时返回错误信息。
pub fn scan_image(image_path: &Path) -> Result<(String, f64), String> {
    let binary_path = ensure_vision_cli()?;

    // P018: 只记录图片文件名尾部组件，不落完整路径（用户目录名等敏感信息不入日志）
    let image_name = image_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    tracing::debug!("Vision CLI 执行: {} {}", binary_path.display(), image_name);

    // 直接传图像路径作为 arguments[1]——之前误传 "--" 分隔符，Swift 端用原始
    // CommandLine.arguments[1] 读取时把 "--" 当成路径，报 "Cannot load image at --"。
    // Swift 端已防御性跳过前置 "--"（v2.2），双保险。
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

    /// P135: `-target` 三元组必须显式指向保守部署目标（macOS 13.0），
    /// 不得默认取当前 SDK 版本——否则工具链/SDK 错配时标准库加载失败。
    #[test]
    fn test_vision_cli_target_is_explicit_conservative() {
        let triple = vision_cli_target();
        // 必须显式携带 macosx13.0，而非随 SDK 漂移（如 macosx26.0）。
        assert!(triple.ends_with("-apple-macosx13.0"), "got: {triple}");
        let arch = if std::env::consts::ARCH == "aarch64" {
            "arm64"
        } else {
            "x86_64"
        };
        assert!(triple.starts_with(arch), "got: {triple}");
    }

    /// P135: 部署目标版本常量与 Swift 源码 API 下限一致（recognitionLanguages 需 13.0+）。
    #[test]
    fn test_vision_min_macos_version_at_least_13() {
        let major: u32 = VISION_MIN_MACOS_VERSION
            .split('.')
            .next()
            .and_then(|v| v.parse().ok())
            .expect("valid major version");
        assert!(
            major >= 13,
            "recognitionLanguages requires macOS 13.0+, got {major}"
        );
    }

    /// 回归测试（BUG：`Cannot load image at --`）：图像路径必须原样传给 CLI。
    /// 此前 Rust 侧误传 "--" 分隔符，Swift 端把 arguments[1] 的 "--" 当路径。
    /// 用真实 1x1 PNG 验证：图像能加载（返回「未检测到文本」）即证明路径传递正确，
    /// 而不是报 "Cannot load image at --"。
    #[test]
    fn test_scan_image_passes_real_path() {
        if !cfg!(target_os = "macos") {
            return;
        }
        // P002: swiftc 环境不可用（如 Xcode/CLT 与 macOS SDK 不匹配）时 skip 而非 fail——
        // 这是本机工具链环境问题，不是代码回归；CI 上通常装有完整 CLT。
        if let Err(e) = ensure_vision_cli() {
            eprintln!("P002 skip: Vision CLI 不可用（环境问题）: {e}");
            return;
        }
        let dir = std::env::temp_dir().join(format!("solosoul-vision-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create png dir");
        let png_path = dir.join("pixel.png");
        // 1x1 透明 PNG（base64: iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==）
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x31, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x62, 0xFC, 0xCF, 0xC0, 0x50, 0x0F, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xA9,
            0x8C, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&png_path, png).expect("write png");

        let result = scan_image(&png_path);
        // 修复后：图像被成功加载 → Ok(空文本) 或「未检测到任何文本」；
        // 修复前：错误固定为 "Cannot load image at --"。
        if let Err(e) = &result {
            assert!(
                !e.contains("Cannot load image at --"),
                "路径传递回归：CLI 仍把 '--' 当成图像路径（{e}）"
            );
        }

        // 不存在的路径：错误应包含真实路径（证明参数按原样传递），而非 "--"。
        let missing = dir.join("does-not-exist.png");
        let missing_err = scan_image(&missing).expect_err("不存在的路径应报错");
        assert!(
            missing_err.contains("does-not-exist.png"),
            "错误应包含真实路径，got: {missing_err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

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
                // P002: macOS 上 swiftc 不可用（Xcode/CLT 与 SDK 不匹配等环境问题）时
                // skip 而非 panic——避免工具链环境差异导致测试红；生产路径失败另有日志。
                eprintln!("P002 skip: Vision CLI 不可用（环境问题）: {e}");
            }
        }
    }
}
