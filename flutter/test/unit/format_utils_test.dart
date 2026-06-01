import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/format_utils.dart';

void main() {
  group('formatBytes', () {
    test('formats bytes', () {
      expect(formatBytes(512), '512 B');
    });

    test('formats kilobytes', () {
      expect(formatBytes(1024), '1.0 KB');
      expect(formatBytes(1536), '1.5 KB');
    });

    test('formats megabytes', () {
      expect(formatBytes(1024 * 1024), '1.0 MB');
      expect(formatBytes(2 * 1024 * 1024), '2.0 MB');
    });

    test('formats zero bytes', () {
      expect(formatBytes(0), '0 B');
    });
  });
}
