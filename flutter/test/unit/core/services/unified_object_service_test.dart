import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

void main() {
  group('ObjectTypeRegistry', () {
    group('getType', () {
      test('returns built-in type by ID', () {
        final type = ObjectTypeRegistry.getType('page');
        expect(type, isNotNull);
        expect(type!.id, 'page');
        expect(type.name, 'Page');
      });

      test('returns note type with properties', () {
        final type = ObjectTypeRegistry.getType('note');
        expect(type, isNotNull);
        expect(type!.properties, isNotEmpty);
        expect(type.properties.first.id, 'content');
      });

      test('returns task type with checkbox and date properties', () {
        final type = ObjectTypeRegistry.getType('task');
        expect(type, isNotNull);
        expect(type!.properties, hasLength(2));
        expect(type.properties.any((p) => p.type == PropertyType.checkbox),
            isTrue);
        expect(
            type.properties.any((p) => p.type == PropertyType.date), isTrue);
      });

      test('returns null for unknown type', () {
        final type = ObjectTypeRegistry.getType('nonexistent');
        expect(type, isNull);
      });

      test('checks custom types when not in builtins', () {
        const customType = ObjectTypeDefinition(
          id: 'myCustom',
          name: 'My Custom',
          iconName: 'star',
          defaultLayout: ObjectLayout.document,
        );
        final type = ObjectTypeRegistry.getType('myCustom',
            customTypes: [customType]);
        expect(type, isNotNull);
        expect(type!.name, 'My Custom');
      });

      test('custom takes priority over built-in with same ID', () {
        const customPage = ObjectTypeDefinition(
          id: 'page',
          name: 'Custom Page',
          iconName: 'star',
          defaultLayout: ObjectLayout.document,
        );
        final type =
            ObjectTypeRegistry.getType('page', customTypes: [customPage]);
        expect(type, isNotNull);
        expect(type!.name, 'Custom Page'); // custom wins
      });
    });

    group('getAllTypes', () {
      test('returns built-in types', () {
        final types = ObjectTypeRegistry.getAllTypes();
        expect(types, isNotEmpty);
        final ids = types.map((t) => t.id).toSet();
        expect(ids, contains('page'));
        expect(ids, contains('collection'));
        expect(ids, contains('note'));
        expect(ids, contains('task'));
        expect(ids, contains('contact'));
      });

      test('includes custom types', () {
        const customType = ObjectTypeDefinition(
          id: 'custom1',
          name: 'Custom 1',
          iconName: 'star',
          defaultLayout: ObjectLayout.document,
        );
        final types =
            ObjectTypeRegistry.getAllTypes(customTypes: [customType]);
        final ids = types.map((t) => t.id).toSet();
        expect(ids, contains('custom1'));
        expect(ids, contains('page'));
      });
    });

    group('defaultType', () {
      test('returns note type', () {
        final type = ObjectTypeRegistry.defaultType;
        expect(type.id, 'note');
        expect(type.name, 'Note');
      });
    });
  });

  group('DefaultPageIds', () {
    test('has expected values', () {
      expect(DefaultPageIds.profile, '__page_profile');
      expect(DefaultPageIds.travel, '__page_travel');
      expect(DefaultPageIds.financial, '__page_financial');
      expect(DefaultPageIds.professional, '__page_professional');
    });

    test('all IDs start with __page_ prefix', () {
      expect(DefaultPageIds.profile, startsWith('__page_'));
      expect(DefaultPageIds.travel, startsWith('__page_'));
      expect(DefaultPageIds.financial, startsWith('__page_'));
      expect(DefaultPageIds.professional, startsWith('__page_'));
    });
  });

  group('DefaultSectionIds', () {
    test('profile sections exist', () {
      expect(DefaultSectionIds.identity, '__section_identity');
      expect(DefaultSectionIds.contact, '__section_contact');
      expect(DefaultSectionIds.idCard, '__section_id_card');
      expect(DefaultSectionIds.address, '__section_address');
    });

    test('travel sections exist', () {
      expect(DefaultSectionIds.passport, '__section_passport');
      expect(DefaultSectionIds.visa, '__section_visa');
      expect(DefaultSectionIds.travelHistory, '__section_travel_history');
    });

    test('financial sections exist', () {
      expect(DefaultSectionIds.bankAccount, '__section_bank_account');
      expect(DefaultSectionIds.card, '__section_card');
      expect(DefaultSectionIds.taxId, '__section_tax_id');
    });

    test('professional sections exist', () {
      expect(DefaultSectionIds.education, '__section_education');
      expect(DefaultSectionIds.employment, '__section_employment');
      expect(DefaultSectionIds.skill, '__section_skill');
      expect(DefaultSectionIds.language, '__section_language');
      expect(DefaultSectionIds.award, '__section_award');
    });

    test('all IDs start with __section_ prefix', () {
      final allIds = [
        DefaultSectionIds.identity,
        DefaultSectionIds.contact,
        DefaultSectionIds.idCard,
        DefaultSectionIds.address,
        DefaultSectionIds.passport,
        DefaultSectionIds.visa,
        DefaultSectionIds.travelHistory,
        DefaultSectionIds.bankAccount,
        DefaultSectionIds.card,
        DefaultSectionIds.taxId,
        DefaultSectionIds.education,
        DefaultSectionIds.employment,
        DefaultSectionIds.skill,
        DefaultSectionIds.language,
        DefaultSectionIds.award,
      ];
      for (final id in allIds) {
        expect(id, startsWith('__section_'));
      }
    });
  });

  group('UnifiedObjectService.getIconFromName', () {
    test('maps known icon names', () {
      expect(UnifiedObjectService.getIconFromName('person'),
          Icons.person_outlined);
      expect(UnifiedObjectService.getIconFromName('flight'), Icons.flight);
      expect(UnifiedObjectService.getIconFromName('work'), Icons.work);
      expect(UnifiedObjectService.getIconFromName('school'), Icons.school);
      expect(UnifiedObjectService.getIconFromName('home'), Icons.home);
      expect(UnifiedObjectService.getIconFromName('language'), Icons.language);
    });

    test('maps all section icon names', () {
      // Section icons used in _kSectionMeta — only test names in the switch
      expect(UnifiedObjectService.getIconFromName('person'),
          Icons.person_outlined);
      expect(UnifiedObjectService.getIconFromName('badge'), Icons.badge);
      expect(UnifiedObjectService.getIconFromName('home'), Icons.home);
      expect(UnifiedObjectService.getIconFromName('flight'), Icons.flight);
      expect(UnifiedObjectService.getIconFromName('description'),
          Icons.description);
      expect(UnifiedObjectService.getIconFromName('history'), Icons.history);
      expect(UnifiedObjectService.getIconFromName('account_balance'),
          Icons.account_balance);
      expect(UnifiedObjectService.getIconFromName('credit_card'),
          Icons.credit_card);
      expect(UnifiedObjectService.getIconFromName('school'), Icons.school);
      expect(UnifiedObjectService.getIconFromName('work'), Icons.work);
      expect(UnifiedObjectService.getIconFromName('language'), Icons.language);
    });

    test('returns folder_outlined for unmapped section icon names', () {
      // These are used in _kSectionMeta but not in getIconFromName switch
      expect(UnifiedObjectService.getIconFromName('contact_mail'),
          Icons.folder_outlined);
      expect(UnifiedObjectService.getIconFromName('receipt'),
          Icons.folder_outlined);
      expect(UnifiedObjectService.getIconFromName('stars'),
          Icons.folder_outlined);
      expect(UnifiedObjectService.getIconFromName('emoji_events'),
          Icons.folder_outlined);
    });

    test('returns folder_outlined for unknown icon name', () {
      expect(UnifiedObjectService.getIconFromName('nonexistent'),
          Icons.folder_outlined);
      expect(UnifiedObjectService.getIconFromName(''), Icons.folder_outlined);
    });
  });

  group('ObjectTypeRegistry.buildPropertiesFromType', () {
    test('returns empty for unknown type', () {
      final props = ObjectTypeRegistry.buildPropertiesFromType('unknown');
      expect(props, isEmpty);
    });

    test('returns properties for task type', () {
      final props = ObjectTypeRegistry.buildPropertiesFromType('task');
      expect(props, hasLength(2));
      expect(props.containsKey('done'), isTrue);
      expect(props.containsKey('dueDate'), isTrue);
      expect(props['done'], isA<CheckboxProperty>());
      expect(props['dueDate'], isA<DateProperty>());
    });

    test('returns properties for contact type', () {
      final props = ObjectTypeRegistry.buildPropertiesFromType('contact');
      expect(props, hasLength(2));
      expect(props['phone'], isA<TextProperty>());
      expect(props['email'], isA<UrlProperty>());
    });

    test('returns empty for type with no properties', () {
      final props = ObjectTypeRegistry.buildPropertiesFromType('page');
      expect(props, isEmpty);
    });

    test('uses custom types when provided', () {
      const customType = ObjectTypeDefinition(
        id: 'custom',
        name: 'Custom',
        iconName: 'star',
        defaultLayout: ObjectLayout.document,
        properties: [
          PropertyDefinition(id: 'title', name: 'Title', type: PropertyType.text),
        ],
      );
      final props = ObjectTypeRegistry.buildPropertiesFromType(
        'custom',
        customTypes: [customType],
      );
      expect(props, hasLength(1));
      expect(props['title'], isA<TextProperty>());
    });
  });

  group('ObjectTypeRegistry.buildPropertyLabelsFromType', () {
    test('returns empty for unknown type', () {
      final labels = ObjectTypeRegistry.buildPropertyLabelsFromType('unknown');
      expect(labels, isEmpty);
    });

    test('returns empty for built-in type', () {
      // Built-in types do not store static labels
      final labels = ObjectTypeRegistry.buildPropertyLabelsFromType('note');
      expect(labels, isEmpty);
    });

    test('returns labels for custom type', () {
      const customType = ObjectTypeDefinition(
        id: 'custom',
        name: 'Custom',
        iconName: 'star',
        defaultLayout: ObjectLayout.document,
        properties: [
          PropertyDefinition(id: 'title', name: 'Title Text', type: PropertyType.text),
          PropertyDefinition(id: 'code', name: 'code', type: PropertyType.text),
        ],
      );
      final labels = ObjectTypeRegistry.buildPropertyLabelsFromType(
        'custom',
        customTypes: [customType],
      );
      expect(labels, hasLength(1));
      expect(labels['title'], 'Title Text');
      // 'code' is skipped because name == id
      expect(labels.containsKey('code'), isFalse);
    });
  });

  group('Section metadata helpers', () {
    test('getSectionMeta returns meta for known section', () {
      final meta = getSectionMeta(DefaultSectionIds.identity);
      expect(meta, isNotNull);
      expect(meta!.name, 'Identity');
    });

    test('getSectionMeta returns null for unknown section', () {
      expect(getSectionMeta('unknown'), isNull);
    });

    test('allSectionMeta is non-empty', () {
      expect(allSectionMeta, isNotEmpty);
    });

    test('getDefaultSectionIdsForPage returns sections for profile page', () {
      final ids = getDefaultSectionIdsForPage(DefaultPageIds.profile);
      expect(ids, isNotEmpty);
      expect(ids, contains(DefaultSectionIds.identity));
      expect(ids, contains(DefaultSectionIds.contact));
    });

    test('getDefaultSectionIdsForPage returns empty for unknown page', () {
      expect(getDefaultSectionIdsForPage('unknown'), isEmpty);
    });

    test('getDefaultSectionIdForItemType reverses lookup', () {
      expect(
        getDefaultSectionIdForItemType('__preset_identity'),
        DefaultSectionIds.identity,
      );
      expect(
        getDefaultSectionIdForItemType('__preset_passport'),
        DefaultSectionIds.passport,
      );
      expect(getDefaultSectionIdForItemType('unknown'), isNull);
    });

    test('getItemTypeIdForSection maps section to type', () {
      expect(
        getItemTypeIdForSection(DefaultSectionIds.identity),
        '__preset_identity',
      );
      expect(getItemTypeIdForSection('unknown'), isNull);
    });
  });

  group('fieldPrefixForTypeId', () {
    test('maps preset types', () {
      expect(fieldPrefixForTypeId('__preset_identity'), 'identity');
      expect(fieldPrefixForTypeId('__preset_contact'), 'contact');
      expect(fieldPrefixForTypeId('__preset_passport'), 'passport');
      expect(fieldPrefixForTypeId('__preset_bank_account'), 'bankAccount');
      expect(fieldPrefixForTypeId('__preset_education'), 'education');
    });

    test('returns input for unknown type', () {
      expect(fieldPrefixForTypeId('customType'), 'customType');
    });
  });

  group('lookupFieldSensitivity', () {
    test('returns registry level for known field', () {
      final level = lookupFieldSensitivity('identity.fullName');
      expect(level, isNotNull);
    });

    test('returns public for unknown field', () {
      expect(lookupFieldSensitivity('unknown.field'), SensitivityLevel.public);
    });
  });

  group('emptyPropertyValueForType', () {
    test('creates TextProperty for text type', () {
      final value = emptyPropertyValueForType(
        PropertyType.text,
        SensitivityLevel.internal,
      );
      expect(value, isA<TextProperty>());
      expect((value as TextProperty).text, '');
      expect(value.sensitivity, SensitivityLevel.internal);
    });

    test('creates NumberProperty for number type', () {
      final value = emptyPropertyValueForType(
        PropertyType.number,
        SensitivityLevel.sensitive,
      );
      expect(value, isA<NumberProperty>());
      expect((value as NumberProperty).sensitivity, SensitivityLevel.sensitive);
    });

    test('creates DateProperty for date type', () {
      final value = emptyPropertyValueForType(PropertyType.date, SensitivityLevel.public);
      expect(value, isA<DateProperty>());
    });

    test('creates CheckboxProperty for checkbox type', () {
      final value = emptyPropertyValueForType(PropertyType.checkbox, SensitivityLevel.public);
      expect(value, isA<CheckboxProperty>());
      expect((value as CheckboxProperty).checked, false);
    });

    test('creates SelectProperty for select type', () {
      final value = emptyPropertyValueForType(PropertyType.select, SensitivityLevel.public);
      expect(value, isA<SelectProperty>());
      expect((value as SelectProperty).options, isEmpty);
    });

    test('creates MultiSelectProperty for multiSelect type', () {
      final value = emptyPropertyValueForType(PropertyType.multiSelect, SensitivityLevel.public);
      expect(value, isA<MultiSelectProperty>());
      expect((value as MultiSelectProperty).selectedIds, isEmpty);
    });

    test('creates RelationProperty for relation type', () {
      final value = emptyPropertyValueForType(PropertyType.relation, SensitivityLevel.public);
      expect(value, isA<RelationProperty>());
    });

    test('creates UrlProperty for url type', () {
      final value = emptyPropertyValueForType(PropertyType.url, SensitivityLevel.public);
      expect(value, isA<UrlProperty>());
    });
  });
}
