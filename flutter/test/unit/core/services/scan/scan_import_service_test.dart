import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/llm/llm_field_mapping_parser.dart';
import 'package:solosoul_flutter/core/services/scan/scan_import_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

void main() {
  group('ScanImportService.mapScanResult', () {
    late ScanImportService service;

    setUp(() {
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, []);
    });

    ScanResult makeResult(List<ScanSection> sections) {
      return ScanResult(
        meta: ScanMeta(
          scanId: 's1',
          createdAt: DateTime.now().millisecondsSinceEpoch,
          sourceFile: '/test/resume.pdf',
          confidence: 0.9,
        ),
        sections: sections,
      );
    }

    test('maps identity section to profile_identity candidate', () {
      final result = makeResult([
        const ScanSection(
          section: 'identity',
          display: 'Identity',
          fields: [
            ScanField(key: 'fullName', value: 'Zhang San', sensitivity: SensitivityLevel.public),
            ScanField(key: 'dateOfBirth', value: '1990-01-01', sensitivity: SensitivityLevel.internal),
          ],
        ),
      ]);

      final candidates = service.mapScanResult(result);
      expect(candidates.length, 1);
      expect(candidates.first.source.section, 'identity');
      expect(candidates.first.existingObjectId, isNull);
      expect(candidates.first.fields.length, 2);
      expect(candidates.first.fields[0].targetPropertyId, 'fullName');
      expect(candidates.first.fields[1].targetPropertyId, 'dateOfBirth');
    });

    test('skips unknown sections', () {
      final result = makeResult([
        const ScanSection(
          section: 'unknown',
          display: 'Unknown',
          fields: [ScanField(key: 'x', value: 'y', sensitivity: SensitivityLevel.public)],
        ),
      ]);
      final candidates = service.mapScanResult(result);
      expect(candidates, isEmpty);
    });

    test('maps passport section to travel_passport', () {
      final result = makeResult([
        const ScanSection(
          section: 'passport',
          display: 'Passport',
          fields: [
            ScanField(key: 'number', value: 'E12345678', sensitivity: SensitivityLevel.critical),
          ],
        ),
      ]);
      final candidates = service.mapScanResult(result);
      expect(candidates.first.source.section, 'passport');
      expect(candidates.first.fields.first.targetPropertyId, 'number');
    });

    test('links candidate to existing object of same type', () {
      const existing = UnifiedObject(
        id: 'existing-1',
        typeId: 'profile_identity',
        name: 'Identity',
        iconName: 'person',
        parentId: 'section-1',
        childrenIds: [],
        properties: {},
        isDeleted: false,
        deletedAt: null,
        createdAt: 0,
        updatedAt: 0,
      );
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, [existing]);

      final result = makeResult([
        const ScanSection(
          section: 'identity',
          display: 'Identity',
          fields: [ScanField(key: 'fullName', value: 'Zhang San', sensitivity: SensitivityLevel.public)],
        ),
      ]);
      final candidates = service.mapScanResult(result);
      expect(candidates.first.existingObjectId, 'existing-1');
    });
  });

  group('ScanImportService.mapScanResultWithLlm', () {
    late ScanImportService service;

    setUp(() {
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, []);
    });

    ScanResult makeResult() {
      return ScanResult(
        meta: ScanMeta(
          scanId: 's1',
          createdAt: DateTime.now().millisecondsSinceEpoch,
          sourceFile: '/test/resume.pdf',
          confidence: 0.9,
        ),
        sections: [
          const ScanSection(
            section: 'identity',
            display: 'Identity',
            fields: [
              ScanField(key: 'fullName', value: 'Zhang San', sensitivity: SensitivityLevel.public),
              ScanField(key: 'nickname', value: 'San', sensitivity: SensitivityLevel.public),
            ],
          ),
        ],
      );
    }

    test('uses rule mapping when no LLM suggestion', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(mappings: [], unmapped: []);
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'fullName');
      expect(field.targetPropertyId, 'fullName');
      expect(field.mappingSource, 'rule');
      expect(field.mappingConfidence, 1.0);
    });

    test('overrides with LLM high-confidence suggestion', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(
        mappings: [
          LlmFieldSuggestion(
            sourceField: 'nickname',
            targetPropertyId: 'givenName',
            confidence: 0.9,
            reason: '昵称对应名字',
            source: 'llm',
          ),
        ],
        unmapped: [],
      );
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'nickname');
      expect(field.targetPropertyId, 'givenName');
      expect(field.mappingSource, 'llm');
      expect(field.mappingConfidence, 0.9);
    });

    test('marks both when LLM agrees with rule', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(
        mappings: [
          LlmFieldSuggestion(
            sourceField: 'fullName',
            targetPropertyId: 'fullName',
            confidence: 0.95,
            reason: '一致',
            source: 'llm',
          ),
        ],
        unmapped: [],
      );
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'fullName');
      expect(field.targetPropertyId, 'fullName');
      expect(field.mappingSource, 'both');
    });

    test('ignores low-confidence LLM suggestion', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(
        mappings: [
          LlmFieldSuggestion(
            sourceField: 'nickname',
            targetPropertyId: 'familyName',
            confidence: 0.5,
            reason: '不确定',
            source: 'llm',
          ),
        ],
        unmapped: [],
      );
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'nickname');
      expect(field.targetPropertyId, 'nickname'); // falls back to key
      expect(field.mappingSource, 'rule');
      expect(field.mappingConfidence, 1.0);
    });

    test('matches LLM suggestion by value for short values', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(
        mappings: [
          LlmFieldSuggestion(
            sourceField: 'Zhang San',
            targetPropertyId: 'fullName',
            confidence: 0.85,
            reason: '值匹配',
            source: 'llm',
          ),
        ],
        unmapped: [],
      );
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'fullName');
      expect(field.targetPropertyId, 'fullName');
    });

    test('matches LLM suggestion by case-insensitive key', () {
      final result = makeResult();
      const llmResult = LlmFieldMappingResult(
        mappings: [
          LlmFieldSuggestion(
            sourceField: 'FULLNAME',
            targetPropertyId: 'fullName',
            confidence: 0.85,
            reason: '忽略大小写',
            source: 'llm',
          ),
        ],
        unmapped: [],
      );
      final candidates = service.mapScanResultWithLlm(result, llmResult);

      final field = candidates.first.fields.firstWhere((f) => f.source.key == 'fullName');
      expect(field.targetPropertyId, 'fullName');
    });
  });

  group('ScanImportService.detectConflicts', () {
    late ScanImportService service;

    UnifiedObject makeObject({
      required String id,
      required String typeId,
      Map<String, PropertyValue> properties = const {},
    }) {
      return UnifiedObject(
        id: id,
        typeId: typeId,
        name: 'Test',
        iconName: 'folder',
        parentId: 'p1',
        childrenIds: const [],
        properties: properties,
        isDeleted: false,
        deletedAt: null,
        createdAt: 0,
        updatedAt: 0,
      );
    }

    test('detects no conflicts for empty candidates', () {
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, []);
      expect(service.detectConflicts([]), isEmpty);
    });

    test('detects conflict when existing value differs for non-identity field', () {
      final existing = makeObject(
        id: 'obj-1',
        typeId: 'profile_identity',
        properties: {
          'dateOfBirth': const TextProperty(text: '1990-01-01', sensitivity: SensitivityLevel.internal),
        },
      );
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, [existing]);

      final candidates = [
        ImportCandidate(
          source: const ScanSection(
            section: 'identity',
            display: 'Identity',
            fields: [
              ScanField(key: 'fullName', value: 'Zhang San', sensitivity: SensitivityLevel.public),
              ScanField(key: 'dateOfBirth', value: '1995-05-05', sensitivity: SensitivityLevel.internal),
            ],
          ),
          existingObjectId: 'obj-1',
          fields: [
            ImportFieldCandidate(
              source: const ScanField(key: 'fullName', value: 'Zhang San', sensitivity: SensitivityLevel.public),
              targetPropertyId: 'fullName',
              suggestedAction: ImportAction.createNew,
            ),
            ImportFieldCandidate(
              source: const ScanField(key: 'dateOfBirth', value: '1995-05-05', sensitivity: SensitivityLevel.internal),
              targetPropertyId: 'dateOfBirth',
              suggestedAction: ImportAction.createNew,
            ),
          ],
        ),
      ];

      final conflicts = service.detectConflicts(candidates);
      expect(conflicts.length, 1);
      expect(conflicts.first.existingValue, '1990-01-01');
      expect(conflicts.first.scannedValue, '1995-05-05');
      expect(conflicts.first.field.userAction, ImportAction.overwrite);
    });

    test('marks skip when values are identical', () {
      final existing = makeObject(
        id: 'obj-1',
        typeId: 'profile_identity',
        properties: {
          'dateOfBirth': const TextProperty(text: '1990-01-01', sensitivity: SensitivityLevel.internal),
        },
      );
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, [existing]);

      final candidates = [
        ImportCandidate(
          source: const ScanSection(
            section: 'identity',
            display: 'Identity',
            fields: [
              ScanField(key: 'dateOfBirth', value: '1990-01-01', sensitivity: SensitivityLevel.internal),
            ],
          ),
          existingObjectId: 'obj-1',
          fields: [
            ImportFieldCandidate(
              source: const ScanField(key: 'dateOfBirth', value: '1990-01-01', sensitivity: SensitivityLevel.internal),
              targetPropertyId: 'dateOfBirth',
              suggestedAction: ImportAction.createNew,
            ),
          ],
        ),
      ];

      final conflicts = service.detectConflicts(candidates);
      expect(conflicts, isEmpty);
      expect(candidates.first.fields.first.userAction, ImportAction.skip);
    });

    test('clears existingObjectId when entity identity differs', () {
      final existing = makeObject(
        id: 'obj-1',
        typeId: 'financial_bank_account',
        properties: {
          'bankName': const TextProperty(text: 'Bank A', sensitivity: SensitivityLevel.internal),
        },
      );
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, [existing]);

      final candidate = ImportCandidate(
        source: const ScanSection(
          section: 'bankAccount',
          display: 'Bank Account',
          fields: [
            ScanField(key: 'bankName', value: 'Bank B', sensitivity: SensitivityLevel.internal),
          ],
        ),
        existingObjectId: 'obj-1',
        fields: [
          ImportFieldCandidate(
            source: const ScanField(key: 'bankName', value: 'Bank B', sensitivity: SensitivityLevel.internal),
            targetPropertyId: 'bankName',
            suggestedAction: ImportAction.createNew,
          ),
        ],
      );

      service.detectConflicts([candidate]);
      expect(candidate.existingObjectId, isNull);
    });

    test('keeps existingObjectId when no identity property conflicts', () {
      final existing = makeObject(
        id: 'obj-1',
        typeId: 'profile_identity',
        properties: {
          'fullName': const TextProperty(text: 'Zhang San', sensitivity: SensitivityLevel.public),
        },
      );
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, [existing]);

      final candidate = ImportCandidate(
        source: const ScanSection(
          section: 'identity',
          display: 'Identity',
          fields: [
            ScanField(key: 'dateOfBirth', value: '1990-01-01', sensitivity: SensitivityLevel.internal),
          ],
        ),
        existingObjectId: 'obj-1',
        fields: [
          ImportFieldCandidate(
            source: const ScanField(key: 'dateOfBirth', value: '1990-01-01', sensitivity: SensitivityLevel.internal),
            targetPropertyId: 'dateOfBirth',
            suggestedAction: ImportAction.createNew,
          ),
        ],
      );

      service.detectConflicts([candidate]);
      expect(candidate.existingObjectId, 'obj-1');
    });

    test('clears existingObjectId when existing object not found', () {
      final container = ProviderContainer();
      final notifier = container.read(unifiedObjectProvider.notifier);
      service = ScanImportService(notifier, []);

      final candidate = ImportCandidate(
        source: const ScanSection(
          section: 'identity',
          display: 'Identity',
          fields: [],
        ),
        existingObjectId: 'missing',
        fields: [],
      );

      service.detectConflicts([candidate]);
      expect(candidate.existingObjectId, isNull);
    });
  });

  group('ScanImportService.preserveSensitivity', () {
    test('preserves sensitivity on TextProperty', () {
      const existing = TextProperty(text: 'old', sensitivity: SensitivityLevel.critical);
      const imported = TextProperty(text: 'new', sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(existing, imported);
      expect(result, isA<TextProperty>());
      expect((result as TextProperty).text, 'new');
      expect(result.sensitivity, SensitivityLevel.critical);
    });

    test('preserves sensitivity on NumberProperty', () {
      const existing = NumberProperty(value: 1, sensitivity: SensitivityLevel.critical);
      const imported = NumberProperty(value: 2, sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(existing, imported);
      expect((result as NumberProperty).value, 2);
      expect(result.sensitivity, SensitivityLevel.critical);
    });

    test('preserves sensitivity on DateProperty', () {
      const existing = DateProperty(isoDate: '2024-01-01', sensitivity: SensitivityLevel.critical);
      const imported = DateProperty(isoDate: '2024-02-01', sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(existing, imported);
      expect((result as DateProperty).isoDate, '2024-02-01');
      expect(result.sensitivity, SensitivityLevel.critical);
    });

    test('preserves sensitivity on CheckboxProperty', () {
      const existing = CheckboxProperty(checked: false, sensitivity: SensitivityLevel.critical);
      const imported = CheckboxProperty(checked: true, sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(existing, imported);
      expect((result as CheckboxProperty).checked, true);
      expect(result.sensitivity, SensitivityLevel.critical);
    });

    test('returns imported when existing is null', () {
      const imported = TextProperty(text: 'new', sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(null, imported);
      expect(identical(result, imported), isTrue);
    });

    test('preserves sensitivity on UrlProperty', () {
      const existing = UrlProperty(url: 'https://old.com', sensitivity: SensitivityLevel.critical);
      const imported = UrlProperty(url: 'https://new.com', sensitivity: SensitivityLevel.public);
      final result = ScanImportService.preserveSensitivity(existing, imported);
      expect((result as UrlProperty).url, 'https://new.com');
      expect(result.sensitivity, SensitivityLevel.critical);
    });
  });
}
