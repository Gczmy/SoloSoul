import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';

void main() {
  group('FieldSensitivity', () {
    test('constructs correctly', () {
      const fs = FieldSensitivity(
        fieldId: 'identity.name',
        fieldName: 'Name',
        fieldSection: 'identity',
        level: SensitivityLevel.public,
      );
      expect(fs.fieldId, 'identity.name');
      expect(fs.level, SensitivityLevel.public);
    });

    test('copyWith updates fields', () {
      const fs = FieldSensitivity(
        fieldId: 'identity.name',
        fieldName: 'Name',
        fieldSection: 'identity',
        level: SensitivityLevel.public,
      );
      final updated = fs.copyWith(level: SensitivityLevel.sensitive);
      expect(updated.level, SensitivityLevel.sensitive);
      expect(updated.fieldId, 'identity.name');
    });

    test('toJson and fromJson round-trip', () {
      const fs = FieldSensitivity(
        fieldId: 'identity.name',
        fieldName: 'Name',
        fieldSection: 'identity',
        level: SensitivityLevel.sensitive,
      );
      final json = fs.toJson();
      final restored = FieldSensitivity.fromJson(json);
      expect(restored.fieldId, fs.fieldId);
      expect(restored.level, fs.level);
    });

    test('fromJson handles invalid level index', () {
      final json = {
        'fieldId': 'test',
        'fieldName': 'Test',
        'fieldSection': 'test',
        'level': 999,
      };
      final fs = FieldSensitivity.fromJson(json);
      expect(fs.level, SensitivityLevel.public);
    });

    test('equality based on fieldId', () {
      const fs1 = FieldSensitivity(
        fieldId: 'a', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public,
      );
      const fs2 = FieldSensitivity(
        fieldId: 'a', fieldName: 'B', fieldSection: 's2', level: SensitivityLevel.sensitive,
      );
      expect(fs1, fs2);
    });

    test('hashCode based on fieldId', () {
      const fs1 = FieldSensitivity(
        fieldId: 'a', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public,
      );
      const fs2 = FieldSensitivity(
        fieldId: 'a', fieldName: 'B', fieldSection: 's2', level: SensitivityLevel.sensitive,
      );
      expect(fs1.hashCode, fs2.hashCode);
    });
  });

  group('Field definitions', () {
    test('identityFields is not empty', () {
      expect(identityFields, isNotEmpty);
    });

    test('contactFields is not empty', () {
      expect(contactFields, isNotEmpty);
    });

    test('passportFields is not empty', () {
      expect(passportFields, isNotEmpty);
    });

    test('visaFields is not empty', () {
      expect(visaFields, isNotEmpty);
    });

    test('bankAccountFields is not empty', () {
      expect(bankAccountFields, isNotEmpty);
    });

    test('all section fields have valid data', () {
      final allFields = [
        ...identityFields,
        ...contactFields,
        ...passportFields,
        ...visaFields,
        ...bankAccountFields,
        ...idCardFields,
        ...addressFields,
        ...cardFields,
        ...taxIdFields,
        ...travelFields,
        ...educationFields,
        ...employmentFields,
        ...skillFields,
        ...languageFields,
        ...awardFields,
        ...articleFields,
      ];
      expect(allFields, isNotEmpty);
      for (final field in allFields) {
        expect(field.fieldId, isNotEmpty);
        expect(field.fieldName, isNotEmpty);
      }
    });
  });

  group('firstWhereOrNull', () {
    test('returns matching element', () {
      final result = firstWhereOrNull(
        identityFields,
        (f) => f.fieldId == 'identity.fullName',
      );
      expect(result, isNotNull);
      expect(result!.fieldName, 'Full Name');
    });

    test('returns null for no match', () {
      final result = firstWhereOrNull(
        identityFields,
        (f) => f.fieldId == 'nonexistent',
      );
      expect(result, isNull);
    });
  });
}
