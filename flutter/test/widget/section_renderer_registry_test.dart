import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';

UnifiedObject _makeObject({
  required String id,
  required String name,
  String? typeId,
  Map<String, PropertyValue> properties = const {},
}) {
  final now = DateTime.now().millisecondsSinceEpoch;
  return UnifiedObject(
    id: id,
    name: name,
    typeId: typeId,
    createdAt: now,
    updatedAt: now,
    properties: properties,
  );
}

void main() {
  group('itemToMap', () {
    test('converts properties to string map', () {
      final item = _makeObject(
        id: '1',
        name: 'Test',
        properties: {
          'Title': const TextProperty(text: 'Hello'),
          'Age': const TextProperty(text: '30'),
        },
      );
      final map = itemToMap(item);
      expect(map['Title'], 'Hello');
      expect(map['Age'], '30');
    });

    test('returns empty map for no properties', () {
      final item = _makeObject(id: '1', name: 'Test');
      expect(itemToMap(item), isEmpty);
    });
  });

  group('SectionRendererRegistry', () {
    test('getConfig returns config for preset typeId', () {
      final config = SectionRendererRegistry.getConfig('__preset_identity');
      expect(config, isNotNull);
      expect(config!.typeId, '__preset_identity');
      expect(config.iconName, 'person');
      expect(config.titlePropertyKey, 'Title');
      expect(config.isRestricted, false);
    });

    test('getConfig returns null for unknown typeId', () {
      expect(SectionRendererRegistry.getConfig('unknown'), isNull);
    });

    test('isPreset returns true for preset typeId', () {
      expect(SectionRendererRegistry.isPreset('__preset_identity'), isTrue);
    });

    test('isPreset returns false for unknown typeId', () {
      expect(SectionRendererRegistry.isPreset('custom'), isFalse);
    });

    test('presetTypeIds contains expected keys', () {
      final ids = SectionRendererRegistry.presetTypeIds.toList();
      expect(ids, contains('__preset_identity'));
      expect(ids, contains('__preset_contact'));
      expect(ids, contains('__preset_passport'));
    });

    test('getConfigBySectionId returns config for passport section', () {
      final config = SectionRendererRegistry.getConfigBySectionId(
        DefaultSectionIds.passport,
      );
      expect(config, isNotNull);
      expect(config!.typeId, '__preset_passport');
      expect(config.fieldPrefix, 'passport');
    });

    test('getConfigBySectionId returns null for custom section', () {
      expect(
        SectionRendererRegistry.getConfigBySectionId('custom_section'),
        isNull,
      );
    });

    test('getSectionLabelByFieldPrefix returns label for identity', () {
      final label = SectionRendererRegistry.getSectionLabelByFieldPrefix(
        'identity',
        AppLocalizationsEn(),
      );
      expect(label, isNotNull);
    });

    test('getSectionLabelByFieldPrefix returns null for unknown prefix', () {
      expect(
        SectionRendererRegistry.getSectionLabelByFieldPrefix(
          'unknown_prefix',
          AppLocalizationsEn(),
        ),
        isNull,
      );
    });

    test('preset configs have consistent titlePropertyKey', () {
      for (final config in SectionRendererRegistry.presetTypeIds
          .map(SectionRendererRegistry.getConfig)
          .whereType<PresetSectionConfig>()) {
        expect(config.titlePropertyKey, isNotEmpty);
        expect(config.fieldPrefix, isNotEmpty);
        expect(config.historyFieldId, isNotEmpty);
      }
    });
  });

  group('getLocalizedObjectName', () {
    test('returns object.name when l10n is null', () {
      final obj = _makeObject(id: '1', name: 'Custom', typeId: 'page');
      expect(getLocalizedObjectName(null, obj), 'Custom');
    });

    test('returns object.name for non-preset page', () {
      final obj = _makeObject(
        id: 'custom_page',
        name: 'Custom Page',
        typeId: 'page',
      );
      expect(getLocalizedObjectName(null, obj), 'Custom Page');
    });

    test('returns object.name for custom section', () {
      final obj = _makeObject(
        id: 'custom_section',
        name: 'My Section',
        typeId: 'collection',
      );
      expect(getLocalizedObjectName(null, obj), 'My Section');
    });
  });
}
