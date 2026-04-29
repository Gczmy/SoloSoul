import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart';

void main() {
  group('FieldRegistry consistency', () {
    test('defaultFields contains every field from all section lists', () {
      final allSectionFields = <String>{
        ...identityFields.map((f) => f.fieldId),
        ...contactFields.map((f) => f.fieldId),
        ...idCardFields.map((f) => f.fieldId),
        ...addressFields.map((f) => f.fieldId),
        ...bankAccountFields.map((f) => f.fieldId),
        ...cardFields.map((f) => f.fieldId),
        ...taxIdFields.map((f) => f.fieldId),
        ...passportFields.map((f) => f.fieldId),
        ...visaFields.map((f) => f.fieldId),
        ...travelFields.map((f) => f.fieldId),
        ...educationFields.map((f) => f.fieldId),
        ...employmentFields.map((f) => f.fieldId),
        ...skillFields.map((f) => f.fieldId),
        ...languageFields.map((f) => f.fieldId),
        ...awardFields.map((f) => f.fieldId),
      };

      final defaultFieldIds = FieldRegistry.defaultFields.map((f) => f.fieldId).toSet();

      expect(
        defaultFieldIds,
        equals(allSectionFields),
        reason: 'defaultFields must be derivable from section lists',
      );
    });

    test('defaultFields length equals sum of all section lists', () {
      final sectionLengths = [
        identityFields.length,
        contactFields.length,
        idCardFields.length,
        addressFields.length,
        bankAccountFields.length,
        cardFields.length,
        taxIdFields.length,
        passportFields.length,
        visaFields.length,
        travelFields.length,
        educationFields.length,
        employmentFields.length,
        skillFields.length,
        languageFields.length,
        awardFields.length,
      ].reduce((a, b) => a + b);

      expect(
        FieldRegistry.defaultFields.length,
        equals(sectionLengths),
        reason: 'No duplicate field IDs across section lists',
      );
    });

    test('no duplicate field IDs across section lists', () {
      final allFieldIds = <String>[
        ...identityFields.map((f) => f.fieldId),
        ...contactFields.map((f) => f.fieldId),
        ...idCardFields.map((f) => f.fieldId),
        ...addressFields.map((f) => f.fieldId),
        ...bankAccountFields.map((f) => f.fieldId),
        ...cardFields.map((f) => f.fieldId),
        ...taxIdFields.map((f) => f.fieldId),
        ...passportFields.map((f) => f.fieldId),
        ...visaFields.map((f) => f.fieldId),
        ...travelFields.map((f) => f.fieldId),
        ...educationFields.map((f) => f.fieldId),
        ...employmentFields.map((f) => f.fieldId),
        ...skillFields.map((f) => f.fieldId),
        ...languageFields.map((f) => f.fieldId),
        ...awardFields.map((f) => f.fieldId),
      ];

      final uniqueIds = allFieldIds.toSet();
      expect(
        allFieldIds.length,
        equals(uniqueIds.length),
        reason: 'Duplicate field IDs found: ${allFieldIds.where((id) => allFieldIds.where((x) => x == id).length > 1).toSet()}',
      );
    });
  });
}
