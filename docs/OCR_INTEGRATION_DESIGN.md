# SoloSoul OCR 集成技术设计文档

> 状态：设计文档（评审后修订 v1.3.1）  
> 优先级：P1  
> 目标版本：v1.3.0  
> 撰写日期：2026-05-06  
> 修订日期：2026-05-06  

---

## 1. 需求概述

在 SoloSoul Flutter 客户端中新增**本地 OCR 识别能力**，覆盖核心场景：

1. **护照 MRZ 提取**：拍照/选图后自动识别 Machine Readable Zone，解析姓名、护照号、国籍、有效期等字段，自动填充 Travel 页面。
2. **身份证/驾照 OCR**：提取证件号码、姓名等关键信息。
3. **通用文本识别**（Phase 2）：名片、文档、发票等非结构化文本提取，支持用户框选区域识别。

**核心约束**：
- 完全本地执行，零网络依赖，符合零知识架构。
- 跨平台覆盖：iOS / Android / macOS / Windows。
- 不引入 Python / Go 后端依赖（Flutter 端独立运行）。
- 复用现有 `flutter_rust_bridge` + `flutter/native` 架构。

---

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Flutter UI 层 (Dart)                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │ OCR 扫描页面  │ → │ 结果预览页面  │ → │ 确认导入 Vault│                  │
│  │(相机/相册选图)│    │(结构化展示)  │    │(创建 Object) │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                   │                   │                          │
│         ▼                   ▼                   ▼                          │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              OcrService (Dart 抽象层)                                    ││
│  │   • 平台引擎分发（Apple Vision vs Rust ONNX）                            ││
│  │   • 统一 OcrResult / MrzData 数据结构                                    ││
│  │   • MRZ 解析复用（纯 Dart，移植自 Go mrz.go）                            ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼ Platform.isIOS / isMacOS                                          │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              Apple Vision (Method Channel)                              ││
│  │   • VNRecognizeTextRequest                                              ││
│  │   • 仅用于非 MRZ 通用 OCR（名片/文档）                                   ││
│  │   • 零额外体积，系统级精度                                               ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼ Platform.isAndroid / isWindows / fallback                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              flutter_rust_bridge                                        ││
│  │   • 异步调用，零拷贝（Rust Vec<u8> → Dart Uint8List）                    ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │              Rust OCR Module (flutter/native/src/ocr/)                  ││
│  │   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             ││
│  │   │ model_loader │ → │  preprocess  │ → │  inference   │             ││
│  │   │(ONNX Session)│    │(图像预处理)   │    │(PP-OCR rec)  │             ││
│  │   └──────────────┘    └──────────────┘    └──────────────┘             ││
│  │         │                                                              ││
│  │         ▼                                                              ││
│  │   ┌──────────────┐                                                     ││
│  │   │ mrz_pipeline │ ──→ 固定 ROI 定位 → 裁剪 → 二值化 → rec 推理        ││
│  │   │(MRZ 专用)    │                                                     ││
│  │   └──────────────┘                                                     ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 技术选型与决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 核心引擎 | Rust + ONNX Runtime (`ort` crate) | 跨平台、与现有 Rust FFI 架构无缝融合、推理可控 |
| 模型 | PP-OCRv4 mobile rec (ONNX) | PaddleOCR 官方导出，移动端优化，中文准确率高 |
| Phase 1 范围 | 仅 rec 模型 + MRZ 专用流水线 | 体积最小、速度最快、避开 det/cls 复杂度 |
| Phase 2 扩展 | det + cls + rec 完整三阶段 | 通用 OCR 能力（文档/名片/发票） |
| Apple Vision | 保留，仅限非 MRZ 场景 | 零体积、高精度，但不能保证 MRZ 行序稳定 |
| MRZ 解析 | Dart 端纯实现（移植 Go mrz.go） | 所有平台统一解析逻辑，引擎只负责"图→字" |
| 图像处理库 | `image` + `imageproc` (Rust) | 生态成熟，功能覆盖现有 Go 预处理逻辑 |

---

## 4. Rust OCR Module 详细设计

### 4.1 目录结构

```
flutter/native/src/
├── lib.rs              # 已有：FRB 入口，新增 ocr mod 注册
├── crypto/
├── vault/
└── ocr/                # 新增
    ├── mod.rs          # 模块入口，FFI 暴露函数
    ├── error.rs        # OcrError 枚举定义
    ├── model.rs        # ONNX 模型加载与管理（Arc + OnceCell Session）
    ├── preprocess.rs   # 图像预处理（旋转/裁剪/灰度/二值化/归一化）
    ├── inference.rs    # ONNX 推理封装（rec 模型单次/批量调用）
    ├── mrz_pipeline.rs # MRZ 专用流水线（ROI 定位 → 裁剪 → 推理）
    └── postprocess.rs  # 后处理（ICAO 字符映射、置信度过滤、行合并）
```

### 4.2 核心数据类型

```rust
// ocr/error.rs
#[derive(Debug)]
pub enum OcrError {
    ModelNotLoaded,
    InvalidImage,
    InferenceFailed(String),
    MrzNotFound { reason: String },
    MrzLowConfidence { line: String, confidence: f32 },
    PreprocessFailed(String),
    Timeout(u64), // 超时秒数
}

// ocr/mod.rs（FRB 自动生成绑定）
#[flutter_rust_bridge::frb]
pub struct OcrResult {
    pub raw_text: String,
    pub blocks: Vec<OcrBlock>,
    pub confidence: f32,
}

#[flutter_rust_bridge::frb]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: BoundingBox,  // 相对坐标 0.0~1.0
}

#[flutter_rust_bridge::frb]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[flutter_rust_bridge::frb]
pub struct MrzResult {
    pub document_type: String,
    pub country: String,
    pub surname: String,
    pub given_names: String,
    pub document_number: String,
    pub nationality: String,
    pub date_of_birth: String,
    pub sex: String,
    pub expiry_date: String,
    pub confidence: f32,
    pub raw_lines: Vec<String>,
}
```

