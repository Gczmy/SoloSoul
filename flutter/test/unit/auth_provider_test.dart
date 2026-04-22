import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
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
      final original = SensitivePageAccessState();
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
}
