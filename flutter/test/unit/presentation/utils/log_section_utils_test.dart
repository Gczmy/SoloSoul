import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';

void main() {
  group('logSectionForTypeId', () {
    test('maps profile types correctly', () {
      expect(
        logSectionForTypeId('__preset_identity'),
        LogSection.identity,
      );
      expect(
        logSectionForTypeId('__preset_contact'),
        LogSection.contactInformation,
      );
      expect(
        logSectionForTypeId('__preset_identity_document'),
        LogSection.idCard,
      );
      expect(
        logSectionForTypeId('__preset_address'),
        LogSection.address,
      );
    });

    test('maps travel types correctly', () {
      expect(
        logSectionForTypeId('__preset_passport'),
        LogSection.passport,
      );
      expect(
        logSectionForTypeId('__preset_visa'),
        LogSection.visa,
      );
      expect(
        logSectionForTypeId('__preset_travel_history'),
        LogSection.travelHistory,
      );
    });

    test('maps financial types correctly', () {
      expect(
        logSectionForTypeId('__preset_bank_account'),
        LogSection.bankAccount,
      );
      expect(
        logSectionForTypeId('__preset_payment_card'),
        LogSection.card,
      );
      expect(
        logSectionForTypeId('__preset_tax_id'),
        LogSection.financial,
      );
    });

    test('maps professional types correctly', () {
      expect(
        logSectionForTypeId('__preset_education'),
        LogSection.education,
      );
      expect(
        logSectionForTypeId('__preset_employment'),
        LogSection.employment,
      );
      expect(
        logSectionForTypeId('__preset_skill'),
        LogSection.skill,
      );
      expect(
        logSectionForTypeId('__preset_language'),
        LogSection.language,
      );
      expect(
        logSectionForTypeId('__preset_award'),
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
