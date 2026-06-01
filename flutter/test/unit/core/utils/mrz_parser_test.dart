import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';

void main() {
  group('MrzParser', () {
    group('parse', () {
      test('returns null for empty input', () {
        expect(MrzParser.parse([]), isNull);
      });

      test('returns null for invalid line count', () {
        expect(MrzParser.parse(['one']), isNull);
        expect(MrzParser.parse(['one', 'two', 'three', 'four']), isNull);
      });

      test('returns null for wrong line lengths', () {
        expect(MrzParser.parse(['A' * 44, 'B' * 40]), isNull);
        expect(MrzParser.parse(['A' * 30, 'B' * 30, 'C' * 25]), isNull);
      });
    });

    group('TD3 passport', () {
      final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
      final line2 = 'E123456788CHN8601018M2801017<<<<<<<<<<<<<<<0';

      test('parses TD3 format', () {
        final result = MrzParser.parse([line1, line2]);
        expect(result, isNotNull);
        expect(result!.documentType, 'P<');
        expect(result.country, 'CHN');
        expect(result.surname, 'ZHANG');
        expect(result.givenNames, 'WEI');
        expect(result.documentNumber, 'E12345678');
        expect(result.nationality, 'CHN');
        expect(result.dateOfBirth, '860101');
        expect(result.sex, 'M');
        expect(result.expiryDate, '280101');
      });

      test('has rawLines', () {
        final result = MrzParser.parse([line1, line2])!;
        expect(result.rawLines, hasLength(2));
      });

      test('normalizes lowercase input', () {
        final result = MrzParser.parse([line1.toLowerCase(), line2.toLowerCase()]);
        expect(result, isNotNull);
        expect(result!.country, 'CHN');
      });

      test('handles names with multiple given names', () {
        final l1 = 'P<CHNLI<<WEI<MING<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final l2 = 'E123456788CHN8601018M2801017<<<<<<<<<<<<<<<0';
        final result = MrzParser.parse([l1, l2])!;
        expect(result.surname, 'LI');
        expect(result.givenNames, 'WEI MING');
      });

      test('handles single name without given names', () {
        final l1 = 'P<CHNWANG<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final l2 = 'E123456788CHN8601018M2801017<<<<<<<<<<<<<<<0';
        final result = MrzParser.parse([l1, l2])!;
        expect(result.surname, 'WANG');
        expect(result.givenNames, '');
      });
    });

    group('TD1 ID card', () {
      final line1 = 'ICCHNLI<<WEI<<<<<<<<<<<<<<<<<<';
      final line2 = 'E1234567888601018M2801017<<<<<';
      final line3 = '<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';

      test('parses TD1 format', () {
        final result = MrzParser.parse([line1, line2, line3]);
        expect(result, isNotNull);
        expect(result!.documentType, 'IC');
        expect(result.country, 'CHN');
        expect(result.surname, 'LI');
        expect(result.givenNames, 'WEI');
        expect(result.documentNumber, 'E12345678');
        expect(result.dateOfBirth, '860101');
        expect(result.sex, 'M');
        expect(result.expiryDate, '280101');
      });

      test('TD1 nationality defaults to country', () {
        final result = MrzParser.parse([line1, line2, line3])!;
        expect(result.nationality, 'CHN');
      });

      test('has 3 rawLines', () {
        final result = MrzParser.parse([line1, line2, line3])!;
        expect(result.rawLines, hasLength(3));
      });
    });

    group('TD2', () {
      final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<';
      final line2 = 'E123456788CHN8601018M2801017<<<<<<<<';

      test('parses TD2 format', () {
        final result = MrzParser.parse([line1, line2]);
        expect(result, isNotNull);
        expect(result!.documentType, 'P<');
        expect(result.country, 'CHN');
        expect(result.surname, 'ZHANG');
        expect(result.givenNames, 'WEI');
      });
    });

    group('check digit validation', () {
      test('reduces confidence when check digit is wrong', () {
        final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final line2 = 'E123456780CHN8601018M2801017<<<<<<<<<<<<<<<0';
        final result = MrzParser.parse([line1, line2])!;
        expect(result.confidence, lessThan(1.0));
      });

      test('full confidence when all check digits are <', () {
        final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final line2 = 'E12345678<CHN860101<M280101<<<<<<<<<<<<<<<<0';
        final result = MrzParser.parse([line1, line2])!;
        expect(result.confidence, 1.0);
      });
    });

    group('edge cases', () {
      test('filters out empty lines', () {
        final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final line2 = 'E123456788CHN8601018M2801017<<<<<<<<<<<<<<<0';
        final result = MrzParser.parse([line1, '', line2, '']);
        expect(result, isNotNull);
      });

      test('returns null when line lengths mismatch', () {
        final line1 = 'P<CHNZHANG<<WEI<<<<<<<<<<<<<<<<<<<<<<<<<<<<<';
        final line2 = 'E12345678<8CHN8601018M2801017<<<<<<<<<<<<<<'; // 43 chars
        expect(MrzParser.parse([line1, line2]), isNull);
      });
    });
  });
}