### 4.3 FFI 接口定义

```rust
// ocr/mod.rs

/// 初始化 OCR 引擎（从内存加载 ONNX 模型）
/// Dart 端在 App 启动后后台调用，避免首次识别阻塞
/// 模型字节通过 FFI 直接传递，无需文件系统复制
#[flutter_rust_bridge::frb]
pub fn ocr_init(model_bytes: Vec<u8>) -> Result<(), OcrError> {
    model::load_models_from_memory(&model_bytes)
}

/// 通用文本识别（Phase 2 实现，Phase 1 可 stub）
#[flutter_rust_bridge::frb]
pub fn ocr_recognize(
    image_data: Vec<u8>,
    language: String,
) -> Result<OcrResult, OcrError> {
    // Phase 2: det → cls → rec 完整流水线
    unimplemented!("General OCR available in Phase 2")
}

/// MRZ 专用识别（Phase 1 核心接口）
/// 输入：图像文件字节（PNG/JPEG）
/// 输出：识别到的原始 MRZ 行（不做 TD1/TD2/TD3 解析，保持引擎纯粹）
#[flutter_rust_bridge::frb]
pub fn ocr_extract_mrz_raw(
    image_data: Vec<u8>,
) -> Result<Vec<String>, OcrError> {
    let img = image::load_from_memory(&image_data)
        .map_err(|e| OcrError::InvalidImage)?;
    mrz_pipeline::extract_mrz_lines(&img)
}

/// 释放 OCR 引擎资源（App 退出或内存紧张时调用）
#[flutter_rust_bridge::frb]
pub fn ocr_release() {
    model::unload_models();
}

/// 获取 OCR 引擎状态与性能指标
#[flutter_rust_bridge::frb]
pub fn ocr_status() -> OcrEngineStatus {
    model::engine_status()
}
```

**设计原则**：
- Rust 端**不做 MRZ 语义解析**（不解析护照号/姓名/日期等），只返回原始 MRZ 字符串行。
- Dart 端统一调用 `MrzParser.parse(raw_lines)`，复用现有 Go 逻辑移植版。
- 这样 Apple Vision 和 Rust ONNX 两套引擎的结果可以走同一套解析代码。

---

## 5. MRZ 专用流水线详细设计

### 5.1 为什么跳过 det 和 cls？

| 对比项 | 完整 PP-OCR（det+cls+rec） | MRZ 专用（仅 rec） |
|--------|---------------------------|-------------------|
| 模型体积 | ~13MB（det 4MB + rec 8MB + cls 1MB） | ~8MB（仅 rec） |
| 首次推理延迟 | 300-600ms | 50-150ms |
| 内存占用 | 60-80MB | 25-40MB |
| MRZ 行序稳定性 | 依赖 det 文本框排序，可能出错 | 手工 ROI 裁剪，行序确定 |
| 实现复杂度 | 高（NMS、角度矫正、框合并） | 低（固定区域 → 单行输入） |

### 5.2 流水线步骤

```
输入图像 (UIImage/Bitmap)
    │
    ▼
┌─────────────────┐
│ Step 1: 预处理   │  强制转为灰度图，尺寸归一化（长边 ≤ 2048）
│ (preprocess.rs) │  高斯模糊去噪，为边缘检测做准备
└────────┬────────┘
         │
         ▼
┌──────────────────────────────────────────────┐
│ Step 2: ROI 定位（三级策略）                  │
│ (mrz_pipeline.rs)                            │
│                                              │
│  策略 A: 形态学连通域过滤（首选）              │
│    • 自适应阈值二值化 → 闭运算连接文字碎片     │
│    • 筛选长宽比 > 5:1 的最大连通域             │
│    • 验证字符密度（黑点占比 15%~40%）          │
│                                              │
│  策略 B: 边缘密度 + 水平投影（备用）           │
│    • Sobel/Canny 边缘检测                      │
│    • 水平投影定位文字行密集区                  │
│    • 垂直投影验证字符宽度均匀性                │
│                                              │
│  策略 C: 固定布局假设（兜底）                  │
│    • 底部 25%~35% 区域                        │
│    • 仅当图像 EXIF 显示为证件标准比例时启用    │
│                                              │
│  降级: 若全部失败 → 返回 OcrError::MrzNotFound │
│         Dart 端引导用户手动框选 MRZ 区域        │
└────────┬─────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│ Step 3: 裁剪与   │  提取 ROI 子图，按行切分（水平投影阈值分割）
│ 行切分           │  每行独立处理，确保行序绝对正确
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 4: 二值化   │  Sauvola 局部自适应阈值（对反光/阴影/渐变鲁棒）
│ 与尺寸标准化     │  每行 resize 到固定高度 48px，宽度按比例或 padding
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 5: ONNX    │  逐行送入 PP-OCRv4 rec 模型
│ 推理 (rec)      │  输入 shape: [batch=1, channels=3, height=48, width=variable]
│                 │  输出: CTC 解码后的字符序列 + 置信度
│                 │  超时保护：单张图像总推理时间 ≤ 3 秒
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Step 6: ICAO    │  • 字符白过滤：仅保留 [A-Z0-9<]
│ 后处理           │  • 易混淆映射：O→0, I→1, B→8（按上下文启发式修正）
│ (postprocess)   │  • 长度校验：TD3 44字符/行，TD1 30字符/行
│                 │  • 填充符标准化：连续空格替换为 <
│                 │  • 校验位验证（权重 7,3,1），低于阈值标记低置信度
└────────┬────────┘
         │
         ▼
    输出: Vec<String>（原始 MRZ 行，如 2~3 行）
```

