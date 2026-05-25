import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/machine_key_generator.dart';

void main() {
  group('MachineKeyGenerator', () {
    test('generate returns auto_ prefix', () {
      final key = MachineKeyGenerator.generate();
      expect(key.startsWith('auto_'), isTrue);
    });

    test('generate returns unique keys', () {
      final key1 = MachineKeyGenerator.generate();
      final key2 = MachineKeyGenerator.generate();
      expect(key1, isNot(equals(key2)));
    });

    test('generate has correct format', () {
      final key = MachineKeyGenerator.generate();
      // Format: auto_{8 hex chars}
      final parts = key.split('_');
      expect(parts.length, 2);
      expect(parts[0], 'auto');
      expect(parts[1].length, 8);
      expect(
        RegExp(r'^[a-f0-9]{8}$').hasMatch(parts[1]),
        isTrue,
      );
    });

    test('isAutoKey returns true for auto key', () {
      expect(MachineKeyGenerator.isAutoKey('auto_a3f7d2e1'), isTrue);
    });

    test('isAutoKey returns false for regular key', () {
      expect(MachineKeyGenerator.isAutoKey('fullName'), isFalse);
      expect(MachineKeyGenerator.isAutoKey('identity.fullName'), isFalse);
    });

    test('isAutoKey returns false for empty string', () {
      expect(MachineKeyGenerator.isAutoKey(''), isFalse);
    });
  });
}
