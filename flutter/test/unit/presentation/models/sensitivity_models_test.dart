import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';

void main() {
  group('FieldRegistry consistency', () {
    test('defaultFields contains every field from all section lists', () {
      final allSectionFields = <String>{
        ...identityFields.map((f) => f.fieldId),
        ...contactFields.map((f) => f.fieldId),
        ...idCardFields.map((f) => f.fieldId),
        ...addressFields.map((f) => f.fieldId),
        ...bankAccountFields.map((f) => f.fieldId),
        ...cardFields.map((f) => f.fieldId),
        ...taxIdFields.map((f) => f.fieldId),
        ...passportFields.map((f) => f.fieldId),
        ...visaFields.map((f) => f.fieldId),
        ...travelFields.map((f) => f.fieldId),
        ...educationFields.map((f) => f.fieldId),
        ...employmentFields.map((f) => f.fieldId),
        ...skillFields.map((f) => f.fieldId),
        ...languageFields.map((f) => f.fieldId),
        ...awardFields.map((f) => f.fieldId),
      };

      final defaultFieldIds =
          FieldRegistry.defaultFields.map((f) => f.fieldId).toSet();

      expect(
        defaultFieldIds,
        equals(allSectionFields),
        reason: 'defaultFields must be derivable from section lists',
      );
    });

    test('defaultFields length equals sum of all section lists', () {
      final sectionLengths = [
        identityFields.length,
        contactFields.length,
        idCardFields.length,
        addressFields.length,
        bankAccountFields.length,
        cardFields.length,
        taxIdFields.length,
        passportFields.length,
        visaFields.length,
        travelFields.length,
        educationFields.length,
        employmentFields.length,
        skillFields.length,
        languageFields.length,
        awardFields.length,
      ].reduce((a, b) => a + b);

      expect(
        FieldRegistry.defaultFields.length,
        equals(sectionLengths),
        reason: 'No duplicate field IDs across section lists',
      );
    });

    test('no duplicate field IDs across section lists', () {
      final allFieldIds = <String>[
        ...identityFields.map((f) => f.fieldId),
        ...contactFields.map((f) => f.fieldId),
        ...idCardFields.map((f) => f.fieldId),
        ...addressFields.map((f) => f.fieldId),
        ...bankAccountFields.map((f) => f.fieldId),
        ...cardFields.map((f) => f.fieldId),
        ...taxIdFields.map((f) => f.fieldId),
        ...passportFields.map((f) => f.fieldId),
        ...visaFields.map((f) => f.fieldId),
        ...travelFields.map((f) => f.fieldId),
        ...educationFields.map((f) => f.fieldId),
        ...employmentFields.map((f) => f.fieldId),
        ...skillFields.map((f) => f.fieldId),
        ...languageFields.map((f) => f.fieldId),
        ...awardFields.map((f) => f.fieldId),
      ];

      final uniqueIds = allFieldIds.toSet();
      expect(
        allFieldIds.length,
        equals(uniqueIds.length),
        reason:
            'Duplicate field IDs found: ${allFieldIds.where((id) => allFieldIds.where((x) => x == id).length > 1).toSet()}',
      );
    });
  });

  group('FieldSensitivity', () {
    test('creates with required fields', () {
      const field = FieldSensitivity(
        fieldId: 'identity.fullName',
        fieldName: 'Full Name',
        fieldSection: 'identity',
        level: SensitivityLevel.public,
      );
      expect(field.fieldId, 'identity.fullName');
      expect(field.fieldName, 'Full Name');
      expect(field.fieldSection, 'identity');
      expect(field.level, SensitivityLevel.public);
    });

    group('fromJson', () {
      test('parses valid JSON', () {
        final json = {
          'fieldId': 'passport.number',
          'fieldName': 'Passport Number',
          'fieldSection': 'passport',
          'level': 3,
        };
        final field = FieldSensitivity.fromJson(json);
        expect(field.fieldId, 'passport.number');
        expect(field.fieldName, 'Passport Number');
        expect(field.fieldSection, 'passport');
        expect(field.level, SensitivityLevel.critical);
      });

      test('falls back to public for invalid level index', () {
        final json = {
          'fieldId': 'test.field',
          'fieldName': 'Test',
          'fieldSection': 'test',
          'level': 99,
        };
        final field = FieldSensitivity.fromJson(json);
        expect(field.level, SensitivityLevel.public);
      });

      test('falls back to public for negative level index', () {
        final json = {
          'fieldId': 'test.field',
          'fieldName': 'Test',
          'fieldSection': 'test',
          'level': -1,
        };
        final field = FieldSensitivity.fromJson(json);
        expect(field.level, SensitivityLevel.public);
      });
    });

    group('toJson', () {
      test('serializes correctly', () {
        const field = FieldSensitivity(
          fieldId: 'card.cvv',
          fieldName: 'CVV',
          fieldSection: 'card',
          level: SensitivityLevel.critical,
        );
        final json = field.toJson();
        expect(json['fieldId'], 'card.cvv');
        expect(json['fieldName'], 'CVV');
        expect(json['fieldSection'], 'card');
        expect(json['level'], 3);
      });
    });

    group('JSON round-trip', () {
      test('preserves all fields', () {
        const original = FieldSensitivity(
          fieldId: 'bankAccount.swiftBic',
          fieldName: 'SWIFT/BIC',
          fieldSection: 'bankAccount',
          level: SensitivityLevel.critical,
        );
        final restored = FieldSensitivity.fromJson(original.toJson());
        expect(restored.fieldId, original.fieldId);
        expect(restored.fieldName, original.fieldName);
        expect(restored.fieldSection, original.fieldSection);
        expect(restored.level, original.level);
      });
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const original = FieldSensitivity(
          fieldId: 'identity.fullName',
          fieldName: 'Full Name',
          fieldSection: 'identity',
          level: SensitivityLevel.public,
        );
        final copy = original.copyWith();
        expect(copy.fieldId, original.fieldId);
        expect(copy.fieldName, original.fieldName);
        expect(copy.level, original.level);
      });

      test('copies with changes', () {
        const original = FieldSensitivity(
          fieldId: 'identity.fullName',
          fieldName: 'Full Name',
          fieldSection: 'identity',
          level: SensitivityLevel.public,
        );
        final copy = original.copyWith(level: SensitivityLevel.sensitive);
        expect(copy.level, SensitivityLevel.sensitive);
        expect(copy.fieldId, original.fieldId);
      });
    });

    group('equality', () {
      test('equal when fieldId matches', () {
        const a = FieldSensitivity(
          fieldId: 'identity.fullName',
          fieldName: 'Name A',
          fieldSection: 'identity',
          level: SensitivityLevel.public,
        );
        const b = FieldSensitivity(
          fieldId: 'identity.fullName',
          fieldName: 'Name B',
          fieldSection: 'other',
          level: SensitivityLevel.critical,
        );
        expect(a, equals(b));
        expect(a.hashCode, equals(b.hashCode));
      });

      test('not equal when fieldId differs', () {
        const a = FieldSensitivity(
          fieldId: 'identity.fullName',
          fieldName: 'Full Name',
          fieldSection: 'identity',
          level: SensitivityLevel.public,
        );
        const b = FieldSensitivity(
          fieldId: 'identity.givenName',
          fieldName: 'Full Name',
          fieldSection: 'identity',
          level: SensitivityLevel.public,
        );
        expect(a, isNot(equals(b)));
      });
    });
  });

  group('firstWhereOrNull', () {
    test('returns matching element', () {
      const items = [
        FieldSensitivity(
          fieldId: 'a',
          fieldName: 'A',
          fieldSection: 's',
          level: SensitivityLevel.public,
        ),
        FieldSensitivity(
          fieldId: 'b',
          fieldName: 'B',
          fieldSection: 's',
          level: SensitivityLevel.sensitive,
        ),
      ];
      final result = firstWhereOrNull(items, (f) => f.fieldId == 'b');
      expect(result, isNotNull);
      expect(result!.fieldName, 'B');
    });

    test('returns null when no match', () {
      const items = [
        FieldSensitivity(
          fieldId: 'a',
          fieldName: 'A',
          fieldSection: 's',
          level: SensitivityLevel.public,
        ),
      ];
      final result = firstWhereOrNull(items, (f) => f.fieldId == 'z');
      expect(result, isNull);
    });

    test('returns null for empty list', () {
      final result = firstWhereOrNull([], (f) => true);
      expect(result, isNull);
    });
  });

  group('FieldRegistry', () {
    test('defaultFields is not empty', () {
      expect(FieldRegistry.defaultFields, isNotEmpty);
    });

    test('contains expected sections', () {
      final sections =
          FieldRegistry.defaultFields.map((f) => f.fieldSection).toSet();
      expect(sections, contains('identity'));
      expect(sections, contains('contact'));
      expect(sections, contains('idCard'));
      expect(sections, contains('address'));
      expect(sections, contains('passport'));
      expect(sections, contains('bankAccount'));
      expect(sections, contains('card'));
    });

    group('isFieldRestricted', () {
      test('returns true for critical fields', () {
        expect(FieldRegistry.isFieldRestricted('idCard.number'), isTrue);
        expect(FieldRegistry.isFieldRestricted('card.cvv'), isTrue);
        expect(FieldRegistry.isFieldRestricted('bankAccount.accountNumber'),
            isTrue);
        expect(FieldRegistry.isFieldRestricted('passport.number'), isTrue);
      });

      test('returns false for non-critical fields', () {
        expect(FieldRegistry.isFieldRestricted('identity.fullName'), isFalse);
        expect(
            FieldRegistry.isFieldRestricted('travel.destination'), isFalse);
      });

      test('returns false for unknown field', () {
        expect(FieldRegistry.isFieldRestricted('nonexistent.field'), isFalse);
      });
    });

    group('getFieldsBySection', () {
      test('returns identity fields', () {
        final fields = FieldRegistry.getFieldsBySection('identity');
        expect(fields, isNotEmpty);
        expect(fields.every((f) => f.fieldSection == 'identity'), isTrue);
      });

      test('returns empty for unknown section', () {
        final fields = FieldRegistry.getFieldsBySection('nonexistent');
        expect(fields, isEmpty);
      });
    });

    group('allSections', () {
      test('contains expected section names', () {
        final sections = FieldRegistry.allSections;
        expect(sections, contains('identity'));
        expect(sections, contains('contact'));
        expect(sections, contains('bankAccount'));
        expect(sections, contains('card'));
        expect(sections, contains('passport'));
        expect(sections, contains('education'));
        expect(sections, contains('employment'));
      });

      test('has 15 sections', () {
        expect(FieldRegistry.allSections.length, 15);
      });
    });

    group('getSectionDisplayName', () {
      test('returns display names for known sections', () {
        expect(FieldRegistry.getSectionDisplayName('identity'), 'Identity');
        expect(FieldRegistry.getSectionDisplayName('contact'), 'Contact');
        expect(FieldRegistry.getSectionDisplayName('idCard'), 'ID Card');
        expect(FieldRegistry.getSectionDisplayName('bankAccount'),
            'Bank Account');
        expect(FieldRegistry.getSectionDisplayName('taxId'), 'Tax ID');
      });

      test('returns raw name for unknown section', () {
        expect(FieldRegistry.getSectionDisplayName('unknown'), 'unknown');
      });
    });

    group('isValidFieldId', () {
      test('returns true for valid field IDs', () {
        expect(FieldRegistry.isValidFieldId('identity.fullName'), isTrue);
        expect(FieldRegistry.isValidFieldId('card.cvv'), isTrue);
        expect(FieldRegistry.isValidFieldId('passport.number'), isTrue);
      });

      test('returns false for invalid field IDs', () {
        expect(FieldRegistry.isValidFieldId('nonexistent.field'), isFalse);
        expect(FieldRegistry.isValidFieldId(''), isFalse);
      });
    });
  });

  group('FieldIds constants', () {
    test('have expected values', () {
      expect(FieldIds.dateOfBirth, 'identity.dateOfBirth');
      expect(FieldIds.nationality, 'identity.nationality');
      expect(FieldIds.passportNumber, 'passport.number');
      expect(FieldIds.cardNumber, 'card.cardNumber');
      expect(FieldIds.accountNumber, 'bankAccount.accountNumber');
      expect(FieldIds.gpa, 'education.gpa');
    });
  });

  group('field definition lists', () {
    test('identityFields has 6 entries', () {
      expect(identityFields, hasLength(6));
    });

    test('contactFields has 3 entries', () {
      expect(contactFields, hasLength(3));
    });

    test('idCardFields has 6 entries', () {
      expect(idCardFields, hasLength(6));
    });

    test('bankAccountFields has 8 entries', () {
      expect(bankAccountFields, hasLength(8));
    });

    test('cardFields has 7 entries', () {
      expect(cardFields, hasLength(7));
    });

    test('passportFields has 13 entries', () {
      expect(passportFields, hasLength(13));
    });

    test('all fields have non-empty fieldId', () {
      for (final field in FieldRegistry.defaultFields) {
        expect(field.fieldId, isNotEmpty);
        expect(field.fieldName, isNotEmpty);
        expect(field.fieldSection, isNotEmpty);
      }
    });
  });
}
