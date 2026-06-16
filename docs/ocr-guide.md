# SoloSoul OCR 功能说明

> 本文档面向终端用户与开发者，说明 Tauri 客户端中 OCR（光学字符识别）功能的使用方式、隐私模型与审计日志。

## 1. 功能概述

SoloSoul 内置基于 **PP-OCRv6** 的本地 OCR 引擎，所有识别过程均在设备本地完成，识别结果（图片、文本、框坐标）不会上传云端。

主要能力：

- 从本地图片或对象附件中提取文字。
- 将识别结果一键导入为 SoloSoul 对象（`document` 类型），便于后续检索与关联。
- 支持三档模型，按需求在速度与精度间切换。
- 所有 OCR 操作（扫描、切换模型、安装/下载模型）写入 Vault 审计日志。

## 2. 模型档位

| 档位 | 大小 | 特点 | 适用场景 |
|------|------|------|---------|
| **tiny** | 约 1.5M 参数 | 速度最快、占用最小 | 简单截图、短文本 |
| **small** | 约 30MB | 速度与精度平衡 | 日常文档、默认推荐 |
| **medium** | 约 132MB | 精度最高 | 复杂排版、小字号、多语言混合 |

- 默认档位为 **small**。
- `small` 档位模型会随应用打包资源分发，首次使用可自动从打包资源复制到应用数据目录。
- `tiny` / `medium` 需要在「设置 → OCR」中从打包资源安装，或从远程地址下载。

## 3. 入口位置

1. **侧边栏**：将 `ocr` 操作添加到自定义侧边栏后，点击打开 OCR 扫描页。
2. **设置页**：「设置 → OCR」可管理模型、切换档位、安装/下载模型。
3. **对象附件菜单**：在对象详情中查看图片附件时，可通过附件菜单直接对当前图片执行 OCR。

> OCR 扫描要求 Vault 已解锁；未解锁时无法调用识别命令。

## 4. 模型管理

### 4.1 从打包资源安装

在 OCR 设置页或扫描页选择目标档位，点击「安装」即可将随应用分发的模型复制到本地数据目录：

- macOS: `~/Library/Application Support/com.solosoul.app/models/`
- Windows: `%APPDATA%\com.solosoul.app\models\`

### 4.2 从远程下载

若需要 tiny / medium 等未随包分发的模型，可在设置页输入模型文件根目录 URL，格式为：

```text
https://your-cdn.example.com/pp-ocr-v6
```

下载器会按如下结构请求文件：

```text
{base_url}/{tier}/det/inference.onnx
{base_url}/{tier}/det/inference.yml
{base_url}/{tier}/rec/inference.onnx
{base_url}/{tier}/rec/inference.yml
```

> 注意：下载 URL 可能包含认证信息，SoloSoul 仅将模型文件保存到本地，不会将 URL 中的密钥上传到任何服务器。

## 5. 扫描流程

1. 用户选择图片（文件选择器、附件菜单传入或拖拽）。
2. 后端检查 Vault 解锁状态。
3. 加载当前激活档位的检测模型（det）和识别模型（rec）。
4. 对图片做预处理、文字检测、框裁剪、识别、CTC 解码。
5. 返回按阅读顺序排列的文本块与平均置信度。
6. 用户可选择「导入为对象」，将结果保存到当前账户。

识别结果仅保留在本地 Vault 中，不会自动同步或上传。

## 6. 隐私与安全

- **本地优先**：OCR 推理使用的 ONNX Runtime 在本地进程运行，不调用云端 API。
- **Vault 解锁控制**：扫描、切换模型、安装/下载模型均要求 Vault 已解锁，避免未授权使用。
- **审计日志**：每次 OCR 操作写入 Vault 的 `audit_log` 表，记录操作类型、账户、文件名（不含完整路径）、识别统计（文本块数、文本长度、置信度）。
- **不记录敏感内容**：审计日志不会存储识别出的文字内容，仅记录元数据。

## 7. 审计日志字段

| 操作类型 | 说明 | 记录字段 |
|---------|------|---------|
| `ocr_scan` | 扫描图片 | `tier`, `boxCount`, `textLength`, `confidence`, `fileName` |
| `ocr_set_active_tier` | 切换激活模型档位 | `tier` |
| `ocr_install_bundled_model` | 从打包资源安装模型 | `tier` |
| `ocr_download_model` | 从远程下载模型 | `tier`, `baseUrl` |

可在「设置 → 操作日志」中查看最近记录，或通过 `/operation_log`（CLI）导出审计日志。

## 8. 技术位置速查

| 组件 | 路径 |
|------|------|
| Rust OCR 引擎 | `tauri/crates/solosoul-core/src/ocr/` |
| Tauri OCR 命令 | `tauri/src-tauri/src/commands/ocr.rs` |
| OCR 扫描页面 | `tauri/src/pages/scan/OcrPage.tsx` |
| OCR 设置页面 | `tauri/src/pages/settings/OcrSettingsPage.tsx` |
| 模型资源 | `tauri/src-tauri/resources/models/pp-ocr-v6-small/{det,rec}/` |
| 运行时模型目录 | `{app_local_data_dir}/models/` |
| OCR 偏好设置 | `{app_local_data_dir}/ocr_preferences.json` |

## 9. CLI 复用

OCR 引擎完全下沉在 `solosoul-core` crate 中，SoloSoul CLI 后续只需调用 `OcrEngine` 即可实现 `/ocr_scan` 等命令，无需重复实现模型加载与后处理。
