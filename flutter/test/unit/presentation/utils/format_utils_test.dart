import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/format_utils.dart';

void main() {
  group('formatBytes', () {
    test('formats bytes under 1024', () {
      expect(formatBytes(0), '0 B');
      expect(formatBytes(1), '1 B');
      expect(formatBytes(512), '512 B');
      expect(formatBytes(1023), '1023 B');
    });

    test('formats kilobytes', () {
      expect(formatBytes(1024), '1.0 KB');
      expect(formatBytes(1536), '1.5 KB');
      expect(formatBytes(10240), '10.0 KB');
      expect(formatBytes(1048575), '1024.0 KB');
    });

    test('formats megabytes', () {
      expect(formatBytes(1048576), '1.0 MB');
      expect(formatBytes(1572864), '1.5 MB');
      expect(formatBytes(10485760), '10.0 MB');
    });
  });
}
