import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
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
    ref.watch(authNotifierProvider.select((a) => a.value));
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

  bool _isUnlocking = false;

  /// Unlock vault with master password
  Future<bool> unlockVault(String password) async {
    if (_isUnlocking) {
      SoloLog.w('Auth', 'unlockVault already in progress, skipping');
      return false;
    }
    _isUnlocking = true;

    final accountId = _accountManager.selectedAccountId;
    SoloLog.d('Auth', 'unlockVault start, selectedAccountId=$accountId, pwdNotEmpty=${password.isNotEmpty}');

    if (accountId == null) {
      SoloLog.w('Auth', '_selectedAccountId is null, returning false');
      _isUnlocking = false;
      return false;
    }
    if (password.isEmpty) {
      SoloLog.w('Auth', 'password is empty, returning false');
      state = const AsyncData(AuthState.locked);
      _isUnlocking = false;
      return false;
    }

    state = const AsyncLoading();

    try {
      return await _unlockVaultInner(accountId, password);
    } on Object catch (e, st) {
      // Top-level safety net — never let an unhandled exception silently fail
      SoloLog.e('Auth', 'unlockVault UNHANDLED EXCEPTION', e, st);
      state = const AsyncData(AuthState.locked);
      return false;
    } finally {
      _isUnlocking = false;
    }
  }

  Future<bool> _unlockVaultInner(String accountId, String password) async {
    // Step 1: Unlock Rust vault
    final timer1 = SoloLog.startTimer('Auth', 'RustVaultService.unlockVault');
    final vaultResult = await _vaultUnlockService.unlockVault(
      accountId: accountId,
      password: password,
    );
    SoloLog.endTimer(timer1);
    SoloLog.d('Auth', 'Step1 result: success=${vaultResult.success}, error=${vaultResult.error}, cv=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      SoloLog.e('Auth', 'Step1 FAILED: ${vaultResult.error}');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    // Step 2: Check for migrations needed
    SoloLog.d('Auth', 'Step2: Checking Keychain for migrations...');
    final timer2 = SoloLog.startTimer('Auth', 'Keychain.getAccountData');
    try {
      final accountData = await _storage
          .getAccountData(accountId)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData timed out'),
          );
      SoloLog.endTimer(timer2);
      SoloLog.d('Auth', 'Step2: accountData=${accountData != null ? "found" : "null"}');

      if (accountData == null) {
        SoloLog.d('Auth', 'Step2: Not in Keychain, migrating from Rust...');
        await _migrationService.migrateAccountFromRust(
          accountId: accountId,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
        SoloLog.d('Auth', 'Step2: Migration complete');
      } else if ((accountData['crypto_version'] as int? ?? 1) < 2) {
        SoloLog.d('Auth', 'Step2: V1→V2 migration...');
        await _migrationService.migrateAccountToV2(
          accountId: accountId,
          password: password,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
        SoloLog.d('Auth', 'Step2: V2 migration complete');
      } else {
        SoloLog.d('Auth', 'Step2: No migration needed (cv=${accountData['crypto_version']})');
      }
    } on Object catch (e, st) {
      SoloLog.endTimer(timer2);
      SoloLog.e('Auth', 'Step2: Migration error (non-fatal)', e, st);
    }

    // Step 3: Validate salt availability (session key is managed by Rust)
    SoloLog.d('Auth', 'Step3: Validating salt...');
    final timer3 = SoloLog.startTimer('Auth', 'Salt validation');
    try {
      final freshData = await _storage
          .getAccountData(accountId)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData(fresh) timed out'),
          );
      SoloLog.endTimer(timer3);

      if (freshData == null) {
        SoloLog.w('Auth', 'Step3: freshData=null, trying Rust fallback...');
        final rustConfig = NativeVaultService.instance.getAccountConfig(accountId: accountId);
        SoloLog.d('Auth', 'Step3: rustConfig=${rustConfig != null}, salt=${rustConfig?.salt != null}');

        if (rustConfig?.salt != null) {
          final salt = base64Decode(rustConfig!.salt!);
          SoloLog.d('Auth', 'Step3: Rust salt len=${salt.length}');
          if (salt.length != 32) {
            SoloLog.e('Auth', 'Step3: Bad salt length ${salt.length}');
            state = const AsyncData(AuthState.locked);
            return false;
          }
        } else {
          // Final fallback: read config.json directly from disk
          SoloLog.w('Auth', 'Step3: Rust FFI returned null, trying direct file read...');
          final vaultRoot = RustVaultService.instance.vaultRoot;
          if (vaultRoot == null) {
            SoloLog.e('Auth', 'Step3: vaultRoot is null — cannot read config.json');
            state = const AsyncData(AuthState.locked);
            return false;
          }
          // Validate accountId format to prevent path traversal
          if (!RegExp(r'^acc_[a-f0-9\-]{36}$').hasMatch(accountId)) {
            SoloLog.e('Auth', 'Step3: Invalid accountId format, possible path traversal attempt');
            state = const AsyncData(AuthState.locked);
            return false;
          }
          final configFile = File('$vaultRoot/$accountId/config.json');
          if (await configFile.exists()) {
            final configJson = jsonDecode(await configFile.readAsString()) as Map<String, dynamic>;
            final saltStr = configJson['salt'] as String?;
            SoloLog.d('Auth', 'Step3: File config.json salt present=${saltStr != null}');
            if (saltStr != null) {
              final salt = base64Decode(saltStr);
              SoloLog.d('Auth', 'Step3: File salt len=${salt.length}');
              if (salt.length == 32) {
                // Migrate to Keychain for future use
                SoloLog.d('Auth', 'Step3: Migrating file config to Keychain...');
                await _storage.saveAccountData(accountId, {
                  'salt': saltStr,
                  'verify_hash': configJson['verify_hash'] as String? ?? '',
                  'crypto_version': configJson['crypto_version'] as int? ?? 2,
                });
              } else {
                SoloLog.e('Auth', 'Step3: File salt bad length ${salt.length}');
                state = const AsyncData(AuthState.locked);
                return false;
              }
            } else {
              SoloLog.e('Auth', 'Step3: No salt in config.json');
              state = const AsyncData(AuthState.locked);
              return false;
            }
          } else {
            SoloLog.e('Auth', 'Step3: config.json does not exist at ${configFile.path}');
            state = const AsyncData(AuthState.locked);
            return false;
          }
        }
      } else {
        final saltStr = freshData['salt'] as String?;
        SoloLog.d('Auth', 'Step3: Keychain salt present=${saltStr != null}');
        if (saltStr != null) {
          final salt = base64Decode(saltStr);
          SoloLog.d('Auth', 'Step3: Keychain salt len=${salt.length}');
        }
      }
    } on Object catch (e, st) {
      SoloLog.endTimer(timer3);
      SoloLog.e('Auth', 'Step3: Salt validation error (non-fatal)', e, st);
    }

    SoloLog.d('Auth', 'UNLOCK SUCCESS — vault is unlocked, proceeding to home');
    state = const AsyncData(AuthState.unlocked);

    // 自动备份：解锁成功后异步创建加密备份（不阻塞登录流程）
    _autoBackupAfterUnlock();

    // 升级保护备份：若检测到 App 版本变化，额外创建一份带版本号的备份
    _upgradeBackupIfNeeded(accountId: _accountManager.selectedAccountId!);

    return true;
  }

  /// Unlock vault with biometric authentication
  /// Uses pre-derived session key from BiometricCredentialService.
  Future<bool> unlockVaultWithBiometric() async {
    SoloLog.d('Auth', 'unlockVaultWithBiometric start, selectedAccountId=${_accountManager.selectedAccountId}');

    if (_accountManager.selectedAccountId == null) {
      SoloLog.w('Auth', '_selectedAccountId is null, returning false');
      return false;
    }

    state = const AsyncLoading();

    // Step 1: Retrieve session key from biometric credential
    final timer1 = SoloLog.startTimer('Auth', 'BiometricCredentialService.unlockWithBiometric');
    final sessionKey = await BiometricCredentialService.instance.unlockWithBiometric(
      _accountManager.selectedAccountId!,
    );
    SoloLog.endTimer(timer1);

    if (sessionKey == null || sessionKey.length != 32) {
      SoloLog.e('Auth', 'Failed to retrieve session key from biometric credential');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    // Step 2: Unlock vault with session key
    final timer2 = SoloLog.startTimer('Auth', 'VaultUnlockService.unlockVaultWithKey');
    final vaultResult = await _vaultUnlockService.unlockVaultWithKey(
      accountId: _accountManager.selectedAccountId!,
      sessionKey: sessionKey,
    );
    SoloLog.endTimer(timer2);
    SoloLog.d('Auth', 'Biometric unlock result: success=${vaultResult.success}, error=${vaultResult.error}, cryptoVersion=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      SoloLog.e('Auth', 'Biometric vault unlock failed: ${vaultResult.error}');
      _secureWipe(sessionKey);
      state = const AsyncData(AuthState.locked);
      return false;
    }

    SoloLog.d('Auth', 'Biometric unlock succeeded, checking Keychain...');

    // Step 3: Check for migrations needed (Rust→Keychain only, no V2 migration)
    final timer3 = SoloLog.startTimer('Auth', 'Keychain.getAccountData');
    try {
      final accountData = await _storage
          .getAccountData(_accountManager.selectedAccountId!)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData timed out'),
          );
      SoloLog.endTimer(timer3);

      if (accountData == null) {
        SoloLog.d('Auth', 'Account not in Keychain, migrating from Rust...');
        await _migrationService.migrateAccountFromRust(
          accountId: _accountManager.selectedAccountId!,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
      } else if ((accountData['crypto_version'] as int? ?? 1) < 2) {
        // V2 migration requires password re-derivation; biometric credential
        // should only exist after at least one password unlock. Log warning.
        SoloLog.w('Auth', 'V1 account detected but biometric unlock cannot perform V2 migration without password. Consider unlocking with password once.');
      }
    } on Object catch (e, st) {
      SoloLog.e('Auth', 'Migration error during biometric unlock', e, st);
    }

    // Step 4: Session key is now managed by Rust — no need to set on Dart side
    _secureWipe(sessionKey);

    state = const AsyncData(AuthState.unlocked);
    SoloLog.d('Auth', 'Vault unlocked with biometric successfully!');

    _autoBackupAfterUnlock();
    _upgradeBackupIfNeeded(accountId: _accountManager.selectedAccountId!);

    return true;
  }

  /// Best-effort secure wipe of a byte buffer.
  void _secureWipe(Uint8List buffer) {
    for (var i = 0; i < buffer.length; i++) {
      buffer[i] = 0;
    }
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
  Future<void> lockVault() async {
    SoloLog.d('Auth', 'Locking vault...');
    await _vaultUnlockService.lockVault();
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
