import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_services.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

part 'auth_notifier.g.dart';

/// Auth state provider
final authNotifierProvider = AsyncNotifierProvider<AuthNotifier, AuthState>(() {
  return AuthNotifier();
});

/// Provider that watches accountsVersion from AuthNotifier
@riverpod
class AccountsVersion extends _$AccountsVersion {
  @override
  int build() {
    ref.watch(authNotifierProvider);
    return ref.read(authNotifierProvider.notifier).accountsVersion;
  }
}

/// Accounts notifier - manages account list with version-based invalidation
class AccountsNotifier extends AsyncNotifier<List<AccountInfo>> {
  @override
  Future<List<AccountInfo>> build() async {
    ref.watch(accountsVersionProvider);
    return ref.read(authNotifierProvider.notifier).getAccountsSortedByRecent();
  }
}

/// Accounts provider - lists all accounts sorted by recent access
final accountsProvider =
    AsyncNotifierProvider<AccountsNotifier, List<AccountInfo>>(() {
  return AccountsNotifier();
});

/// AuthNotifier - Facade that delegates to services (AsyncNotifier)
class AuthNotifier extends AsyncNotifier<AuthState> {
  @override
  Future<AuthState> build() async {
    // Initial state - vault starts locked until user unlocks it
    return AuthState.initial;
  }

  final SecureAccountStorage _storage;
  final ProfileStorageService _profileStorage;
  final VaultUnlockService _vaultUnlockService;
  final MigrationService _migrationService;
  final PasswordService _passwordService;
  final AccountManager _accountManager;

  AuthNotifier({
    SecureAccountStorage? storage,
    ProfileStorageService? profileStorage,
  })  : _storage = storage ?? SecureAccountStorage.instance,
        _profileStorage = profileStorage ?? ProfileStorageService.instance,
        _vaultUnlockService = const VaultUnlockService(),
        _migrationService = MigrationService(storage ?? SecureAccountStorage.instance),
        _passwordService = PasswordService(
          storage ?? SecureAccountStorage.instance,
        ),
        _accountManager = AccountManager(
          storage ?? SecureAccountStorage.instance,
          profileStorage ?? ProfileStorageService.instance,
        );

  // Convenience getters delegating to services
  String? get selectedAccountId => _accountManager.selectedAccountId;
  AccountInfo? get selectedAccount => _accountManager.selectedAccount;
  bool get isUnlocked => state.value == AuthState.unlocked;
  int get accountsVersion => _accountManager.accountsVersion;

  /// Get all accounts sorted by most recent access
  Future<List<AccountInfo>> getAccountsSortedByRecent() {
    return _accountManager.getAccountsSortedByRecent();
  }

  /// Get all accounts
  Future<List<AccountInfo>> getAccounts() {
    return _accountManager.getAccounts();
  }

  /// Select an account
  Future<void> selectAccount(String? accountId) async {
    await _accountManager.selectAccount(accountId);
    // Trigger rebuild by setting state to current value
    state = AsyncData(state.value ?? AuthState.locked);
  }

  /// Create a new account
  Future<({bool success, String? error})> createAccount(
    String name,
    String password, {
    String? passwordHint,
  }) async {
    DebugLogger.instance.logInfo('AUTH', 'CHECKPOINT: createAccount start');

    state = const AsyncLoading();

    final result = await _accountManager.createAccount(
      name,
      password,
      passwordHint: passwordHint,
    );

    if (result.success) {
      // Keep vault locked after account creation
      state = const AsyncData(AuthState.locked);
      return (success: true, error: null);
    } else {
      state = const AsyncData(AuthState.locked);
      return (success: false, error: result.error);
    }
  }

