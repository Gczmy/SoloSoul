import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations_en.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';

void main() {
  group('formatFieldLabel', () {
    test('formats camelCase', () {
      expect(formatFieldLabel('givenName'), 'Given Name');
      expect(formatFieldLabel('dateOfBirth'), 'Date Of Birth');
    });

    test('formats snake_case', () {
      expect(formatFieldLabel('visa_type'), 'Visa Type');
      expect(formatFieldLabel('account_number'), 'Account Number');
    });

    test('formats mixed case', () {
      expect(formatFieldLabel('passportNumber'), 'Passport Number');
    });

    test('handles single word', () {
      expect(formatFieldLabel('name'), 'Name');
      expect(formatFieldLabel('NAME'), 'Name');
    });

    test('handles empty string', () {
      expect(formatFieldLabel(''), '');
    });
  });

  group('translateFieldLabel', () {
    final l10n = AppLocalizationsEn();

    test('translates known fields', () {
      expect(translateFieldLabel('fullName', l10n), isNotEmpty);
      expect(translateFieldLabel('givenName', l10n), isNotEmpty);
      expect(translateFieldLabel('dateOfBirth', l10n), isNotEmpty);
      expect(translateFieldLabel('gender', l10n), isNotEmpty);
    });

    test('translates snake_case aliases', () {
      expect(translateFieldLabel('bank_name', l10n), isNotEmpty);
      expect(translateFieldLabel('account_number', l10n), isNotEmpty);
      expect(translateFieldLabel('account_holder', l10n), isNotEmpty);
    });

    test('falls back to formatFieldLabel for unknown keys', () {
      expect(translateFieldLabel('customField', l10n), 'Custom Field');
      expect(translateFieldLabel('my_unknown_key', l10n), 'My Unknown Key');
    });

    test('handles Title key', () {
      expect(translateFieldLabel('Title', l10n), isNotEmpty);
      expect(translateFieldLabel('title', l10n), isNotEmpty);
    });
  });
}
