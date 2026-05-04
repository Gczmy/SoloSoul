import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_notifier.dart';
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

  group('AuthNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() => container.dispose());

    test('build returns AuthState.initial', () async {
      final state = await container.read(authNotifierProvider.future);
      expect(state, AuthState.initial);
    });

    test('initial state value is AuthState.initial', () async {
      await container.read(authNotifierProvider.future);
      final asyncState = container.read(authNotifierProvider);
      expect(asyncState.value, AuthState.initial);
    });

    test('isUnlocked is false in initial state', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      expect(notifier.isUnlocked, isFalse);
    });

    test('selectedAccountId is null initially', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      expect(notifier.selectedAccountId, isNull);
    });

    test('selectedAccount is null initially', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      expect(notifier.selectedAccount, isNull);
    });

    test('accountsVersion starts at 0', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      expect(notifier.accountsVersion, 0);
    });

    test('lastUnlockError is null initially', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      expect(notifier.lastUnlockError, isNull);
    });

    test('vaultExists returns false when no accounts', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final exists = await notifier.vaultExists();
      expect(exists, isFalse);
    });

    test('getAccounts returns empty list', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final accounts = await notifier.getAccounts();
      expect(accounts, isEmpty);
    });

    test('selectAccount with null deselects and bumps version', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      await notifier.selectAccount(null);
      expect(notifier.selectedAccountId, isNull);
      expect(notifier.accountsVersion, 1);
    });

    test('selectAccount bumps version even when null', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final v0 = notifier.accountsVersion;
      await notifier.selectAccount(null);
      expect(notifier.accountsVersion, v0 + 1);
    });

    test('vaultExists returns true when accounts exist', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Test","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final exists = await notifier.vaultExists();
      expect(exists, isTrue);
    });

    test('unlockVault with empty password returns false', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final result = await notifier.unlockVault('');
      expect(result, isFalse);
    });

    test('unlockVault with no selected account returns false', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final result = await notifier.unlockVault('somepassword');
      expect(result, isFalse);
    });

    test('verifyPasswordForSensitiveData with empty password returns false', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final result = await notifier.verifyPasswordForSensitiveData('');
      expect(result, isFalse);
    });

    test('verifyPasswordForSensitiveData with no selected account returns false', () async {
      await container.read(authNotifierProvider.future);
      final notifier = container.read(authNotifierProvider.notifier);
      final result = await notifier.verifyPasswordForSensitiveData('somepassword');
      expect(result, isFalse);
    });
  });

  group('AuthState', () {
    test('has four values', () {
      expect(AuthState.values, hasLength(4));
      expect(AuthState.values, contains(AuthState.initial));
      expect(AuthState.values, contains(AuthState.locked));
      expect(AuthState.values, contains(AuthState.unlocked));
      expect(AuthState.values, contains(AuthState.loading));
    });

    test('values have correct index order', () {
      expect(AuthState.initial.index, 0);
      expect(AuthState.locked.index, 1);
      expect(AuthState.unlocked.index, 2);
      expect(AuthState.loading.index, 3);
    });
  });
}
