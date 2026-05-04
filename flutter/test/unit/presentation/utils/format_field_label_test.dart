import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';

void main() {
  group('formatFieldLabel', () {
    test('converts camelCase to Title Case', () {
      expect(formatFieldLabel('givenName'), 'Given Name');
      expect(formatFieldLabel('dateOfBirth'), 'Date Of Birth');
      expect(formatFieldLabel('firstName'), 'First Name');
      expect(formatFieldLabel('phoneNumber'), 'Phone Number');
    });

    test('converts snake_case to Title Case', () {
      expect(formatFieldLabel('visa_type'), 'Visa Type');
      expect(formatFieldLabel('first_name'), 'First Name');
      expect(formatFieldLabel('date_of_birth'), 'Date Of Birth');
    });

    test('handles single word', () {
      expect(formatFieldLabel('name'), 'Name');
      expect(formatFieldLabel('email'), 'Email');
      expect(formatFieldLabel('id'), 'Id');
    });

    test('handles empty string', () {
      expect(formatFieldLabel(''), '');
    });

    test('handles already capitalized input', () {
      expect(formatFieldLabel('Name'), 'Name');
      expect(formatFieldLabel('EMAIL'), 'Email');
    });

    test('handles mixed camelCase and snake_case', () {
      expect(formatFieldLabel('visa_Type'), 'Visa Type');
      // 'ID' split on _ → each word capitalized → 'Id'
      expect(formatFieldLabel('ID_Card'), 'Id Card');
    });

    test('handles consecutive uppercase letters', () {
      // regex matches [a-z][A-Z] → 'user ID' → each word capitalized
      expect(formatFieldLabel('userID'), 'User Id');
      // 'HTMLElement' → no [a-z][A-Z] match → lowered → 'Htmlelement'
      expect(formatFieldLabel('HTMLElement'), 'Htmlelement');
    });

    test('handles string with numbers', () {
      expect(formatFieldLabel('address1'), 'Address1');
      // Regex only matches [a-z][A-Z], digit-uppercase is not split
      expect(formatFieldLabel('field2Name'), 'Field2name');
    });
  });
}