### 5.3 关键算法细节

**ROI 定位（三级策略 + 降级 UI）：**

```rust
fn locate_mrz_region(gray: &GrayImage) -> Result<Vec<Rect>, OcrError> {
    // 策略 A: 形态学连通域过滤（首选）
    if let Some(regions) = locate_by_connected_components(gray) {
        return Ok(regions);
    }
    
    // 策略 B: 边缘密度 + 水平/垂直投影（备用）
    if let Some(regions) = locate_by_projection(gray) {
        return Ok(regions);
    }
    
    // 策略 C: 固定布局假设（兜底，仅标准比例图像）
    if let Some(regions) = locate_by_fixed_layout(gray) {
        return Ok(regions);
    }
    
    // 全部失败：返回详细错误原因，引导 Dart 端显示手动框选 UI
    Err(OcrError::MrzNotFound {
        reason: "无法自动定位 MRZ 区域，请尝试调整拍摄角度或手动框选".to_string(),
    })
}

/// 策略 A: 形态学连通域过滤
fn locate_by_connected_components(gray: &GrayImage) -> Option<Vec<Rect>> {
    // 1. 自适应阈值二值化
    let binary = adaptive_threshold(gray, 15, 10.0);  // 邻域 15x15, C=10
    
    // 2. 形态学闭运算：连接断裂的文字笔画
    let kernel = imageproc::morphology::rect_kernel(3, 1);  // 水平方向连接
    let closed = imageproc::morphology::close(&binary, &kernel, 1);
    
    // 3. 查找连通域
    let components = imageproc::connected_components::connected_components(
        &closed,
        Connectivity::Four,
        gray::Gray(0),  // 背景色
    );
    
    // 4. 筛选：长宽比 > 5:1，面积占整图 5%~25%，字符密度 15%~40%
    let candidates: Vec<Rect> = components
        .iter()
        .filter(|c| {
            let aspect = c.width as f32 / c.height as f32;
            let area_ratio = (c.width * c.height) as f32 / (gray.width() * gray.height()) as f32;
            let density = compute_char_density(gray, c);
            aspect > 5.0 && aspect < 20.0 
                && area_ratio > 0.05 && area_ratio < 0.25
                && density > 0.15 && density < 0.40
        })
        .map(|c| Rect::from(c))
        .collect();
    
    // 5. 取最大候选区域，若置信度足够则返回
    candidates.into_iter().max_by_key(|r| r.width * r.height)
        .map(|r| vec![r])
}

/// 策略 B: 边缘密度 + 水平/垂直投影
fn locate_by_projection(gray: &GrayImage) -> Option<Vec<Rect>> {
    // 1. Canny 边缘检测
    let edges = imageproc::edges::canny(gray, 50.0, 150.0);
    
    // 2. 水平投影：找文字行密集区
    let h_proj = horizontal_projection(&edges);
    let text_rows = detect_line_peaks(&h_proj, min_peak_height=10, min_gap=5);
    
    // 3. 垂直投影：验证字符宽度均匀性（MRZ 字符等宽）
    if text_rows.len() >= 2 {
        let roi = merge_rows(&text_rows);
        let v_proj = vertical_projection(&edges, &roi);
        if is_uniform_spacing(&v_proj) {  // 字符间距均匀性检查
            return Some(vec![roi]);
        }
    }
    None
}

/// 策略 C: 固定布局假设（仅当图像比例接近证件标准时）
fn locate_by_fixed_layout(gray: &GrayImage) -> Option<Vec<Rect>> {
    let (w, h) = gray.dimensions();
    let ratio = w as f32 / h as f32;
    
    // 仅对接近护照/身份证比例的图像启用（如 ~0.7 或 ~1.4）
    if (0.6..0.9).contains(&ratio) || (1.2..1.6).contains(&ratio) {
        let y = (h as f32 * 0.65) as u32;
        let height = (h as f32 * 0.15) as u32;
        Some(vec![Rect::at(0, y).of_size(w, height)])
    } else {
        None
    }
}
```

**图像预处理参数（与 PP-OCRv4 rec 模型输入对齐）：**

```rust
// preprocess.rs
const REC_INPUT_HEIGHT: u32 = 48;
const REC_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const REC_STD: [f32; 3] = [0.229, 0.224, 0.225];

fn prepare_rec_input(img: &RgbImage) -> ndarray::Array4<f32> {
    // Resize: height = 48, width 保持比例，不足则 pad
    // Normalize: (pixel / 255.0 - mean) / std
    // Layout: NCHW [1, 3, 48, W]
}
```

**ICAO 字符映射表：**

```rust
// postprocess.rs
fn icao_normalize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'O' => '0',  // 字母 O → 数字 0
            'I' => '1',  // 字母 I → 数字 1
            'B' if context_suggests_numeric(c) => '8',
            ' ' => '<',  // 空格 → 填充符
            c if c.is_ascii_alphanumeric() || c == '<' => c,
            _ => '<',    // 其他非法字符 → 填充符
        })
        .collect()
}
```

---

## 6. ONNX Runtime 集成配置

### 6.1 `ort` crate 配置

