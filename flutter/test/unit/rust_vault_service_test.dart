import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

void main() {
  // Skip tests that require FFI (macOS/Android only) when running on Linux CI
  final isLinux = Platform.operatingSystem == 'linux';
  final skipOnLinux = isLinux ? 'RustVaultService requires FFI (macOS/Android only)' : null;
  group('BridgeProfileSummary', () {
    test('creates with all fields', () {
      const summary = BridgeProfileSummary(
        id: 'profile_1',
        name: 'Test Profile',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-02T00:00:00Z',
        version: 2,
      );

      expect(summary.id, 'profile_1');
      expect(summary.name, 'Test Profile');
      expect(summary.createdAt, '2024-01-01T00:00:00Z');
      expect(summary.updatedAt, '2024-01-02T00:00:00Z');
      expect(summary.version, 2);
    });

    test('fromJson parses correctly', () {
      final json = {
        'id': 'profile_2',
        'name': 'JSON Profile',
        'created_at': '2024-01-01T00:00:00Z',
        'updated_at': '2024-01-02T00:00:00Z',
        'version': 3,
      };

      final summary = BridgeProfileSummary.fromJson(json);

      expect(summary.id, 'profile_2');
      expect(summary.name, 'JSON Profile');
      expect(summary.version, 3);
    });

    test('toJson produces correct output', () {
      const summary = BridgeProfileSummary(
        id: 'profile_3',
        name: 'Test',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-02T00:00:00Z',
        version: 1,
      );

      final json = summary.toJson();

      expect(json['id'], 'profile_3');
      expect(json['name'], 'Test');
      expect(json['created_at'], '2024-01-01T00:00:00Z');
      expect(json['version'], 1);
    });

    test('roundtrip: fromJson -> toJson preserves data', () {
      const original = BridgeProfileSummary(
        id: 'roundtrip',
        name: 'Roundtrip Test',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-02T00:00:00Z',
        version: 4,
      );

      final json = original.toJson();
      final restored = BridgeProfileSummary.fromJson(json);

      expect(restored.id, original.id);
      expect(restored.name, original.name);
      expect(restored.createdAt, original.createdAt);
      expect(restored.updatedAt, original.updatedAt);
      expect(restored.version, original.version);
    });
  });

  group('RustVaultService account operations', skip: skipOnLinux, () {
    late RustVaultService service;

    setUp(() {
      service = RustVaultService.instance;
    });

    test('createAccount returns failure result type', () {
      // Without actual Rust library, returns error result
      final result = service.createAccount(
        name: 'test_account',
        password: 'short', // Too short
      );

      expect(result.success, false);
      expect(result.error, isNotNull);
    });

    test('unlockVault returns failure for empty password', () {
      final result = service.unlockVault(
        accountId: 'acc_123',
        password: '',
      );

      expect(result.success, false);
    });

    test('deleteAccount returns bool', () {
      // Without Rust library, returns false
      final result = service.deleteAccount('nonexistent');

      expect(result, isA<bool>());
    });

    test('getVaultStats returns null without library', () {
      final result = service.getVaultStats();

      // Without native library, returns null
      expect(result, isNull);
    });
  });

  group('RustVaultService initialization', skip: skipOnLinux, () {
    test('initAccountManager is callable', () {
      final service = RustVaultService.instance;

      // Method should be callable even without library
      // Returns false on Android or when library not loaded
      final result = service.initAccountManager('/test/path');

      expect(result, isA<bool>());
    });

    test('isVaultUnlocked is callable', () {
      final service = RustVaultService.instance;

      final result = service.isVaultUnlocked();

      expect(result, isA<bool>());
    });
  });
}
