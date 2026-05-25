import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/plugin_field_mapping.dart';

void main() {
  group('PluginFieldMapping', () {
    test('creates with required fields', () {
      const mapping = PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
      );
      expect(mapping.pluginId, 'plugin_1');
      expect(mapping.semanticTypeToKey['pet.name'], 'auto_a3f7d2e1');
      expect(mapping.targetSectionId, isNull);
    });

    test('toJson serializes correctly', () {
      const mapping = PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
        targetSectionId: 'section_1',
      );
      final json = mapping.toJson();
      expect(json['pluginId'], 'plugin_1');
      expect(json['semanticTypeToKey'], {'pet.name': 'auto_a3f7d2e1'});
      expect(json['targetSectionId'], 'section_1');
    });

    test('fromJson deserializes correctly', () {
      final json = {
        'pluginId': 'plugin_1',
        'semanticTypeToKey': {'pet.name': 'auto_a3f7d2e1'},
        'targetSectionId': 'section_1',
      };
      final mapping = PluginFieldMapping.fromJson(json);
      expect(mapping.pluginId, 'plugin_1');
      expect(mapping.semanticTypeToKey['pet.name'], 'auto_a3f7d2e1');
      expect(mapping.targetSectionId, 'section_1');
    });

    test('copyWith creates independent copy', () {
      const mapping = PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
      );
      final copy = mapping.copyWith(targetSectionId: 'section_2');
      expect(copy.pluginId, 'plugin_1');
      expect(copy.targetSectionId, 'section_2');
      expect(mapping.targetSectionId, isNull);
    });
  });

  group('PluginFieldMappingCollection', () {
    test('empty collection has no mappings', () {
      final collection = PluginFieldMappingCollection();
      expect(collection.getMapping('plugin_1'), isNull);
    });

    test('setMapping and getMapping', () {
      final collection = PluginFieldMappingCollection();
      const mapping = PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
      );
      collection.setMapping(mapping);
      expect(collection.getMapping('plugin_1'), isNotNull);
      expect(collection.getMapping('plugin_1')!.pluginId, 'plugin_1');
    });

    test('removeMapping deletes mapping', () {
      final collection = PluginFieldMappingCollection();
      const mapping = PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
      );
      collection.setMapping(mapping);
      collection.removeMapping('plugin_1');
      expect(collection.getMapping('plugin_1'), isNull);
    });

    test('toJson and fromJson roundtrip', () {
      final collection = PluginFieldMappingCollection();
      collection.setMapping(const PluginFieldMapping(
        pluginId: 'plugin_1',
        semanticTypeToKey: {'pet.name': 'auto_a3f7d2e1'},
      ));
      collection.setMapping(const PluginFieldMapping(
        pluginId: 'plugin_2',
        semanticTypeToKey: {'identity.name': 'fullName'},
      ));

      final json = collection.toJson();
      final restored = PluginFieldMappingCollection.fromJson(json);

      expect(restored.getMapping('plugin_1'), isNotNull);
      expect(restored.getMapping('plugin_1')!.semanticTypeToKey['pet.name'],
          'auto_a3f7d2e1');
      expect(restored.getMapping('plugin_2'), isNotNull);
      expect(restored.getMapping('plugin_2')!.semanticTypeToKey['identity.name'],
          'fullName');
    });
  });
}
