import 'package:solosoul_flutter/core/models/ocr_result.dart';

/// MRZ 解析器（ICAO Doc 9303）
///
/// 将原始 MRZ 文本行解析为结构化的 [MrzData]。
/// 支持 TD1（身份证，3行×30字符）、TD2（部分证件，2行×36字符）、
/// TD3（护照，2行×44字符）。
class MrzParser {
  MrzParser._();

  /// 解析 MRZ 文本行
  ///
  /// 根据行数和每行长度自动判断格式（TD1/TD2/TD3）。
  static MrzData? parse(List<String> lines) {
    // 标准化：去除首尾空白，转为大写
    final normalized = lines
        .map((l) => l.trim().toUpperCase())
        .where((l) => l.isNotEmpty)
        .toList();

    if (normalized.length == 2 && normalized.every((l) => l.length == 44)) {
      return _parseTD3(normalized[0], normalized[1]);
    } else if (normalized.length == 3 &&
        normalized.every((l) => l.length == 30)) {
      return _parseTD1(normalized[0], normalized[1], normalized[2]);
    } else if (normalized.length == 2 &&
        normalized.every((l) => l.length == 36)) {
      return _parseTD2(normalized[0], normalized[1]);
    }

    return null;
  }

  // ==========================================================================
  // TD3: 护照（2 行 × 44 字符）
  // ==========================================================================

  static MrzData? _parseTD3(String line1, String line2) {
    if (line1.length != 44 || line2.length != 44) return null;

    final docType = line1.substring(0, 2);
    final country = line1.substring(2, 5);
    final names = _parseNames(line1.substring(5));

    final docNumber = line2.substring(0, 9).replaceAll('<', '');
    final checkDigitDoc = line2[9];
    final nationality = line2.substring(10, 13);
    final dob = line2.substring(13, 19);
    final checkDigitDob = line2[19];
    final sex = line2[20];
    final expiry = line2.substring(21, 27);
    final checkDigitExp = line2[27];

    // 校验位验证（失败不阻断，仅降低置信度）
    var confidence = 1.0;
    if (!_validateCheckDigit(docNumber, checkDigitDoc)) confidence -= 0.1;
    if (!_validateCheckDigit(dob, checkDigitDob)) confidence -= 0.1;
    if (!_validateCheckDigit(expiry, checkDigitExp)) confidence -= 0.1;

    return MrzData(
      documentType: docType,
      country: country,
      surname: names.$1,
      givenNames: names.$2,
      documentNumber: docNumber,
      nationality: nationality,
      dateOfBirth: dob,
      sex: sex,
      expiryDate: expiry,
      confidence: confidence,
      rawLines: [line1, line2],
    );
  }

  // ==========================================================================
  // TD1: 身份证（3 行 × 30 字符）
  // ==========================================================================

  static MrzData? _parseTD1(String line1, String line2, String line3) {
    if (line1.length != 30 || line2.length != 30 || line3.length != 30) {
      return null;
    }

    final docType = line1.substring(0, 2);
    final country = line1.substring(2, 5);
    final names = _parseNames(line1.substring(5));

    final docNumber = line2.substring(0, 9).replaceAll('<', '');
    final checkDigitDoc = line2[9];
    final dob = line2.substring(10, 16);
    final checkDigitDob = line2[16];
    final sex = line2[17];
    final expiry = line2.substring(18, 24);
    final checkDigitExp = line2[24];

    var confidence = 1.0;
    if (!_validateCheckDigit(docNumber, checkDigitDoc)) confidence -= 0.1;
    if (!_validateCheckDigit(dob, checkDigitDob)) confidence -= 0.1;
    if (!_validateCheckDigit(expiry, checkDigitExp)) confidence -= 0.1;

    return MrzData(
      documentType: docType,
      country: country,
      surname: names.$1,
      givenNames: names.$2,
      documentNumber: docNumber,
      nationality: country, // TD1 国籍通常与签发国相同
      dateOfBirth: dob,
      sex: sex,
      expiryDate: expiry,
      confidence: confidence,
      rawLines: [line1, line2, line3],
    );
  }

  // ==========================================================================
  // TD2: 部分证件（2 行 × 36 字符）
  // ==========================================================================

  static MrzData? _parseTD2(String line1, String line2) {
    if (line1.length != 36 || line2.length != 36) return null;

    final docType = line1.substring(0, 2);
    final country = line1.substring(2, 5);
    final names = _parseNames(line1.substring(5));

    final docNumber = line2.substring(0, 9).replaceAll('<', '');
    final checkDigitDoc = line2[9];
    final nationality = line2.substring(10, 13);
    final dob = line2.substring(13, 19);
    final checkDigitDob = line2[19];
    final sex = line2[20];
    final expiry = line2.substring(21, 27);
    final checkDigitExp = line2[27];

    var confidence = 1.0;
    if (!_validateCheckDigit(docNumber, checkDigitDoc)) confidence -= 0.1;
    if (!_validateCheckDigit(dob, checkDigitDob)) confidence -= 0.1;
    if (!_validateCheckDigit(expiry, checkDigitExp)) confidence -= 0.1;

    return MrzData(
      documentType: docType,
      country: country,
      surname: names.$1,
      givenNames: names.$2,
      documentNumber: docNumber,
      nationality: nationality,
      dateOfBirth: dob,
      sex: sex,
      expiryDate: expiry,
      confidence: confidence,
      rawLines: [line1, line2],
    );
  }

  // ==========================================================================
  // 辅助函数
  // ==========================================================================

  /// 解析姓名部分（以 << 分隔姓和名）
  static (String, String) _parseNames(String nameField) {
    final parts = nameField.split('<<');
    final surname = parts[0].replaceAll('<', ' ').trim();
    final givenNames = parts.length > 1
        ? parts.sublist(1).join(' ').replaceAll('<', ' ').trim()
        : '';
    return (surname, givenNames);
  }

  /// 校验位验证（权重 7, 3, 1 循环）
  static bool _validateCheckDigit(String data, String checkDigit) {
    if (checkDigit == '<') return true;

    final weights = [7, 3, 1];
    var sum = 0;

    for (var i = 0; i < data.length; i++) {
      final c = data[i];
      int val;
      if (c.compareTo('0') >= 0 && c.compareTo('9') <= 0) {
        val = c.codeUnitAt(0) - '0'.codeUnitAt(0);
      } else if (c.compareTo('A') >= 0 && c.compareTo('Z') <= 0) {
        val = c.codeUnitAt(0) - 'A'.codeUnitAt(0) + 10;
      } else if (c == '<') {
        val = 0;
      } else {
        return false;
      }
      sum += val * weights[i % 3];
    }

    final expected = sum % 10;
    final actual = int.tryParse(checkDigit);
    return actual == expected;
  }
}
