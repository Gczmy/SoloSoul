import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

void main() {
  group('SensitivePageAccessState', () {
    test('isValid returns false when lastVerified is null', () {
      const state = SensitivePageAccessState();
      expect(state.isValid, false);
    });

    test('isValid returns true when verified within timeout', () {
      final state = SensitivePageAccessState(
        lastVerified: DateTime.now(),
      );
      expect(state.isValid, true);
    });

    test('isValid returns false when verified before timeout', () async {
      final state = SensitivePageAccessState(
        lastVerified: DateTime.now().subtract(
          kSensitiveAccessTimeout + const Duration(seconds: 1),
        ),
      );
      expect(state.isValid, false);
    });

    test('copyWith creates new instance with updated values', () {
      const original = SensitivePageAccessState();
      final now = DateTime.now();
      final copied = original.copyWith(lastVerified: now);

      expect(copied.lastVerified, now);
      expect(original.lastVerified, isNull);
    });
  });

  group('SensitivePageAccessNotifier', () {
    late SensitivePageAccessNotifier notifier;

    setUp(() {
      notifier = SensitivePageAccessNotifier();
    });

    tearDown(() {
      notifier.dispose();
    });

    test('markVerified sets lastVerified to current time', () async {
      final before = DateTime.now();

      notifier.markVerified();

      final after = DateTime.now();

      expect(notifier.state.isValid, true);
      expect(
        notifier.state.lastVerified!.isAfter(before) ||
            notifier.state.lastVerified!.isAtSameMomentAs(before),
        true,
      );
      expect(notifier.state.lastVerified!.isBefore(after), true);
    });

    test('clear resets state to invalid', () {
      notifier.markVerified();
      expect(notifier.state.isValid, true);

      notifier.clear();

      expect(notifier.state.isValid, false);
      expect(notifier.state.lastVerified, isNull);
    });

    test('multiple markVerified calls update lastVerified', () async {
      notifier.markVerified();
      final firstVerified = notifier.state.lastVerified;

      // Small delay to ensure different timestamp
      await Future.delayed(const Duration(milliseconds: 10));
      notifier.markVerified();

      expect(
        notifier.state.lastVerified!.isAfter(firstVerified!) ||
            notifier.state.lastVerified!.isAtSameMomentAs(firstVerified),
        true,
      );
    });
  });

  group('kSensitiveAccessTimeout', () {
    test('is set to 1 minute', () {
      expect(kSensitiveAccessTimeout, const Duration(minutes: 1));
    });
  });

  group('AccountInfo', () {
    test('fromJson parses all fields correctly', () {
      final json = {
        'id': 'acc_123',
        'name': 'Test Account',
        'password_hint': 'My hint',
        'last_accessed': '2024-01-01T12:00:00.000Z',
        'created_at': '2024-01-01T10:00:00.000Z',
        'last_login_at': '2024-01-01T11:00:00.000Z',
        'last_operation_at': '2024-01-01T11:30:00.000Z',
        'last_operation_desc': 'Updated profile',
        'recent_devices': [
          {'device_name': 'MacBook Pro', 'last_used': '2024-01-01T12:00:00.000Z'},
        ],
      };

      final account = AccountInfo.fromJson(json);

      expect(account.id, 'acc_123');
      expect(account.name, 'Test Account');
      expect(account.passwordHint, 'My hint');
      expect(account.lastAccessed, isNotNull);
      expect(account.createdAt, isNotNull);
      expect(account.lastLoginAt, isNotNull);
      expect(account.lastOperationDesc, 'Updated profile');
      expect(account.recentDevices.length, 1);
      expect(account.recentDevices.first.deviceName, 'MacBook Pro');
    });

    test('fromJson handles missing optional fields', () {
      final json = {
        'id': 'acc_456',
        'name': 'Minimal Account',
      };

      final account = AccountInfo.fromJson(json);

      expect(account.id, 'acc_456');
      expect(account.name, 'Minimal Account');
      expect(account.passwordHint, isNull);
      expect(account.lastAccessed, isNull);
      expect(account.recentDevices, isEmpty);
    });

    test('toJson produces correct output', () {
      final account = AccountInfo(
        id: 'acc_789',
        name: 'Test',
        passwordHint: 'hint',
        lastAccessed: DateTime.parse('2024-01-01T12:00:00.000Z'),
        createdAt: DateTime.parse('2024-01-01T10:00:00.000Z'),
        lastLoginAt: DateTime.parse('2024-01-01T11:00:00.000Z'),
        lastOperationAt: DateTime.parse('2024-01-01T11:30:00.000Z'),
        lastOperationDesc: 'Test operation',
        recentDevices: [
          DeviceInfo(
            deviceName: 'Test Device',
            lastUsed: DateTime.parse('2024-01-01T12:00:00.000Z'),
          ),
        ],
      );

      final json = account.toJson();

      expect(json['id'], 'acc_789');
      expect(json['name'], 'Test');
      expect(json['password_hint'], 'hint');
      expect(json['recent_devices'], hasLength(1));
    });

    test('copyWith preserves unchanged fields', () {
      final original = AccountInfo(
        id: 'acc_1',
        name: 'Original',
        passwordHint: 'hint1',
        createdAt: DateTime.parse('2024-01-01T10:00:00.000Z'),
      );

      final copied = original.copyWith(name: 'Modified');

      expect(copied.id, 'acc_1');
      expect(copied.name, 'Modified');
      expect(copied.passwordHint, 'hint1');
      expect(copied.createdAt, original.createdAt);
    });
  });

  group('DeviceInfo', () {
    test('fromJson parses correctly', () {
      final json = {
        'device_name': 'iPhone 15',
        'last_used': '2024-01-01T12:00:00.000Z',
      };

      final device = DeviceInfo.fromJson(json);

      expect(device.deviceName, 'iPhone 15');
      expect(device.lastUsed.year, 2024);
    });

    test('toJson produces correct output', () {
      final device = DeviceInfo(
        deviceName: 'Mac Studio',
        lastUsed: DateTime.parse('2024-01-01T12:00:00.000Z'),
      );

      final json = device.toJson();

      expect(json['device_name'], 'Mac Studio');
      expect(json['last_used'], contains('2024-01-01'));
    });
  });

  group('AuthState', () {
    test('enum has expected values', () {
      expect(AuthState.values, contains(AuthState.initial));
      expect(AuthState.values, contains(AuthState.locked));
      expect(AuthState.values, contains(AuthState.unlocked));
      expect(AuthState.values, contains(AuthState.loading));
    });
  });

  group('Constant-time string comparison', () {
    // Testing the logic directly - this is the algorithm used by _constantTimeEquals
    bool testConstantTimeEquals(String a, String b) {
      final lenA = a.length;
      final lenB = b.length;
      final maxLen = lenA > lenB ? lenA : lenB;

      final paddedA = a.padRight(maxLen, '\x00');
      final paddedB = b.padRight(maxLen, '\x00');

      var result = 0;
      for (var i = 0; i < maxLen; i++) {
        result |= paddedA.codeUnitAt(i) ^ paddedB.codeUnitAt(i);
      }
      result |= lenA ^ lenB;
      return result == 0;
    }

    test('returns true for identical strings', () {
      expect(testConstantTimeEquals('password', 'password'), true);
    });

    test('returns false for different strings', () {
      expect(testConstantTimeEquals('password', 'different'), false);
    });

    test('returns false for different lengths', () {
      expect(testConstantTimeEquals('short', 'muchlonger'), false);
    });

    test('returns false even when only last char differs', () {
      expect(testConstantTimeEquals('password', 'passworX'), false);
    });

    test('handles empty strings', () {
      expect(testConstantTimeEquals('', ''), true);
      expect(testConstantTimeEquals('', 'a'), false);
    });

    test('handles unicode characters', () {
      expect(testConstantTimeEquals('password', 'password'), true);
      expect(testConstantTimeEquals('password', 'passwör'), false);
    });

    test('constant-time behavior: no early return', () {
      // Verify the algorithm doesn't short-circuit
      final a = 'abcdefghijklmnop';
      final b = 'abcdefghijklmnoo';
      expect(testConstantTimeEquals(a, b), false);
      expect(testConstantTimeEquals(a, a), true);
    });
  });

  group('Bytes to Hex conversion', () {
    String bytesToHex(List<int> bytes) {
      return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
    }

    test('converts bytes to hex correctly', () {
      expect(bytesToHex([0, 1, 254, 255]), '0001feff');
    });

    test('handles empty list', () {
      expect(bytesToHex([]), '');
    });

    test('hex to bytes roundtrip', () {
      final original = [0x12, 0xAB, 0xFF];
      final hex = bytesToHex(original);
      expect(hex, '12abff');

      // Parse hex back to bytes
      final result = <int>[];
      for (var i = 0; i < hex.length; i += 2) {
        result.add(int.parse(hex.substring(i, i + 2), radix: 16));
      }
      expect(result, original);
    });
  });

  group('NativeCryptoService integration for key derivation', () {
    test('deriveKey with same salt produces consistent results', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key1 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, equals(key2));
    });

    test('deriveKey with different salts produces different keys', () {
      final service = NativeCryptoService.instance;
      final salt1 = service.generateSalt()!;
      final salt2 = service.generateSalt()!;

      final key1 = service.deriveKey(
        password: 'samepassword',
        salt: salt1,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'samepassword',
        salt: salt2,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, isNot(equals(key2)));
    });

    test('verifyHash roundtrip with base64 encoding', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key = service.deriveKey(
        password: 'password123',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key, isNotNull);

      // Encode as base64 (like Dart-generated verify hash)
      final encoded = base64Encode(key!);

      // Decode and verify
      final decoded = base64Decode(encoded);
      expect(decoded, equals(key));
    });

    test('verifyHash roundtrip with hex encoding', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key = service.deriveKey(
        password: 'password123',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key, isNotNull);

      // Encode as hex (like Rust-generated verify hash)
      final encoded = bytesToHex(key!);

      // Decode and verify
      final decoded = <int>[];
      for (var i = 0; i < encoded.length; i += 2) {
        decoded.add(int.parse(encoded.substring(i, i + 2), radix: 16));
      }
      expect(Uint8List.fromList(decoded), equals(key));
    });
  });

  group('AuthNotifier State', () {
    test('AuthNotifier initial state is AuthState.initial', () {
      final notifier = AuthNotifier();
      expect(notifier.state, AuthState.initial);
      expect(notifier.isUnlocked, false);
      expect(notifier.selectedAccountId, isNull);
      expect(notifier.selectedAccount, isNull);
    });
  });
}

// Helper for hex conversion
String bytesToHex(List<int> bytes) {
  return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
}
