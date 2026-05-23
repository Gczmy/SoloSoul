import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/services/scan/local_search_service.dart';

void main() {
  group('LocalSearchService', () {
    group('mapSectionToTypeId', () {
      test('maps identity to profile_identity', () {
        expect(LocalSearchService.mapSectionToTypeId('identity'), 'profile_identity');
      });

      test('maps passport to travel_passport', () {
        expect(LocalSearchService.mapSectionToTypeId('passport'), 'travel_passport');
      });

      test('maps bankAccount to financial_bank_account', () {
        expect(LocalSearchService.mapSectionToTypeId('bankAccount'), 'financial_bank_account');
      });

      test('returns null for unknown section', () {
        expect(LocalSearchService.mapSectionToTypeId('unknown'), isNull);
      });
    });

    group('mapFieldToPropertyId', () {
      test('maps fullName in identity section', () {
        expect(
          LocalSearchService.mapFieldToPropertyId('identity', 'fullName'),
          'fullName',
        );
      });

      test('maps passport number', () {
        expect(
          LocalSearchService.mapFieldToPropertyId('passport', 'number'),
          'number',
        );
      });

      test('maps institution in education', () {
        expect(
          LocalSearchService.mapFieldToPropertyId('education', 'institution'),
          'institution',
        );
      });
    });

    group('getDefaultSensitivity', () {
      test('delegates to FieldRegistry as single source of truth', () {
        // Verify that all fields in FieldRegistry are correctly resolved
        for (final field in FieldRegistry.defaultFields) {
          final parts = field.fieldId.split('.');
          final sectionId = parts[0];
          final propertyId = parts[1];
          expect(
            LocalSearchService.getDefaultSensitivity(sectionId, propertyId),
            field.level,
            reason: 'Field ${field.fieldId} should match FieldRegistry level',
          );
        }
      });

      test('falls back to public for unknown fields', () {
        expect(
          LocalSearchService.getDefaultSensitivity('unknown', 'field'),
          SensitivityLevel.public,
        );
      });

      test('idCard.number is critical', () {
        expect(
          LocalSearchService.getDefaultSensitivity('idCard', 'number'),
          SensitivityLevel.critical,
        );
      });

      test('passport.number is critical', () {
        expect(
          LocalSearchService.getDefaultSensitivity('passport', 'number'),
          SensitivityLevel.critical,
        );
      });

      test('identity.fullName is public', () {
        expect(
          LocalSearchService.getDefaultSensitivity('identity', 'fullName'),
          SensitivityLevel.public,
        );
      });

      test('bankAccount.accountNumber is critical', () {
        expect(
          LocalSearchService.getDefaultSensitivity('bankAccount', 'accountNumber'),
          SensitivityLevel.critical,
        );
      });

      test('contact.value is internal (previously mismatched)', () {
        expect(
          LocalSearchService.getDefaultSensitivity('contact', 'value'),
          SensitivityLevel.internal,
        );
      });

      test('education.gpa is internal (previously uncovered)', () {
        expect(
          LocalSearchService.getDefaultSensitivity('education', 'gpa'),
          SensitivityLevel.internal,
        );
      });
    });

    group('filename hints personal', () {
      test('resume hints personal', () {
        expect(ScanSectionDetector.filenameHintsPersonal('my_resume.pdf'), isTrue);
      });

      test('passport hints personal', () {
        expect(ScanSectionDetector.filenameHintsPersonal('passport_scan.pdf'), isTrue);
      });

      test('random file does not hint personal', () {
        expect(ScanSectionDetector.filenameHintsPersonal('report_2024.pdf'), isFalse);
      });
    });
  });
}
