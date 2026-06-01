import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
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

  group('translateFieldLabel', () {
    final l10n = AppLocalizationsEn();

    test('translates known camelCase keys', () {
      expect(translateFieldLabel('fullName', l10n), l10n.fieldFullName);
      expect(translateFieldLabel('givenName', l10n), l10n.fieldGivenName);
      expect(translateFieldLabel('dateOfBirth', l10n), l10n.fieldDateOfBirth);
      expect(translateFieldLabel('passportNumber', l10n), l10n.fieldPassportNumber);
      expect(translateFieldLabel('bankName', l10n), l10n.fieldBankName);
    });

    test('translates known snake_case keys', () {
      expect(translateFieldLabel('full_name', l10n), l10n.fieldFullName);
      expect(translateFieldLabel('date_of_birth', l10n), l10n.fieldDateOfBirth);
      expect(translateFieldLabel('bank_name', l10n), l10n.fieldBankName);
      expect(translateFieldLabel('account_number', l10n), l10n.fieldAccountNumber);
    });

    test('translates section name keys', () {
      expect(translateFieldLabel('financial', l10n), l10n.sectionFinancial);
      expect(translateFieldLabel('medical', l10n), l10n.sectionMedical);
      expect(translateFieldLabel('security', l10n), l10n.sectionSecurity);
      expect(translateFieldLabel('digitalAccounts', l10n), l10n.sectionDigitalAccounts);
      expect(translateFieldLabel('insurance', l10n), l10n.sectionInsurance);
    });

    test('falls back to formatFieldLabel for unknown keys', () {
      expect(translateFieldLabel('customField', l10n), 'Custom Field');
      expect(translateFieldLabel('unknownKey', l10n), 'Unknown Key');
    });

    test('handles title key mapping', () {
      expect(translateFieldLabel('title', l10n), l10n.fieldTitle);
      expect(translateFieldLabel('Title', l10n), l10n.fieldTitle);
    });

    test('handles bank name snake_case variant', () {
      expect(translateFieldLabel('bank_name', l10n), l10n.fieldBankName);
      expect(translateFieldLabel('bankName', l10n), l10n.fieldBankName);
    });

    test('handles sort code variants', () {
      expect(translateFieldLabel('sortCode', l10n), l10n.fieldSortCode);
      expect(translateFieldLabel('sort_code', l10n), l10n.fieldSortCode);
    });

    test('handles routing number variants', () {
      expect(translateFieldLabel('routingNumber', l10n), l10n.fieldRoutingNumber);
      expect(translateFieldLabel('routing_number', l10n), l10n.fieldRoutingNumber);
    });

    test('handles account type variants', () {
      expect(translateFieldLabel('accountType', l10n), l10n.fieldAccountType);
      expect(translateFieldLabel('account_type', l10n), l10n.fieldAccountType);
    });

    test('handles branch name variants', () {
      expect(translateFieldLabel('branchName', l10n), l10n.fieldBranchName);
      expect(translateFieldLabel('branch_name', l10n), l10n.fieldBranchName);
    });
  });
}
