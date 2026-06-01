import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';

void main() {
  group('SemanticFieldType', () {
    test('getLabel returns language-specific label', () {
      final type = SemanticTypeRegistry.getType('person.name');
      expect(type, isNotNull);
      expect(type!.getLabel('zh'), '姓名');
      expect(type.getLabel('en'), 'Full Name');
    });

    test('getLabel falls back to en then zh', () {
      final type = SemanticTypeRegistry.getType('person.name');
      expect(type!.getLabel('unknown'), isNotEmpty); // falls back to en/zh
    });

    test('getLabel formats id segment as fallback', () {
      final type = const SemanticFieldType(
        id: 'test.unknownField',
        labels: {},
        descriptions: {},
        category: 'test',
        suggestedPropertyType: 'text',
        defaultSensitivity: SensitivityLevel.public,
        icon: Icons.text_fields,
      );
      expect(type.getLabel('any'), 'Unknown Field');
    });

    test('getDescription returns language-specific description', () {
      final type = SemanticTypeRegistry.getType('person.name');
      expect(type!.getDescription('zh'), isNotEmpty);
      expect(type.getDescription('en'), isNotEmpty);
    });

    test('getDescription returns empty when no match', () {
      final type = const SemanticFieldType(
        id: 'test.empty',
        labels: {'en': 'Test'},
        descriptions: {},
        category: 'test',
        suggestedPropertyType: 'text',
        defaultSensitivity: SensitivityLevel.public,
        icon: Icons.text_fields,
      );
      expect(type.getDescription('any'), '');
    });
  });

  group('SemanticTypeRegistry', () {
    test('getType returns type by id', () {
      expect(SemanticTypeRegistry.getType('person.name'), isNotNull);
      expect(SemanticTypeRegistry.getType('contact.email'), isNotNull);
      expect(SemanticTypeRegistry.getType('contact.phone'), isNotNull);
    });

    test('getType returns null for unknown id', () {
      expect(SemanticTypeRegistry.getType('unknown.type'), isNull);
    });

    test('hasType returns true for known type', () {
      expect(SemanticTypeRegistry.hasType('person.name'), isTrue);
    });

    test('hasType returns false for unknown type', () {
      expect(SemanticTypeRegistry.hasType('unknown.type'), isFalse);
    });

    test('getTypesByCategory returns types for person', () {
      final types = SemanticTypeRegistry.getTypesByCategory('person');
      expect(types, isNotEmpty);
      expect(types.every((t) => t.category == 'person'), isTrue);
    });

    test('getTypesByCategory returns empty for unknown category', () {
      expect(SemanticTypeRegistry.getTypesByCategory('unknown'), isEmpty);
    });

    test('getCategoryLabel returns label', () {
      expect(SemanticTypeRegistry.getCategoryLabel('person', 'en'), isNotEmpty);
      expect(SemanticTypeRegistry.getCategoryLabel('person', 'zh'), isNotEmpty);
    });

    test('getCategoryIcon returns icon for known category', () {
      expect(SemanticTypeRegistry.getCategoryIcon('person'), isNotNull);
    });

    test('getCategoryIcon returns default for unknown category', () {
      expect(SemanticTypeRegistry.getCategoryIcon('unknown'), isNotNull);
    });

    test('search finds types by id', () {
      final results = SemanticTypeRegistry.search('email', 'en');
      expect(results.any((t) => t.id == 'contact.email'), isTrue);
    });

    test('search finds types by label', () {
      final results = SemanticTypeRegistry.search('name', 'en');
      expect(results, isNotEmpty);
    });

    test('search returns empty for no match', () {
      expect(SemanticTypeRegistry.search('xyz_nonexistent', 'en'), isEmpty);
    });

    test('getDefaultSensitivity returns sensitivity for known type', () {
      expect(SemanticTypeRegistry.getDefaultSensitivity('person.name'), isNotNull);
    });

    test('getDefaultSensitivity returns null for unknown type', () {
      expect(SemanticTypeRegistry.getDefaultSensitivity('unknown'), isNull);
    });
  });
}
