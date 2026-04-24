import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

// ============================================================================
// VaultUnlockService - Rust FFI unlock/lock operations
// ============================================================================

/// Service for Rust vault unlock/lock operations
class VaultUnlockService {
  const VaultUnlockService();

  /// Unlock vault with accountId and password
  ({bool success, String? error, int? cryptoVersion}) unlockVault({
    required String accountId,
    required String password,
  }) {
    return RustVaultService.instance.unlockVault(
      accountId: accountId,
      password: password,
    );
  }

  /// Lock the vault
  void lockVault() {
    RustVaultService.instance.lockVault();
  }

  /// Check if vault exists (has accounts)
  bool vaultExists(List<AccountInfo> accounts) {
    return accounts.isNotEmpty;
  }
}

// ============================================================================
// MigrationService - V1→V2 and Rust→Keychain migrations
// ============================================================================

/// Service for account migrations (V1→V2, Rust→Keychain)
class MigrationService {
  final SecureAccountStorage _storage;

  const MigrationService(this._storage);

  /// Migrate a V1 account to V2 crypto
  Future<void> migrateAccountToV2({
    required String accountId,
    required String password,
    required int cryptoVersion,
  }) async {
    try {
      final salt = NativeCryptoService.instance.generateSalt();
      if (salt == null) return;

      // Step 1: Derive master_key from password (same as Rust)
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (masterKey == null) return;

      // Step 2: Hex-encode master_key and use as password for verify derivation (same as Rust)
      final masterKeyHex = bytesToHex(masterKey);
      const verifyData = 'SOLOSOUL_VAULT_VERIFY_v1';
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode(verifyData)),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      );
      if (verifyKey == null) return;

      await _storage.updateAccountSalt(accountId, salt, verifyKey);

      // Clear sensitive data from memory
      for (var i = 0; i < salt.length; i++) {
        salt[i] = 0;
      }
      for (var i = 0; i < masterKey.length; i++) {
        masterKey[i] = 0;
      }
      for (var i = 0; i < verifyKey.length; i++) {
        verifyKey[i] = 0;
      }

      // Update crypto version marker with retry logic
      bool versionUpdated =
          await _storage.updateAccountCryptoVersion(accountId, cryptoVersion);
      if (!versionUpdated) {
        await Future.delayed(const Duration(milliseconds: 100));
        versionUpdated =
            await _storage.updateAccountCryptoVersion(accountId, cryptoVersion);
      }
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('AUTH', 'Migration error: $e\nStack trace: $st');
    }
  }

  /// Migrate account from Rust to Keychain
  Future<void> migrateAccountFromRust({
    required String accountId,
    required int cryptoVersion,
  }) async {
    try {
      DebugLogger.instance
          .logInfo('AUTH', 'Migrating account from Rust, calling getAccountConfig...');
      final rustConfig = await Future.delayed(
        Duration.zero,
        () => NativeVaultService.instance.getAccountConfig(accountId: accountId),
      ).timeout(
        const Duration(seconds: 5),
        onTimeout: () => throw TimeoutException('getAccountConfig timed out'),
      );
      DebugLogger.instance
          .logInfo('AUTH', 'getAccountConfig returned: ${rustConfig != null}');

      if (rustConfig == null ||
          rustConfig.salt == null ||
          rustConfig.verifyHash == null) {
        try {
          await _storage
              .updateAccountCryptoVersion(accountId, cryptoVersion)
              .timeout(
                const Duration(seconds: 5),
                onTimeout: () =>
                    throw TimeoutException('updateAccountCryptoVersion timed out'),
              );
        } on Exception catch (e, st) {
          DebugLogger.instance.logError('AUTH', 'Failed to update crypto version: $e\nStack trace: $st');
        }
        return;
      }

      final saltBytes = base64Decode(rustConfig.salt!);
      final verifyHashBytes = hexToBytes(rustConfig.verifyHash!);

      final accounts = await _storage.listAccounts().timeout(
            const Duration(seconds: 5),
            onTimeout: () => throw TimeoutException('listAccounts timed out'),
          );
      final existingAccount = accounts.cast<AccountInfo?>().firstWhere(
            (a) => a?.id == accountId,
            orElse: () => null,
          );

      if (existingAccount == null) {
        try {
          await _storage
              .saveAccountData(accountId, {
                'salt': rustConfig.salt,
                'verify_hash': rustConfig.verifyHash,
                'crypto_version': cryptoVersion,
              })
              .timeout(
                const Duration(seconds: 5),
                onTimeout: () =>
                    throw TimeoutException('saveAccountData timed out'),
              );
        } on Exception catch (e, st) {
          DebugLogger.instance
              .logError('AUTH', 'Failed to save new account data during migration: $e\nStack trace: $st');
        }
      } else {
        try {
          await _storage
              .updateAccountSalt(
                accountId,
                Uint8List.fromList(saltBytes),
                Uint8List.fromList(verifyHashBytes),
              )
              .timeout(
                const Duration(seconds: 5),
                onTimeout: () =>
                    throw TimeoutException('updateAccountSalt timed out'),
              );
        } on Exception catch (e, st) {
          DebugLogger.instance
              .logError('AUTH', 'Failed to update account salt during migration: $e\nStack trace: $st');
        }
        try {
          await _storage
              .updateAccountCryptoVersion(accountId, cryptoVersion)
              .timeout(
                const Duration(seconds: 5),
                onTimeout: () =>
                    throw TimeoutException('updateAccountCryptoVersion timed out'),
              );
        } on Exception catch (e, st) {
          DebugLogger.instance
              .logError('AUTH', 'Failed to update crypto version during migration: $e\nStack trace: $st');
        }
      }
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('AUTH', 'Migration error: $e\nStack trace: $st');
    }
  }
}

