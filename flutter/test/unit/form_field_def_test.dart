import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/widgets/form_field_def.dart';

void main() {
  group('FormFieldDef', () {
    test('constructs with required fields', () {
      const field = FormFieldDef(
        fieldId: 'name',
        label: 'Full Name',
      );
      expect(field.fieldId, 'name');
      expect(field.label, 'Full Name');
      expect(field.sensitivity, SensitivityLevel.public);
      expect(field.hintText, isNull);
      expect(field.initialValue, isNull);
    });

    test('constructs with all fields', () {
      const field = FormFieldDef(
        fieldId: 'email',
        label: 'Email',
        hintText: 'Enter your email',
        sensitivity: SensitivityLevel.sensitive,
        initialValue: 'test@example.com',
      );
      expect(field.hintText, 'Enter your email');
      expect(field.sensitivity, SensitivityLevel.sensitive);
      expect(field.initialValue, 'test@example.com');
    });
  });
}