  /// Unlock vault with master password
  Future<bool> unlockVault(String password) async {
    SoloLog.d('Auth', 'unlockVault start, selectedAccountId=${_accountManager.selectedAccountId}');

    if (_accountManager.selectedAccountId == null) {
      SoloLog.w('Auth', '_selectedAccountId is null, returning false');
      return false;
    }
    if (password.isEmpty) {
      SoloLog.w('Auth', 'password is empty, returning false');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    state = const AsyncLoading();

    // Step 1: Unlock Rust vault
    final timer1 = SoloLog.startTimer('Auth', 'RustVaultService.unlockVault');
    final vaultResult = _vaultUnlockService.unlockVault(
      accountId: _accountManager.selectedAccountId!,
      password: password,
    );
    SoloLog.endTimer(timer1);
    SoloLog.d('Auth', 'Rust unlock result: success=${vaultResult.success}, error=${vaultResult.error}, cryptoVersion=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      SoloLog.e('Auth', 'Rust unlock failed: ${vaultResult.error}');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    SoloLog.d('Auth', 'Rust unlock succeeded, checking Keychain...');

    // Step 2: Check for migrations needed
    final timer2 = SoloLog.startTimer('Auth', 'Keychain.getAccountData');
    try {
      final accountData = await _storage
          .getAccountData(_accountManager.selectedAccountId!)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData timed out'),
          );
      SoloLog.endTimer(timer2);
      SoloLog.d('Auth', 'Keychain accountData: ${accountData != null ? "found" : "null"}');

      if (accountData == null) {
        SoloLog.d('Auth', 'Account not in Keychain, migrating from Rust...');
        await _migrationService.migrateAccountFromRust(
          accountId: _accountManager.selectedAccountId!,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
      } else if ((accountData['crypto_version'] as int? ?? 1) < 2) {
        SoloLog.d('Auth', 'V1 account detected, migrating to V2...');
        await _migrationService.migrateAccountToV2(
          accountId: _accountManager.selectedAccountId!,
          password: password,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
      }
    } on Object catch (e, st) {
      SoloLog.e('Auth', 'Migration error', e, st);
    }

    // Step 3: Get session key for profile encryption
    SoloLog.d('Auth', 'Step 3: Getting fresh account data after migration...');
    final timer3 = SoloLog.startTimer('Auth', 'Keychain.getAccountData(fresh)');
    Uint8List? salt;
    try {
      final freshData = await _storage
          .getAccountData(_accountManager.selectedAccountId!)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData timed out'),
          );
      SoloLog.endTimer(timer3);
      SoloLog.d('Auth', 'freshData: ${freshData != null ? "found" : "null"}');

      if (freshData == null) {
        SoloLog.w('Auth', 'freshData is null, getting salt from Rust...');
        final rustConfig = await Future.delayed(
          Duration.zero,
          () => NativeVaultService.instance.getAccountConfig(
              accountId: _accountManager.selectedAccountId!),
        )
            .timeout(const Duration(seconds: 5),
                onTimeout: () => throw TimeoutException('getAccountConfig timed out'));
        if (rustConfig?.salt == null) {
          SoloLog.e('Auth', 'Cannot get salt from Rust - returning false');
          state = const AsyncData(AuthState.locked);
          return false;
        }
        salt = base64Decode(rustConfig!.salt!);
        SoloLog.d('Auth', 'Got salt from Rust');
      } else {
        salt = base64Decode(freshData['salt'] as String);
        // If Keychain has corrupted salt, fall back to Rust
        if (salt.length != 32) {
          SoloLog.w('Auth', 'Keychain salt length=${salt.length} invalid, falling back to Rust');
          final rustConfig = NativeVaultService.instance.getAccountConfig(
              accountId: _accountManager.selectedAccountId!);
          if (rustConfig?.salt != null) {
            salt = base64Decode(rustConfig!.salt!);
          }
        }
      }
    } on Object catch (e, st) {
      SoloLog.endTimer(timer3);
      SoloLog.e('Auth', 'Step 3 error (getAccountData)', e, st);
      // Try to get salt from Rust as last resort
      try {
        final rustConfig = NativeVaultService.instance.getAccountConfig(
            accountId: _accountManager.selectedAccountId!);
        if (rustConfig?.salt != null) {
          salt = base64Decode(rustConfig!.salt!);
          SoloLog.d('Auth', 'Got salt from Rust as fallback');
        }
      } on Object catch (e2, st2) {
        SoloLog.e('Auth', 'Rust fallback also failed', e2, st2);
      }
      if (salt == null || salt.length != 32) {
        state = const AsyncData(AuthState.locked);
        return false;
      }
    }

    if (salt.length != 32) {
      SoloLog.e('Auth', 'Invalid salt length ${salt.length}, expected 32');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    final timer4 = SoloLog.startTimer('Auth', 'NativeCryptoService.deriveKey');
    final sessionKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );
    SoloLog.endTimer(timer4);

    if (sessionKey == null) {
      SoloLog.e('Auth', 'Session key derivation failed');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    SoloLog.d('Auth', 'Session key derived, setting profile encryption...');
    _profileStorage.setEncryptionKey(sessionKey);

    state = const AsyncData(AuthState.unlocked);
    SoloLog.d('Auth', 'Vault unlocked successfully!');

    // 自动备份：解锁成功后异步创建加密备份（不阻塞登录流程）
    _autoBackupAfterUnlock();

    // 升级保护备份：若检测到 App 版本变化，额外创建一份带版本号的备份
    _upgradeBackupIfNeeded(accountId: _accountManager.selectedAccountId!);

    return true;
  }

  /// 解锁成功后自动创建加密备份（不阻塞 UI）
  void _autoBackupAfterUnlock() {
    final accountId = _accountManager.selectedAccountId;
    if (accountId == null) return;
    SoloLog.d('Auth', 'Auto-backup triggered after unlock for $accountId');
    BackupService.instance.createBackup(accountId).then((fileName) {
      if (fileName != null) {
        SoloLog.d('Auth', 'Auto-backup created: $fileName');
      } else {
        SoloLog.w('Auth', 'Auto-backup failed (ignored)');
      }
    }).catchError((Object e, StackTrace st) {
      SoloLog.w('Auth', 'Auto-backup error (ignored): $e');
    });
  }

  /// 升级保护备份：若 App 版本变化，创建带版本号的备份
  void _upgradeBackupIfNeeded({required String accountId}) {
    if (!AppVersionTracker.instance.pendingUpgradeBackup) return;
    final version = AppVersionTracker.instance.currentVersion;
    SoloLog.d('Auth', 'Upgrade backup triggered (version: $version)');
    BackupService.instance.createBackup(accountId, appVersion: version).then((fileName) {
      if (fileName != null) {
        SoloLog.d('Auth', 'Upgrade backup created: $fileName');
      } else {
        SoloLog.w('Auth', 'Upgrade backup failed (ignored)');
      }
      AppVersionTracker.instance.clearPendingBackup();
    }).catchError((Object e, StackTrace st) {
      SoloLog.w('Auth', 'Upgrade backup error (ignored): $e');
    });
  }

  /// Verify password for sensitive data access
  /// Uses Dart-side verification which reads salt/verifyHash directly from
  /// storage (Keychain or Rust config as fallback) without depending on
  /// Rust vault unlock state.
  Future<bool> verifyPasswordForSensitiveData(String password) async {
    if (_accountManager.selectedAccountId == null) return false;
    if (password.isEmpty) return false;

    SoloLog.d('Auth', 'verifyPasswordForSensitiveData: Starting verification...');
    final result = await _storage.verifyPassword(
      _accountManager.selectedAccountId!,
      password,
    );
    SoloLog.d('Auth', 'verifyPasswordForSensitiveData: result=$result');
    return result;
  }

  /// Lock the vault
  void lockVault() {
    SoloLog.d('Auth', 'Locking vault...');
    _vaultUnlockService.lockVault();
    _profileStorage.clearEncryptionKey();
    state = const AsyncData(AuthState.locked);
  }

  /// Check if vault exists
  Future<bool> vaultExists() async {
    final accounts = await _storage.listAccounts();
    return accounts.isNotEmpty;
  }

  /// Delete the current account
  Future<bool> deleteAccount(String password) async {
    final success = await _accountManager.deleteAccount(password);
    if (success) {
      state = const AsyncData(AuthState.locked);
    }
    return success;
  }

  /// Change master password
  Future<({bool success, String? error})> changePassword({
    required String currentPassword,
    required String newPassword,
    String? newPasswordHint,
  }) async {
    if (_accountManager.selectedAccountId == null) {
      return (success: false, error: 'No account selected');
    }

    return _passwordService.changePassword(
      accountId: _accountManager.selectedAccountId!,
      currentPassword: currentPassword,
      newPassword: newPassword,
      profileStorage: _profileStorage,
      newPasswordHint: newPasswordHint,
    );
  }

  /// Update operation metadata
  Future<void> updateOperation(String operationDesc) async {
    await _accountManager.updateOperation(operationDesc);
  }
}
