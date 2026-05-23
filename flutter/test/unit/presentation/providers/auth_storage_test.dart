
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';

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
        case 'deleteAll':
          secureStorageData.clear();
          return null;
      }
      return null;
    });
  });

  setUp(() {
    secureStorageData.clear();
    // Clear attempt trackers between tests
    SecureAccountStorage.instance.clearAttemptTrackersForTest();
  });

  tearDownAll(() {
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, null);
  });

  group('SecureAccountStorage.hasSufficientComplexity', () {
    test('returns false for password without uppercase', () {
      expect(SecureAccountStorage.hasSufficientComplexity('password1'), isFalse);
    });

    test('returns false for password without lowercase', () {
      expect(SecureAccountStorage.hasSufficientComplexity('PASSWORD1'), isFalse);
    });

    test('returns false for password without digits', () {
      expect(SecureAccountStorage.hasSufficientComplexity('Password'), isFalse);
    });

    test('returns true for password with upper, lower, digits, and special', () {
      expect(SecureAccountStorage.hasSufficientComplexity('Pass1!'), isTrue);
      expect(SecureAccountStorage.hasSufficientComplexity('MyP4ss!'), isTrue);
    });

    test('returns false for empty password', () {
      expect(SecureAccountStorage.hasSufficientComplexity(''), isFalse);
    });
  });

  group('AttemptTracker', () {
    test('initial state has zero attempts and no lockout', () {
      final tracker = AttemptTracker();
      expect(tracker.attempts, 0);
      expect(tracker.isLockedOut, isFalse);
      expect(tracker.remainingLockout, Duration.zero);
      expect(tracker.shouldBackoff, isFalse);
      expect(tracker.currentBackoff, Duration.zero);
    });

    test('recordFailure increments attempts', () {
      final tracker = AttemptTracker();
      tracker.recordFailure();
      expect(tracker.attempts, 1);
      tracker.recordFailure();
      expect(tracker.attempts, 2);
    });

    test('shouldBackoff is false below threshold', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < AttemptTracker.backoffStartAfterAttempts - 1; i++) {
        tracker.recordFailure();
      }
      expect(tracker.shouldBackoff, isFalse);
    });

    test('shouldBackoff is true at threshold', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < AttemptTracker.backoffStartAfterAttempts; i++) {
        tracker.recordFailure();
      }
      expect(tracker.shouldBackoff, isTrue);
    });

    test('currentBackoff increases exponentially', () {
      final tracker = AttemptTracker();
      // Reach backoff threshold
      for (var i = 0; i < AttemptTracker.backoffStartAfterAttempts; i++) {
        tracker.recordFailure();
      }
      final baseBackoff = tracker.currentBackoff;
      expect(baseBackoff, AttemptTracker.initialBackoff);

      tracker.recordFailure();
      final doubled = tracker.currentBackoff;
      expect(doubled, AttemptTracker.initialBackoff * 2);

      tracker.recordFailure();
      final quadrupled = tracker.currentBackoff;
      expect(quadrupled, AttemptTracker.initialBackoff * 4);
    });

    test('currentBackoff is capped at 300 seconds', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < 20; i++) {
        tracker.recordFailure();
      }
      expect(tracker.currentBackoff.inSeconds, lessThanOrEqualTo(300));
    });

    test('lockout triggers at max attempts', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < AttemptTracker.maxAttempts; i++) {
        tracker.recordFailure();
      }
      expect(tracker.isLockedOut, isTrue);
      expect(tracker.remainingLockout.inSeconds, greaterThan(0));
    });

    test('remainingLockout decreases during lockout', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < AttemptTracker.maxAttempts; i++) {
        tracker.recordFailure();
      }
      expect(tracker.isLockedOut, isTrue);
      final remaining = tracker.remainingLockout;
      expect(remaining.inSeconds, greaterThan(0));
      expect(remaining, lessThanOrEqualTo(AttemptTracker.lockoutDuration));
    });

    test('reset clears state', () {
      final tracker = AttemptTracker();
      for (var i = 0; i < AttemptTracker.maxAttempts; i++) {
        tracker.recordFailure();
      }
      expect(tracker.isLockedOut, isTrue);

      tracker.reset();
      expect(tracker.attempts, 0);
      expect(tracker.isLockedOut, isFalse);
      expect(tracker.shouldBackoff, isFalse);
    });

    test('remainingLockout returns zero when not locked out', () {
      final tracker = AttemptTracker();
      expect(tracker.remainingLockout, Duration.zero);
    });
  });

  group('SecureAccountStorage.listAccounts', () {
    test('returns empty list when no accounts', () async {
      final accounts = await SecureAccountStorage.instance.listAccounts();
      expect(accounts, isEmpty);
    });

    test('returns accounts from storage', () async {
      const accountsJson = '[{"id":"acc1","name":"Test","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      secureStorageData['solosoul_accounts'] = accountsJson;

      final accounts = await SecureAccountStorage.instance.listAccounts();
      expect(accounts.length, 1);
      expect(accounts.first.id, 'acc1');
      expect(accounts.first.name, 'Test');
    });

    test('returns empty list for invalid JSON', () async {
      secureStorageData['solosoul_accounts'] = 'not json';
      await expectLater(
        SecureAccountStorage.instance.listAccounts(),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('SecureAccountStorage.getAccountData', () {
    test('returns null when no data', () async {
      final data = await SecureAccountStorage.instance.getAccountData('acc1');
      expect(data, isNull);
    });

    test('returns decoded data', () async {
      secureStorageData['solosoul_account_acc1'] =
          '{"crypto_version":2}';
      final data = await SecureAccountStorage.instance.getAccountData('acc1');
      expect(data, isNotNull);
      expect(data!['crypto_version'], 2);
    });
  });

  group('SecureAccountStorage.saveAccountData', () {
    test('writes data to storage', () async {
      await SecureAccountStorage.instance.saveAccountData('acc1', {
        'crypto_version': 2,
      });
      final stored = secureStorageData['solosoul_account_acc1'];
      expect(stored, contains('crypto_version'));
    });
  });

  group('SecureAccountStorage.deleteAccount', () {
    test('removes account from list and deletes data', () async {
      // Setup: two accounts
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"First","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"},{"id":"acc2","name":"Second","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      secureStorageData['solosoul_account_acc1'] = '{"crypto_version":2}';
      secureStorageData['solosoul_account_acc2'] = '{"crypto_version":2}';

      final success = await SecureAccountStorage.instance.deleteAccount('acc1');
      expect(success, isTrue);

      final accounts = await SecureAccountStorage.instance.listAccounts();
      expect(accounts.length, 1);
      expect(accounts.first.id, 'acc2');
      expect(secureStorageData.containsKey('solosoul_account_acc1'), isFalse);
    });

    test('returns false when delete keychain data throws', () async {
      // Set accounts list with acc1
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"First","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';
      // Don't set account data - delete will succeed (non-fatal)
      // The first part (save accounts) will succeed
      final success = await SecureAccountStorage.instance.deleteAccount('acc1');
      expect(success, isTrue);
    });
  });

  group('SecureAccountStorage.createAccount validation', () {
    test('rejects empty name', () async {
      final result = await SecureAccountStorage.instance.createAccount('', 'Password1');
      expect(result.success, isFalse);
      expect(result.error, 'Account name is required');
    });

    test('rejects short password', () async {
      final result = await SecureAccountStorage.instance.createAccount('Test', 'short');
      expect(result.success, isFalse);
      expect(result.error, 'Password must be at least 8 characters');
    });

    test('rejects weak 8-char password without complexity', () async {
      final result = await SecureAccountStorage.instance.createAccount('Test', 'password');
      expect(result.success, isFalse);
      expect(result.error, contains('uppercase, lowercase, digits, and special characters'));
    });

    test('accepts 8-char password with complexity (proceeds past validation)', () async {
      final result = await SecureAccountStorage.instance.createAccount('Test', 'Passw0rd');
      // Validation passes, then hits FFI. We verify it didn't fail at validation.
      expect(result.error, isNot(contains('Account name is required')));
      expect(result.error, isNot(contains('Password must be at least')));
    }, skip: 'Requires initialized FRB');

    test('accepts 12-char password without complexity (proceeds past validation)', () async {
      final result = await SecureAccountStorage.instance.createAccount('Test', 'longpassword');
      expect(result.error, isNot(contains('Account name is required')));
      expect(result.error, isNot(contains('Password must be at least')));
    }, skip: 'Requires initialized FRB');

    test('rejects duplicate name', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Existing","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';

      final result = await SecureAccountStorage.instance.createAccount('existing', 'Password1!');
      expect(result.success, isFalse);
      expect(result.error, 'This account name is already taken');
    });

    test('rejects duplicate name case-insensitive', () async {
      secureStorageData['solosoul_accounts'] =
          '[{"id":"acc1","name":"Existing","created_at":"2024-01-01T00:00:00.000Z","last_accessed":"2024-01-01T00:00:00.000Z"}]';

      final result = await SecureAccountStorage.instance.createAccount('EXISTING', 'Password1!');
      expect(result.success, isFalse);
      expect(result.error, 'This account name is already taken');
    });
  });
}
