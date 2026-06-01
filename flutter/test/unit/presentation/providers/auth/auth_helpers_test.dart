import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_helpers.dart';

void main() {
  group('bytesToHex', () {
    test('converts empty list to empty string', () {
      expect(bytesToHex([]), '');
    });

    test('converts single byte', () {
      expect(bytesToHex([0]), '00');
      expect(bytesToHex([255]), 'ff');
    });

    test('pads single digit hex with zero', () {
      expect(bytesToHex([1]), '01');
      expect(bytesToHex([15]), '0f');
    });

    test('converts multiple bytes', () {
      expect(bytesToHex([0xDE, 0xAD, 0xBE, 0xEF]), 'deadbeef');
    });
  });

  group('hexToBytes', () {
    test('converts empty string to empty list', () {
      expect(hexToBytes(''), Uint8List(0));
    });

    test('converts single byte', () {
      expect(hexToBytes('00'), Uint8List.fromList([0]));
      expect(hexToBytes('ff'), Uint8List.fromList([255]));
    });

    test('converts multiple bytes', () {
      expect(hexToBytes('deadbeef'), Uint8List.fromList([0xDE, 0xAD, 0xBE, 0xEF]));
    });

    test('handles uppercase hex', () {
      expect(hexToBytes('DEADBEEF'), Uint8List.fromList([0xDE, 0xAD, 0xBE, 0xEF]));
    });
  });

  group('bytesToHex and hexToBytes round-trip', () {
    test('round-trips random bytes', () {
      final bytes = Uint8List.fromList([0, 1, 127, 128, 255, 0xAB, 0xCD]);
      final hex = bytesToHex(bytes);
      final restored = hexToBytes(hex);
      expect(restored, bytes);
    });
  });

  group('constantTimeEqualsSync', () {
    test('returns true for identical strings', () {
      expect(constantTimeEqualsSync('hello', 'hello'), isTrue);
    });

    test('returns false for different strings', () {
      expect(constantTimeEqualsSync('hello', 'world'), isFalse);
    });

    test('returns false for different lengths', () {
      expect(constantTimeEqualsSync('hello', 'hell'), isFalse);
      expect(constantTimeEqualsSync('hell', 'hello'), isFalse);
    });

    test('returns true for empty strings', () {
      expect(constantTimeEqualsSync('', ''), isTrue);
    });

    test('returns false for one empty string', () {
      expect(constantTimeEqualsSync('', 'x'), isFalse);
      expect(constantTimeEqualsSync('x', ''), isFalse);
    });

    test('returns false for single char difference', () {
      expect(constantTimeEqualsSync('abc', 'abC'), isFalse);
    });
  });
}
