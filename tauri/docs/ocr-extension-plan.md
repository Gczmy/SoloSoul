# SoloSoul OCR 扩展实施计划

> 文档性质：技术规范与实施路线图
> 范围：`tauri/crates/solosoul-core` 本地 OCR 引擎 + Tauri 前端
> 目标：扩展 OCR 支持 PDF 混合识别与 MRZ（机读区）识别

---

## 1. 总体目标

### 1.1 PDF 混合识别

- 用户选择 `.pdf` 文件后，先尝试提取 PDF 文本层。
- 若文本层平均单页字符数 ≥ 20 且至少一页非空，直接返回文本结果。
- 若文本层为空或过少，将 PDF 每页渲染为 PNG 图片，再用现有 PP-OCRv6 small 模型逐页识别。
- 多页 PDF 结果按页拼接，页间用 `\n--- Page N ---\n` 分隔。
- 前端文件选择器支持 `.pdf`。

### 1.2 MRZ 区域识别

- OCR 页面提供"通用"与"证件/MRZ"两种模式。
- 证件模式下，对护照/身份证图片检测底部 MRZ 区域（两条等宽字符带）。
- 对 MRZ 区域做增强后 OCR，再解析为结构化字段（证件类型、签发国、证件号、出生日期、有效期等）。
- 校验 MRZ 校验位并提示是否通过。
- 未检测到 MRZ 时返回普通 OCR 结果作为兜底。

---

## 2. 依赖配置

修改 `crates/solosoul-core/Cargo.toml`：

```toml
[dependencies]
# ... 现有依赖 ...

# PDF 文本提取
lopdf = "0.35"
pdf-extract = "0.8"

# PDF 渲染为图片
pdfium-render = { version = "0.8", features = ["image"] }
```

> 注意：`pdfium-render` 0.8 默认依赖 `image` 0.24，而项目已使用 `image` 0.25。为避免类型冲突，渲染后统一**保存为临时 PNG 文件**，再用项目现有的 `image::open`（0.25）读取。不直接在内存中传递 `image::DynamicImage`。

---

## 3. PDF 混合识别

### 3.1 PDFium 二进制配置

`pdfium-render` 在运行时需要 PDFium 动态库。

#### 开发环境

手动下载对应平台 PDFium 二进制并放入 `src-tauri/resources/`：

```bash
# macOS (Apple Silicon)
curl -L -o pdfium-mac-arm64.tgz https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-arm64.tgz
tar xzf pdfium-mac-arm64.tgz
# 将 libpdfium.dylib 放到 src-tauri/resources/

# Windows
# 下载 pdfium-win-x64.tgz，将 pdfium.dll 放到 src-tauri/resources/

# Linux
# 下载 pdfium-linux-x64.tgz，将 libpdfium.so 放到 src-tauri/resources/
```

#### 生产打包

在 `src-tauri/tauri.conf.json` 的 `bundle.resources` 中注册 PDFium 动态库：

```json
"resources": {
  "resources/docs": "docs",
  "resources/models": "models",
  "resources/libpdfium.dylib": "libpdfium.dylib"
}
```

运行时从 `RESOURCE_DIR` 定位动态库，设置环境变量或传入 `PdfiumLibraryConfig`：

```rust
std::env::set_var("PDFIUM_LIBRARY_PATH", pdfium_dylib_path);
```

### 3.2 新增 `crates/solosoul-core/src/ocr/pdf.rs`

```rust
//! PDF 处理：文本提取 + 无文本时渲染为图片。

use std::path::{Path, PathBuf};

/// 单页 PDF 处理结果。
pub struct PdfPageResult {
    pub page_number: usize,
    pub text_layer: Option<String>,
    pub rendered_image: Option<PathBuf>,
}

/// 提取 PDF 文本层，返回每页文本。
pub fn extract_pdf_text(path: &Path) -> Result<Vec<String>, String>;

/// 判断文本层是否"有意义"。
/// 规则：平均每页字符数 ≥ min_chars_per_page（默认 20），且至少有一页非空。
pub fn has_meaningful_text(pages: &[String], min_chars_per_page: usize) -> bool;

/// 将 PDF 每页渲染为临时 PNG 图片。
/// 返回按页排序的图片路径列表。调用方负责删除临时文件。
pub fn render_pdf_pages(
    path: &Path,
    dpi: u32,
    temp_dir: &Path,
) -> Result<Vec<PathBuf>, String>;

/// 清理渲染产生的临时图片。
pub fn cleanup_rendered_pages(paths: &[PathBuf]);
```

