import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

void main() {
  group('DeviceInfo', () {
    test('creates with required fields', () {
      final now = DateTime(2024, 6, 15);
      final info = DeviceInfo(deviceName: 'MacBook Pro', lastUsed: now);
      expect(info.deviceName, 'MacBook Pro');
      expect(info.lastUsed, now);
    });

    group('JSON serialization', () {
      test('toJson produces correct map', () {
        final now = DateTime(2024, 6, 15, 10, 30);
        final info = DeviceInfo(deviceName: 'MacBook', lastUsed: now);
        final json = info.toJson();
        expect(json['device_name'], 'MacBook');
        expect(json['last_used'], now.toIso8601String());
      });

      test('fromJson round-trips correctly', () {
        final now = DateTime(2024, 6, 15, 10, 30);
        final original = DeviceInfo(deviceName: 'iPhone', lastUsed: now);
        final json = original.toJson();
        final restored = DeviceInfo.fromJson(json);
        expect(restored.deviceName, original.deviceName);
        expect(restored.lastUsed, original.lastUsed);
      });
    });
  });

  group('AccountInfo', () {
    test('creates with required fields only', () {
      const info = AccountInfo(id: 'acc-1', name: 'Test User');
      expect(info.id, 'acc-1');
      expect(info.name, 'Test User');
      expect(info.passwordHint, isNull);
      expect(info.lastAccessed, isNull);
      expect(info.createdAt, isNull);
      expect(info.lastLoginAt, isNull);
      expect(info.lastOperationAt, isNull);
      expect(info.lastOperationDesc, isNull);
      expect(info.recentDevices, isEmpty);
    });

    test('creates with all fields', () {
      final now = DateTime(2024, 6, 15);
      final device = DeviceInfo(deviceName: 'Mac', lastUsed: now);
      final info = AccountInfo(
        id: 'acc-1',
        name: 'Test',
        passwordHint: 'hint',
        lastAccessed: now,
        createdAt: now,
        lastLoginAt: now,
        lastOperationAt: now,
        lastOperationDesc: 'Created profile',
        recentDevices: [device],
      );
      expect(info.passwordHint, 'hint');
      expect(info.recentDevices, hasLength(1));
    });

    group('JSON serialization', () {
      test('toJson produces correct map', () {
        const info = AccountInfo(
          id: 'acc-1',
          name: 'Test',
          passwordHint: 'my hint',
        );
        final json = info.toJson();
        expect(json['id'], 'acc-1');
        expect(json['name'], 'Test');
        expect(json['password_hint'], 'my hint');
        expect(json['last_accessed'], isNull);
        expect(json['recent_devices'], isEmpty);
      });

      test('fromJson round-trips correctly', () {
        final now = DateTime(2024, 6, 15, 10, 30);
        final original = AccountInfo(
          id: 'acc-1',
          name: 'Test',
          passwordHint: 'hint',
          lastAccessed: now,
          createdAt: now,
          lastLoginAt: now,
          lastOperationAt: now,
          lastOperationDesc: 'Did something',
          recentDevices: [
            DeviceInfo(deviceName: 'Mac', lastUsed: now),
          ],
        );
        final json = original.toJson();
        final restored = AccountInfo.fromJson(json);
        expect(restored.id, original.id);
        expect(restored.name, original.name);
        expect(restored.passwordHint, original.passwordHint);
        expect(restored.lastAccessed, original.lastAccessed);
        expect(restored.createdAt, original.createdAt);
        expect(restored.lastLoginAt, original.lastLoginAt);
        expect(restored.lastOperationAt, original.lastOperationAt);
        expect(restored.lastOperationDesc, original.lastOperationDesc);
        expect(restored.recentDevices, hasLength(1));
        expect(restored.recentDevices.first.deviceName, 'Mac');
      });

      test('fromJson handles missing optional fields', () {
        final json = {'id': 'acc-1', 'name': 'Test'};
        final info = AccountInfo.fromJson(json);
        expect(info.passwordHint, isNull);
        expect(info.lastAccessed, isNull);
        expect(info.recentDevices, isEmpty);
      });
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const original = AccountInfo(id: 'acc-1', name: 'Test');
        final copy = original.copyWith();
        expect(copy.id, 'acc-1');
        expect(copy.name, 'Test');
      });

      test('copies with changes', () {
        const original = AccountInfo(id: 'acc-1', name: 'Test');
        final copy = original.copyWith(name: 'Updated');
        expect(copy.id, 'acc-1');
        expect(copy.name, 'Updated');
      });
    });
  });

  group('AuthState', () {
    test('has expected values', () {
      expect(AuthState.values, hasLength(4));
      expect(AuthState.values, contains(AuthState.initial));
      expect(AuthState.values, contains(AuthState.locked));
      expect(AuthState.values, contains(AuthState.unlocked));
      expect(AuthState.values, contains(AuthState.loading));
    });
  });
}
