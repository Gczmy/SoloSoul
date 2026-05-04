// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'scan_result_model.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ScannedFile _$ScannedFileFromJson(Map<String, dynamic> json) => ScannedFile(
  path: json['path'] as String,
  name: json['name'] as String,
  size: (json['size'] as num).toInt(),
  modifiedAt: (json['modifiedAt'] as num).toInt(),
  extension: json['extension'] as String,
  mimeType: json['mimeType'] as String?,
);

Map<String, dynamic> _$ScannedFileToJson(ScannedFile instance) =>
    <String, dynamic>{
      'path': instance.path,
      'name': instance.name,
      'size': instance.size,
      'modifiedAt': instance.modifiedAt,
      'extension': instance.extension,
      'mimeType': instance.mimeType,
    };

ScanResult _$ScanResultFromJson(Map<String, dynamic> json) => ScanResult(
  meta: ScanMeta.fromJson(json['meta'] as Map<String, dynamic>),
  sections: (json['sections'] as List<dynamic>)
      .map((e) => ScanSection.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$ScanResultToJson(ScanResult instance) =>
    <String, dynamic>{
      'meta': instance.meta.toJson(),
      'sections': instance.sections.map((e) => e.toJson()).toList(),
    };

ScanMeta _$ScanMetaFromJson(Map<String, dynamic> json) => ScanMeta(
  scanId: json['scanId'] as String,
  createdAt: (json['createdAt'] as num).toInt(),
  sourceFile: json['sourceFile'] as String,
  confidence: (json['confidence'] as num).toDouble(),
  fileType: json['fileType'] as String?,
);

Map<String, dynamic> _$ScanMetaToJson(ScanMeta instance) => <String, dynamic>{
  'scanId': instance.scanId,
  'createdAt': instance.createdAt,
  'sourceFile': instance.sourceFile,
  'confidence': instance.confidence,
  'fileType': instance.fileType,
};

ScanSection _$ScanSectionFromJson(Map<String, dynamic> json) => ScanSection(
  section: json['section'] as String,
  display: json['display'] as String,
  fields: (json['fields'] as List<dynamic>)
      .map((e) => ScanField.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$ScanSectionToJson(ScanSection instance) =>
    <String, dynamic>{
      'section': instance.section,
      'display': instance.display,
      'fields': instance.fields.map((e) => e.toJson()).toList(),
    };

ScanField _$ScanFieldFromJson(Map<String, dynamic> json) => ScanField(
  key: json['key'] as String,
  value: json['value'] as String,
  sensitivity: $enumDecode(_$SensitivityLevelEnumMap, json['sensitivity']),
  confidence: (json['confidence'] as num?)?.toDouble(),
);

Map<String, dynamic> _$ScanFieldToJson(ScanField instance) => <String, dynamic>{
  'key': instance.key,
  'value': instance.value,
  'sensitivity': _$SensitivityLevelEnumMap[instance.sensitivity]!,
  'confidence': instance.confidence,
};

const _$SensitivityLevelEnumMap = {
  SensitivityLevel.public: 'public',
  SensitivityLevel.internal: 'internal',
  SensitivityLevel.sensitive: 'sensitive',
  SensitivityLevel.critical: 'critical',
};