实现要点：

- `extract_pdf_text` 使用 `pdf_extract::extract_text` 提取整份文本，再按 `\x0c`（换页符）拆分为每页。
- `render_pdf_pages` 使用 `pdfium-render` 的 `PdfRenderConfig`，默认 `dpi=150`，输出宽度约 `8.5 * dpi`。
- 临时目录命名：`solosoul-pdf-{uuid}-pages`，渲染完成后清理。

### 3.3 修改 `crates/solosoul-core/src/ocr/engine.rs`

新增方法：

```rust
impl OcrEngine {
    /// 扫描 PDF 文件。
    /// 优先提取文本层；无文本时渲染为图片再 OCR。
    pub fn scan_pdf(&mut self, pdf_path: &Path) -> Result<OcrResult, String>;
}
```

实现逻辑：

1. 调用 `extract_pdf_text`。
2. 若 `has_meaningful_text` 返回 true：
   - 合并每页文本，构造一个 `OcrBox`（置信度 1.0）。
   - 返回 `OcrResult`。
3. 若文本不足：
   - 创建临时目录。
   - 调用 `render_pdf_pages(pdf_path, 150, temp_dir)`。
   - 对每页调用 `scan_image`。
   - 合并所有页的 `boxes` 与 `text`，页间插入分页标记。
   - 清理临时文件。

### 3.4 修改 Tauri 命令

修改 `src-tauri/src/commands/ocr.rs` 中的 `ocr_scan_image`：

```rust
#[tauri::command]
pub async fn ocr_scan_image(
    state: tauri::State<'_, AppState>,
    file_path: String,
    _language: Option<String>,
) -> Result<OcrResult, String>;
```

根据文件扩展名分发：

- `pdf` → `engine.scan_pdf(&path)`
- 其他图片格式 → `engine.scan_image(&path)`

审计日志中新增 `fileType` 字段。

### 3.5 前端调整

- `OcrPage` 文件选择器扩展过滤条件，加入 `pdf`。
- 结果展示保持现有 `OcrResult` 结构。
- （可选）在结果中标注"文本层提取"或"OCR 识别"。

---

## 4. MRZ 区域识别

### 4.1 新增 `crates/solosoul-core/src/ocr/mrz.rs`

#### 4.1.1 类型定义

```rust
//! MRZ（机读区）检测、识别与解析。

use serde::{Deserialize, Serialize};

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
```

#### 4.1.2 区域检测（启发式）

```rust
use image::RgbImage;

/// 在图像中检测 MRZ 区域。
/// 第一阶段基于图像下半部分的水平投影，找两条等长、等高的水平文本行。
pub fn detect_mrz_region(image: &RgbImage) -> Option<[(f32, f32); 4]>;
```

实现步骤：

1. 截取图像下半部分（`y >= 0.6 * height`）。
2. 转换为灰度图。
3. 自适应二值化。
4. 计算水平投影（每行非零像素数）。
5. 找两条连续的非零行带，行带中心间距约等于一个字符高度。
6. 返回 MRZ 区域四角点（原图坐标）。

辅助函数：

```rust
fn to_grayscale(img: &RgbImage) -> image::GrayImage;
fn adaptive_binarize(img: &image::GrayImage) -> image::GrayImage;
fn horizontal_projection(img: &image::GrayImage) -> Vec<u32>;
fn find_two_text_lines(projection: &[u32]) -> Option<(u32, u32)>;
```

#### 4.1.3 MRZ 解析与校验

