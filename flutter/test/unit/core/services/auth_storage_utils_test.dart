import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';

void main() {
  group('SecureAccountStorage.secureWipe', () {
    test('fills buffer with zeros', () {
      final buffer = Uint8List.fromList([1, 2, 3, 4, 5]);
      SecureAccountStorage.secureWipe(buffer);
      expect(buffer, [0, 0, 0, 0, 0]);
    });

    test('works on empty buffer', () {
      final buffer = Uint8List(0);
      SecureAccountStorage.secureWipe(buffer);
      expect(buffer, isEmpty);
    });

    test('works on single byte', () {
      final buffer = Uint8List.fromList([255]);
      SecureAccountStorage.secureWipe(buffer);
      expect(buffer, [0]);
    });

    test('works on large buffer', () {
      final buffer = Uint8List(1024);
      for (var i = 0; i < buffer.length; i++) {
        buffer[i] = i % 256;
      }
      SecureAccountStorage.secureWipe(buffer);
      expect(buffer.every((b) => b == 0), isTrue);
    });

    test('modifies buffer in place', () {
      final buffer = Uint8List.fromList([10, 20, 30]);
      final original = buffer;
      SecureAccountStorage.secureWipe(buffer);
      expect(identical(original, buffer), isTrue);
      expect(buffer, [0, 0, 0]);
    });
  });
}
