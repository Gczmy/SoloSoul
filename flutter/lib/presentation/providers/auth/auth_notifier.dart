import 'dart:async';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
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
  final PasswordService _passwordService;
  final AccountManager _accountManager;

  AuthNotifier({
    SecureAccountStorage? storage,
    ProfileStorageService? profileStorage,
  })  : _storage = storage ?? SecureAccountStorage.instance,
        _profileStorage = profileStorage ?? ProfileStorageService.instance,
        _vaultUnlockService = const VaultUnlockService(),
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
    final timer1 = SoloLog.startTimer('Auth', 'RustVaultService.unlockVault');
    final vaultResult = await _vaultUnlockService.unlockVault(
      accountId: accountId,
      password: password,
    );
    SoloLog.endTimer(timer1);
    SoloLog.d('Auth', 'Step1 result: success=${vaultResult.success}, error=${vaultResult.error}, cv=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      SoloLog.e('Auth', 'Step1 FAILED: ${vaultResult.error}');
      _lastUnlockError = vaultResult.error ?? 'Invalid password';
      state = const AsyncData(AuthState.locked);
      return false;
    }

    // Step 2: Non-critical post-unlock tasks (migration, Keychain sync).
    // These run asynchronously and NEVER block the unlock flow.
    // The vault is already open — if these fail, the user can still use the app.
    SoloLog.d('Auth', 'Step2: Scheduling post-unlock tasks (non-blocking)');

    // Step 3: Validate salt availability (session key is managed by Rust).
    // The vault is already unlocked (Step 1 succeeded), so salt is valid.
    // Just log that we're good — no need to re-validate.
    SoloLog.d('Auth', 'Step3: Vault already unlocked, salt validated by Rust');

    SoloLog.d('Auth', 'UNLOCK SUCCESS — vault is unlocked, proceeding to home');
    // Reset brute-force tracker on successful login
    _storage.resetAttemptTracker(accountId);
    state = const AsyncData(AuthState.unlocked);

    // 自动备份：解锁成功后异步创建加密备份（不阻塞登录流程）
    _autoBackupAfterUnlock();

    // 升级保护备份：若检测到 App 版本变化，额外创建一份带版本号的备份
    _upgradeBackupIfNeeded(accountId: accountId);

    return true;
  }

  /// Unlock vault with biometric authentication
  /// Uses pre-derived session key from BiometricCredentialService.
  Future<bool> unlockVaultWithBiometric() async {
    final accountId = _accountManager.selectedAccountId;
    SoloLog.d('Auth', 'unlockVaultWithBiometric start, selectedAccountId=$accountId');

    if (accountId == null) {
      SoloLog.w('Auth', '_selectedAccountId is null, returning false');
      return false;
    }

    state = const AsyncLoading();

    // Step 1: Retrieve session key from biometric credential
    final timer1 = SoloLog.startTimer('Auth', 'BiometricCredentialService.unlockWithBiometric');
    final sessionKey = await BiometricCredentialService.instance.unlockWithBiometric(
      accountId,
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
      accountId: accountId,
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

    SoloLog.d('Auth', 'Biometric unlock succeeded');

    // Step 3: Verify vault is actually unlocked via C FFI
    final isUnlocked = NativeVaultService.instance.isVaultUnlocked();
    SoloLog.d('Auth', 'Vault unlocked check after biometric: isUnlocked=$isUnlocked');
    if (!isUnlocked) {
      SoloLog.e('Auth', 'Vault reported unlocked but isVaultUnlocked() returns false');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    // Step 4: Session key is now managed by Rust — no need to set on Dart side
    _secureWipe(sessionKey);

    state = const AsyncData(AuthState.unlocked);
    // Reset brute-force tracker on successful biometric login
    _storage.resetAttemptTracker(accountId);
    SoloLog.d('Auth', 'Vault unlocked with biometric successfully!');

    _autoBackupAfterUnlock();
    _upgradeBackupIfNeeded(accountId: accountId);

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
    final accountId = _accountManager.selectedAccountId;
    if (accountId == null) return false;
    if (password.isEmpty) return false;

    SoloLog.d('Auth', 'verifyPasswordForSensitiveData: Starting verification...');
    // Re-throw backoff exceptions so the UI layer can show a countdown
    final result = await _storage.verifyPassword(
      accountId,
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
    final accountId = _accountManager.selectedAccountId;
    if (accountId == null) {
      return (success: false, error: 'No account selected');
    }

    final result = await _passwordService.changePassword(
      accountId: accountId,
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

  /// Update only the password hint (no password change).
  Future<({bool success, String? error})> updatePasswordHintOnly({
    required String currentPassword,
    required String newPasswordHint,
  }) async {
    final accountId = _accountManager.selectedAccountId;
    if (accountId == null) {
      return (success: false, error: 'No account selected');
    }

    final result = await _passwordService.updatePasswordHintOnly(
      accountId: accountId,
      currentPassword: currentPassword,
      newPasswordHint: newPasswordHint,
    );
    if (result.success) {
      await updateOperation('Updated password hint');
    }
    return result;
  }

  /// Update operation metadata and bump accounts version to trigger UI refresh.
  Future<void> updateOperation(String operationDesc) async {
    await _accountManager.updateOperation(operationDesc);
    _accountManager.bumpAccountsVersion();
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
