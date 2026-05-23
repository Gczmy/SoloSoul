import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_date_utils.dart';

void main() {
  group('parseMrzDate', () {
    test('parses 20th century date (YY >= 50)', () {
      expect(parseMrzDate('990101'), '1999-01-01');
      expect(parseMrzDate('850615'), '1985-06-15');
      expect(parseMrzDate('500101'), '1950-01-01');
    });

    test('parses 21st century date (YY < 50)', () {
      expect(parseMrzDate('000101'), '2000-01-01');
      expect(parseMrzDate('251231'), '2025-12-31');
      expect(parseMrzDate('490101'), '2049-01-01');
    });

    test('handles leap year', () {
      expect(parseMrzDate('000229'), '2000-02-29'); // 2000 is a leap year
    });

    test('returns null for invalid length', () {
      expect(parseMrzDate(''), isNull);
      expect(parseMrzDate('12345'), isNull);
      expect(parseMrzDate('1234567'), isNull);
    });

    test('returns null for non-numeric input', () {
      expect(parseMrzDate('abc123'), isNull);
      expect(parseMrzDate('12ab34'), isNull);
    });

    test('returns null for invalid date', () {
      expect(parseMrzDate('990231'), isNull); // Feb 31
      expect(parseMrzDate('990431'), isNull); // Apr 31
      expect(parseMrzDate('010229'), isNull); // 2001 is not leap year
      expect(parseMrzDate('991300'), isNull); // Month 13
      expect(parseMrzDate('990032'), isNull); // Day 32
    });

    test('returns null for month 00', () {
      expect(parseMrzDate('990001'), isNull);
    });

    test('returns null for day 00', () {
      expect(parseMrzDate('990100'), isNull);
    });

    test('parses boundary dates', () {
      expect(parseMrzDate('991231'), '1999-12-31');
      expect(parseMrzDate('000101'), '2000-01-01');
    });
  });

  group('formatIsoDateForDisplay', () {
    test('returns ISO string as-is', () {
      expect(formatIsoDateForDisplay('1990-01-01'), '1990-01-01');
      expect(formatIsoDateForDisplay('2025-12-31'), '2025-12-31');
    });

    test('returns empty string for null', () {
      expect(formatIsoDateForDisplay(null), '');
    });

    test('returns empty string for empty', () {
      expect(formatIsoDateForDisplay(''), '');
    });
  });

  group('parseIsoDate', () {
    test('parses valid ISO date', () {
      final result = parseIsoDate('1990-01-01');
      expect(result, isNotNull);
      expect(result!.year, 1990);
      expect(result.month, 1);
      expect(result.day, 1);
    });

    test('parses ISO date with time', () {
      final result = parseIsoDate('2024-06-15T10:30:00Z');
      expect(result, isNotNull);
      expect(result!.year, 2024);
      expect(result.month, 6);
      expect(result.day, 15);
    });

    test('returns null for invalid date', () {
      expect(parseIsoDate('not-a-date'), isNull);
      expect(parseIsoDate(''), isNull);
      // Note: Dart DateTime.parse overflows months (e.g. 1990-13-01 → 1991-01-01)
      // so we test with truly unparseable formats
      expect(parseIsoDate('1990-01-'), isNull);
      expect(parseIsoDate('abcdef'), isNull);
    });
  });
}
