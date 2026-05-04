import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/format_relative_time.dart';

void main() {
  group('formatRelativeTime', () {
    test('returns "Just now" for current time', () {
      final now = DateTime.now();
      expect(formatRelativeTime(now), 'Just now');
    });

    test('returns minutes ago', () {
      final fiveMinAgo = DateTime.now().subtract(const Duration(minutes: 5));
      expect(formatRelativeTime(fiveMinAgo), '5 minute(s) ago');
    });

    test('returns hours ago', () {
      final threeHoursAgo = DateTime.now().subtract(const Duration(hours: 3));
      expect(formatRelativeTime(threeHoursAgo), '3 hour(s) ago');
    });

    test('returns days ago', () {
      final fiveDaysAgo = DateTime.now().subtract(const Duration(days: 5));
      expect(formatRelativeTime(fiveDaysAgo), '5 day(s) ago');
    });

    test('returns months ago', () {
      final twoMonthsAgo = DateTime.now().subtract(const Duration(days: 65));
      expect(formatRelativeTime(twoMonthsAgo), '2 month(s) ago');
    });

    test('returns years ago', () {
      final twoYearsAgo = DateTime.now().subtract(const Duration(days: 730));
      expect(formatRelativeTime(twoYearsAgo), '2 year(s) ago');
    });
  });

  group('formatRelativeTimeShort', () {
    test('returns "Just now" for current time', () {
      final now = DateTime.now();
      expect(formatRelativeTimeShort(now), 'Just now');
    });

    test('returns minutes shorthand', () {
      final fiveMinAgo = DateTime.now().subtract(const Duration(minutes: 5));
      expect(formatRelativeTimeShort(fiveMinAgo), '5m ago');
    });

    test('returns hours shorthand', () {
      final threeHoursAgo = DateTime.now().subtract(const Duration(hours: 3));
      expect(formatRelativeTimeShort(threeHoursAgo), '3h ago');
    });

    test('returns days shorthand', () {
      final fiveDaysAgo = DateTime.now().subtract(const Duration(days: 5));
      expect(formatRelativeTimeShort(fiveDaysAgo), '5d ago');
    });

    test('returns months shorthand', () {
      final twoMonthsAgo = DateTime.now().subtract(const Duration(days: 65));
      expect(formatRelativeTimeShort(twoMonthsAgo), '2mo ago');
    });

    test('returns years shorthand', () {
      final twoYearsAgo = DateTime.now().subtract(const Duration(days: 730));
      expect(formatRelativeTimeShort(twoYearsAgo), '2y ago');
    });
  });
}