```toml
# flutter/native/Cargo.toml
[dependencies]
ort = { version = "2.0", features = ["download-binaries"] }
image = "0.25"
imageproc = "0.25"
ndarray = "0.15"
once_cell = "1.20"

[target.'cfg(target_os = "ios")'.dependencies]
# iOS 使用 xnnpack 后端加速
ort = { version = "2.0", features = ["download-binaries", "xnnpack"] }

[target.'cfg(target_os = "android")'.dependencies]
# Android 使用 NNAPI 后端
ort = { version = "2.0", features = ["download-binaries", "nnapi"] }
```

> **注意**：`download-binaries` 会在 build 时自动下载对应 target 的 ONNX Runtime 预编译库。若 CI 环境无外网，需提前缓存到 `~/.cache/ort` 或手动管理。

### 6.1.1 CI 环境本地缓存方案

```bash
# 开发/CI 环境预下载 ONNX Runtime 库
# 在项目仓库中创建缓存目录，避免构建时联网下载
mkdir -p .ort-cache
export ORT_CACHE_DIR=$(pwd)/.ort-cache

# 手动下载各平台库（示例：macOS arm64）
curl -L -o .ort-cache/onnxruntime-osx-arm64-1.17.0.tgz \
  https://github.com/microsoft/onnxruntime/releases/download/v1.17.0/\
onnxruntime-osx-arm64-1.17.0.tgz

# CI 中设置环境变量使 ort 优先使用缓存
# .github/workflows/ci_cd.yml:
# env:
#   ORT_CACHE_DIR: ${{ github.workspace }}/.ort-cache
```

```toml
# Cargo.toml 中可通过环境变量覆盖下载源
# 或在 build.rs 中检测 ORT_CACHE_DIR
[package]
build = "build.rs"
```

### 6.2 各平台构建配置

#### iOS

```bash
# 使用 cargo-lipo 构建 universal binary（真机 arm64 + 模拟器 x86_64/arm64）
cd flutter/native
cargo install cargo-lipo
cargo lipo --release

# Podfile 中确保禁止 bitcode（ONNX Runtime 不支持）
# ios/Podfile:
# post_install do |installer|
#   installer.pods_project.targets.each do |target|
#     target.build_configurations.each do |config|
#       config.build_settings['ENABLE_BITCODE'] = 'NO'
#     end
#   end
# end
```

**关键配置：**
- `ort` 自动下载的 iOS XCFramework 已包含 `xnnpack` 后端（NEON 加速）。
- 需在 `ios/Runner/Info.plist` 中声明相机/相册权限（已有则忽略）。

#### Android

```bash
# 使用 cargo-ndk 交叉编译
cargo install cargo-ndk
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../android/app/src/main/jniLibs build --release
```

**关键配置：**
- `build.gradle` 中启用 `extractNativeLibs = true` 以减小 APK 体积。
- NNAPI 后端在 Android 10+ 自动启用 NPU/GPU 加速。
- 目标 API ≥ 24（`minSdkVersion 24`）。

#### macOS / Windows

| 链接方式 | 优点 | 缺点 | 推荐 |
|---------|------|------|------|
| 动态链接（`.dylib` / `.dll`） | 可执行文件小，避免 C++ 运行时冲突 | 分发需带库文件 | **macOS / Windows（推荐）** |
| 静态链接 | 单文件部署 | 可执行文件 +15~20MB，Windows 易引发 `vcruntime` 符号冲突 | 不推荐 |

**Windows 动态链接注意事项**：
- 构建时将 `onnxruntime.dll` 复制到输出目录（与 `.exe` 同级）。
- Rust 启动时通过 `std::env::set_current_dir` 或 `SetDllDirectoryW` 确保 DLL 可被加载。
- 或使用 `embed-resource` crate 将 DLL 嵌入资源，启动时释放到临时目录。

```toml
# macOS / Windows 动态链接配置（推荐）
[target.'cfg(any(target_os = "macos", target_os = "windows"))'.dependencies]
ort = { version = "2.0", features = ["download-binaries"] }  # 默认即为动态链接
```

```rust
// Windows DLL 加载路径设置（main.rs 或 lib.rs 入口）
#[cfg(target_os = "windows")]
fn setup_dll_search_path() {
    use std::env;
    use std::path::PathBuf;
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let dll_path: PathBuf = exe_dir.join("onnxruntime.dll");
            if dll_path.exists() {
                // ort crate 会自动处理，此处仅做保险
            }
        }
    }
}
```

### 6.3 模型加载与 Session 管理（线程安全方案）

**关键变更**：使用 `Arc<ort::Session>` + `OnceCell` 替代 `Mutex<Option>`，避免跨线程 panic。

```rust
// ocr/model.rs
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::time::{Duration, Instant};

static REC_SESSION: OnceCell<Arc<ort::Session>> = OnceCell::new();
static INIT_TIME: OnceCell<Instant> = OnceCell::new();

/// 从内存字节加载模型（零文件复制）
pub fn load_models_from_memory(model_bytes: &[u8]) -> Result<(), OcrError> {
    let session = ort::Session::builder()
        .map_err(|_| OcrError::ModelNotLoaded)?
        .with_model_from_memory(model_bytes)
        .map_err(|e| OcrError::InferenceFailed(e.to_string()))?;
    
    REC_SESSION.set(Arc::new(session))
        .map_err(|_| OcrError::InferenceFailed("Session already initialized".to_string()))?;
    INIT_TIME.set(Instant::now())
        .map_err(|_| OcrError::InferenceFailed("Init time already set".to_string()))?;
    
    Ok(())
}

/// 获取 Session 引用（克隆 Arc，不持有锁）
pub fn get_rec_session() -> Result<Arc<ort::Session>, OcrError> {
    REC_SESSION.get()
        .cloned()
        .ok_or(OcrError::ModelNotLoaded)
}

/// 释放资源（将 OnceCell 重置需要特殊处理，生产环境建议进程重启）
pub fn unload_models() {
    // Note: OnceCell 不支持安全重置。对于移动端，建议：
    // 1. 进入后台时不释放，仅暂停推理
    // 2. 内存紧张时由 OS 决定进程回收
    // 3. 如需显式释放，改用 parking_lot::RwLock<Option<Arc<Session>>>
}

/// 引擎状态查询（用于调试与性能监控）
pub fn engine_status() -> OcrEngineStatus {
    OcrEngineStatus {
        is_loaded: REC_SESSION.get().is_some(),
        uptime_secs: INIT_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0),
    }
}

#[flutter_rust_bridge::frb]
pub struct OcrEngineStatus {
    pub is_loaded: bool,
    pub uptime_secs: u64,
}
```

