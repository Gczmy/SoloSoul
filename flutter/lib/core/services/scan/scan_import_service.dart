import 'dart:async';

import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/scan/local_search_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

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

      final parentSectionId = _findParentSectionId(typeId);

      // Separate fields by action
      final fieldsToWrite = candidate.fields.where((f) {
        return f.userAction == ImportAction.autoFill ||
            f.userAction == ImportAction.overwrite ||
            f.userAction == ImportAction.createNew;
      }).toList();

      final fieldsToSkip = candidate.fields.where((f) {
        return f.userAction == ImportAction.skip;
      }).toList();

      fieldsSkipped += fieldsToSkip.length;

      if (fieldsToWrite.isEmpty) continue;

      // Build properties map
      final properties = <String, PropertyValue>{};
      for (final field in fieldsToWrite) {
        final propertyId = field.targetPropertyId ?? field.source.key;
        final sensitivity = field.source.sensitivity;
        properties[propertyId] = TextProperty(
          text: field.source.value,
          sensitivity: sensitivity,
        );
        fieldsWritten++;
      }

      if (candidate.existingObjectId != null) {
        // Update existing object
        final existing = UnifiedObjectService.instance.getObjectById(
          _objects,
          candidate.existingObjectId!,
        );
        if (existing != null) {
          // Preserve original sensitivity levels when updating — the scan-detected
          // sensitivity reflects the *content* (e.g. phone = sensitive) but the
          // item's schema sensitivity (e.g. contact.value = internal) should not
          // be overwritten, otherwise non-sensitive items start requiring password.
          final mergedProperties = <String, PropertyValue>{
            ...existing.properties,
            for (final entry in properties.entries)
              entry.key: _preserveSensitivity(
                existing.properties[entry.key],
                entry.value,
              ),
          };
          await _objectNotifier.updateObject(
            candidate.existingObjectId!,
            properties: mergedProperties,
          );
          itemsUpdated++;
        } else {
          warnings.add('Existing object not found: ${candidate.existingObjectId}');
        }
      } else {
        // Create new object (auto-creates missing section/page via createDefaultItem)
        final name = candidate.source.display;
        if (parentSectionId != null) {
          await _objectNotifier.createDefaultItem(
            sectionId: parentSectionId,
            typeId: typeId,
            name: name,
            properties: properties,
          );
        } else {
          await _objectNotifier.createObject(
            name: name,
            typeId: typeId,
            properties: properties,
          );
        }
        itemsCreated++;
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

  String? _findParentSectionId(String itemTypeId) {
    // Map item type to its default section
    const typeToSection = {
      'profile_identity': '__section_identity',
      'profile_contact': '__section_contact',
      'profile_id_card': '__section_id_card',
      'profile_address': '__section_address',
      'travel_passport': '__section_passport',
      'travel_visa': '__section_visa',
      'travel_history': '__section_travel_history',
      'financial_bank_account': '__section_bank_account',
      'financial_card': '__section_card',
      'financial_tax_id': '__section_tax_id',
      'professional_education': '__section_education',
      'professional_employment': '__section_employment',
      'professional_skill': '__section_skill',
      'professional_language': '__section_language',
      'professional_award': '__section_award',
    };
    return typeToSection[itemTypeId];
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
  static PropertyValue _preserveSensitivity(
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
      SelectProperty() => imported,
      MultiSelectProperty() => imported,
      RelationProperty() => imported,
      UrlProperty(:final url) => UrlProperty(url: url, sensitivity: sensitivity),
    };
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
