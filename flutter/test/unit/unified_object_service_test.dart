import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

void main() {
  group('ObjectTypeRegistry', () {
    test('getType returns built-in type', () {
      expect(ObjectTypeRegistry.getType('note'), isNotNull);
      expect(ObjectTypeRegistry.getType('task'), isNotNull);
      expect(ObjectTypeRegistry.getType('contact'), isNotNull);
    });

    test('getType returns null for unknown type', () {
      expect(ObjectTypeRegistry.getType('unknown_type'), isNull);
    });

    test('getType prefers custom types over built-ins', () {
      final customType = ObjectTypeDefinition(
        id: 'note',
        name: 'Custom Note',
        properties: [],
      );
      final result = ObjectTypeRegistry.getType('note', customTypes: [customType]);
      expect(result, isNotNull);
      expect(result!.name, 'Custom Note');
    });

    test('getAllTypes includes built-ins', () {
      final types = ObjectTypeRegistry.getAllTypes();
      expect(types, isNotEmpty);
    });

    test('getAllTypes includes custom types', () {
      final customType = ObjectTypeDefinition(
        id: 'custom',
        name: 'Custom',
        properties: [],
      );
      final types = ObjectTypeRegistry.getAllTypes(customTypes: [customType]);
      expect(types.any((t) => t.id == 'custom'), isTrue);
    });

    test('defaultType is note', () {
      expect(ObjectTypeRegistry.defaultType.id, 'note');
    });

    test('buildPropertiesFromType returns properties for built-in type', () {
      final props = ObjectTypeRegistry.buildPropertiesFromType('contact');
      expect(props, isNotEmpty);
    });

    test('buildPropertiesFromType returns empty for unknown type', () {
      expect(ObjectTypeRegistry.buildPropertiesFromType('unknown'), isEmpty);
    });

    test('buildPropertyLabelsFromType returns empty for built-in type', () {
      expect(ObjectTypeRegistry.buildPropertyLabelsFromType('note'), isEmpty);
      expect(ObjectTypeRegistry.buildPropertyLabelsFromType('__preset_identity'), isEmpty);
    });

    test('buildPropertyLabelsFromType returns labels for custom type', () {
      final customType = ObjectTypeDefinition(
        id: 'custom',
        name: 'Custom',
        properties: [
          PropertyDefinition(id: 'field1', name: 'Display Name', type: PropertyType.text),
        ],
      );
      final labels = ObjectTypeRegistry.buildPropertyLabelsFromType(
        'custom',
        customTypes: [customType],
      );
      expect(labels['field1'], 'Display Name');
    });
  });

  group('Default IDs', () {
    test('DefaultPageIds are non-empty', () {
      expect(DefaultPageIds.profile, isNotEmpty);
      expect(DefaultPageIds.travel, isNotEmpty);
      expect(DefaultPageIds.financial, isNotEmpty);
      expect(DefaultPageIds.professional, isNotEmpty);
    });

    test('DefaultSectionIds are non-empty', () {
      expect(DefaultSectionIds.identity, isNotEmpty);
      expect(DefaultSectionIds.passport, isNotEmpty);
      expect(DefaultSectionIds.bankAccount, isNotEmpty);
      expect(DefaultSectionIds.education, isNotEmpty);
    });
  });
}
