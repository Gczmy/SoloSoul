import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';

// ============================================================================
// 智能 OCR 结果模型
// ============================================================================

/// 智能OCR扫描后的联合结果类型。
///
/// 通用OCR识别后，系统自动尝试MRZ提取：
/// - 若检测到护照/ID卡的MRZ特征 → [SmartOcrMrzResult]
/// - 否则 → [SmartOcrTextResult]
sealed class SmartOcrResult {
  const SmartOcrResult();
}

/// 通用文本识别结果（非MRZ文档）
class SmartOcrTextResult extends SmartOcrResult {
  final OcrResult ocrResult;

  /// 规则引擎推断出的结构化字段（0MB 提取器链）
  final ExtractionResult extraction;

  const SmartOcrTextResult(this.ocrResult, this.extraction);
}

/// 结构化MRZ识别结果（护照/ID卡等旅行证件）
class SmartOcrMrzResult extends SmartOcrResult {
  /// 解析后的MRZ结构化数据
  final MrzData mrzData;

  /// 原始OCR结果（保留供参考或fallback）
  final OcrResult rawOcrResult;

  const SmartOcrMrzResult({
    required this.mrzData,
    required this.rawOcrResult,
  });
}
