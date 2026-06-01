import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/plugin_field_mapping.dart';

void main() {
  group('PluginFieldMapping', () {
    const mapping = PluginFieldMapping(
      pluginId: 'com.test.plugin',
      semanticTypeToKey: {'pet.name': 'auto_a3f7', 'pet.breed': 'auto_b2e1'},
      targetSectionId: 'pets',
    );

    test('toJson serializes correctly', () {
      final json = mapping.toJson();
      expect(json['pluginId'], 'com.test.plugin');
      expect(json['semanticTypeToKey'], {'pet.name': 'auto_a3f7', 'pet.breed': 'auto_b2e1'});
      expect(json['targetSectionId'], 'pets');
    });

    test('fromJson deserializes correctly', () {
      final json = {
        'pluginId': 'com.test.plugin',
        'semanticTypeToKey': {'a': 'b'},
        'targetSectionId': 'section1',
      };
      final restored = PluginFieldMapping.fromJson(json);
      expect(restored.pluginId, 'com.test.plugin');
      expect(restored.semanticTypeToKey, {'a': 'b'});
      expect(restored.targetSectionId, 'section1');
    });

    test('fromJson handles null optional fields', () {
      final json = {
        'pluginId': 'com.test.plugin',
      };
      final restored = PluginFieldMapping.fromJson(json);
      expect(restored.semanticTypeToKey, isEmpty);
      expect(restored.targetSectionId, isNull);
    });

    test('copyWith updates fields', () {
      final copy = mapping.copyWith(targetSectionId: 'new_section');
      expect(copy.pluginId, 'com.test.plugin');
      expect(copy.targetSectionId, 'new_section');
      expect(copy.semanticTypeToKey, mapping.semanticTypeToKey);
    });

    test('copyWith preserves fields when null', () {
      final copy = mapping.copyWith();
      expect(copy.pluginId, mapping.pluginId);
      expect(copy.semanticTypeToKey, mapping.semanticTypeToKey);
      expect(copy.targetSectionId, mapping.targetSectionId);
    });

    test('round-trip serialization', () {
      final json = mapping.toJson();
      final restored = PluginFieldMapping.fromJson(json);
      expect(restored.pluginId, mapping.pluginId);
      expect(restored.semanticTypeToKey, mapping.semanticTypeToKey);
      expect(restored.targetSectionId, mapping.targetSectionId);
    });
  });

  group('PluginFieldMappingCollection', () {
    test('getMapping returns null when empty', () {
      final collection = PluginFieldMappingCollection();
      expect(collection.getMapping('any'), isNull);
    });

    test('setMapping adds new mapping', () {
      final collection = PluginFieldMappingCollection();
      const mapping = PluginFieldMapping(pluginId: 'p1');
      collection.setMapping(mapping);
      expect(collection.getMapping('p1'), isNotNull);
      expect(collection.getMapping('p1')!.pluginId, 'p1');
    });

    test('setMapping overwrites existing mapping', () {
      final collection = PluginFieldMappingCollection();
      collection.setMapping(const PluginFieldMapping(pluginId: 'p1', targetSectionId: 'a'));
      collection.setMapping(const PluginFieldMapping(pluginId: 'p1', targetSectionId: 'b'));
      expect(collection.getMapping('p1')!.targetSectionId, 'b');
    });

    test('removeMapping deletes mapping', () {
      final collection = PluginFieldMappingCollection();
      collection.setMapping(const PluginFieldMapping(pluginId: 'p1'));
      collection.removeMapping('p1');
      expect(collection.getMapping('p1'), isNull);
    });

    test('removeMapping is no-op for missing plugin', () {
      final collection = PluginFieldMappingCollection();
      collection.removeMapping('missing');
      expect(collection.getMapping('missing'), isNull);
    });

    test('toJson serializes all mappings', () {
      final collection = PluginFieldMappingCollection();
      collection.setMapping(const PluginFieldMapping(pluginId: 'p1', targetSectionId: 's1'));
      collection.setMapping(const PluginFieldMapping(pluginId: 'p2', targetSectionId: 's2'));
      final json = collection.toJson();
      expect(json.keys, containsAll(['p1', 'p2']));
    });

    test('fromJson deserializes collection', () {
      final json = {
        'p1': <String, dynamic>{'pluginId': 'p1', 'semanticTypeToKey': <String, dynamic>{}, 'targetSectionId': 's1'},
        'p2': <String, dynamic>{'pluginId': 'p2', 'semanticTypeToKey': <String, dynamic>{'a': 'b'}},
      };
      final collection = PluginFieldMappingCollection.fromJson(json);
      expect(collection.getMapping('p1')!.targetSectionId, 's1');
      expect(collection.getMapping('p2')!.semanticTypeToKey, {'a': 'b'});
    });

    test('fromJson skips invalid entries', () {
      final json = {
        'p1': {'pluginId': 'p1'},
        'invalid': 'not a map',
      };
      final collection = PluginFieldMappingCollection.fromJson(json);
      expect(collection.getMapping('p1'), isNotNull);
      expect(collection.getMapping('invalid'), isNull);
    });

    test('round-trip serialization', () {
      final original = PluginFieldMappingCollection();
      original.setMapping(const PluginFieldMapping(
        pluginId: 'p1',
        semanticTypeToKey: {'a': 'b'},
        targetSectionId: 's1',
      ));
      final json = original.toJson();
      final restored = PluginFieldMappingCollection.fromJson(json);
      final mapping = restored.getMapping('p1')!;
      expect(mapping.pluginId, 'p1');
      expect(mapping.semanticTypeToKey, {'a': 'b'});
      expect(mapping.targetSectionId, 's1');
    });
  });
}
