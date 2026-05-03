import 'dart:async';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
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
    // Force Riverpod to detect a state change by toggling through loading.
    // Setting AsyncData(same value) is deduplicated and doesn't trigger rebuilds.
    final current = state.value ?? AuthState.locked;
    state = const AsyncLoading();
    state = AsyncData(current);
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
      await updateOperation('Created account');
      return (success: true, error: null);
    } else {
      state = const AsyncData(AuthState.locked);
      return (success: false, error: result.error);
    }
  }

  bool _isUnlocking = false;
  String? _lastUnlockError;

  /// Human-readable error from the last unlock attempt.
  /// Null means no error (or no unlock attempted yet).
  String? get lastUnlockError => _lastUnlockError;

  /// Unlock vault with master password
  Future<bool> unlockVault(String password) async {
    if (_isUnlocking) {
      SoloLog.w('Auth', 'unlockVault already in progress, skipping');
      return false;
    }
    _isUnlocking = true;
    _lastUnlockError = null;

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
      // ignore: avoid_print
      print('[UNLOCK-DEBUG] UNHANDLED EXCEPTION: $e');
      SoloLog.e('Auth', 'unlockVault UNHANDLED EXCEPTION', e, st);
      _lastUnlockError = 'Internal error: $e';
      state = const AsyncData(AuthState.locked);
      return false;
    } finally {
      _isUnlocking = false;
    }
  }

  Future<bool> _unlockVaultInner(String accountId, String password) async {
    // Step 1: Unlock Rust vault
    // ignore: avoid_print
    print('[UNLOCK-DEBUG] Step1: calling RustVaultService.unlockVault for $accountId');
    final timer1 = SoloLog.startTimer('Auth', 'RustVaultService.unlockVault');
    final vaultResult = await _vaultUnlockService.unlockVault(
      accountId: accountId,
      password: password,
    );
    SoloLog.endTimer(timer1);
    // ignore: avoid_print
    print('[UNLOCK-DEBUG] Step1 result: success=${vaultResult.success}, error=${vaultResult.error}, cv=${vaultResult.cryptoVersion}');
    SoloLog.d('Auth', 'Step1 result: success=${vaultResult.success}, error=${vaultResult.error}, cv=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      // ignore: avoid_print
      print('[UNLOCK-DEBUG] Step1 FAILED: ${vaultResult.error}');
      SoloLog.e('Auth', 'Step1 FAILED: ${vaultResult.error}');
      _lastUnlockError = vaultResult.error ?? 'Invalid password';
      state = const AsyncData(AuthState.locked);
      return false;
    }

    // Step 2: Non-critical post-unlock tasks (migration, Keychain sync).
    // These run asynchronously and NEVER block the unlock flow.
    // The vault is already open — if these fail, the user can still use the app.
    // ignore: avoid_print
    print('[UNLOCK-DEBUG] Step2: Scheduling post-unlock tasks (non-blocking)');
    SoloLog.d('Auth', 'Step2: Scheduling post-unlock tasks (non-blocking)');

    // Fire-and-forget: try to sync account data to Keychain in background
    unawaited(_postUnlockSync(accountId, vaultResult.cryptoVersion ?? 2).catchError((Object e) {
      // ignore: avoid_print
      print('[UNLOCK-DEBUG] Step2: post-unlock sync error (ignored): $e');
    }));

    // Step 3: Validate salt availability (session key is managed by Rust).
    // The vault is already unlocked (Step 1 succeeded), so salt is valid.
    // Just log that we're good — no need to re-validate.
    // ignore: avoid_print
    print('[UNLOCK-DEBUG] Step3: Vault already unlocked, salt validated by Rust');
    SoloLog.d('Auth', 'Step3: Vault already unlocked, salt validated by Rust');

    // ignore: avoid_print
    print('[UNLOCK-DEBUG] UNLOCK SUCCESS — vault is unlocked');
    SoloLog.d('Auth', 'UNLOCK SUCCESS — vault is unlocked, proceeding to home');
    state = const AsyncData(AuthState.unlocked);

    // 自动备份：解锁成功后异步创建加密备份（不阻塞登录流程）
    _autoBackupAfterUnlock();

    // 升级保护备份：若检测到 App 版本变化，额外创建一份带版本号的备份
    _upgradeBackupIfNeeded(accountId: _accountManager.selectedAccountId!);

    return true;
  }

  /// Post-unlock background sync: ensure Keychain has account data.
  /// Runs asynchronously — never blocks the unlock flow.
  Future<void> _postUnlockSync(String accountId, int cryptoVersion) async {
    try {
      // Try Rust config first (synchronous, fast)
      final rustCfg = NativeVaultService.instance.getAccountConfig(accountId: accountId);
      if (rustCfg?.salt != null && rustCfg?.verifyHash != null) {
        // Sync to Keychain in background
        await _storage.saveAccountData(accountId, {
          'salt': rustCfg!.salt,
          'verify_hash': rustCfg.verifyHash,
          'crypto_version': cryptoVersion,
        });
        // ignore: avoid_print
        print('[UNLOCK-DEBUG] postUnlockSync: Synced account data to Keychain from Rust config');
      }
    } on Object catch (e) {
      // ignore: avoid_print
      print('[UNLOCK-DEBUG] postUnlockSync error (ignored): $e');
    }
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
      await updateOperation('Deleted account');
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

    final result = await _passwordService.changePassword(
      accountId: _accountManager.selectedAccountId!,
      currentPassword: currentPassword,
      newPassword: newPassword,
      profileStorage: _profileStorage,
      newPasswordHint: newPasswordHint,
    );
    if (result.success) {
      await updateOperation('Changed password');
    }
    return result;
  }

  /// Update operation metadata
  Future<void> updateOperation(String operationDesc) async {
    await _accountManager.updateOperation(operationDesc);
  }

  /// Update account metadata and bump accounts version to trigger UI rebuild.
  Future<void> updateAccountMetadata({
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    Map<String, dynamic>? device,
  }) async {
    final accountId = _accountManager.selectedAccountId;
    if (accountId == null) return;
    await _storage.updateAccountMetadata(
      accountId,
      lastLoginAt: lastLoginAt,
      lastOperationAt: lastOperationAt,
      lastOperationDesc: lastOperationDesc,
      device: device,
    );
    _accountManager.bumpAccountsVersion();
  }
}