```rust
/// 解析 MRZ 文本行。
/// 支持 TD-1（3 行 × 30 字符）和 TD-3（护照，2 行 × 44 字符）。
pub fn parse_mrz(lines: &[String]) -> Result<MrzResult, String>;

/// TD-3 护照格式解析。
fn parse_td3(lines: &[String]) -> Result<MrzResult, String>;

/// TD-1 身份证格式解析。
fn parse_td1(lines: &[String]) -> Result<MrzResult, String>;

/// MRZ 校验位算法。
fn mrz_checksum(s: &str) -> char;
```

校验规则：

- TD-3：校验 `document_number`、`date_of_birth`、`expiry_date` 的校验位，以及 `composite_check_digit`。
- TD-1：校验字段顺序与 TD-3 略有不同，需分别实现。

### 4.2 修改 `OcrEngine`

新增方法：

```rust
impl OcrEngine {
    /// 扫描图片中的 MRZ 区域并解析。
    /// 若未检测到 MRZ 或解析失败，返回 Ok(None)。
    pub fn scan_mrz(&mut self, image_path: &Path) -> Result<Option<MrzResult>, String>;
}
```

实现步骤：

1. 加载图片。
2. 调用 `detect_mrz_region`。
3. 若未检测到，返回 `None`。
4. 使用 `perspective_crop` 裁剪 MRZ 区域。
5. 对裁剪图做增强（灰度、二值化、放大）。
6. 调用 OCR 识别。
7. 清理文本行后调用 `parse_mrz`。
8. 填充 `confidence` 并返回。

同时，将 `scan_image` 内部逻辑抽取为 `scan_rgb`，供 `scan_mrz` 直接传入 `RgbImage`：

```rust
pub fn scan_image(&mut self, image_path: &Path) -> Result<OcrResult, String> {
    let img = load_rgb_image(image_path)?;
    self.scan_rgb(&img)
}

pub fn scan_rgb(&mut self, img: &RgbImage) -> Result<OcrResult, String>;
```

### 4.3 新增 Tauri 命令

在 `src-tauri/src/commands/ocr.rs` 中新增：

```rust
#[tauri::command]
pub async fn ocr_scan_mrz(
    state: tauri::State<'_, AppState>,
    file_path: String,
) -> Result<Option<MrzResult>, String>;
```

实现要点：

- 加载当前激活的 OCR 模型。
- 调用 `engine.scan_mrz(&path)`。
- 写入审计日志，字段 `hasMrz` 表示是否识别到 MRZ。

在 `src-tauri/src/lib.rs` 注册命令：

```rust
commands::ocr::ocr_scan_mrz,
```

### 4.4 前端改动

#### 4.4.1 IPC 封装

`src/lib/ipc.ts` 新增类型与命令：

```typescript
export interface MrzResult {
  documentType: string;
  documentTypeSub: string;
  issuingCountry: string;
  documentNumber: string;
  checkDigitDocumentNumber: string;
  nationality: string;
  dateOfBirth: string;
  checkDigitDateOfBirth: string;
  sex: string;
  expiryDate: string;
  checkDigitExpiry: string;
  optionalData: string;
  compositeCheckDigit: string;
  rawLines: string[];
  confidence: number;
  checksumValid: boolean;
}

async ocrScanMrz(filePath: string): Promise<MrzResult | null> {
  return invoke('ocr_scan_mrz', { filePath });
}
```

#### 4.4.2 `OcrPage` 模式切换

新增状态：

```tsx
const [scanMode, setScanMode] = useState<'general' | 'mrz'>('general');
```

文件选择器过滤：

```typescript
const filters = scanMode === 'mrz'
  ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }]
  : [{ name: 'Images & PDFs', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'] }];
```

扫描分发：

```typescript
const res = scanMode === 'mrz'
  ? await commands.ocrScanMrz(path)
  : await commands.ocrScanImage(path);
```

#### 4.4.3 MRZ 结果展示

新增组件 `src/components/ocr/MrzResultCard.tsx`：

- 网格展示：证件类型、签发国、证件号、国籍、出生日期、性别、有效期至。
- 底部显示校验状态："✓ 校验通过" 或 "✗ 校验未通过"。
- 可展开显示原始 MRZ 行。

---

## 5. 文件清单

