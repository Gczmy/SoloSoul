import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';

void main() {
  // Valid TD3 MRZ (2 lines × 44 chars)
  final td3Line1 = 'P<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
  final td3Line2 = 'AB1234567<CHN900101<M250101<9<<<<<<<<<<<<<<0';

  // Valid TD1 MRZ (3 lines × 30 chars)
  final td1Line1 = 'I<CHNLI<<WEI<<<<<<<<<<<<<<<<<<';
  final td1Line2 = 'AB123456<39001013M2501019<<<<<';
  final td1Line3 = '<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';

  // Valid TD2 MRZ (2 lines × 36 chars)
  final td2Line1 = 'P<CHNLI<<WEI<<<<<<<<<<<<<<<<<<<<<<<<';
  final td2Line2 = 'AB1234567<CHN900101<M250101<<<<<<<<<';

  group('MrzParser.parse', () {
    test('parses valid TD3 passport MRZ', () {
      final result = MrzParser.parse([td3Line1, td3Line2]);
      expect(result, isNotNull);
      expect(result!.documentType, 'P<');
      expect(result.country, 'CHN');
      expect(result.surname, 'LI');
      expect(result.givenNames, 'WEI');
      expect(result.documentNumber, 'AB1234567');
      expect(result.nationality, 'CHN');
      expect(result.dateOfBirth, '900101');
      expect(result.sex, 'M');
      expect(result.expiryDate, '250101');
      expect(result.confidence, 1.0);
      expect(result.rawLines.length, 2);
    });

    test('parses valid TD1 ID card MRZ', () {
      final result = MrzParser.parse([td1Line1, td1Line2, td1Line3]);
      expect(result, isNotNull);
      expect(result!.documentType, 'I<');
      expect(result.country, 'CHN');
      expect(result.surname, 'LI');
      expect(result.givenNames, 'WEI');
      expect(result.documentNumber, 'AB123456');
      expect(result.nationality, 'CHN');
      expect(result.dateOfBirth, '900101');
      expect(result.sex, 'M');
      expect(result.expiryDate, '250101');
      expect(result.rawLines.length, 3);
    });

    test('parses valid TD2 MRZ', () {
      final result = MrzParser.parse([td2Line1, td2Line2]);
      expect(result, isNotNull);
      expect(result!.documentType, 'P<');
      expect(result.country, 'CHN');
      expect(result.surname, 'LI');
      expect(result.givenNames, 'WEI');
      expect(result.documentNumber, 'AB1234567');
      expect(result.nationality, 'CHN');
      expect(result.dateOfBirth, '900101');
      expect(result.sex, 'M');
      expect(result.expiryDate, '250101');
    });

    test('normalizes input (trims whitespace, uppercases)', () {
      final result = MrzParser.parse([
        '  ${td3Line1.toLowerCase()}  ',
        '  ${td3Line2.toLowerCase()}  ',
      ]);
      expect(result, isNotNull);
      expect(result!.documentType, 'P<');
    });

    test('filters out empty lines', () {
      final result = MrzParser.parse(['', td3Line1, '', td3Line2, '']);
      expect(result, isNotNull);
    });

    test('returns null for invalid line count', () {
      expect(MrzParser.parse([td3Line1]), isNull);
      expect(MrzParser.parse([td3Line1, td3Line2, td1Line3]), isNull);
    });

    test('returns null for wrong line lengths', () {
      expect(MrzParser.parse(['SHORT', 'SHORT']), isNull);
      expect(MrzParser.parse([td3Line1 + 'X', td3Line2 + 'X']), isNull);
    });

    test('handles single-word names', () {
      final line1 = 'P<CHNLI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
      final line2 = 'AB1234567<CHN900101<M250101<9<<<<<<<<<<<<<<0';
      final result = MrzParser.parse([line1, line2]);
      expect(result, isNotNull);
      expect(result!.surname, 'LI');
      expect(result.givenNames, '');
    });

    test('handles multi-part given names', () {
      final line1 = 'P<CHNLI<<WEI<<MING<<<<<<<<<<<<<<<<<<<<<<<<<<';
      final line2 = 'AB1234567<CHN900101<M250101<9<<<<<<<<<<<<<<0';
      final result = MrzParser.parse([line1, line2]);
      expect(result, isNotNull);
      expect(result!.surname, 'LI');
      expect(result.givenNames, 'WEI MING');
    });

    test('reduces confidence on invalid check digit', () {
      final line2Bad = 'AB1234567XCHN900101XM250101X9<<<<<<<<<<<<<<0';
      final result = MrzParser.parse([td3Line1, line2Bad]);
      expect(result, isNotNull);
      expect(result!.confidence, lessThan(1.0));
    });
  });
}
