import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_date_utils.dart';

void main() {
  group('parseMrzDate', () {
    test('parses 20th century date', () {
      expect(parseMrzDate('900101'), '1990-01-01');
    });

    test('parses 21st century date', () {
      expect(parseMrzDate('001231'), '2000-12-31');
    });

    test('parses boundary year 50 as 1950', () {
      expect(parseMrzDate('500101'), '1950-01-01');
    });

    test('parses year 49 as 2049', () {
      expect(parseMrzDate('490101'), '2049-01-01');
    });

    test('handles leap year', () {
      expect(parseMrzDate('200229'), '2020-02-29');
    });

    test('returns null for invalid leap year', () {
      expect(parseMrzDate('210229'), isNull);
    });

    test('returns null for invalid month', () {
      expect(parseMrzDate('991300'), isNull);
    });

    test('returns null for invalid day', () {
      expect(parseMrzDate('990432'), isNull);
    });

    test('returns null for wrong length', () {
      expect(parseMrzDate('90010'), isNull);
      expect(parseMrzDate('9001011'), isNull);
    });

    test('returns null for non-numeric input', () {
      expect(parseMrzDate('abcdef'), isNull);
      expect(parseMrzDate('90a101'), isNull);
    });

    test('returns null for empty string', () {
      expect(parseMrzDate(''), isNull);
    });
  });

  group('formatIsoDateForDisplay', () {
    test('returns iso date as-is', () {
      expect(formatIsoDateForDisplay('1990-01-01'), '1990-01-01');
    });

    test('returns empty string for null', () {
      expect(formatIsoDateForDisplay(null), '');
    });

    test('returns empty string for empty', () {
      expect(formatIsoDateForDisplay(''), '');
    });
  });

  group('parseIsoDate', () {
    test('parses valid iso date', () {
      final result = parseIsoDate('1990-01-01');
      expect(result, isNotNull);
      expect(result!.year, 1990);
      expect(result.month, 1);
      expect(result.day, 1);
    });

    test('returns null for invalid date', () {
      expect(parseIsoDate('not-a-date'), isNull);
    });
  });
}
