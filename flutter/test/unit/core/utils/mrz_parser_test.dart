import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';

void main() {
  group('MrzParser.parse', () {
    test('parses valid TD3 (passport)', () {
      final lines = [
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.documentType, 'P<');
      expect(result.country, 'GBR');
      expect(result.surname, 'SMITH');
      expect(result.givenNames, 'SARAH');
      expect(result.documentNumber, '012345678');
      expect(result.nationality, 'GBR');
      expect(result.dateOfBirth, '810101');
      expect(result.sex, 'F');
      expect(result.expiryDate, '250714');
      expect(result.confidence, closeTo(1.0, 0.001));
      expect(result.rawLines.length, 2);
    });

    test('parses valid TD1 (ID card)', () {
      final lines = [
        'I<GBRSMITH<<SARAH<<<<<<<<<<<<<',
        '0123456784GBR8101017F2507145<<',
        '<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.documentType, 'I<');
      expect(result.country, 'GBR');
      expect(result.surname, 'SMITH');
      expect(result.givenNames, 'SARAH');
      expect(result.nationality, 'GBR'); // TD1 uses country as nationality
    });

    test('parses valid TD2', () {
      final lines = [
        'I<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR8101017F2507145<<<<<<<<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.documentType, 'I<');
      expect(result.surname, 'SMITH');
      expect(result.givenNames, 'SARAH');
      expect(result.documentNumber, '012345678');
      expect(result.nationality, 'GBR');
    });

    test('returns null for invalid line count', () {
      expect(MrzParser.parse(['only_one_line']), isNull);
      expect(MrzParser.parse([]), isNull);
    });

    test('returns null for wrong lengths', () {
      // 2 lines but wrong length (not 44 or 36)
      expect(MrzParser.parse(['SHORT', 'SHORT']), isNull);
    });

    test('normalizes lowercase input', () {
      final lines = [
        'p<gbrsmith<<sarah<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784gbr8101017f2507145<<<<<<<<<<<<<<6<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.country, 'GBR');
    });

    test('trims whitespace', () {
      final lines = [
        '  P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<  ',
        '  0123456784GBR8101017F2507145<<<<<<<<<<<<<<6<  ',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.documentType, 'P<');
    });

    test('reduces confidence on invalid check digit', () {
      // Invalid check digit for document number
      final lines = [
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456789GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.confidence, lessThan(1.0));
    });
  });

  group('MrzParser._parseNames', () {
    test('parses simple name', () {
      final result = MrzParser.parse([
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ]);
      expect(result!.surname, 'SMITH');
      expect(result.givenNames, 'SARAH');
    });

    test('parses name with multiple given names', () {
      final result = MrzParser.parse([
        'P<GBRDOE<<JOHN<MICHAEL<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR9001017M3007145<<<<<<<<<<<<<<6<',
      ]);
      expect(result!.surname, 'DOE');
      expect(result.givenNames, 'JOHN MICHAEL');
    });

    test('parses name with only surname', () {
      final result = MrzParser.parse([
        'P<GBRONLY<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR9001017M3007145<<<<<<<<<<<<<<6<',
      ]);
      expect(result!.surname, 'ONLY');
      expect(result.givenNames, '');
    });

    test('handles single < in name field', () {
      // When only single < exists (no <<), entire name field becomes surname
      final result = MrzParser.parse([
        'P<GBRSMITH<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR9001017F3007145<<<<<<<<<<<<<<6<',
      ]);
      expect(result!.surname, 'SMITH SARAH');
      expect(result.givenNames, '');
    });
  });

  group('MrzParser._validateCheckDigit', () {
    test('validates correct check digit', () {
      // "012345678" with weights 7,3,1:
      // 0*7 + 1*3 + 2*1 + 3*7 + 4*3 + 5*1 + 6*7 + 7*3 + 8*1 = 4 mod 10
      expect(MrzParser.parse([
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456784GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ])!.confidence, closeTo(1.0, 0.001));
    });

    test('accepts < as check digit (no validation)', () {
      final lines = [
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '<<<<<<<<<<GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.confidence, closeTo(1.0, 0.001));
    });

    test('detects invalid check digit', () {
      final lines = [
        'P<GBRSMITH<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<<',
        '0123456780GBR8101017F2507145<<<<<<<<<<<<<<6<',
      ];
      final result = MrzParser.parse(lines);
      expect(result, isNotNull);
      expect(result!.confidence, lessThan(1.0));
    });
  });
}
