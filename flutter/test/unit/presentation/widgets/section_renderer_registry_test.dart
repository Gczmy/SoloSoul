import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';

void main() {
  group('itemToMap', () {
    test('converts properties to string map', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Test',
        properties: {
          'name': TextProperty(text: 'Alice'),
          'age': NumberProperty(value: 30),
        },
        createdAt: 0,
        updatedAt: 0,
      );
      final map = itemToMap(item);
      expect(map, equals({'name': 'Alice', 'age': '30.0'}));
    });

    test('returns empty map for no properties', () {
      const item = UnifiedObject(
        id: '1',
        name: 'Test',
        createdAt: 0,
        updatedAt: 0,
      );
      final map = itemToMap(item);
      expect(map, isEmpty);
    });
  });

  group('SectionRendererRegistry', () {
    test('getConfig returns config for preset type', () {
      final config = SectionRendererRegistry.getConfig('__preset_identity');
      expect(config, isNotNull);
      expect(config!.typeId, '__preset_identity');
    });

    test('getConfig returns null for unknown type', () {
      expect(SectionRendererRegistry.getConfig('unknown'), isNull);
    });

    test('isPreset returns true for preset type', () {
      expect(SectionRendererRegistry.isPreset('__preset_passport'), isTrue);
    });

    test('isPreset returns false for unknown type', () {
      expect(SectionRendererRegistry.isPreset('custom'), isFalse);
    });

    test('presetTypeIds is non-empty', () {
      expect(SectionRendererRegistry.presetTypeIds, isNotEmpty);
    });

    test('getConfigBySectionId returns config for default section', () {
      final config = SectionRendererRegistry.getConfigBySectionId(
        DefaultSectionIds.passport,
      );
      expect(config, isNotNull);
      expect(config!.fieldPrefix, 'passport');
    });

    test('getConfigBySectionId returns null for unknown section', () {
      expect(SectionRendererRegistry.getConfigBySectionId('unknown'), isNull);
    });

    test('getSectionLabelByFieldPrefix returns label for known prefix', () {
      final l10n = AppLocalizationsEn();
      final label = SectionRendererRegistry.getSectionLabelByFieldPrefix(
        'identity',
        l10n,
      );
      expect(label, isNotNull);
      expect(label, isNotEmpty);
    });

    test('getSectionLabelByFieldPrefix returns null for unknown prefix', () {
      final l10n = AppLocalizationsEn();
      final label = SectionRendererRegistry.getSectionLabelByFieldPrefix(
        'unknown',
        l10n,
      );
      expect(label, isNull);
    });
  });

  group('getLocalizedObjectName', () {
    test('returns object name when l10n is null', () {
      const object = UnifiedObject(
        id: 'custom',
        name: 'Custom Page',
        typeId: 'page',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(getLocalizedObjectName(null, object), 'Custom Page');
    });

    test('returns localized name for profile page', () {
      final l10n = AppLocalizationsEn();
      const object = UnifiedObject(
        id: DefaultPageIds.profile,
        name: 'Profile',
        typeId: 'page',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(getLocalizedObjectName(l10n, object), l10n.profileTitle);
    });

    test('returns localized name for travel page', () {
      final l10n = AppLocalizationsEn();
      const object = UnifiedObject(
        id: DefaultPageIds.travel,
        name: 'Travel',
        typeId: 'page',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(getLocalizedObjectName(l10n, object), l10n.travelTitle);
    });

    test('returns object name for custom page', () {
      final l10n = AppLocalizationsEn();
      const object = UnifiedObject(
        id: 'custom_page',
        name: 'My Page',
        typeId: 'page',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(getLocalizedObjectName(l10n, object), 'My Page');
    });

    test('returns localized name for preset section', () {
      final l10n = AppLocalizationsEn();
      const object = UnifiedObject(
        id: DefaultSectionIds.passport,
        name: 'Passport',
        typeId: 'collection',
        createdAt: 0,
        updatedAt: 0,
      );
      final result = getLocalizedObjectName(l10n, object);
      expect(result, isNotNull);
      expect(result, isNotEmpty);
    });

    test('returns object name for custom section', () {
      final l10n = AppLocalizationsEn();
      const object = UnifiedObject(
        id: 'custom_section',
        name: 'Custom Section',
        typeId: 'collection',
        createdAt: 0,
        updatedAt: 0,
      );
      expect(getLocalizedObjectName(l10n, object), 'Custom Section');
    });
  });
}