**线程安全说明**：
- `ort::Session` 本身为线程安全设计（内部引用计数），可安全地在多线程间共享。
- `Arc::clone()` 仅增加引用计数，不复制模型数据，获取代价为原子操作（~10ns）。
- `flutter_rust_bridge` 的 `async` 任务池可自由调度，无需担心 `!Send` 问题。

---

## 7. 模型文件部署策略（内存加载方案）

### 7.1 打包结构

```
flutter/
├── assets/
│   └── models/
│       └── v1/
│           └── ppocrv4_rec.onnx      # ~8MB，Phase 1 唯一模型，带版本目录
├── pubspec.yaml
└── lib/
    └── core/services/
        └── ocr_service.dart          # Dart 封装层
```

```yaml
# pubspec.yaml
flutter:
  assets:
    - assets/models/v1/ppocrv4_rec.onnx
```

### 7.2 运行时加载流程（零文件复制）

```dart
// lib/core/services/ocr_service.dart
class OcrService {
  static bool _initialized = false;
  static const String _modelVersion = 'v1';
  
  static Future<void> initialize() async {
    if (_initialized) return;
    
    // 1. 直接从 asset bundle 读取模型字节（不复制到文件系统）
    final byteData = await rootBundle.load(
      'assets/models/$_modelVersion/ppocrv4_rec.onnx'
    );
    final modelBytes = byteData.buffer.asUint8List();
    
    // 2. 通过 FFI 将字节直接传递给 Rust，Rust 从内存创建 ONNX Session
    await NativeOcr.ocrInit(modelBytes: modelBytes);
    _initialized = true;
    
    SoloLog.d('OcrService', 'OCR engine initialized from memory, '
        'bytes: ${modelBytes.length}, version: $_modelVersion');
  }
  
  /// 模型版本校验：确保 Rust 端与 Dart 端模型版本兼容
  static Future<bool> checkModelCompatibility() async {
    final status = await NativeOcr.ocrStatus();
    return status.isLoaded;
  }
}
```

**内存加载优势**：
- **零磁盘复制**：8MB 模型从 asset 直接经 FFI 传入 Rust，无文件 IO。
- **无权限问题**：Windows/macOS 桌面端无需写入 `Application Documents`。
- **启动更快**：避免 `File.writeAsBytes` 的同步/异步开销。
- **内存占用可控**：Dart 端 `ByteData` 在 FFI 调用后由 GC 回收；Rust 端 ONNX Runtime 持有模型内存（~8MB）。

### 7.3 模型版本管理

```dart
// lib/core/services/ocr_model_manager.dart
class OcrModelManager {
  static const Map<String, String> _modelVersions = {
    'v1': 'assets/models/v1/ppocrv4_rec.onnx',
    'v2': 'assets/models/v2/ppocrv4_rec.onnx',  // 未来更高精度模型
  };
  
  /// 检查本地模型版本是否与 Rust 推理代码兼容
  static Future<bool> isCompatible(String version) async {
    // Rust 端硬编码支持的模型版本
    final supportedVersions = await NativeOcr.getSupportedModelVersions();
    return supportedVersions.contains(version);
  }
  
  /// 升级模型：新模型随 App 更新打包，旧版本保留一段时间做回退
  static Future<void> loadModel(String version) async {
    if (!await isCompatible(version)) {
      throw OcrException('模型版本 $version 与当前引擎不兼容');
    }
    final path = _modelVersions[version]!;
    final bytes = await rootBundle.load(path);
    await NativeOcr.ocrInit(modelBytes: bytes.buffer.asUint8List());
  }
}
```

### 7.4 加载性能优化

| 优化点 | 策略 |
|--------|------|
| 首次加载 | App 启动后 2 秒后台异步初始化（`compute()` 隔离线程），不阻塞首页渲染 |
| 模型缓存 | `OnceCell<Arc<Session>>` 常驻内存，全局单例 |
| 内存压力 | 进入后台不释放 Session（OnceCell 无法安全重置），由 OS 管理进程内存 |
| 热更新 | 不支持运行时替换模型（需重启 App）；新版本随发版更新 |

---

## 8. Apple Vision Fallback 策略（Phase 2 实现）

> **Phase 1 决策**：**不实现 Apple Vision**。所有平台（iOS/macOS/Android/Windows）统一走 Rust ONNX + PP-OCRv4 rec 模型。避免 Phase 1 范围膨胀，确保核心 MRZ 功能快速交付。
>
> Apple Vision 作为 Phase 2 通用 OCR 的优化路径，届时再评估是否接入。

### 8.1 Phase 1 统一策略

| 场景 | Phase 1 引擎 | 理由 |
|------|-------------|------|
| MRZ 识别 | **Rust ONNX**（所有平台） | 统一实现、统一精度、统一测试基准 |
| 通用 OCR | **暂不支持** | Phase 2 再实现 |

### 8.2 Phase 2 规划（预留接口）