// ============================================================================
// PasswordService - Password change flow
// ============================================================================

/// Service for password modification operations
class PasswordService {
  final SecureAccountStorage _storage;

  const PasswordService(this._storage);

  /// Change master password for current account
  /// 1. Verify current password
  /// 2. Call Rust's change_password
  /// 3. Update Keychain with new salt/verify_hash
  /// 4. Derive new session key
  Future<({bool success, String? error})> changePassword({
    required String accountId,
    required String currentPassword,
    required String newPassword,
    required ProfileStorageService profileStorage,
    String? newPasswordHint,
  }) async {
    // Step 1: Verify current password via Dart-side storage
    // (consistent with verifyPasswordForSensitiveData, avoids Rust vault state issues)
    final verifyResult = await _storage.verifyPassword(accountId, currentPassword);
    if (!verifyResult) {
      return (success: false, error: 'Invalid current password');
    }

    // Step 2: Load profile data with current encryption key BEFORE password change
    final currentProfile = await profileStorage.loadProfile(accountId);

    // Step 3: Call Rust to change password (updates Rust's config.json)
    final rustResult = NativeVaultService.instance.changePassword(
      accountId: accountId,
      oldPassword: currentPassword,
      newPassword: newPassword,
    );

    if (rustResult == null || !rustResult.success) {
      return (
        success: false,
        error: rustResult?.error ?? 'Failed to change password in vault'
      );
    }

    // Step 4: Update Dart's Keychain with new salt/verify_hash from Rust
    Uint8List saltBytes;
    if (rustResult.salt != null && rustResult.verifyHash != null) {
      saltBytes = base64Decode(rustResult.salt!);
      final verifyHashBytes = base64Decode(rustResult.verifyHash!);
      await _storage.updateAccountSalt(
        accountId,
        saltBytes,
        Uint8List.fromList(verifyHashBytes),
      );
    } else {
      return (
        success: false,
        error: 'Failed to get new credentials from vault'
      );
    }

    // Step 5: Derive new session key from new password
    final newSessionKey = NativeCryptoService.instance.deriveKey(
      password: newPassword,
      salt: saltBytes,
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );
    if (newSessionKey == null) {
      return (success: false, error: 'Failed to derive new session key');
    }

    // Step 6: Update encryption key and re-save profile if it exists
    profileStorage.setEncryptionKey(newSessionKey);
    if (currentProfile != null) {
      await profileStorage.saveProfile(accountId, currentProfile);
    }

    // Step 7: Update password hint if provided
    if (newPasswordHint != null) {
      await _storage.updatePasswordHint(accountId, newPasswordHint);
    }

    return (success: true, error: null);
  }
}

// ============================================================================
// AccountManager - Account CRUD operations
// ============================================================================

/// Service for account CRUD operations
class AccountManager {
  final SecureAccountStorage _storage;
  final ProfileStorageService _profileStorage;

  String? _selectedAccountId;
  AccountInfo? _selectedAccountInfo;
  int _accountsVersion = 0;

  AccountManager(this._storage, this._profileStorage);

  String? get selectedAccountId => _selectedAccountId;
  AccountInfo? get selectedAccount => _selectedAccountInfo;
  int get accountsVersion => _accountsVersion;

  /// Get all accounts sorted by most recent access
  Future<List<AccountInfo>> getAccountsSortedByRecent() async {
    final rustAccounts = RustVaultService.instance.listAccountsFromRust();
    List<AccountInfo> accounts;

    if (rustAccounts != null && rustAccounts.isNotEmpty) {
      final rustMappedAccounts = rustAccounts
          .map((r) => AccountInfo(
                id: r['id'] as String? ?? '',
                name: r['name'] as String? ?? '',
                lastAccessed: r['last_accessed'] != null
                    ? DateTime.tryParse(r['last_accessed'] as String)
                    : null,
                createdAt: r['created_at'] != null
                    ? DateTime.tryParse(r['created_at'] as String)
                    : null,
              ))
          .toList();

      final storageAccounts = await _storage.listAccounts();
      final storageById = {for (final a in storageAccounts) a.id: a};

      accounts = rustMappedAccounts.map((rustAccount) {
        final storageAccount = storageById[rustAccount.id];
        if (storageAccount != null) {
          return rustAccount.copyWith(
            createdAt: rustAccount.createdAt ?? storageAccount.createdAt,
            lastLoginAt: storageAccount.lastLoginAt,
            lastOperationAt: storageAccount.lastOperationAt,
            lastOperationDesc: storageAccount.lastOperationDesc,
            recentDevices: storageAccount.recentDevices,
          );
        }
        return rustAccount;
      }).toList();
    } else {
      accounts = await _storage.listAccounts();
    }

    accounts.sort((a, b) {
      if (a.lastAccessed == null && b.lastAccessed == null) return 0;
      if (a.lastAccessed == null) return 1;
      if (b.lastAccessed == null) return -1;
      return b.lastAccessed!.compareTo(a.lastAccessed!);
    });
    return accounts;
  }

