import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';

void main() {
  group('SemanticTypeRegistry', () {
    test('getType returns built-in type', () {
      final type = SemanticTypeRegistry.getType('pet.name');
      expect(type, isNotNull);
      expect(type!.id, 'pet.name');
      expect(type.category, 'pet');
      expect(type.defaultSensitivity, SensitivityLevel.public);
    });

    test('getType returns null for unknown type', () {
      final type = SemanticTypeRegistry.getType('unknown.type');
      expect(type, isNull);
    });

    test('getLabel returns localized label', () {
      final type = SemanticTypeRegistry.getType('pet.name');
      expect(type, isNotNull);
      expect(type!.getLabel('en'), 'Pet Name');
      expect(type.getLabel('zh'), '宠物名字');
    });

    test('getLabel falls back to id for unknown language', () {
      final type = SemanticTypeRegistry.getType('pet.name');
      expect(type, isNotNull);
      // pet.name has 'en' and 'zh' labels, so it should not fall back to id
      expect(type!.getLabel('fr'), isNot(equals('pet.name')));
    });

    test('allTypes contains built-in types', () {
      expect(SemanticTypeRegistry.allTypes, isNotEmpty);
    });

    test('search returns matching types', () {
      final results = SemanticTypeRegistry.search('name', 'en');
      expect(results, isNotEmpty);
      expect(
        results.any((t) => t.id == 'pet.name'),
        isTrue,
      );
    });

    test('recommend prioritizes matching category', () {
      final results = SemanticTypeRegistry.recommend(
        'name',
        'pet',
        'en',
      );
      expect(results, isNotEmpty);
      // pet.name should be highly ranked when section is 'pet'
      expect(results.any((t) => t.id == 'pet.name'), isTrue);
    });

    test('semantic types have unique IDs', () {
      final ids = SemanticTypeRegistry.allTypes.map((t) => t.id).toList();
      expect(ids.toSet().length, ids.length);
    });

    test('getTypesByCategory filters correctly', () {
      final petTypes = SemanticTypeRegistry.getTypesByCategory('pet');
      expect(petTypes, isNotEmpty);
      expect(petTypes.every((t) => t.category == 'pet'), isTrue);
    });
  });
}
