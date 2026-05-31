import 'dart:async';
import 'dart:io';

import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_field_mapping_parser.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/scan/local_search_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Scan Import Service
// =============================================================================

/// Maps scan results to Vault objects, detects conflicts, and executes imports.
class ScanImportService {
  final UnifiedObjectNotifier _objectNotifier;
  final List<UnifiedObject> _objects;

  ScanImportService(this._objectNotifier, this._objects);

  // ---------------------------------------------------------------------------
  // Step 1: Map scan results to import candidates
  // ---------------------------------------------------------------------------

  List<ImportCandidate> mapScanResult(ScanResult result) {
    final candidates = <ImportCandidate>[];

    for (final section in result.sections) {
      final typeId = LocalSearchService.mapSectionToTypeId(section.section);
      if (typeId == null) continue;

      // Find existing object of this type
      final existing = _findExistingObject(typeId);

      final fieldCandidates = <ImportFieldCandidate>[];
      for (final field in section.fields) {
        final propertyId = LocalSearchService.mapFieldToPropertyId(
              section.section,
              field.key,
            ) ??
            field.key;

        fieldCandidates.add(ImportFieldCandidate(
          source: field,
          targetPropertyId: propertyId,
          suggestedAction: ImportAction.createNew,
        ));
      }

      candidates.add(ImportCandidate(
        source: section,
        existingObjectId: existing?.id,
        fields: fieldCandidates,
        sourceFilePath: result.meta.sourceFile,
      ));
    }

    return candidates;
  }

  // ---------------------------------------------------------------------------
  // Step 1b: AI-assisted field mapping (optional enhancement)
  // ---------------------------------------------------------------------------

  /// 将规则引擎结果与 LLM 建议按优先级合并。
  ///
  /// 优先级：规则引擎高置信度 > LLM 高置信度 (≥0.8) > LLM 低置信度 > 规则引擎默认。
  /// LLM 建议的字段标记 `mappingSource = 'llm'`，绝不静默替换用户已确认的映射。
  List<ImportCandidate> mapScanResultWithLlm(
    ScanResult result,
    LlmFieldMappingResult llmResult, {
    String llmSource = 'local',
  }) {
    final candidates = <ImportCandidate>[];

    for (final section in result.sections) {
      final typeId = LocalSearchService.mapSectionToTypeId(section.section);
      if (typeId == null) continue;

      final existing = _findExistingObject(typeId);
      final fieldCandidates = <ImportFieldCandidate>[];

      for (final field in section.fields) {
        // 1. 规则引擎映射
        final rulePropertyId = LocalSearchService.mapFieldToPropertyId(
              section.section,
              field.key,
            ) ??
            field.key;

        // 2. 查找 LLM 建议
        // 匹配优先级：精确 key > 精确短 value（≤50 字符）> 忽略大小写 key
        LlmFieldSuggestion? llmSuggestion;
        for (final s in llmResult.mappings) {
          final src = s.sourceField;
          if (src == field.key) {
            llmSuggestion = s;
            break;
          }
          if (field.value.length <= 50 && src == field.value) {
            llmSuggestion = s;
            break;
          }
          if (src.toLowerCase() == field.key.toLowerCase()) {
            llmSuggestion = s;
            break;
          }
        }

        // 3. 合并决策
        String finalPropertyId = rulePropertyId;
        String mappingSource = 'rule';
        double confidence = 1.0;

        if (llmSuggestion != null) {
          final targetId = llmSuggestion.targetPropertyId;
          if (targetId != null && targetId.isNotEmpty) {
            if (llmSuggestion.confidence >= 0.8) {
              // LLM 高置信度：优先采用，若与规则不同则标记为 both
              finalPropertyId = targetId;
              if (finalPropertyId == rulePropertyId) {
                mappingSource = 'both';
              } else {
                mappingSource = 'llm';
              }
              confidence = llmSuggestion.confidence;
            } else {
              // LLM 低置信度：保留规则引擎，但记录 LLM 建议供 UI 展示
              mappingSource = 'rule';
              confidence = 1.0;
              // 注意：低置信度 LLM 建议不覆盖规则结果，
              // UI 层可通过逐字段 AI 按钮单独触发高精度映射
            }
          }
        }

        fieldCandidates.add(ImportFieldCandidate(
          source: field,
          targetPropertyId: finalPropertyId,
          suggestedAction: ImportAction.createNew,
          mappingSource: mappingSource,
          mappingConfidence: confidence,
        ));
      }

      candidates.add(ImportCandidate(
        source: section,
        existingObjectId: existing?.id,
        fields: fieldCandidates,
        sourceFilePath: result.meta.sourceFile,
      ));
    }

    return candidates;
  }

