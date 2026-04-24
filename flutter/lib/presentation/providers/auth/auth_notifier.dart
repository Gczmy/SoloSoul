import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
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
          const VaultUnlockService(),
        ),
        _accountManager = AccountManager(
          storage ?? SecureAccountStorage.instance,
          profileStorage ?? ProfileStorageService.instance,
        );

  // Convenience getters delegating to services
  String? get selectedAccountId => _accountManager.selectedAccountId;
  AccountInfo? get selectedAccount => _accountManager.selectedAccount;
  bool get isUnlocked => state.valueOrNull == AuthState.unlocked;
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
    state = AsyncData(state.valueOrNull ?? AuthState.locked);
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
    DebugLogger.instance
        .logInfo('AUTH', 'unlockVault start, selectedAccountId=${_accountManager.selectedAccountId}');

    if (_accountManager.selectedAccountId == null) {
      DebugLogger.instance
          .logInfo('AUTH', 'CHECKPOINT: _selectedAccountId is null, returning false');
      return false;
    }
    if (password.isEmpty) {
      DebugLogger.instance
          .logInfo('AUTH', 'CHECKPOINT: password is empty, returning false');
      state = const AsyncData(AuthState.locked);
      return false;
    }

    state = const AsyncLoading();

    // Step 1: Unlock Rust vault
    DebugLogger.instance
        .logInfo('AUTH', 'CHECKPOINT: calling RustVaultService.unlockVault');
    final vaultResult = _vaultUnlockService.unlockVault(
      accountId: _accountManager.selectedAccountId!,
      password: password,
    );
    DebugLogger.instance.logInfo(
        'AUTH',
        'CHECKPOINT: RustVaultService.unlockVault returned, '
        'success=${vaultResult.success}');

    DebugLogger.instance.logInfo(
        'AUTH',
        'Rust unlock result: success=${vaultResult.success}, '
        'error=${vaultResult.error}, cryptoVersion=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      state = const AsyncData(AuthState.locked);
      return false;
    }

    DebugLogger.instance.logInfo('AUTH', 'Rust unlock succeeded, checking Keychain...');

    // Step 2: Check for migrations needed
    try {
      final accountData = await _storage
          .getAccountData(_accountManager.selectedAccountId!)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('getAccountData timed out'),
          );
      DebugLogger.instance
          .logInfo('AUTH', 'Keychain accountData: ${accountData != null ? "found" : "null"}');

      if (accountData == null) {
        DebugLogger.instance
            .logInfo('AUTH', 'Account not in Keychain, migrating from Rust...');
        await _migrationService.migrateAccountFromRust(
          accountId: _accountManager.selectedAccountId!,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
      } else if ((accountData['crypto_version'] as int? ?? 1) < 2) {
        DebugLogger.instance
            .logInfo('AUTH', 'V1 account detected, migrating to V2...');
        await _migrationService.migrateAccountToV2(
          accountId: _accountManager.selectedAccountId!,
          password: password,
          cryptoVersion: vaultResult.cryptoVersion ?? 2,
        );
      }
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('AUTH', 'Migration error: $e\nStack trace: $st');
    }

    // Step 3: Get session key for profile encryption
    DebugLogger.instance.logInfo('AUTH', 'Getting fresh account data after migration...');
    final freshData = await _storage
        .getAccountData(_accountManager.selectedAccountId!)
        .timeout(
          const Duration(seconds: 5),
          onTimeout: () => throw TimeoutException('getAccountData timed out'),
        );
    DebugLogger.instance
        .logInfo('AUTH', 'freshData: ${freshData != null ? "found" : "null"}');

    Uint8List salt;
    if (freshData == null) {
      DebugLogger.instance
          .logInfo('AUTH', 'freshData is null, getting salt from Rust...');
      final rustConfig = await Future.delayed(
        Duration.zero,
        () => NativeVaultService.instance.getAccountConfig(
            accountId: _accountManager.selectedAccountId!),
      )
          .timeout(const Duration(seconds: 5),
              onTimeout: () => throw TimeoutException('getAccountConfig timed out'));
      if (rustConfig?.salt == null) {
        DebugLogger.instance
            .logError('AUTH', 'Cannot get salt from Rust - returning false');
        state = const AsyncData(AuthState.locked);
        return false;
      }
      salt = base64Decode(rustConfig!.salt!);
    } else {
      salt = base64Decode(freshData['salt'] as String);
    }

    final sessionKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );

    if (sessionKey == null) {
      state = const AsyncData(AuthState.locked);
      return false;
    }

    _profileStorage.setEncryptionKey(sessionKey);

    state = const AsyncData(AuthState.unlocked);

    return true;
  }

  /// Verify password for sensitive data access
  /// Uses Rust unlockVault for verification since it handles all crypto properly
  Future<bool> verifyPasswordForSensitiveData(String password) async {
    if (_accountManager.selectedAccountId == null) return false;
    if (password.isEmpty) return false;

    // Use Rust unlockVault - it does its own password verification and handles
    // all crypto correctly, including the salt length issues we see with Keychain
    final rustResult = _vaultUnlockService.unlockVault(
      accountId: _accountManager.selectedAccountId!,
      password: password,
    );
    DebugLogger.instance.logInfo('AUTH', 'verifyPasswordForSensitiveData: rustResult=${rustResult.success}');
    return rustResult.success;
  }

  /// Lock the vault
  void lockVault() {
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