| 文件 | 改动 |
|------|------|
| `crates/solosoul-core/Cargo.toml` | 新增 `lopdf`、`pdf-extract`、`pdfium-render` 依赖 |
| `crates/solosoul-core/src/ocr/mod.rs` | 导出 `pdf`、`mrz` 模块 |
| `crates/solosoul-core/src/ocr/pdf.rs` | 新增 PDF 文本提取与渲染 |
| `crates/solosoul-core/src/ocr/mrz.rs` | 新增 MRZ 检测、解析、校验 |
| `crates/solosoul-core/src/ocr/engine.rs` | 新增 `scan_pdf`、`scan_mrz`、`scan_rgb` |
| `crates/solosoul-core/src/ocr/types.rs` | 新增 `MrzResult` 类型 |
| `crates/solosoul-core/src/ocr/preprocess.rs` | 新增 `enhance_mrz_crop`、`to_grayscale` 等辅助函数 |
| `src-tauri/src/commands/ocr.rs` | 修改 `ocr_scan_image`，新增 `ocr_scan_mrz` |
| `src-tauri/src/lib.rs` | 注册 `ocr_scan_mrz` |
| `src/lib/ipc.ts` | 暴露 `ocrScanMrz` 与类型 |
| `src/pages/scan/OcrPage.tsx` | 模式切换、PDF 过滤、结果展示 |
| `src/components/ocr/MrzResultCard.tsx` | 新增 MRZ 结果卡片 |
| `src/locales/zh-CN/ocr.json` / `en-US/ocr.json` | 新增 MRZ/PDF 相关文案 |
| `src-tauri/tauri.conf.json` | 注册 PDFium 动态库到 resources |
| `crates/solosoul-core/tests/fixtures/` | 添加测试 PDF、护照图片 |

---

## 6. 测试计划

### 6.1 单元测试

- `ocr/pdf.rs`：
  - `test_extract_pdf_text`：从含文本层的 PDF 提取文本。
  - `test_has_meaningful_text`：空文本、少文本、多文本三种场景。
- `ocr/mrz.rs`：
  - `test_mrz_checksum`：已知样本的校验位计算。
  - `test_parse_td3_valid`：标准护照 MRZ 解析。
  - `test_parse_td3_invalid_checksum`：校验位错误时 `checksum_valid=false`。
  - `test_parse_td1_valid`：身份证 MRZ 解析。

### 6.2 集成测试

在 `crates/solosoul-core/tests/fixtures/` 放置：

- `text_only.pdf`：含文本层的 PDF。
- `scanned.pdf`：扫描版 PDF（无文本层）。
- `passport_sample.jpg`：带 MRZ 的护照图片。

测试用例：

- `test_scan_pdf_text_layer`：文本层 PDF 直接返回文本。
- `test_scan_pdf_image`：扫描版 PDF 渲染后 OCR。
- `test_scan_mrz_passport`：护照 MRZ 检测与解析。

### 6.3 回归测试

- `npm run check-all`
- `cargo test -p solosoul-core`

---

## 7. 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| `pdfium-render` 与 `image` 0.25 类型冲突 | 渲染后保存为 PNG 文件，再用 `image::open` 读取 |
| PDFium 动态库分发问题 | 通过 `tauri.conf.json` resources 打包，运行时从 `RESOURCE_DIR` 加载 |
| 大 PDF 渲染慢/内存高 | 限制最大处理页数（如 50 页），可配置 dpi（默认 150） |
| MRZ 启发式检测在低质量图上失效 | 失败时返回普通 OCR 结果；UI 提示用户手动核对 |
| MRZ 专用模型未来替换 | `detect_mrz_region` 预留 `use_dedicated_model` 参数，后续可无缝切换 |

---

## 8. 后续演进

1. **MRZ 专用模型**：训练或引入针对 OCR-B 字体的轻量 CRNN 模型，替换启发式检测。
2. **PDF 表格识别**：在文本层提取基础上，保留段落和表格结构。
3. **多语言 PDF**：结合 `ocr_get_supported_languages`，按用户选择语言渲染/识别。
4. **扫描质量增强**：自动旋转、去噪、透视校正。
