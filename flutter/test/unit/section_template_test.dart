import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/section_template.dart';

void main() {
  group('PresetSectionTemplates', () {
    test('templates list is not empty', () {
      expect(PresetSectionTemplates.templates, isNotEmpty);
    });

    test('contains china bank account template', () {
      final template = PresetSectionTemplates.templates
          .firstWhere((t) => t.id == 'china_bank_account');
      expect(template.pageTag, 'bank');
      expect(template.icon, '🏦');
      expect(template.fields.length, 4);
    });

    test('contains UK bank account template', () {
      final template = PresetSectionTemplates.templates
          .firstWhere((t) => t.id == 'uk_bank_account');
      expect(template.pageTag, 'bank');
      expect(template.icon, '🏛️');
    });

    test('all templates have valid fields', () {
      for (final template in PresetSectionTemplates.templates) {
        expect(template.id, isNotEmpty);
        expect(template.nameKey, isNotEmpty);
        expect(template.pageTag, isNotEmpty);
        expect(template.fields, isNotEmpty);

        for (final field in template.fields) {
          expect(field.key, isNotEmpty);
          expect(field.type, isNotEmpty);
        }
      }
    });

    test('critical fields exist in bank templates', () {
      final chinaBank = PresetSectionTemplates.templates
          .firstWhere((t) => t.id == 'china_bank_account');
      final accountNumber = chinaBank.fields
          .firstWhere((f) => f.key == 'account_number');
      expect(accountNumber.sensitivity, SensitivityLevel.critical);
    });
  });

  group('SectionTemplate', () {
    test('constructs correctly', () {
      const template = SectionTemplate(
        id: 'test',
        nameKey: 'testName',
        descriptionKey: 'testDesc',
        fields: [],
        icon: '📋',
        pageTag: 'profile',
      );
      expect(template.id, 'test');
      expect(template.icon, '📋');
    });
  });

  group('TemplateField', () {
    test('constructs with required fields', () {
      const field = TemplateField(
        key: 'name',
        type: 'text',
        sensitivity: SensitivityLevel.public,
      );
      expect(field.key, 'name');
      expect(field.type, 'text');
      expect(field.config, isNull);
    });

    test('constructs with optional config', () {
      const field = TemplateField(
        key: 'type',
        type: 'select',
        sensitivity: SensitivityLevel.public,
        config: 'options',
      );
      expect(field.config, 'options');
    });
  });
}
