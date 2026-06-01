import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/scan/scan_section_detector.dart';

void main() {
  group('ScanSectionDetector', () {
    group('filenameHintsPersonal', () {
      test('returns true for resume filenames', () {
        expect(ScanSectionDetector.filenameHintsPersonal('my_resume.pdf'), isTrue);
        expect(ScanSectionDetector.filenameHintsPersonal('CV_2025.docx'), isTrue);
      });

      test('returns true for passport filenames', () {
        expect(ScanSectionDetector.filenameHintsPersonal('passport_scan.jpg'), isTrue);
      });

      test('returns true for bank filenames', () {
        expect(ScanSectionDetector.filenameHintsPersonal('bank_statement.pdf'), isTrue);
      });

      test('returns true for identity filenames', () {
        expect(ScanSectionDetector.filenameHintsPersonal('identity_card.png'), isTrue);
      });

      test('returns false for unrelated filenames', () {
        expect(ScanSectionDetector.filenameHintsPersonal('vacation_photo.jpg'), isFalse);
        expect(ScanSectionDetector.filenameHintsPersonal('shopping_list.txt'), isFalse);
      });

      test('is case insensitive', () {
        expect(ScanSectionDetector.filenameHintsPersonal('RESUME.PDF'), isTrue);
        expect(ScanSectionDetector.filenameHintsPersonal('Passport.jpg'), isTrue);
      });
    });

    group('detectSectionsFromFilename', () {
      test('detects resume sections', () {
        final sections = ScanSectionDetector.detectSectionsFromFilename('my_resume.pdf');
        expect(sections.length, 2);
        expect(sections.map((s) => s.section), contains('identity'));
        expect(sections.map((s) => s.section), contains('education'));
      });

      test('detects cv sections', () {
        final sections = ScanSectionDetector.detectSectionsFromFilename('cv.docx');
        expect(sections.map((s) => s.section), contains('identity'));
      });

      test('detects passport section', () {
        final sections = ScanSectionDetector.detectSectionsFromFilename('passport_scan.pdf');
        expect(sections.length, 1);
        expect(sections.first.section, 'passport');
      });

      test('detects bank section', () {
        final sections = ScanSectionDetector.detectSectionsFromFilename('bank_statement.pdf');
        expect(sections.length, 1);
        expect(sections.first.section, 'bankAccount');
      });

      test('returns empty list for unrelated files', () {
        final sections = ScanSectionDetector.detectSectionsFromFilename('vacation.jpg');
        expect(sections, isEmpty);
      });
    });

    group('sectionDisplayName', () {
      test('returns mapped names for known sections', () {
        expect(ScanSectionDetector.sectionDisplayName('identity'), 'Personal Information');
        expect(ScanSectionDetector.sectionDisplayName('contact'), 'Contact');
        expect(ScanSectionDetector.sectionDisplayName('education'), 'Education');
        expect(ScanSectionDetector.sectionDisplayName('passport'), 'Passport');
        expect(ScanSectionDetector.sectionDisplayName('visa'), 'Visa');
        expect(ScanSectionDetector.sectionDisplayName('bankAccount'), 'Bank Account');
        expect(ScanSectionDetector.sectionDisplayName('card'), 'Card');
        expect(ScanSectionDetector.sectionDisplayName('employment'), 'Employment');
      });

      test('returns sectionId for unknown sections', () {
        expect(ScanSectionDetector.sectionDisplayName('unknown'), 'unknown');
        expect(ScanSectionDetector.sectionDisplayName('custom_section'), 'custom_section');
      });
    });

    group('extractIdentityFields', () {
      test('extracts phone number from text', () {
        final fields = ScanSectionDetector.extractIdentityFields(
          'Contact me at 13800138000 for details.',
        );
        final phoneField = fields.where((f) => f.key == 'phone').toList();
        expect(phoneField, isNotEmpty);
        expect(phoneField.first.value, '13800138000');
      });

      test('extracts email from text', () {
        final fields = ScanSectionDetector.extractIdentityFields(
          'Email: test@example.com',
        );
        final emailField = fields.where((f) => f.key == 'email').toList();
        expect(emailField, isNotEmpty);
        expect(emailField.first.value, 'test@example.com');
      });

      test('extracts Chinese ID card', () {
        final fields = ScanSectionDetector.extractIdentityFields(
          'ID: 110101199001011234',
        );
        final idField = fields.where((f) => f.key == 'idCard').toList();
        expect(idField, isNotEmpty);
        expect(idField.first.value, '110101199001011234');
      });

      test('returns empty list when no matches', () {
        final fields = ScanSectionDetector.extractIdentityFields('Hello world');
        expect(fields, isEmpty);
      });
    });

    group('extractPassportFields', () {
      test('extracts passport number', () {
        final fields = ScanSectionDetector.extractPassportFields(
          'Passport No: E12345678',
        );
        final passportField = fields.where((f) => f.key == 'number').toList();
        expect(passportField, isNotEmpty);
        expect(passportField.first.value, 'E12345678');
      });

      test('extracts holder name', () {
        final fields = ScanSectionDetector.extractPassportFields(
          'Name: Zhang Wei, Passport: E12345678',
        );
        final nameField = fields.where((f) => f.key == 'holderName').toList();
        expect(nameField, isNotEmpty);
        expect(nameField.first.value, 'Zhang Wei');
      });
    });

    group('extractContactFields', () {
      test('extracts phone and email', () {
        final fields = ScanSectionDetector.extractContactFields(
          'Phone: 13800138000, Email: contact@test.com',
        );
        expect(fields.where((f) => f.key == 'value' && f.value == '13800138000'), isNotEmpty);
        expect(fields.where((f) => f.key == 'value' && f.value == 'contact@test.com'), isNotEmpty);
      });
    });

    group('extractBankAccountFields', () {
      test('extracts bank name', () {
        final fields = ScanSectionDetector.extractBankAccountFields(
          'Bank: China Bank, Account: 6222021234567890123',
        );
        final bankField = fields.where((f) => f.key == 'bankName').toList();
        expect(bankField, isNotEmpty);
      });

      test('extracts account number', () {
        final fields = ScanSectionDetector.extractBankAccountFields(
          'Account: 6222021234567890123',
        );
        final accountField = fields.where((f) => f.key == 'accountNumber').toList();
        expect(accountField, isNotEmpty);
        expect(accountField.first.value, '6222021234567890123');
      });

      test('extracts SWIFT code', () {
        final fields = ScanSectionDetector.extractBankAccountFields(
          'SWIFT: ABCDCNBJ123',
        );
        final swiftField = fields.where((f) => f.key == 'swiftBic').toList();
        expect(swiftField, isNotEmpty);
      });
    });

    group('detectSections', () {
      test('detects identity section from text', () {
        final sections = ScanSectionDetector.detectSections(
          'Name: Zhang Wei, Phone: 13800138000',
        );
        expect(sections.map((s) => s.section), contains('identity'));
      });

      test('detects contact section from text', () {
        final sections = ScanSectionDetector.detectSections(
          'Email me at test@example.com',
        );
        expect(sections.map((s) => s.section), contains('contact'));
      });

      test('returns empty list for unrelated text', () {
        final sections = ScanSectionDetector.detectSections('The quick brown fox');
        expect(sections, isEmpty);
      });
    });
  });
}
