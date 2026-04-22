import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

void main() {
  group('SensitivityLevel', () {
    test('has correct rank values', () {
      expect(SensitivityLevel.public.rank, 0);
      expect(SensitivityLevel.internal.rank, 1);
      expect(SensitivityLevel.sensitive.rank, 2);
      expect(SensitivityLevel.critical.rank, 3);
    });

    test('isAtLeast returns true for equal or higher levels', () {
      expect(SensitivityLevel.public.isAtLeast(SensitivityLevel.public), true);
      expect(SensitivityLevel.internal.isAtLeast(SensitivityLevel.public), true);
      expect(SensitivityLevel.sensitive.isAtLeast(SensitivityLevel.internal), true);
      expect(SensitivityLevel.critical.isAtLeast(SensitivityLevel.sensitive), true);
    });

    test('isAtLeast returns false for lower levels', () {
      expect(SensitivityLevel.public.isAtLeast(SensitivityLevel.internal), false);
      expect(SensitivityLevel.internal.isAtLeast(SensitivityLevel.sensitive), false);
      expect(SensitivityLevel.sensitive.isAtLeast(SensitivityLevel.critical), false);
    });

    test('label returns human-readable string', () {
      expect(SensitivityLevel.public.label, 'Public');
      expect(SensitivityLevel.internal.label, 'Internal');
      expect(SensitivityLevel.sensitive.label, 'Sensitive');
      expect(SensitivityLevel.critical.label, 'Critical');
    });

    test('description returns descriptive string', () {
      expect(SensitivityLevel.public.description, contains('Freely'));
      expect(SensitivityLevel.critical.description, contains('Maximum'));
    });
  });

  group('FieldSensitivity', () {
    test('creates correctly with all fields', () {
      final field = FieldSensitivity(
        fieldId: 'test.field',
        fieldName: 'Test Field',
        fieldSection: 'test',
        level: SensitivityLevel.sensitive,
      );

      expect(field.fieldId, 'test.field');
      expect(field.fieldName, 'Test Field');
      expect(field.fieldSection, 'test');
      expect(field.level, SensitivityLevel.sensitive);
    });

    test('copyWith creates new instance with updated values', () {
      final original = FieldSensitivity(
        fieldId: 'test.field',
        fieldName: 'Original',
        fieldSection: 'test',
        level: SensitivityLevel.public,
      );

      final copied = original.copyWith(
        fieldName: 'Updated',
        level: SensitivityLevel.critical,
      );

      expect(copied.fieldId, 'test.field');
      expect(copied.fieldName, 'Updated');
      expect(copied.level, SensitivityLevel.critical);
      expect(original.fieldName, 'Original');
      expect(original.level, SensitivityLevel.public);
    });

    test('toJson and fromJson roundtrip preserves data', () {
      final original = FieldSensitivity(
        fieldId: 'test.field',
        fieldName: 'Test Field',
        fieldSection: 'test',
        level: SensitivityLevel.sensitive,
      );

      final json = original.toJson();
      final restored = FieldSensitivity.fromJson(json);

      expect(restored.fieldId, original.fieldId);
      expect(restored.fieldName, original.fieldName);
      expect(restored.fieldSection, original.fieldSection);
      expect(restored.level, original.level);
    });

    test('equality is based on fieldId', () {
      final field1 = FieldSensitivity(
        fieldId: 'same.id',
        fieldName: 'Field 1',
        fieldSection: 'section',
        level: SensitivityLevel.public,
      );

      final field2 = FieldSensitivity(
        fieldId: 'same.id',
        fieldName: 'Field 2',
        fieldSection: 'different',
        level: SensitivityLevel.critical,
      );

      final field3 = FieldSensitivity(
        fieldId: 'different.id',
        fieldName: 'Field 1',
        fieldSection: 'section',
        level: SensitivityLevel.public,
      );

      expect(field1 == field2, true);
      expect(field1 == field3, false);
      expect(field1.hashCode, field2.hashCode);
    });
  });

  group('FieldRegistry', () {
    test('defaultFields contains expected sections', () {
      final sections = FieldRegistry.allSections;

      expect(sections, contains('identity'));
      expect(sections, contains('travel'));
      expect(sections, contains('bankAccount'));
      expect(sections, contains('card'));
      expect(sections, contains('education'));
      expect(sections, contains('employment'));
    });

    test('isFieldRestricted returns true for critical fields', () {
      expect(FieldRegistry.isFieldRestricted('idCard.number'), true);
      expect(FieldRegistry.isFieldRestricted('passport.number'), true);
      expect(FieldRegistry.isFieldRestricted('bankAccount.accountNumber'), true);
    });

    test('isFieldRestricted returns false for non-critical fields', () {
      expect(FieldRegistry.isFieldRestricted('identity.fullName'), false);
      expect(FieldRegistry.isFieldRestricted('travel.destination'), false);
    });

    test('isValidFieldId returns true for known fields', () {
      expect(FieldRegistry.isValidFieldId('identity.fullName'), true);
      expect(FieldRegistry.isValidFieldId('passport.number'), true);
    });

    test('isValidFieldId returns false for unknown fields', () {
      expect(FieldRegistry.isValidFieldId('unknown.field'), false);
    });

    test('getFieldsBySection returns only matching section', () {
      final identityFields = FieldRegistry.getFieldsBySection('identity');
      expect(identityFields, isNotEmpty);
      expect(
        identityFields.every((f) => f.fieldSection == 'identity'),
        true,
      );
    });

    test('getSectionDisplayName returns human-readable names', () {
      expect(FieldRegistry.getSectionDisplayName('identity'), 'Identity');
      expect(FieldRegistry.getSectionDisplayName('bankAccount'), 'Bank Account');
      expect(FieldRegistry.getSectionDisplayName('unknown'), 'unknown');
    });
  });

  group('FormFieldRegistry', () {
    setUp(() {
      FormFieldRegistry.reset();
    });

    tearDown(() {
      FormFieldRegistry.reset();
    });

    test('register adds field to registry', () {
      final field = FieldSensitivity(
        fieldId: 'custom.field',
        fieldName: 'Custom Field',
        fieldSection: 'custom',
        level: SensitivityLevel.internal,
      );

      FormFieldRegistry.register(field);

      expect(FormFieldRegistry.getField('custom.field'), field);
    });

    test('register replaces existing field with same id', () {
      final field1 = FieldSensitivity(
        fieldId: 'custom.field',
        fieldName: 'Original',
        fieldSection: 'custom',
        level: SensitivityLevel.public,
      );

      final field2 = FieldSensitivity(
        fieldId: 'custom.field',
        fieldName: 'Updated',
        fieldSection: 'custom',
        level: SensitivityLevel.critical,
      );

      FormFieldRegistry.register(field1);
      FormFieldRegistry.register(field2);

      final retrieved = FormFieldRegistry.getField('custom.field');
      expect(retrieved!.fieldName, 'Updated');
      expect(retrieved.level, SensitivityLevel.critical);
    });

    test('registerAll adds multiple fields', () {
      final fields = [
        FieldSensitivity(
          fieldId: 'field1',
          fieldName: 'Field 1',
          fieldSection: 'test',
          level: SensitivityLevel.public,
        ),
        FieldSensitivity(
          fieldId: 'field2',
          fieldName: 'Field 2',
          fieldSection: 'test',
          level: SensitivityLevel.sensitive,
        ),
      ];

      FormFieldRegistry.registerAll(fields);

      expect(FormFieldRegistry.getField('field1'), isNotNull);
      expect(FormFieldRegistry.getField('field2'), isNotNull);
    });

    test('getField returns null for unknown field', () {
      expect(FormFieldRegistry.getField('unknown'), isNull);
    });

    test('getField falls back to FieldRegistry default', () {
      FormFieldRegistry.reset();
      final field = FormFieldRegistry.getField('identity.fullName');
      expect(field, isNotNull);
      expect(field!.fieldId, 'identity.fullName');
    });

    test('reset clears all registered fields', () {
      FormFieldRegistry.register(FieldSensitivity(
        fieldId: 'temp.field',
        fieldName: 'Temp',
        fieldSection: 'temp',
        level: SensitivityLevel.public,
      ));

      FormFieldRegistry.reset();

      expect(FormFieldRegistry.getField('temp.field'), isNull);
      expect(FormFieldRegistry.isRegistered('temp.field'), false);
    });

    test('isRegistered returns true for registered fields', () {
      FormFieldRegistry.register(FieldSensitivity(
        fieldId: 'registered.field',
        fieldName: 'Registered',
        fieldSection: 'test',
        level: SensitivityLevel.public,
      ));

      expect(FormFieldRegistry.isRegistered('registered.field'), true);
      expect(FormFieldRegistry.isRegistered('unregistered.field'), false);
    });
  });

  group('FormFieldRegistryNotifier', () {
    late FormFieldRegistryNotifier notifier;

    setUp(() {
      notifier = FormFieldRegistryNotifier();
    });

    test('register adds field to state', () {
      final field = FieldSensitivity(
        fieldId: 'notifier.field',
        fieldName: 'Notifier Field',
        fieldSection: 'notifier',
        level: SensitivityLevel.sensitive,
      );

      notifier.register(field);

      expect(notifier.getField('notifier.field'), field);
    });

    test('registerAll adds multiple fields', () {
      final fields = [
        FieldSensitivity(
          fieldId: 'n1',
          fieldName: 'N1',
          fieldSection: 'test',
          level: SensitivityLevel.public,
        ),
        FieldSensitivity(
          fieldId: 'n2',
          fieldName: 'N2',
          fieldSection: 'test',
          level: SensitivityLevel.internal,
        ),
      ];

      notifier.registerAll(fields);

      expect(notifier.getField('n1'), isNotNull);
      expect(notifier.getField('n2'), isNotNull);
    });

    test('reset clears state', () {
      notifier.register(FieldSensitivity(
        fieldId: 'temp',
        fieldName: 'Temp',
        fieldSection: 'temp',
        level: SensitivityLevel.public,
      ));

      notifier.reset();

      expect(notifier.getField('temp'), isNull);
    });

    test('registerAllForms registers all form sections', () {
      notifier.registerAllForms();
      final fields = notifier.getAllFields();

      expect(fields.length, greaterThan(50));
    });
  });

  group('SensitivityResolver', () {
    const resolver = SensitivityResolver();

    test('returns public for unknown field with no overrides', () {
      final result = resolver.resolve(
        fieldId: 'unknown.field',
        fieldSettings: {},
        revealedFields: {},
      );

      expect(result, SensitivityLevel.public);
    });

    test('returns public for revealed field', () {
      final result = resolver.resolve(
        fieldId: 'secret.field',
        fieldSettings: {'secret.field': SensitivityLevel.critical},
        revealedFields: {'secret.field'},
      );

      expect(result, SensitivityLevel.public);
    });

    test('returns user override when present', () {
      final result = resolver.resolve(
        fieldId: 'custom.field',
        fieldSettings: {'custom.field': SensitivityLevel.internal},
        revealedFields: {},
      );

      expect(result, SensitivityLevel.internal);
    });

    test('returns tag-based default when no user override', () {
      final result = resolver.resolve(
        fieldId: 'tagged.field',
        fieldSettings: {},
        revealedFields: {},
        tags: ['financial'],
      );

      expect(result, SensitivityLevel.critical);
    });

    test('returns FormFieldRegistry default when no override or tag', () {
      FormFieldRegistry.register(FieldSensitivity(
        fieldId: 'registry.field',
        fieldName: 'Registry Field',
        fieldSection: 'test',
        level: SensitivityLevel.sensitive,
      ));

      final result = resolver.resolve(
        fieldId: 'registry.field',
        fieldSettings: {},
        revealedFields: {},
        tags: [],
      );

      expect(result, SensitivityLevel.sensitive);

      FormFieldRegistry.reset();
    });

    test('precedence: reveal > user override > tag > registry > fallback', () {
      FormFieldRegistry.register(FieldSensitivity(
        fieldId: 'priority.field',
        fieldName: 'Priority',
        fieldSection: 'test',
        level: SensitivityLevel.public,
      ));

      // Revealed wins over user override
      var result = resolver.resolve(
        fieldId: 'priority.field',
        fieldSettings: {'priority.field': SensitivityLevel.critical},
        revealedFields: {'priority.field'},
        tags: [],
      );
      expect(result, SensitivityLevel.public);

      // User override wins over tag
      result = resolver.resolve(
        fieldId: 'priority.field',
        fieldSettings: {'priority.field': SensitivityLevel.internal},
        revealedFields: {},
        tags: ['financial'],
      );
      expect(result, SensitivityLevel.internal);

      // Tag wins over registry
      result = resolver.resolve(
        fieldId: 'priority.field',
        fieldSettings: {},
        revealedFields: {},
        tags: ['health'],
      );
      expect(result, SensitivityLevel.critical);

      // Registry wins over fallback
      result = resolver.resolve(
        fieldId: 'priority.field',
        fieldSettings: {},
        revealedFields: {},
        tags: [],
      );
      expect(result, SensitivityLevel.public);

      FormFieldRegistry.reset();
    });
  });

  group('firstWhereOrNull', () {
    test('returns first matching element', () {
      final fields = [
        FieldSensitivity(
          fieldId: 'a',
          fieldName: 'A',
          fieldSection: 'test',
          level: SensitivityLevel.public,
        ),
        FieldSensitivity(
          fieldId: 'b',
          fieldName: 'B',
          fieldSection: 'test',
          level: SensitivityLevel.sensitive,
        ),
      ];

      final result = firstWhereOrNull(
        fields,
        (f) => f.fieldId == 'b',
      );

      expect(result, fields[1]);
    });

    test('returns null when no match', () {
      final fields = [
        FieldSensitivity(
          fieldId: 'a',
          fieldName: 'A',
          fieldSection: 'test',
          level: SensitivityLevel.public,
        ),
      ];

      final result = firstWhereOrNull(
        fields,
        (f) => f.fieldId == 'nonexistent',
      );

      expect(result, isNull);
    });

    test('returns null for empty list', () {
      final result = firstWhereOrNull(
        <FieldSensitivity>[],
        (f) => true,
      );

      expect(result, isNull);
    });
  });
}