  // ---------------------------------------------------------------------------
  // Step 2: Detect conflicts with existing Vault data
  // ---------------------------------------------------------------------------

  /// Identity fields used to determine if a scanned item and an existing Vault
  /// object represent the same entity (e.g. same bank, same person).
  static const List<String> _kIdentityPropertyKeys = [
    'title',
    'name',
    'fullName',
    'bankName',
    'institution',
    'company',
  ];

  List<ImportConflict> detectConflicts(List<ImportCandidate> candidates) {
    final conflicts = <ImportConflict>[];

    for (final candidate in candidates) {
      var existingId = candidate.existingObjectId;
      if (existingId == null) continue;

      final existing = UnifiedObjectService.instance.getObjectById(
        _objects,
        existingId,
      );
      if (existing == null) {
        candidate.existingObjectId = null;
        continue;
      }

      // -----------------------------------------------------------------------
      // Entity identity check: if the scanned item has an identity property
      // (e.g. bankName) and the existing object has a different non-empty value
      // for the same property, they are NOT the same entity.
      // In that case treat as a new object instead of updating the wrong one.
      // -----------------------------------------------------------------------
      if (!_isSameEntity(candidate, existing)) {
        candidate.existingObjectId = null;
        continue;
      }

      for (final field in candidate.fields) {
        final propertyId = field.targetPropertyId;
        if (propertyId == null) continue;

        final existingValue = existing.properties[propertyId];
        if (existingValue == null) {
          // Field does not exist yet in the existing object: create it.
          // Do NOT auto-fill empty fields of a matched entity — the user
          // should explicitly confirm via the preview UI.
          continue;
        }

        // Extract text from existing PropertyValue
        final existingText = _extractText(existingValue);
        if (existingText == null) continue;

        if (existingText == field.source.value) {
          // Same value: skip
          field.userAction = ImportAction.skip;
        } else if (existingText.isNotEmpty) {
          // Different value: conflict
          field.userAction = ImportAction.overwrite;
          conflicts.add(ImportConflict(
            candidate: candidate,
            field: field,
            existingValue: existingText,
            scannedValue: field.source.value,
          ));
        }
      }
    }

    return conflicts;
  }

  /// Check whether the scanned candidate and the existing Vault object
  /// represent the same logical entity (e.g. same bank account, same school).
  /// Returns `true` when:
  ///   - no identity property is present in the scan, OR
  ///   - the identity property value matches the existing object, OR
  ///   - the existing object has no value for that identity property.
  /// Returns `false` when both have a non-empty identity value and they differ.
  bool _isSameEntity(ImportCandidate candidate, UnifiedObject existing) {
    for (final key in _kIdentityPropertyKeys) {
      ImportFieldCandidate? scannedField;
      for (final f in candidate.fields) {
        if ((f.targetPropertyId ?? f.source.key) == key) {
          scannedField = f;
          break;
        }
      }
      if (scannedField == null) continue;

      final scannedValue = scannedField.source.value.trim();
      if (scannedValue.isEmpty) continue;

      final existingValue = existing.properties[key];
      if (existingValue == null) continue;

      final existingText = _extractText(existingValue)?.trim() ?? '';
      if (existingText.isEmpty) continue;

      // Both have a non-empty identity value — they must match.
      if (existingText != scannedValue) return false;
    }
    return true;
  }

