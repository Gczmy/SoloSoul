import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/format_relative_time.dart';

void main() {
  group('formatRelativeTime', () {
    test('returns years ago', () {
      final ts = DateTime.now().subtract(const Duration(days: 800));
      expect(formatRelativeTime(ts), contains('year'));
      expect(formatRelativeTime(ts), contains('ago'))
;
    });

    test('returns months ago', () {
      final ts = DateTime.now().subtract(const Duration(days: 100));
      expect(formatRelativeTime(ts), contains('month'));
    });

    test('returns days ago', () {
      final ts = DateTime.now().subtract(const Duration(days: 5));
      expect(formatRelativeTime(ts), '5 day(s) ago');
    });

    test('returns hours ago', () {
      final ts = DateTime.now().subtract(const Duration(hours: 3));
      expect(formatRelativeTime(ts), '3 hour(s) ago');
    });

    test('returns minutes ago', () {
      final ts = DateTime.now().subtract(const Duration(minutes: 15));
      expect(formatRelativeTime(ts), '15 minute(s) ago');
    });

    test('returns Just now', () {
      final ts = DateTime.now();
      expect(formatRelativeTime(ts), 'Just now');
    });
  });

  group('formatRelativeTimeShort', () {
    test('returns years short', () {
      final ts = DateTime.now().subtract(const Duration(days: 800));
      expect(formatRelativeTimeShort(ts), contains('y ago'));
    });

    test('returns months short', () {
      final ts = DateTime.now().subtract(const Duration(days: 100));
      expect(formatRelativeTimeShort(ts), contains('mo ago'));
    });

    test('returns days short', () {
      final ts = DateTime.now().subtract(const Duration(days: 5));
      expect(formatRelativeTimeShort(ts), '5d ago');
    });

    test('returns hours short', () {
      final ts = DateTime.now().subtract(const Duration(hours: 3));
      expect(formatRelativeTimeShort(ts), '3h ago');
    });

    test('returns minutes short', () {
      final ts = DateTime.now().subtract(const Duration(minutes: 15));
      expect(formatRelativeTimeShort(ts), '15m ago');
    });

    test('returns Just now short', () {
      final ts = DateTime.now();
      expect(formatRelativeTimeShort(ts), 'Just now');
    });
  });
}
