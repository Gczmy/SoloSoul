import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/apple_vision_ocr.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/frb/api.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

/// OCR 服务异常基类
class OcrException implements Exception {
  final String message;
  OcrException(this.message);
  @override
  String toString() => 'OcrException: $message';
}

/// OCR 引擎未初始化
class OcrNotInitializedException extends OcrException {
  OcrNotInitializedException() : super('OCR engine not initialized');
}

/// MRZ 识别超时
class OcrTimeoutException extends OcrException {
  OcrTimeoutException() : super('MRZ recognition timeout');
}

/// MRZ 定位失败
class OcrMrzNotFoundException extends OcrException {
  final String reason;
  OcrMrzNotFoundException(this.reason) : super('MRZ not found: $reason');
}

/// MRZ 低置信度
class OcrLowConfidenceException extends OcrException {
  final String line;
  final double confidence;
  OcrLowConfidenceException(this.line, this.confidence)
      : super('Low confidence MRZ: $line ($confidence)');
}

/// 通用 OCR 未检测到文本
class OcrTextNotDetectedException extends OcrException {
  OcrTextNotDetectedException() : super('No text detected in image');
}

/// OCR 服务
///
/// 封装 Rust ONNX OCR 引擎的 Dart 接口，提供：
/// - 引擎初始化（从 asset 内存加载模型）
/// - MRZ 识别（含超时保护）
/// - 通用文本识别（Phase 2：det + rec）
/// - 错误分级处理
class OcrService {
  static bool _initialized = false;
  static const String _modelVersion = 'v1';

  // Phase 2: 三模型路径
  static const String _detAssetPath =
      'assets/models/$_modelVersion/ppocrv4_det.onnx';
  static const String _clsAssetPath =
      'assets/models/$_modelVersion/ppocrv4_cls.onnx';
  static const String _recAssetPath =
      'assets/models/$_modelVersion/ppocrv4_rec.onnx';

  static const Duration _mrzTimeout = Duration(seconds: 3);
  static const Duration _generalTimeout = Duration(seconds: 5);

  /// 初始化 OCR 引擎
  ///
  /// Phase 2: 同时加载 det + rec 模型（cls 暂缺，传空字节）。
  /// 从 Flutter asset bundle 读取 ONNX 模型，通过 FFI 传递给 Rust。
  /// 建议在 App 启动后后台异步调用。
  static Future<void> initialize() async {
    if (_initialized) return;

    try {
      final detData = await rootBundle.load(_detAssetPath);
      final detBytes = detData.buffer.asUint8List();

      final clsData = await rootBundle.load(_clsAssetPath);
      final clsBytes = clsData.buffer.asUint8List();

      final recData = await rootBundle.load(_recAssetPath);
      final recBytes = recData.buffer.asUint8List();

      SoloLog.d('OcrService',
          'Loading models: DET=${detBytes.length} bytes, CLS=${clsBytes.length} bytes, REC=${recBytes.length} bytes');

      await frbOcrInitV2(
        detModelBytes: detBytes,
        clsModelBytes: clsBytes,
        recModelBytes: recBytes,
      );
      _initialized = true;

      SoloLog.d('OcrService', 'OCR engine initialized successfully (Phase 2, det+cls+rec)');
    } catch (e) {
      SoloLog.e('OcrService', 'Failed to initialize OCR engine', e);
      throw OcrException('Failed to initialize OCR engine: $e');
    }
  }

  // ========================================================================
  // MRZ 识别（Phase 1）
  // ========================================================================

