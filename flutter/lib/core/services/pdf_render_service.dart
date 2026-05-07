import 'dart:math' show sqrt;
import 'dart:typed_data';

import 'package:pdfx/pdfx.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// PDF Render Service
// =============================================================================

/// 将 PDF 页面本地渲染为 raster 图像，供 OCR 引擎识别。
///
/// 整个流程完全在设备本地完成，不经过任何网络：
/// PDF 文件 → pdfx (平台原生 PDF 渲染) → PNG/JPEG Uint8List → OcrService
class PdfRenderService {
  PdfRenderService._();
  static final PdfRenderService _instance = PdfRenderService._();
  factory PdfRenderService() => _instance;

  /// 渲染 PDF 指定页面为图像字节。
  ///
  /// [filePath] 为本地 PDF 文件绝对路径。
  /// [pageNumber] 从 1 开始，默认第 1 页（MRZ 扫描通常只需首页）。
  /// [dpi] 渲染分辨率，默认 300 DPI。OCR 精度对此敏感，不建议低于 200。
  /// [format] 输出图像格式，默认 PNG（无损，OCR 效果最好）。
  ///
  /// 返回 PNG/JPEG 压缩后的图像字节，可直接传入 [OcrService.extractMrz]
  /// 或 [OcrService.recognizeText]。
  Future<Uint8List?> renderPage(
    String filePath, {
    int pageNumber = 1,
    int dpi = 300,
    PdfPageImageFormat format = PdfPageImageFormat.png,
  }) async {
    PdfDocument? document;
    PdfPage? page;

    try {
      document = await PdfDocument.openFile(filePath);
      if (pageNumber < 1 || pageNumber > document.pagesCount) {
        SoloLog.w(
          'PdfRender',
          'Invalid page number $pageNumber (total ${document.pagesCount})',
        );
        return null;
      }

      page = await document.getPage(pageNumber);

      // PDF 页面尺寸基于 72 DPI；按目标 DPI 等比放大
      final scale = dpi / 72.0;
      final renderWidth = (page.width * scale).round();
      final renderHeight = (page.height * scale).round();

      // 限制最大分辨率，防止内存爆炸（如超大海报 PDF）
      const maxPixelCount = 4096 * 4096; // ~16MP
      final pixelCount = renderWidth * renderHeight;
      double finalScale = scale;
      if (pixelCount > maxPixelCount) {
        finalScale = sqrt(maxPixelCount / (page.width * page.height));
        SoloLog.w(
          'PdfRender',
          'Downscaling from ${renderWidth}x$renderHeight '
          'to fit memory limit',
        );
      }

      final width = (page.width * finalScale).round();
      final height = (page.height * finalScale).round();

      SoloLog.d(
        'PdfRender',
        'Rendering page $pageNumber at ${width}x$height (DPI: ${(finalScale * 72).round()})',
      );

      final pageImage = await page.render(
        width: width.toDouble(),
        height: height.toDouble(),
        format: format,
        backgroundColor: '#ffffff',
      );

      if (pageImage == null) {
        SoloLog.e('PdfRender', 'Page render returned null');
        return null;
      }

      return pageImage.bytes;
    } on Exception catch (e, st) {
      SoloLog.e('PdfRender', 'Failed to render PDF page', e, st);
      return null;
    } finally {
      await page?.close();
      await document?.close();
    }
  }
}