```dart
// Phase 2 时可能的引擎分发逻辑
class OcrService implements OcrEngine {
  @override
  Future<OcrResult> recognizeText(Uint8List imageData, {OcrOptions? options}) async {
    if (Platform.isIOS || Platform.isMacOS) {
      // Phase 2: 非 MRZ 场景可接入 Apple Vision
      // return _appleVision.recognizeText(imageData);
    }
    // 默认走 Rust ONNX
    return _rustOnnx.recognizeText(imageData);
  }
  
  @override
  Future<MrzData?> extractMrz(Uint8List imageData) async {
    // MRZ 始终走 Rust ONNX，保证跨平台一致性
    final rawLines = await NativeOcr.ocrExtractMrzRaw(imageData: imageData);
    return MrzParser.parse(rawLines);
  }
}
```

### 8.3 Apple Vision Method Channel 接口（Phase 2 预留）

```dart
// 当前不实现，仅做接口占位
class AppleVisionOcr {
  static const platform = MethodChannel('com.solosoul/ocr.vision');
  
  Future<OcrResult> recognizeText(Uint8List imageData) async {
    throw UnimplementedError('Apple Vision OCR available in Phase 2');
  }
}
```

---

## 9. Flutter Dart 层设计

### 9.1 统一数据结构

```dart
// lib/core/models/ocr_result.dart
@freezed
class OcrResult with _$OcrResult {
  const factory OcrResult({
    required String rawText,
    required List<OcrBlock> blocks,
    required double confidence,
  }) = _OcrResult;
}

@freezed
class OcrBlock with _$OcrBlock {
  const factory OcrBlock({
    required String text,
    required double confidence,
    required BoundingBox bbox,
  }) = _OcrBlock;
}

@freezed
class MrzData with _$MrzData {
  const factory MrzData({
    required String documentType,
    required String country,
    required String surname,
    required String givenNames,
    required String documentNumber,
    required String nationality,
    required String dateOfBirth,
    required String sex,
    required String expiryDate,
    required double confidence,
    required List<String> rawLines,
  }) = _MrzData;
}
```

### 9.2 MrzParser（Dart 移植版）

将 `core/ocr/mrz.go` 的 `ParseTD1` / `ParseTD2` / `ParseTD3` / `validateCheckDigit` 逻辑翻译为 Dart，保持算法完全一致：

```dart
// lib/core/utils/mrz_parser.dart
class MrzParser {
  static MrzData? parse(List<String> lines) {
    if (lines.length == 2 && lines.every((l) => l.length == 44)) {
      return _parseTD3(lines[0], lines[1]);
    } else if (lines.length == 3 && lines.every((l) => l.length == 30)) {
      return _parseTD1(lines[0], lines[1], lines[2]);
    } else if (lines.length == 2 && lines.every((l) => l.length == 36)) {
      return _parseTD2(lines[0], lines[1]);
    }
    return null;
  }
  
  static bool _validateCheckDigit(String data, String checkDigit) {
    // 与 Go 版完全一致：权重 7,3,1 循环
  }
}
```

### 9.3 与现有业务集成

```dart
// Travel 页面调用示例
Future<void> _scanPassport() async {
  final imageData = await _pickImage();  // image_picker
  final mrz = await ref.read(ocrServiceProvider).extractMrz(imageData);
  
  if (mrz != null) {
    // 自动填充 UnifiedObject
    ref.read(unifiedObjectProvider.notifier).updateProperties(
      objectId: currentPassportId,
      values: {
        'document_number': mrz.documentNumber,
        'surname': mrz.surname,
        'given_names': mrz.givenNames,
        'nationality': mrz.nationality,
        'date_of_birth': mrz.dateOfBirth,
        'expiry_date': mrz.expiryDate,
      },
    );
  }
}
```

### 9.4 错误处理与超时保护

Rust 端错误通过 `OcrError` 枚举传递到 Dart，Dart 层需做分级处理：

```dart
// lib/core/services/ocr_service.dart
class OcrService {
  static const Duration _mrzTimeout = Duration(seconds: 3);
  
  Future<MrzData?> extractMrz(Uint8List imageData) async {
    try {
      // 1. 超时保护：单张图像推理不超过 3 秒
      final rawLines = await NativeOcr.ocrExtractMrzRaw(imageData: imageData)
          .timeout(_mrzTimeout, onTimeout: () {
        throw OcrTimeoutException('识别超时，请尝试重新拍摄');
      });
      
      // 2. MRZ 语义解析
      final mrz = MrzParser.parse(rawLines);
      if (mrz == null) {
        throw OcrParseException('无法解析 MRZ 格式，请检查图像清晰度');
      }
      return mrz;
      
    } on OcrError_ModelNotLoaded {
      // 引擎未初始化，尝试后台加载后重试一次
      await initialize();
      return extractMrz(imageData);  // 单次重试
      
    } on OcrError_MrzNotFound catch (e) {
      // 自动定位失败，引导用户手动框选
      SoloLog.w('OcrService', 'MRZ not found: ${e.reason}');
      throw OcrManualSelectionRequired(e.reason);
      
    } on OcrError_MrzLowConfidence catch (e) {
      // 识别到内容但置信度低，展示预览让用户确认
      SoloLog.w('OcrService', 'Low confidence line: ${e.line}');
      throw OcrLowConfidenceException(e.line, e.confidence);
      
    } catch (e, st) {
      SoloLog.e('OcrService', 'Unexpected OCR error', e, st);
      throw OcrUnknownException('识别失败，请稍后重试');
    }
  }
}
```

**UI 降级策略**：