  /// 从图像中提取 MRZ 数据
  ///
  /// [imageData] 为 PNG/JPEG 图像字节。
  /// 返回解析后的 [MrzData]，若识别失败则抛出相应异常。
  static Future<MrzData?> extractMrz(Uint8List imageData) async {
    if (!_initialized) {
      try {
        await initialize();
      } on Exception {
        throw OcrNotInitializedException();
      }
    }

    try {
      final rawLines = await frbOcrExtractMrzRaw(imageData: imageData)
          .timeout(_mrzTimeout, onTimeout: () {
        throw OcrTimeoutException();
      });

      if (rawLines.isEmpty) {
        throw OcrMrzNotFoundException('No text recognized');
      }

      final mrz = MrzParser.parse(rawLines);
      if (mrz == null) {
        throw OcrMrzNotFoundException(
            'Could not parse MRZ format from ${rawLines.length} lines');
      }

      SoloLog.d('OcrService', 'MRZ extracted: docType=${mrz.documentType}, '
          'docNo=${mrz.documentNumber}, confidence=${mrz.confidence}');

      return mrz;
    } on OcrException {
      rethrow;
    } on FormatException catch (_) {
      throw OcrMrzNotFoundException('Invalid MRZ format');
    } on Exception catch (e) {
      SoloLog.e('OcrService', 'MRZ extraction failed', e);

      final errStr = e.toString().toLowerCase();
      if (errStr.contains('not found') || errStr.contains('定位')) {
        throw OcrMrzNotFoundException(errStr);
      } else if (errStr.contains('timeout') || errStr.contains('超时')) {
        throw OcrTimeoutException();
      } else if (errStr.contains('confidence') || errStr.contains('置信度')) {
        throw OcrLowConfidenceException(errStr, 0.0);
      } else {
        throw OcrException('MRZ extraction failed: $e');
      }
    }
  }

  // ========================================================================
  // 通用 OCR（Phase 2）
  // ========================================================================

  /// 对任意图像执行通用 OCR 识别
  ///
  /// 平台自动分发：
  /// - iOS/macOS → Apple Vision（系统级，零额外体积）
  /// - Android/Windows → Rust ONNX（det+cls+rec）
  ///
  /// Apple Vision 失败时自动 fallback 到 Rust ONNX。
  /// 返回结构化结果 [OcrResult]，包含每个文本块的坐标、文本和置信度。
  static Future<OcrResult> recognizeText(Uint8List imageData) async {
    // iOS/macOS 优先使用 Apple Vision
    if (Platform.isIOS || Platform.isMacOS) {
      try {
        final result = await AppleVisionOcr.recognizeText(imageData);
        SoloLog.d('OcrService',
            'Apple Vision: ${result.blocks.length} blocks, avgConfidence=${result.confidence}');
        return result;
      } on Exception catch (e) {
        SoloLog.w('OcrService', 'Apple Vision failed, falling back to Rust ONNX: $e');
        // fallback 到 Rust ONNX
      }
    }

    // Android/Windows 或 Apple Vision fallback → Rust ONNX
    return _rustRecognizeText(imageData);
  }

  static Future<OcrResult> _rustRecognizeText(Uint8List imageData) async {
    if (!_initialized) {
      try {
        await initialize();
      } on Exception {
        throw OcrNotInitializedException();
      }
    }

    try {
      final frbResult = await frbOcrRecognize(imageData: imageData)
          .timeout(_generalTimeout, onTimeout: () {
        throw OcrTimeoutException();
      });

      // 将 FRB 类型转换为 Dart 模型类型
      final blocks = frbResult.blocks.map((b) => OcrBlock(
            text: b.text,
            confidence: b.confidence,
            bbox: BoundingBox(
              x: b.bbox.x,
              y: b.bbox.y,
              width: b.bbox.width,
              height: b.bbox.height,
            ),
          )).toList();

      SoloLog.d('OcrService',
          'Rust ONNX: ${blocks.length} blocks, avgConfidence=${frbResult.confidence}');

      return OcrResult(
        rawText: frbResult.rawText,
        blocks: blocks,
        confidence: frbResult.confidence,
      );
    } on OcrException {
      rethrow;
    } on Exception catch (e) {
      SoloLog.e('OcrService', 'Rust ONNX OCR failed', e);

      final errStr = e.toString().toLowerCase();
      if (errStr.contains('not detected') || errStr.contains('未检测到')) {
        throw OcrTextNotDetectedException();
      } else if (errStr.contains('timeout') || errStr.contains('超时')) {
        throw OcrTimeoutException();
      } else {
        throw OcrException('General OCR failed: $e');
      }
    }
  }

  // ========================================================================
  // 状态查询与资源管理
  // ========================================================================

  /// 获取 OCR 引擎状态
  static Future<bool> isInitialized() async {
    if (_initialized) return true;
    try {
      final status = await frbOcrStatus();
      return status.isLoaded;
    } on Exception {
      return false;
    }
  }

  /// 释放 OCR 引擎资源
  static Future<void> release() async {
    if (!_initialized) return;
    try {
      await frbOcrRelease();
      _initialized = false;
      SoloLog.d('OcrService', 'OCR engine released');
    } on Exception catch (e) {
      SoloLog.w('OcrService', 'Error releasing OCR engine: $e');
    }
  }
}