  /// Get all accounts
  Future<List<AccountInfo>> getAccounts() async {
    return _storage.listAccounts();
  }

  /// Select an account
  Future<void> selectAccount(String? accountId) async {
    _selectedAccountId = accountId;
    if (accountId != null) {
      final accounts = await _storage.listAccounts();
      _selectedAccountInfo = accounts.cast<AccountInfo?>().firstWhere(
            (a) => a?.id == accountId,
            orElse: () => null,
          );
    } else {
      _selectedAccountInfo = null;
    }
    _accountsVersion++;
  }

  /// Create a new account
  Future<({bool success, String? error})> createAccount(
    String name,
    String password, {
    String? passwordHint,
  }) async {
    DebugLogger.instance.logInfo('AUTH', 'CHECKPOINT: createAccount start');

    // First create account in Rust vault
    DebugLogger.instance
        .logInfo('AUTH', 'CHECKPOINT: calling RustVaultService.createAccount');
    final vaultResult = RustVaultService.instance.createAccount(
      name: name,
      password: password,
    );
    DebugLogger.instance.logInfo(
        'AUTH',
        'CHECKPOINT: RustVaultService.createAccount returned, '
        'success=${vaultResult.success}');

    if (!vaultResult.success) {
      DebugLogger.instance
          .logInfo('AUTH', 'CHECKPOINT: vaultResult failed, returning error');
      return (success: false, error: vaultResult.error ?? 'Failed to create vault account');
    }

    // Also create account in SecureAccountStorage (Dart Keychain)
    // If Keychain fails, we can still use Rust-only mode
    DebugLogger.instance
        .logInfo('AUTH', 'CHECKPOINT: calling _storage.createAccount');
    try {
      final result = await _storage.createAccount(
        name,
        password,
        passwordHint: passwordHint,
        accountId: vaultResult.accountId,
        salt: vaultResult.salt,
        verifyHashFromRust: vaultResult.verifyHash,
      );
      DebugLogger.instance.logInfo(
          'AUTH',
          'CHECKPOINT: _storage.createAccount returned, success=${result.success}');

      if (result.success && result.account != null && result.sessionKey != null) {
        _selectedAccountId = result.account!.id;
        _selectedAccountInfo = result.account;
        _profileStorage.setEncryptionKey(result.sessionKey!);
        _accountsVersion++;
        return (success: true, error: null);
      } else if (result.error != null) {
        return (success: false, error: result.error);
      }
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('AUTH', 'Keychain createAccount failed, using Rust-only mode: $e\nStack: $st');
    }

    // If Keychain failed, we still have Rust account - derive session key from Rust data
    DebugLogger.instance.logInfo('AUTH', 'Using Rust-only mode (Keychain unavailable)');
    final salt = base64Decode(vaultResult.salt!);
    final sessionKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: salt,
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );
    if (sessionKey == null) {
      return (success: false, error: 'Failed to derive session key');
    }

    _selectedAccountId = vaultResult.accountId;
    _selectedAccountInfo = AccountInfo(
      id: vaultResult.accountId!,
      name: name,
      passwordHint: passwordHint,
      lastAccessed: DateTime.now(),
      createdAt: DateTime.now(),
    );
    _profileStorage.setEncryptionKey(sessionKey);
    _accountsVersion++;
    return (success: true, error: null);
  }

  /// Delete the current account
  Future<bool> deleteAccount(String password) async {
    if (_selectedAccountId == null) return false;

    // Verify password using Dart-side verification (consistent with
    // verifyPasswordForSensitiveData, bypasses Rust vault state issues)
    final verifyResult = await _storage.verifyPassword(_selectedAccountId!, password);
    if (!verifyResult) return false;

    final rustDeleted = RustVaultService.instance.deleteAccount(_selectedAccountId!);

    // Clean up Keychain if possible, but don't fail if Keychain is unavailable
    // Rust is the source of truth for account data
    await _storage.deleteAccount(_selectedAccountId!);

    if (rustDeleted) {
      _profileStorage.clearEncryptionKey();
      _selectedAccountId = null;
      _selectedAccountInfo = null;
      _accountsVersion++;
    }
    return rustDeleted;
  }

  /// Update operation metadata
  Future<void> updateOperation(String operationDesc) async {
    if (_selectedAccountId == null) return;
    await _storage.updateAccountOperation(_selectedAccountId!, operationDesc);
  }
}
