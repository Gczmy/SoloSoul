import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
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
        final customType = const ObjectTypeDefinition(
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

      test('built-in takes priority over custom with same ID', () {
        final customPage = const ObjectTypeDefinition(
          id: 'page',
          name: 'Custom Page',
          iconName: 'star',
          defaultLayout: ObjectLayout.document,
        );
        final type =
            ObjectTypeRegistry.getType('page', customTypes: [customPage]);
        expect(type, isNotNull);
        expect(type!.name, 'Page'); // built-in wins
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
        final customType = const ObjectTypeDefinition(
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
}
