import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

/// OCR扫描相关的工具函数
///
/// 设计原则：无状态函数，不依赖任何实例状态或 BuildContext
class OcrScannerUtils {
  OcrScannerUtils._();

  /// 从MRZ候选行列表中解析MRZ数据
  ///
  /// 支持TD3(护照2行×44字符)、TD1(身份证3行×30字符)、TD2(2行×36字符)
  /// 返回解析成功的[MrzData]，解析失败返回null
  static MrzData? parseMrzFromCandidates(List<String> candidates) {
    if (candidates.isEmpty) return null;

    // 优先尝试 TD3 护照（2 行 × 44 字符）— 取最后 2 个 44 字符行
    final td3Lines = candidates.where((l) => l.length == 44).toList();
    if (td3Lines.length >= 2) {
      final lastTwo = td3Lines.sublist(td3Lines.length - 2);
      final result = MrzParser.parse(lastTwo);
      SoloLog.d('OcrScannerUtils', 'Trying TD3 with ${lastTwo.length} lines');
      if (result != null) return result;
    }

    // 尝试 TD1 身份证（3 行 × 30 字符）— 取最后 3 个 30 字符行
    final td1Lines = candidates.where((l) => l.length == 30).toList();
    if (td1Lines.length >= 3) {
      final lastThree = td1Lines.sublist(td1Lines.length - 3);
      final result = MrzParser.parse(lastThree);
      SoloLog.d('OcrScannerUtils', 'Trying TD1 with ${lastThree.length} lines');
      if (result != null) return result;
    }

    // 尝试 TD2（2 行 × 36 字符）— 取最后 2 个 36 字符行
    final td2Lines = candidates.where((l) => l.length == 36).toList();
    if (td2Lines.length >= 2) {
      final lastTwo = td2Lines.sublist(td2Lines.length - 2);
      final result = MrzParser.parse(lastTwo);
      SoloLog.d('OcrScannerUtils', 'Trying TD2 with ${lastTwo.length} lines');
      if (result != null) return result;
    }

    return null;
  }

  /// 从OCR结果中提取并解析MRZ
  ///
  /// [ocrResult] OCR扫描结果
  /// 返回[MrzData]如果MRZ解析成功，否则返回null
  static MrzData? extractMrzFromOcrResult(OcrResult ocrResult) {
    final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);
    SoloLog.d('OcrScannerUtils',
        'MRZ candidate lines: ${mrzCandidates.length}');
    return parseMrzFromCandidates(mrzCandidates);
  }
}
