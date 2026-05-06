import 'dart:io';

import 'package:flutter/services.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

/// Apple Vision OCR 封装
///
/// 仅在 iOS/macOS 上可用。通过 Method Channel 调用原生 VNRecognizeTextRequest。
class AppleVisionOcr {
  static const _channel = MethodChannel('com.solosoul/ocr.vision');

  /// 使用 Apple Vision 对图像进行文本识别
  ///
  /// [imageData] 为 PNG/JPEG 图像字节。
  /// 返回结构化的 [OcrResult]。
  ///
  /// 若当前平台不是 iOS/macOS，直接抛出 [UnsupportedError]。
  static Future<OcrResult> recognizeText(Uint8List imageData) async {
    if (!Platform.isIOS && !Platform.isMacOS) {
      throw UnsupportedError('Apple Vision OCR is only available on iOS/macOS');
    }

    final result = await _channel.invokeMethod<Map<dynamic, dynamic>>(
      'recognizeText',
      {'imageData': imageData},
    );

    if (result == null) {
      throw Exception('Apple Vision returned null result');
    }

    final rawText = result['rawText'] as String? ?? '';
    final confidence = (result['confidence'] as num?)?.toDouble() ?? 0.0;
    final blocksRaw = result['blocks'] as List<dynamic>? ?? [];

    final blocks = blocksRaw.map((b) {
      final map = b as Map<dynamic, dynamic>;
      final bboxMap = map['bbox'] as Map<dynamic, dynamic>;
      return OcrBlock(
        text: map['text'] as String? ?? '',
        confidence: (map['confidence'] as num?)?.toDouble() ?? 0.0,
        bbox: BoundingBox(
          x: (bboxMap['x'] as num?)?.toDouble() ?? 0.0,
          y: (bboxMap['y'] as num?)?.toDouble() ?? 0.0,
          width: (bboxMap['width'] as num?)?.toDouble() ?? 0.0,
          height: (bboxMap['height'] as num?)?.toDouble() ?? 0.0,
        ),
      );
    }).toList();

    SoloLog.d('AppleVisionOcr',
        'Recognized ${blocks.length} blocks, avgConfidence=${confidence.toStringAsFixed(2)}');

    return OcrResult(
      rawText: rawText,
      blocks: blocks,
      confidence: confidence,
    );
  }
}
