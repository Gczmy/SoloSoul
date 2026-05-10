import 'dart:io';
import 'dart:typed_data';
import 'dart:math';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/core/services/scan/scan_section_detector.dart';
import 'package:uuid/uuid.dart';

// =============================================================================
// Scan Image Scanner
// =============================================================================

/// Handles OCR-based scanning of image files.
class ScanImageScanner {
  /// Scan an image file using OCR and MRZ detection.
  static Future<ScanResult?> scanImage(ScannedFile file) async {
    try {
      final fileHandle = File(file.path);
      if (!await fileHandle.exists()) return null;

      final bytes = await fileHandle.readAsBytes();
      if (bytes.isEmpty) return null;

      // 通用 OCR 识别
      final ocrResult = await OcrService.recognizeText(Uint8List.fromList(bytes));
      if (ocrResult.rawText.trim().isEmpty) return null;

      // 尝试 MRZ 提取（从候选行中精确筛选标准格式行）
      final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);
      MrzData? mrzData;
      if (mrzCandidates.isNotEmpty) {
        // TD3 护照: 2 行 × 44 字符
        final td3Lines = mrzCandidates.where((l) => l.length == 44).toList();
        if (td3Lines.length >= 2) {
          mrzData = MrzParser.parse(td3Lines.sublist(td3Lines.length - 2));
        }
        // TD1 身份证: 3 行 × 30 字符
        if (mrzData == null) {
          final td1Lines = mrzCandidates.where((l) => l.length == 30).toList();
          if (td1Lines.length >= 3) {
            mrzData = MrzParser.parse(td1Lines.sublist(td1Lines.length - 3));
          }
        }
        // TD2: 2 行 × 36 字符
        if (mrzData == null) {
          final td2Lines = mrzCandidates.where((l) => l.length == 36).toList();
          if (td2Lines.length >= 2) {
            mrzData = MrzParser.parse(td2Lines.sublist(td2Lines.length - 2));
          }
        }

        if (mrzData != null) {
          SoloLog.d('LocalSearchService',
              'MRZ detected in image: ${file.name} type=${mrzData.documentType}');
          return _buildMrzScanResult(file, mrzData);
        }
      }

      // MRZ 未识别到，将 OCR 文本作为普通文档处理
      SoloLog.d('LocalSearchService',
          'No MRZ in image: ${file.name}, falling back to text detection');
      return _buildTextScanResultFromOcr(file, ocrResult);
    } on Exception catch (e) {
      SoloLog.w('LocalSearchService', 'Image OCR failed: ${file.name}', e);
      return null;
    }
  }

  /// 从 MRZ 数据构建 ScanResult
  static ScanResult _buildMrzScanResult(ScannedFile file, MrzData mrzData) {
    final isPassport = mrzData.documentType.startsWith('P');
    final sectionId = isPassport ? 'passport' : 'idCard';
    final displayName = isPassport ? 'Passport' : 'ID Card';

    final fields = <ScanField>[
      ScanField(
        key: 'number',
        value: mrzData.documentNumber,
        sensitivity: SensitivityLevel.critical,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'holderName',
        value: '${mrzData.surname} ${mrzData.givenNames}'.trim(),
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'country',
        value: mrzData.country,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'dateOfBirth',
        value: mrzData.dateOfBirth,
        sensitivity: SensitivityLevel.sensitive,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'sex',
        value: mrzData.sex,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ),
      ScanField(
        key: 'expiryDate',
        value: mrzData.expiryDate,
        sensitivity: SensitivityLevel.sensitive,
        confidence: mrzData.confidence,
      ),
    ];

    if (isPassport) {
      fields.add(ScanField(
        key: 'nationality',
        value: mrzData.nationality,
        sensitivity: SensitivityLevel.public,
        confidence: mrzData.confidence,
      ));
    }

    return ScanResult(
      meta: ScanMeta(
        scanId: const Uuid().v4(),
        createdAt: DateTime.now().millisecondsSinceEpoch,
        sourceFile: file.path,
        confidence: mrzData.confidence,
        fileType: file.extension,
      ),
      sections: [
        ScanSection(
          section: sectionId,
          display: displayName,
          fields: fields,
        ),
      ],
    );
  }

  /// 从通用 OCR 结果构建 ScanResult（将 OCR 文本当作普通文档文本处理）
  static ScanResult? _buildTextScanResultFromOcr(
    ScannedFile file,
    OcrResult ocrResult,
  ) {
    final sections = ScanSectionDetector.detectSections(ocrResult.rawText);
    if (sections.isEmpty) return null;

    final totalFields = sections.fold<int>(0, (sum, s) => sum + s.fields.length);
    final confidence = min(ocrResult.confidence + (totalFields * 0.05), 1.0);

    return ScanResult(
      meta: ScanMeta(
        scanId: const Uuid().v4(),
        createdAt: DateTime.now().millisecondsSinceEpoch,
        sourceFile: file.path,
        confidence: confidence,
        fileType: file.extension,
      ),
      sections: sections,
    );
  }
}
