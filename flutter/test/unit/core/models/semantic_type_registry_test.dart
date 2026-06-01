import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';

void main() {
  group('SemanticFieldType', () {
    const type = SemanticFieldType(
      id: 'test.field_name',
      labels: {'zh': '测试字段', 'en': 'Test Field'},
      descriptions: {'zh': '描述', 'en': 'Description'},
      category: 'test',
      suggestedPropertyType: 'text',
      defaultSensitivity: SensitivityLevel.internal,
      icon: Icons.person,
    );

    group('getLabel', () {
      test('returns label for exact language code', () {
        expect(type.getLabel('zh'), '测试字段');
        expect(type.getLabel('en'), 'Test Field');
      });

      test('falls back to English when language not found', () {
        expect(type.getLabel('ja'), 'Test Field');
      });

      test('falls back to Chinese when English not available', () {
        const zhOnly = SemanticFieldType(
          id: 'test.zh_only',
          labels: {'zh': '中文'},
          descriptions: {},
          category: 'test',
          suggestedPropertyType: 'text',
          defaultSensitivity: SensitivityLevel.public,
          icon: Icons.person,
        );
        expect(zhOnly.getLabel('en'), '中文');
      });

      test('formats id last segment when no labels', () {
        const noLabels = SemanticFieldType(
          id: 'test.myFieldName',
          labels: {},
          descriptions: {},
          category: 'test',
          suggestedPropertyType: 'text',
          defaultSensitivity: SensitivityLevel.public,
          icon: Icons.person,
        );
        expect(noLabels.getLabel('en'), 'My Field Name');
      });
    });

    group('getDescription', () {
      test('returns description for exact language code', () {
        expect(type.getDescription('zh'), '描述');
        expect(type.getDescription('en'), 'Description');
      });

      test('falls back to English when language not found', () {
        expect(type.getDescription('ja'), 'Description');
      });

      test('returns empty string when no descriptions', () {
        const noDesc = SemanticFieldType(
          id: 'test.no_desc',
          labels: {'en': 'Label'},
          descriptions: {},
          category: 'test',
          suggestedPropertyType: 'text',
          defaultSensitivity: SensitivityLevel.public,
          icon: Icons.person,
        );
        expect(noDesc.getDescription('en'), '');
      });
    });
  });

  group('SemanticTypeRegistry', () {
    group('getType', () {
      test('returns type for existing id', () {
        final type = SemanticTypeRegistry.getType('person.name');
        expect(type, isNotNull);
        expect(type!.id, 'person.name');
      });

      test('returns null for unknown id', () {
        expect(SemanticTypeRegistry.getType('nonexistent.type'), isNull);
      });
    });

    group('hasType', () {
      test('returns true for existing type', () {
        expect(SemanticTypeRegistry.hasType('person.name'), isTrue);
      });

      test('returns false for unknown type', () {
        expect(SemanticTypeRegistry.hasType('unknown'), isFalse);
      });
    });

    group('getTypesByCategory', () {
      test('returns types for existing category', () {
        final types = SemanticTypeRegistry.getTypesByCategory('person');
        expect(types, isNotEmpty);
        expect(types.every((t) => t.category == 'person'), isTrue);
      });

      test('returns empty list for unknown category', () {
        expect(SemanticTypeRegistry.getTypesByCategory('unknown'), isEmpty);
      });
    });

    group('categories', () {
      test('returns non-empty sorted list', () {
        final cats = SemanticTypeRegistry.categories;
        expect(cats, isNotEmpty);
        expect(cats, orderedEquals(cats.toList()..sort()));
      });

      test('contains person category', () {
        expect(SemanticTypeRegistry.categories, contains('person'));
      });
    });

    group('getCategoryLabel', () {
      test('returns label in requested language', () {
        expect(SemanticTypeRegistry.getCategoryLabel('person', 'zh'), '人物');
        expect(SemanticTypeRegistry.getCategoryLabel('person', 'en'), 'Person');
      });

      test('falls back to English', () {
        expect(SemanticTypeRegistry.getCategoryLabel('person', 'fr'), 'Person');
      });

      test('returns unknown category id when not found', () {
        expect(SemanticTypeRegistry.getCategoryLabel('unknown', 'en'), 'unknown');
      });
    });

    group('resolveByFieldPath', () {
      test('resolves known field path', () {
        final type = SemanticTypeRegistry.resolveByFieldPath('person.name');
        expect(type, isNotNull);
        expect(type!.id, 'person.name');
      });

      test('resolves via fuzzy match on last segment', () {
        final type = SemanticTypeRegistry.resolveByFieldPath('name');
        expect(type, isNotNull);
        expect(type!.id, 'person.name');
      });

      test('returns null for unknown path', () {
        expect(SemanticTypeRegistry.resolveByFieldPath('unknown.field'), isNull);
      });
    });

    group('getDefaultSensitivity', () {
      test('returns sensitivity for existing type', () {
        final sensitivity = SemanticTypeRegistry.getDefaultSensitivity('person.name');
        expect(sensitivity, isNotNull);
      });

      test('returns null for unknown type', () {
        expect(SemanticTypeRegistry.getDefaultSensitivity('unknown'), isNull);
      });
    });
  });
}
