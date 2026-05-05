import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

part 'scan_result_model.g.dart';

// =============================================================================
// Scan Result Data Models
// =============================================================================

/// Represents a single file discovered during scanning.
@JsonSerializable(explicitToJson: true)
class ScannedFile {
  final String path;
  final String name;
  final int size;
  final int modifiedAt;
  final String extension;
  final String? mimeType;

  const ScannedFile({
    required this.path,
    required this.name,
    required this.size,
    required this.modifiedAt,
    required this.extension,
    this.mimeType,
  });

  factory ScannedFile.fromJson(Map<String, dynamic> json) =>
      _$ScannedFileFromJson(json);

  Map<String, dynamic> toJson() => _$ScannedFileToJson(this);

  ScannedFile copyWith({
    String? path,
    String? name,
    int? size,
    int? modifiedAt,
    String? extension,
    String? mimeType,
  }) {
    return ScannedFile(
      path: path ?? this.path,
      name: name ?? this.name,
      size: size ?? this.size,
      modifiedAt: modifiedAt ?? this.modifiedAt,
      extension: extension ?? this.extension,
      mimeType: mimeType ?? this.mimeType,
    );
  }
}

/// Top-level scan result for a single source file.
@JsonSerializable(explicitToJson: true)
class ScanResult {
  final ScanMeta meta;
  final List<ScanSection> sections;

  const ScanResult({
    required this.meta,
    required this.sections,
  });

  factory ScanResult.fromJson(Map<String, dynamic> json) =>
      _$ScanResultFromJson(json);

  Map<String, dynamic> toJson() => _$ScanResultToJson(this);

  ScanResult copyWith({
    ScanMeta? meta,
    List<ScanSection>? sections,
  }) {
    return ScanResult(
      meta: meta ?? this.meta,
      sections: sections ?? this.sections,
    );
  }
}

/// Metadata for a scan result.
@JsonSerializable(explicitToJson: true)
class ScanMeta {
  final String scanId;
  final int createdAt;
  final String sourceFile;
  final double confidence;
  final String? fileType;

  const ScanMeta({
    required this.scanId,
    required this.createdAt,
    required this.sourceFile,
    required this.confidence,
    this.fileType,
  });

  factory ScanMeta.fromJson(Map<String, dynamic> json) =>
      _$ScanMetaFromJson(json);

  Map<String, dynamic> toJson() => _$ScanMetaToJson(this);

  ScanMeta copyWith({
    String? scanId,
    int? createdAt,
    String? sourceFile,
    double? confidence,
    String? fileType,
  }) {
    return ScanMeta(
      scanId: scanId ?? this.scanId,
      createdAt: createdAt ?? this.createdAt,
      sourceFile: sourceFile ?? this.sourceFile,
      confidence: confidence ?? this.confidence,
      fileType: fileType ?? this.fileType,
    );
  }
}

/// A section within a scan result (e.g. identity, education, passport).
@JsonSerializable(explicitToJson: true)
class ScanSection {
  final String section;
  final String display;
  final List<ScanField> fields;

  const ScanSection({
    required this.section,
    required this.display,
    required this.fields,
  });

  factory ScanSection.fromJson(Map<String, dynamic> json) =>
      _$ScanSectionFromJson(json);

  Map<String, dynamic> toJson() => _$ScanSectionToJson(this);

  ScanSection copyWith({
    String? section,
    String? display,
    List<ScanField>? fields,
  }) {
    return ScanSection(
      section: section ?? this.section,
      display: display ?? this.display,
      fields: fields ?? this.fields,
    );
  }
}

/// A single field extracted from a scanned file.
@JsonSerializable(explicitToJson: true)
class ScanField {
  final String key;
  final String value;
  final SensitivityLevel sensitivity;
  final double? confidence;

  const ScanField({
    required this.key,
    required this.value,
    required this.sensitivity,
    this.confidence,
  });

  factory ScanField.fromJson(Map<String, dynamic> json) =>
      _$ScanFieldFromJson(json);

  Map<String, dynamic> toJson() => _$ScanFieldToJson(this);

  ScanField copyWith({
    String? key,
    String? value,
    SensitivityLevel? sensitivity,
    double? confidence,
  }) {
    return ScanField(
      key: key ?? this.key,
      value: value ?? this.value,
      sensitivity: sensitivity ?? this.sensitivity,
      confidence: confidence ?? this.confidence,
    );
  }
}

// =============================================================================
// Import Pipeline Models
// =============================================================================

/// Represents a candidate for import derived from a ScanSection.
class ImportCandidate {
  final ScanSection source;
  String? existingObjectId; // null = needs creation
  final List<ImportFieldCandidate> fields;
  bool isSelected;

  ImportCandidate({
    required this.source,
    this.existingObjectId,
    required this.fields,
    this.isSelected = true,
  });
}

/// Represents a single field candidate for import.
class ImportFieldCandidate {
  final ScanField source;
  final String? targetPropertyId;
  final ImportAction suggestedAction;
  ImportAction userAction;

  /// 映射来源：'rule' 规则引擎 | 'llm' AI 建议 | 'both' 两者一致。
  final String mappingSource;

  /// 映射置信度（0.0 ~ 1.0）。规则引擎默认为 1.0。
  final double mappingConfidence;

  ImportFieldCandidate({
    required this.source,
    this.targetPropertyId,
    required this.suggestedAction,
    ImportAction? userAction,
    this.mappingSource = 'rule',
    this.mappingConfidence = 1.0,
  }) : userAction = userAction ?? suggestedAction;
}

/// Action to take for an import field.
enum ImportAction {
  autoFill,   // vault field is empty, auto-fill
  skip,       // vault already has same value, skip
  overwrite,  // vault has different value, overwrite
  createNew,  // new field, create
}

/// Result of a scan import operation.
class ScanImportResult {
  final int itemsCreated;
  final int itemsUpdated;
  final int fieldsWritten;
  final int fieldsSkipped;
  final List<String> warnings;

  const ScanImportResult({
    this.itemsCreated = 0,
    this.itemsUpdated = 0,
    this.fieldsWritten = 0,
    this.fieldsSkipped = 0,
    this.warnings = const [],
  });

  ScanImportResult copyWith({
    int? itemsCreated,
    int? itemsUpdated,
    int? fieldsWritten,
    int? fieldsSkipped,
    List<String>? warnings,
  }) {
    return ScanImportResult(
      itemsCreated: itemsCreated ?? this.itemsCreated,
      itemsUpdated: itemsUpdated ?? this.itemsUpdated,
      fieldsWritten: fieldsWritten ?? this.fieldsWritten,
      fieldsSkipped: fieldsSkipped ?? this.fieldsSkipped,
      warnings: warnings ?? this.warnings,
    );
  }
}
