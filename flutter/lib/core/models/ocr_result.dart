import 'package:json_annotation/json_annotation.dart';

part 'ocr_result.g.dart';

// ============================================================================
// OCR 识别结果模型
// ============================================================================

/// 边界框（相对坐标 0.0~1.0）
@JsonSerializable()
class BoundingBox {
  final double x;
  final double y;
  final double width;
  final double height;

  const BoundingBox({
    required this.x,
    required this.y,
    required this.width,
    required this.height,
  });

  factory BoundingBox.fromJson(Map<String, dynamic> json) =>
      _$BoundingBoxFromJson(json);

  Map<String, dynamic> toJson() => _$BoundingBoxToJson(this);
}

/// OCR 文本块
@JsonSerializable()
class OcrBlock {
  final String text;
  final double confidence;
  final BoundingBox bbox;

  const OcrBlock({
    required this.text,
    required this.confidence,
    required this.bbox,
  });

  factory OcrBlock.fromJson(Map<String, dynamic> json) =>
      _$OcrBlockFromJson(json);

  Map<String, dynamic> toJson() => _$OcrBlockToJson(this);
}

/// OCR 通用识别结果
@JsonSerializable()
class OcrResult {
  final String rawText;
  final List<OcrBlock> blocks;
  final double confidence;

  const OcrResult({
    required this.rawText,
    required this.blocks,
    required this.confidence,
  });

  factory OcrResult.fromJson(Map<String, dynamic> json) =>
      _$OcrResultFromJson(json);

  Map<String, dynamic> toJson() => _$OcrResultToJson(this);
}

// ============================================================================
// MRZ 解析结果模型
// ============================================================================

/// MRZ 解析后的结构化数据
@JsonSerializable()
class MrzData {
  final String documentType;
  final String country;
  final String surname;
  final String givenNames;
  final String documentNumber;
  final String nationality;
  final String dateOfBirth;
  final String sex;
  final String expiryDate;
  final double confidence;
  final List<String> rawLines;

  const MrzData({
    required this.documentType,
    required this.country,
    required this.surname,
    required this.givenNames,
    required this.documentNumber,
    required this.nationality,
    required this.dateOfBirth,
    required this.sex,
    required this.expiryDate,
    required this.confidence,
    required this.rawLines,
  });

  factory MrzData.fromJson(Map<String, dynamic> json) =>
      _$MrzDataFromJson(json);

  Map<String, dynamic> toJson() => _$MrzDataToJson(this);
}


