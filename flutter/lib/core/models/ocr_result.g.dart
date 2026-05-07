// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ocr_result.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BoundingBox _$BoundingBoxFromJson(Map<String, dynamic> json) => BoundingBox(
  x: (json['x'] as num).toDouble(),
  y: (json['y'] as num).toDouble(),
  width: (json['width'] as num).toDouble(),
  height: (json['height'] as num).toDouble(),
);

Map<String, dynamic> _$BoundingBoxToJson(BoundingBox instance) =>
    <String, dynamic>{
      'x': instance.x,
      'y': instance.y,
      'width': instance.width,
      'height': instance.height,
    };

OcrBlock _$OcrBlockFromJson(Map<String, dynamic> json) => OcrBlock(
  text: json['text'] as String,
  confidence: (json['confidence'] as num).toDouble(),
  bbox: BoundingBox.fromJson(json['bbox'] as Map<String, dynamic>),
);

Map<String, dynamic> _$OcrBlockToJson(OcrBlock instance) => <String, dynamic>{
  'text': instance.text,
  'confidence': instance.confidence,
  'bbox': instance.bbox,
};

OcrResult _$OcrResultFromJson(Map<String, dynamic> json) => OcrResult(
  rawText: json['rawText'] as String,
  blocks: (json['blocks'] as List<dynamic>)
      .map((e) => OcrBlock.fromJson(e as Map<String, dynamic>))
      .toList(),
  confidence: (json['confidence'] as num).toDouble(),
);

Map<String, dynamic> _$OcrResultToJson(OcrResult instance) => <String, dynamic>{
  'rawText': instance.rawText,
  'blocks': instance.blocks,
  'confidence': instance.confidence,
};

MrzData _$MrzDataFromJson(Map<String, dynamic> json) => MrzData(
  documentType: json['documentType'] as String,
  country: json['country'] as String,
  surname: json['surname'] as String,
  givenNames: json['givenNames'] as String,
  documentNumber: json['documentNumber'] as String,
  nationality: json['nationality'] as String,
  dateOfBirth: json['dateOfBirth'] as String,
  sex: json['sex'] as String,
  expiryDate: json['expiryDate'] as String,
  confidence: (json['confidence'] as num).toDouble(),
  rawLines: (json['rawLines'] as List<dynamic>)
      .map((e) => e as String)
      .toList(),
);

Map<String, dynamic> _$MrzDataToJson(MrzData instance) => <String, dynamic>{
  'documentType': instance.documentType,
  'country': instance.country,
  'surname': instance.surname,
  'givenNames': instance.givenNames,
  'documentNumber': instance.documentNumber,
  'nationality': instance.nationality,
  'dateOfBirth': instance.dateOfBirth,
  'sex': instance.sex,
  'expiryDate': instance.expiryDate,
  'confidence': instance.confidence,
  'rawLines': instance.rawLines,
};
