import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_helpers.dart';

void main() {
  group('bytesToHex', () {
    test('converts empty list', () {
      expect(bytesToHex([]), '');
    });

    test('converts single byte', () {
      expect(bytesToHex([0]), '00');
      expect(bytesToHex([255]), 'ff');
      expect(bytesToHex([10]), '0a');
    });

    test('converts multiple bytes', () {
      expect(bytesToHex([0xde, 0xad, 0xbe, 0xef]), 'deadbeef');
    });

    test('pads single-digit hex with zero', () {
      expect(bytesToHex([0x01, 0x02, 0x0f]), '01020f');
    });
  });

  group('hexToBytes', () {
    test('converts empty string', () {
      expect(hexToBytes(''), isEmpty);
    });

    test('converts hex string', () {
      final result = hexToBytes('deadbeef');
      expect(result, [0xde, 0xad, 0xbe, 0xef]);
    });

    test('converts uppercase hex', () {
      final result = hexToBytes('DEADBEEF');
      expect(result, [0xde, 0xad, 0xbe, 0xef]);
    });

    test('round-trips with bytesToHex', () {
      final original = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
      final hex = bytesToHex(original);
      final restored = hexToBytes(hex);
      expect(restored, original);
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
      expect(constantTimeEqualsSync('abc', 'abcd'), isFalse);
    });

    test('returns true for empty strings', () {
      expect(constantTimeEqualsSync('', ''), isTrue);
    });

    test('returns false for empty vs non-empty', () {
      expect(constantTimeEqualsSync('', 'a'), isFalse);
    });

    test('returns true for same content different allocation', () {
      final a = 'a' * 100;
      final b = 'a' * 100;
      expect(constantTimeEqualsSync(a, b), isTrue);
    });

    test('detects single character difference', () {
      expect(constantTimeEqualsSync('abcdef', 'abcdeg'), isFalse);
    });
  });
}