  // ---------------------------------------------------------------------------
  // Step 3: Execute import (after user confirmation)
  // ---------------------------------------------------------------------------

  Future<ScanImportResult> executeImport(
    List<ImportCandidate> confirmedCandidates, {
    ConflictResolution defaultResolution = ConflictResolution.skip,
    String? accountId,
  }) async {
    var itemsCreated = 0;
    var itemsUpdated = 0;
    var fieldsWritten = 0;
    var fieldsSkipped = 0;
    final warnings = <String>[];

    for (final candidate in confirmedCandidates) {
      if (!candidate.isSelected) {
        fieldsSkipped += candidate.fields.length;
        continue;
      }

      final typeId = LocalSearchService.mapSectionToTypeId(candidate.source.section);
      if (typeId == null) {
        warnings.add('Unknown section type: ${candidate.source.section}');
        continue;
      }

      final fieldsToWrite = _fieldsToWrite(candidate.fields);
      fieldsSkipped += candidate.fields.length - fieldsToWrite.length;

      if (fieldsToWrite.isEmpty) continue;

      final properties = _buildProperties(fieldsToWrite, candidate.source.section, () => fieldsWritten++);
      final completeProperties = _ensureSchemaFields(
        properties,
        typeId,
        candidate.source.section,
      );

      String? targetId = candidate.existingObjectId;
      if (candidate.existingObjectId != null) {
        final updated = await _updateExisting(candidate, completeProperties, warnings);
        if (updated) itemsUpdated++;
      } else {
        targetId = await _objectNotifier.createObjectAndReturnId(
          name: candidate.source.display,
          typeId: typeId,
          properties: completeProperties,
        );
        if (targetId != null) itemsCreated++;
      }

      if (candidate.attachOriginalFile &&
          candidate.sourceFilePath != null &&
          targetId != null &&
          accountId != null) {
        await _attachSourceFile(
          accountId: accountId,
          objectId: targetId,
          sourceFilePath: candidate.sourceFilePath!,
        );
      }
    }

    return ScanImportResult(
      itemsCreated: itemsCreated,
      itemsUpdated: itemsUpdated,
      fieldsWritten: fieldsWritten,
      fieldsSkipped: fieldsSkipped,
      warnings: warnings,
    );
  }

  List<ImportFieldCandidate> _fieldsToWrite(List<ImportFieldCandidate> fields) {
    return fields.where((f) {
      return f.userAction == ImportAction.autoFill ||
          f.userAction == ImportAction.overwrite ||
          f.userAction == ImportAction.createNew;
    }).toList();
  }

  Map<String, PropertyValue> _buildProperties(
    List<ImportFieldCandidate> fields,
    String sectionId,
    void Function() onWrite,
  ) {
    final properties = <String, PropertyValue>{};
    for (final field in fields) {
      final propertyId = field.targetPropertyId ?? field.source.key;
      // 使用 Schema 定义的敏感度作为唯一真理来源
      // 扫描检测的敏感度仅作为 UI 提示，不用于存储
      final schemaSensitivity = LocalSearchService.getDefaultSensitivity(
        sectionId,
        propertyId,
      );
      properties[propertyId] = TextProperty(
        text: field.source.value,
        sensitivity: schemaSensitivity,
      );
      onWrite();
    }
    return properties;
  }

  Future<bool> _updateExisting(
    ImportCandidate candidate,
    Map<String, PropertyValue> properties,
    List<String> warnings,
  ) async {
    final existing = UnifiedObjectService.instance.getObjectById(
      _objects,
      candidate.existingObjectId!,
    );
    if (existing == null) {
      warnings.add('Existing object not found: ${candidate.existingObjectId}');
      return false;
    }
    final mergedProperties = <String, PropertyValue>{
      ...existing.properties,
      for (final entry in properties.entries)
        entry.key: preserveSensitivity(
          existing.properties[entry.key],
          entry.value,
        ),
    };
    await _objectNotifier.updateObject(
      candidate.existingObjectId!,
      properties: mergedProperties,
    );
    return true;
  }