| 错误类型 | 用户提示 | 后续操作 |
|---------|---------|---------|
| `ModelNotLoaded` | "正在初始化识别引擎，请稍候…" | 自动重试一次 |
| `MrzNotFound` | "未找到证件识别区域，请手动框选" | 弹出裁剪框选 UI |
| `MrzLowConfidence` | "识别结果可能不准确，请核对" | 展示预览页，高亮低置信度字段 |
| `Timeout` | "识别超时，请确保光线充足后重试" | 返回拍摄页 |
| `InvalidImage` | "图像格式不支持或已损坏" | 返回拍摄页 |

**手动框选 UI 设计（降级方案）**：

```dart
// lib/presentation/widgets/mrz_manual_cropper.dart
class MrzManualCropper extends StatefulWidget {
  final Uint8List imageData;
  final void Function(Rect cropRect) onConfirm;
  
  // 提供固定比例裁剪框（支持 TD3 2:1、TD1 3:1）
  // 用户拖动/缩放后，裁剪区域送入 Rust 进行精确识别
}
```

---

## 10. 性能基准与测试策略

### 10.1 目标性能指标

| 指标 | 目标值 | 测试设备基准 |
|------|--------|-------------|
| 模型加载时间 | < 500ms | iPhone 12 / Pixel 6 / M1 MacBook |
| MRZ 单张识别（端到端） | 50-150ms | 同上，图像长边 1024px |
| 端到端分解 | 预处理 10-30ms + 推理 30-100ms + 后处理 5-10ms | - |
| 字符级准确率 | ≥ 95% | 50+ 样本集（含模糊/反光/倾斜） |
| 行级识别成功率 | ≥ 90% | 同上 |
| 内存占用（模型常驻） | ~35MB | 含 ONNX Runtime + rec 模型 |
| 单次推理峰值内存 | ~50MB | 含输入图像 + 中间 tensor |

### 10.2 测试数据集

准备至少 **50 张标注样本**，覆盖以下场景：

| 场景 | 数量 | 说明 |
|------|------|------|
| 标准光照护照 | 15 | 正面拍摄，光照均匀，TD3 格式 |
| 倾斜角度护照 | 10 | 15°~30° 倾斜，测试 ROI 定位鲁棒性 |
| 反光/阴影护照 | 10 | 玻璃台面反光、手指阴影遮挡 |
| 模糊/低分辨率 | 5 | 运动模糊或压缩 artifacts |
| 身份证（TD1） | 5 | 中国二代身份证 MRZ 区域 |
| 驾照/其他证件 | 5 | 非标准 MRZ，测试降级行为 |

Ground Truth 标注格式（JSON）：
```json
{
  "image": "passport_001.jpg",
  "format": "TD3",
  "lines": [
    "P<CHNZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<<<",
    "E12345678<8CHN8601018M2601017<<<<<<<<<<<<<"
  ],
  "fields": {
    "surname": "ZHANG",
    "given_names": "SAN",
    "document_number": "E12345678"
  }
}
```

### 10.3 自动化测试策略

| 测试层级 | 范围 | 工具 | CI 运行 |
|---------|------|------|---------|
| Rust 单元测试 | `preprocess.rs` 二值化、归一化、`mrz_pipeline.rs` 投影算法 | `cargo test` | ✅ 每次 PR |
| Rust 集成测试 | 端到端 `ocr_extract_mrz_raw`（使用样本集） | `cargo test --test ocr_integration` | ⚠️ 需模型文件，手动触发 |
| Flutter Widget 测试 | OCR 扫描页面 UI 交互、错误状态展示 | `flutter test test/widget/` | ✅ 每次 PR |
| 精度回归测试 | 50 样本集字符级准确率计算 | Python 脚本 + `flutter drive` | ⚠️ 每周/发版前 |

### 10.4 性能剖析计划

Phase 1 D8（精度调优日）产出：
- `docs/OCR_PERFORMANCE_BASELINE.md` —— 记录各平台实际耗时与内存占用
- `docs/OCR_ACCURACY_REPORT.md` —— 50 样本集字符级/行级准确率统计
- 使用 Instruments (iOS/macOS) 和 Android Profiler 验证内存无泄漏

---

## 11. 隐私合规说明

SoloSoul 的 OCR 模块严格遵守**零知识架构**与**本地优先原则**：

1. **所有图像处理均在设备本地完成**，图像数据不离开设备内存。
2. **无网络传输**：OCR 推理不调用任何云端 API，不上传图像、不传输识别结果。
3. **模型文件本地分发**：PP-OCRv4 ONNX 模型随 App 打包，不运行时下载。
4. **内存安全**：Rust 侧通过 `Arc<Session>` 管理模型内存，App 生命周期结束后由 OS 回收。
5. **用户知情权**：OCR 扫描页面需展示提示文案——
   > "识别过程完全在您的设备上进行，图像不会上传至任何服务器。"

---

## 12. 构建与发布配置

### 12.1 CI/CD 集成

`.github/workflows/ci_cd.yml` 需新增：

```yaml
# Rust OCR 模块构建验证
rust-ocr-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Test OCR module
      working-directory: flutter/native
      run: cargo test --features ocr  # 纯 Rust 单元测试（预处理算法）

# macOS Release 构建需确保模型文件打包
macos-release:
  steps:
    - name: Verify assets
      run: test -f flutter/assets/models/ppocrv4_rec.onnx
    - name: Build Release
      run: flutter build macos --release
```

### 12.2 发布产物体积估算

| 平台 | 基础 App | + ONNX Runtime | + rec 模型 | 总增量 |
|------|---------|---------------|-----------|--------|
| iOS (arm64) | - | +8MB (xnnpack) | +8MB | **~16MB** |
| Android (arm64-v8a) | - | +6MB (NNAPI) | +8MB | **~14MB** |
| macOS (x86_64+arm64) | - | +10MB (universal) | +8MB | **~18MB** |
| Windows (x64) | - | +10MB | +8MB | **~18MB** |

