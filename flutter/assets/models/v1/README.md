# PP-OCRv4 rec 模型

> **注意**：此目录需要放置 PP-OCRv4 mobile rec ONNX 模型文件。

## 文件

- `ppocrv4_rec.onnx` (~8MB) — PP-OCRv4 文字识别模型（ONNX 格式）

## 获取方式

### 方式 1：从 PaddleOCR 官方导出

```bash
# 安装 PaddleOCR
pip install paddlepaddle paddleocr

# 导出 ONNX 模型
python -c "
from paddleocr import PaddleOCR
# 或使用 Paddle2ONNX 工具
# https://github.com/PaddlePaddle/Paddle2ONNX
"
```

### 方式 2：直接下载预转换模型

从 PaddleOCR GitHub Release 或社区分享的 ONNX 模型下载：
- https://github.com/PaddlePaddle/PaddleOCR/releases

## 模型要求

| 属性 | 值 |
|------|-----|
| 模型名称 | PP-OCRv4 mobile rec |
| 格式 | ONNX |
| 输入 shape | [batch, 3, 48, width] |
| 输出 shape | [time_steps, num_classes] |
| 字典 | 英文 + 数字 + `<` + 空格（约 65 类） |

## 验证

模型文件放置后，App 启动时会自动加载。可通过以下方式验证：

```dart
final status = await frbOcrStatus();
print('OCR loaded: ${status.isLoaded}');
```