  /// Ensure all schema-defined properties exist, filling missing ones with empty values.
  Map<String, PropertyValue> _ensureSchemaFields(
    Map<String, PropertyValue> properties,
    String typeId,
    String sectionId,
  ) {
    final type = ObjectTypeRegistry.getType(typeId);
    if (type == null) return properties;

    final result = Map<String, PropertyValue>.from(properties);
    for (final propDef in type.properties) {
      if (result.containsKey(propDef.id)) continue;
      if (propDef.id == 'Title' || propDef.id == 'Item Name') continue;

      result[propDef.id] = TextProperty(
        text: '',
        sensitivity: LocalSearchService.getDefaultSensitivity(sectionId, propDef.id),
      );
    }
    return result;
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  UnifiedObject? _findExistingObject(String typeId) {
    for (final obj in _objects) {
      if (obj.typeId == typeId && !obj.isDeleted) {
        return obj;
      }
    }
    return null;
  }

  String? _extractText(PropertyValue value) {
    if (value is TextProperty) return value.text;
    if (value is NumberProperty) return value.value?.toString();
    if (value is DateProperty) return value.isoDate;
    if (value is UrlProperty) return value.url;
    return null;
  }

  /// When updating an existing item, preserve the original property's sensitivity
  /// level so that scan-detected content sensitivity (e.g. phone = sensitive)
  /// does not overwrite the schema-defined sensitivity (e.g. contact.value = internal).
  static PropertyValue preserveSensitivity(
    PropertyValue? existing,
    PropertyValue imported,
  ) {
    if (existing == null) return imported;
    final sensitivity = existing.sensitivity;
    return switch (imported) {
      TextProperty(:final text) => TextProperty(text: text, sensitivity: sensitivity),
      NumberProperty(:final value) => NumberProperty(value: value, sensitivity: sensitivity),
      DateProperty(:final isoDate) => DateProperty(isoDate: isoDate, sensitivity: sensitivity),
      CheckboxProperty(:final checked) => CheckboxProperty(checked: checked, sensitivity: sensitivity),
      SelectProperty(:final options, :final selectedId) => SelectProperty(
          options: options,
          selectedId: selectedId,
          sensitivity: sensitivity,
        ),
      MultiSelectProperty(:final options, :final selectedIds) => MultiSelectProperty(
          options: options,
          selectedIds: selectedIds,
          sensitivity: sensitivity,
        ),
      RelationProperty(:final targetTypeId, :final targetObjectId) => RelationProperty(
          targetTypeId: targetTypeId,
          targetObjectId: targetObjectId,
          sensitivity: sensitivity,
        ),
      UrlProperty(:final url) => UrlProperty(url: url, sensitivity: sensitivity),
    };
  }

  Future<void> _attachSourceFile({
    required String accountId,
    required String objectId,
    required String sourceFilePath,
  }) async {
    try {
      final file = File(sourceFilePath);
      if (!await file.exists()) {
        SoloLog.w('ScanImport', 'Source file not found: $sourceFilePath');
        return;
      }
      final bytes = await file.readAsBytes();
      final attachment = await AttachmentStorageService().saveAttachment(
        accountId: accountId,
        fileName: file.uri.pathSegments.last,
        bytes: bytes,
      );
      final objects = _objectNotifier.currentObjects;
      final obj = UnifiedObjectService.instance.getObjectById(objects, objectId);
      if (obj != null) {
        await _objectNotifier.updateObject(
          objectId,
          attachments: [...obj.attachments, attachment],
        );
      }
    } on Exception catch (e, st) {
      SoloLog.e('ScanImport', 'Failed to attach source file', e, st);
    }
  }
}

// =============================================================================
// Import Conflict Model
// =============================================================================

class ImportConflict {
  final ImportCandidate candidate;
  final ImportFieldCandidate field;
  final String existingValue;
  final String scannedValue;

  ImportConflict({
    required this.candidate,
    required this.field,
    required this.existingValue,
    required this.scannedValue,
  });
}

enum ConflictResolution {
  skip,
  overwrite,
  createNew,
}
