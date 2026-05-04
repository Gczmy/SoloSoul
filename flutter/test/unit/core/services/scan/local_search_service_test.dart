import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
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
    });

    group('filename hints personal', () {
      test('resume hints personal', () {
        expect(LocalSearchService.filenameHintsPersonal('my_resume.pdf'), isTrue);
      });

      test('passport hints personal', () {
        expect(LocalSearchService.filenameHintsPersonal('passport_scan.pdf'), isTrue);
      });

      test('random file does not hint personal', () {
        expect(LocalSearchService.filenameHintsPersonal('report_2024.pdf'), isFalse);
      });
    });
  });
}
