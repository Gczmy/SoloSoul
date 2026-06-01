import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';

void main() {
  group('logSectionForTypeId', () {
    test('maps identity preset', () {
      expect(logSectionForTypeId('__preset_identity'), LogSection.identity);
    });

    test('maps contact preset', () {
      expect(logSectionForTypeId('__preset_contact'), LogSection.contactInformation);
    });

    test('maps passport preset', () {
      expect(logSectionForTypeId('__preset_passport'), LogSection.passport);
    });

    test('maps visa preset', () {
      expect(logSectionForTypeId('__preset_visa'), LogSection.visa);
    });

    test('maps bank_account preset', () {
      expect(logSectionForTypeId('__preset_bank_account'), LogSection.bankAccount);
    });

    test('maps payment_card preset', () {
      expect(logSectionForTypeId('__preset_payment_card'), LogSection.card);
    });

    test('maps education preset', () {
      expect(logSectionForTypeId('__preset_education'), LogSection.education);
    });

    test('maps employment preset', () {
      expect(logSectionForTypeId('__preset_employment'), LogSection.employment);
    });

    test('returns null for unknown typeId', () {
      expect(logSectionForTypeId('custom_type'), isNull);
      expect(logSectionForTypeId('collection'), isNull);
    });
  });
}
