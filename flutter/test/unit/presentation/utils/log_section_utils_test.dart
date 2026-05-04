import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';

void main() {
  group('logSectionForTypeId', () {
    test('maps profile types correctly', () {
      expect(
        logSectionForTypeId('profile_identity'),
        LogSection.identity,
      );
      expect(
        logSectionForTypeId('profile_contact'),
        LogSection.contactInformation,
      );
      expect(
        logSectionForTypeId('profile_id_card'),
        LogSection.idCard,
      );
      expect(
        logSectionForTypeId('profile_address'),
        LogSection.address,
      );
    });

    test('maps travel types correctly', () {
      expect(
        logSectionForTypeId('travel_passport'),
        LogSection.passport,
      );
      expect(
        logSectionForTypeId('travel_visa'),
        LogSection.visa,
      );
      expect(
        logSectionForTypeId('travel_history'),
        LogSection.travelHistory,
      );
    });

    test('maps financial types correctly', () {
      expect(
        logSectionForTypeId('financial_bank_account'),
        LogSection.bankAccount,
      );
      expect(
        logSectionForTypeId('financial_card'),
        LogSection.card,
      );
      expect(
        logSectionForTypeId('financial_tax_id'),
        LogSection.financial,
      );
    });

    test('maps professional types correctly', () {
      expect(
        logSectionForTypeId('professional_education'),
        LogSection.education,
      );
      expect(
        logSectionForTypeId('professional_employment'),
        LogSection.employment,
      );
      expect(
        logSectionForTypeId('professional_skill'),
        LogSection.skill,
      );
      expect(
        logSectionForTypeId('professional_language'),
        LogSection.language,
      );
      expect(
        logSectionForTypeId('professional_award'),
        LogSection.professional,
      );
    });

    test('returns null for unknown types', () {
      expect(logSectionForTypeId('custom_type'), isNull);
      expect(logSectionForTypeId('unknown'), isNull);
      expect(logSectionForTypeId(''), isNull);
    });
  });
}
