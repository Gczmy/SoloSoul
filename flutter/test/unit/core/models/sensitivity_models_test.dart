import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';

void main() {
  group('firstWhereOrNull', () {
    test('returns first matching element', () {
      final list = [
        const FieldSensitivity(fieldId: 'a', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public),
        const FieldSensitivity(fieldId: 'b', fieldName: 'B', fieldSection: 's1', level: SensitivityLevel.sensitive),
      ];
      final result = firstWhereOrNull(list, (f) => f.level == SensitivityLevel.sensitive);
      expect(result, isNotNull);
      expect(result!.fieldId, 'b');
    });

    test('returns null when no match', () {
      final list = [
        const FieldSensitivity(fieldId: 'a', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public),
      ];
      final result = firstWhereOrNull(list, (f) => f.level == SensitivityLevel.critical);
      expect(result, isNull);
    });

    test('returns null for empty list', () {
      expect(firstWhereOrNull(<FieldSensitivity>[], (_) => true), isNull);
    });
  });

  group('FieldSensitivity', () {
    const field = FieldSensitivity(
      fieldId: 'test.field',
      fieldName: 'Test Field',
      fieldSection: 'test',
      level: SensitivityLevel.internal,
    );

    test('copyWith creates updated copy', () {
      final copy = field.copyWith(level: SensitivityLevel.critical);
      expect(copy.fieldId, 'test.field');
      expect(copy.level, SensitivityLevel.critical);
    });

    test('copyWith preserves unchanged fields', () {
      final copy = field.copyWith();
      expect(copy.fieldId, field.fieldId);
      expect(copy.fieldName, field.fieldName);
      expect(copy.fieldSection, field.fieldSection);
      expect(copy.level, field.level);
    });

    test('toJson serializes correctly', () {
      final json = field.toJson();
      expect(json['fieldId'], 'test.field');
      expect(json['fieldName'], 'Test Field');
      expect(json['fieldSection'], 'test');
      expect(json['level'], SensitivityLevel.internal.index);
    });

    test('fromJson deserializes correctly', () {
      final json = {
        'fieldId': 'test.field',
        'fieldName': 'Test Field',
        'fieldSection': 'test',
        'level': 2,
      };
      final restored = FieldSensitivity.fromJson(json);
      expect(restored.fieldId, 'test.field');
      expect(restored.level, SensitivityLevel.values[2]);
    });

    test('fromJson defaults to public for invalid level index', () {
      final json = {
        'fieldId': 'test.field',
        'fieldName': 'Test',
        'fieldSection': 'test',
        'level': 999,
      };
      final restored = FieldSensitivity.fromJson(json);
      expect(restored.level, SensitivityLevel.public);
    });

    test('fromJson defaults to public for negative level index', () {
      final json = {
        'fieldId': 'test.field',
        'fieldName': 'Test',
        'fieldSection': 'test',
        'level': -1,
      };
      final restored = FieldSensitivity.fromJson(json);
      expect(restored.level, SensitivityLevel.public);
    });

    test('equality based on fieldId only', () {
      const a = FieldSensitivity(fieldId: 'same', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public);
      const b = FieldSensitivity(fieldId: 'same', fieldName: 'B', fieldSection: 's2', level: SensitivityLevel.critical);
      expect(a, b);
      expect(a.hashCode, b.hashCode);
    });

    test('inequality for different fieldIds', () {
      const a = FieldSensitivity(fieldId: 'a', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public);
      const b = FieldSensitivity(fieldId: 'b', fieldName: 'A', fieldSection: 's1', level: SensitivityLevel.public);
      expect(a, isNot(b));
    });

    test('round-trip serialization', () {
      final json = field.toJson();
      final restored = FieldSensitivity.fromJson(json);
      expect(restored, field);
    });
  });

  group('FieldRegistry', () {
    test('defaultFields is non-empty', () {
      expect(FieldRegistry.defaultFields, isNotEmpty);
    });

    test('defaultFields contains identity fields', () {
      expect(
        FieldRegistry.defaultFields.any((f) => f.fieldSection == 'identity'),
        isTrue,
      );
    });

    test('defaultFields contains no duplicate fieldIds', () {
      final ids = FieldRegistry.defaultFields.map((f) => f.fieldId).toList();
      expect(ids.toSet().length, ids.length);
    });
  });
}