> 模型文件在 iOS/Android 上会被压缩（`assets` 经过 zip 压缩后约 5-6MB）。

---

## 13. 风险评估与回退方案

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| `ort` crate iOS 模拟器构建失败 | 中 | 阻塞开发 | 本地缓存 `.ort-cache` + `ORT_CACHE_DIR` 环境变量；必要时 `cargo xcodebuild` |
| Android NNAPI 在某些设备上崩溃 | 中 | 运行时故障 | 配置 `ORT_ANDROID_NNAPI_DISABLED` 环境变量回退到 CPU |
| MRZ 识别率不足（模糊/反光） | 中 | 用户体验差 | 形态学连通域定位 + Sauvola 二值化 + **手动框选降级 UI** |
| 模型加载耗时过长 | 低 | 首次识别卡顿 | 内存加载（零文件复制）+ 后台异步初始化 |
| Rust FFI 跨线程 panic（Session !Send） | 低 | 崩溃 | 使用 `Arc<ort::Session>` 替代 `Mutex<Option>` |
| Windows DLL 加载失败 | 低 | 启动失败 | 动态链接 + 启动时 `SetDllDirectoryW` 配置 |
| 体积增长超出预期 | 低 | 下载转化 | Phase 1 仅 rec 模型（~8MB），Phase 2 det/cls 按需下载 |
| CI 无外网导致 `download-binaries` 失败 | 中 | 构建阻塞 | 预上传 ONNX Runtime 库到 `.ort-cache`，CI 优先本地加载 |

**回退策略**：
- **平台构建阻塞 > 3 天**：该平台 MRZ 识别降级为**手动输入 + MRZ 格式校验**（纯 Dart 实现）。
- **模型精度不达标**：替换为 `ppocrv4_server_rec.onnx`（+4MB，精度更高），或启用全图滑动窗口兜底识别。
- **内存紧张（低端设备）**：App 进入后台时不释放 Session（OnceCell 限制），但暂停接受新的 OCR 请求；由 OS 决定进程回收。

---

## 14. 实施路线图

### Phase 1：MRZ 快速交付（目标 6-8 天）

| 天数 | 任务 | 产出 |
|------|------|------|
| D1 | 配置 `ort` crate，解决 iOS/macOS 编译 | `Cargo.toml` 更新，iOS 构建通过 |
| D2 | 移植图像预处理（Go → Rust） | `preprocess.rs` + 单元测试 |
| D3 | 实现 MRZ 专用流水线（ROI → 二值化 → rec） | `mrz_pipeline.rs` |
| D4 | ONNX 推理封装 + ICAO 后处理 | `inference.rs` + `postprocess.rs` |
| D5 | FRB 接口暴露 + Dart 层封装 | `ocr_service.dart` + `mrz_parser.dart` |
| D6 | Flutter UI 集成（Travel 页面扫描入口） | `travel_page.dart` 新增扫描按钮 |
| D7 | Android 构建验证 + NNAPI 调优 | `cargo-ndk` 配置，真机测试 |
| D8 | 精度调优 + 边界 case 处理 | 模糊/反光/倾斜样本测试 |

### Phase 2：通用 OCR 能力（目标 +4-5 天）

| 任务 | 说明 |
|------|------|
| det + cls 模型集成 | 文本区域检测 + 方向分类 |
| 通用 `ocr_recognize` 实现 | 完整 PP-OCR 三阶段流水线 |
| Apple Vision Method Channel | iOS/macOS 非 MRZ 场景优化 |
| 文档扫描 UI | 框选区域、多页扫描、导出 PDF |

### Phase 3：优化与扩展（目标 +3 天）

| 任务 | 说明 |
|------|------|
| 模型量化（INT8） | 体积减半，速度提升 |
| GPU 后端（CoreML / Vulkan） | 极致推理速度 |
| 连续扫描模式 | 相机实时预览 + 自动触发 |

---

## 15. 附录

### A. 相关文件速查

| 目的 | 路径 |
|------|------|
| Go MRZ 解析（参考移植） | `core/ocr/mrz.go` |
| Go 图像预处理（参考移植） | `core/ocr/preprocess.go` |
| Go OCR Job 管理 | `core/ocr/job.go` |
| Go OCR Engine 接口 | `core/ocr/engine.go` |
| Flutter Rust 桥接配置 | `flutter/native/Cargo.toml` |
| Flutter 桥接入口 | `flutter/native/src/lib.rs` |
| FRB 生成 Dart 代码 | `flutter/lib/frb_generated.dart` |

### B. 模型下载来源

- PP-OCRv4 模型 ONNX 导出：
  - GitHub: `PaddlePaddle/PaddleOCR` → `deploy/models/`
  - 或直接从 PaddleOCR 官方文档获取导出脚本

### C. 参考资源

- `ort` crate 文档：https://docs.rs/ort/latest/ort/
- ONNX Runtime 预编译库：https://github.com/microsoft/onnxruntime/releases
- PP-OCRv4 技术报告：https://arxiv.org/abs/2309.XXXXX
- ICAO Doc 9303（MRZ 规范）

---

> 本文档版本：v1.3.1（评审后修订）  
> 修订内容：MRZ 定位算法细化（形态学连通域）、Session 管理重构（Arc + OnceCell）、模型内存加载、CI 缓存方案、Phase 1 明确不实现 Apple Vision、性能基准与测试策略、隐私合规说明。  
> 作为 SoloSoul v1.3.0 OCR 模块的权威设计依据。
