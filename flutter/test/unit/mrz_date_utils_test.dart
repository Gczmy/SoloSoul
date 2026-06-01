import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/utils/mrz_date_utils.dart';

void main() {
  group('parseMrzDate', () {
    test('parses 20th century date (>=50)', () {
      expect(parseMrzDate('900101'), '1990-01-01');
      expect(parseMrzDate('991231'), '1999-12-31');
    });

    test('parses 21st century date (<50)', () {
      expect(parseMrzDate('000101'), '2000-01-01');
      expect(parseMrzDate('010101'), '2001-01-01');
      expect(parseMrzDate('251231'), '2025-12-31');
    });

    test('returns null for wrong length', () {
      expect(parseMrzDate('90010'), isNull);
      expect(parseMrzDate('9001010'), isNull);
    });

    test('returns null for non-numeric', () {
      expect(parseMrzDate('90AB01'), isNull);
    });

    test('returns null for invalid date', () {
      expect(parseMrzDate('900231'), isNull); // Feb 31
      expect(parseMrzDate('901332'), isNull); // Month 13
    });

    test('handles leap year', () {
      expect(parseMrzDate('960229'), '1996-02-29');
      expect(parseMrzDate('970229'), isNull); // 1997 is not leap year
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
    test('parses valid ISO date', () {
      final dt = parseIsoDate('1990-01-01');
      expect(dt, isNotNull);
      expect(dt!.year, 1990);
      expect(dt.month, 1);
      expect(dt.day, 1);
    });

    test('returns null for invalid date', () {
      expect(parseIsoDate('not-a-date'), isNull);
    });
  });
}
