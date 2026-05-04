import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_services.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final secureStorageData = <String, String?>{};

  setUpAll(() {
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, (call) async {
      final args = call.arguments as Map<dynamic, dynamic>?;
      final key = args?['key'] as String?;
      switch (call.method) {
        case 'read':
          return secureStorageData[key];
        case 'write':
          if (key != null) {
            secureStorageData[key] = args?['value'] as String?;
          }
          return null;
        case 'delete':
          if (key != null) {
            secureStorageData.remove(key);
          }
          return null;
      }
      return null;
    });
  });

  setUp(() {
    secureStorageData.clear();
  });

  tearDownAll(() {
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, null);
  });

  group('VaultUnlockService', () {
    const service = VaultUnlockService();

    test('vaultExists returns false for empty accounts', () {
      expect(service.vaultExists([]), isFalse);
    });

    test('vaultExists returns true for non-empty accounts', () {
      final accounts = [
        const AccountInfo(id: 'acc_test', name: 'Test'),
      ];
      expect(service.vaultExists(accounts), isTrue);
    });
  });

  group('AccountManager', () {
    test('initial getters return default values', () {
      final manager = AccountManager(SecureAccountStorage.instance);
      expect(manager.selectedAccountId, isNull);
      expect(manager.selectedAccount, isNull);
      expect(manager.accountsVersion, 0);
    });

    test('bumpAccountsVersion increments version', () {
      final manager = AccountManager(SecureAccountStorage.instance);
      manager.bumpAccountsVersion();
      expect(manager.accountsVersion, 1);
      manager.bumpAccountsVersion();
      expect(manager.accountsVersion, 2);
    });

    test('getAccounts returns empty list when no accounts', () async {
      final manager = AccountManager(SecureAccountStorage.instance);
      final accounts = await manager.getAccounts();
      expect(accounts, isEmpty);
    });

    test('getAccounts returns accounts from storage', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"First","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"},{"id":"acc2","name":"Second","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      final manager = AccountManager(SecureAccountStorage.instance);
      final accounts = await manager.getAccounts();
      expect(accounts.length, 2);
      expect(accounts.first.name, 'First');
    });

    test('selectAccount sets selected account and info', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Test Account","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      final manager = AccountManager(SecureAccountStorage.instance);
      await manager.selectAccount('acc1');
      expect(manager.selectedAccountId, 'acc1');
      expect(manager.selectedAccount, isNotNull);
      expect(manager.selectedAccount!.name, 'Test Account');
      expect(manager.accountsVersion, 1);
    });

    test('selectAccount with null deselects account', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Test","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      final manager = AccountManager(SecureAccountStorage.instance);
      await manager.selectAccount('acc1');
      expect(manager.selectedAccountId, isNotNull);

      await manager.selectAccount(null);
      expect(manager.selectedAccountId, isNull);
      expect(manager.selectedAccount, isNull);
      expect(manager.accountsVersion, 2);
    });

    test('selectAccount with non-existent id sets null info', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Test","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      final manager = AccountManager(SecureAccountStorage.instance);
      await manager.selectAccount('nonexistent');
      expect(manager.selectedAccountId, 'nonexistent');
      expect(manager.selectedAccount, isNull);
    });
  });
}
