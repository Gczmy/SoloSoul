import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

// ============================================================================
// VaultUnlockService - Rust FFI unlock/lock operations
// ============================================================================

/// Service for Rust vault unlock/lock operations
class VaultUnlockService {
  const VaultUnlockService();

  /// Unlock vault with accountId and password
  Future<({bool success, String? error, int? cryptoVersion})> unlockVault({
    required String accountId,
    required String password,
  }) async {
    return RustVaultService.instance.unlockVault(
      accountId: accountId,
      password: password,
    );
  }

  /// Unlock vault with a pre-derived session key (for biometric unlock)
  Future<({bool success, String? error, int? cryptoVersion})> unlockVaultWithKey({
    required String accountId,
    required Uint8List sessionKey,
  }) async {
    return RustVaultService.instance.unlockVaultWithKey(
      accountId: accountId,
      sessionKey: sessionKey,
    );
  }

  /// Lock the vault
  Future<void> lockVault() async {
    await RustVaultService.instance.lockVault();
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
      final salt = await frb.frbGenerateSalt(length: 32);

      // Step 1: Derive master_key from password (same as Rust)
      final masterKey = await frb.frbDeriveKey(
        password: password,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );

      // Step 2: Hex-encode master_key and use as password for verify derivation (same as Rust)
      final masterKeyHex = bytesToHex(masterKey);
      const verifyData = 'SOLOSOUL_VAULT_VERIFY_v1';
      final verifyKey = await frb.frbDeriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode(verifyData)),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      );

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
      SoloLog.d('AUTH', 'Migrating account from Rust, calling getAccountConfig...');
      final rustConfig = await Future.delayed(
        Duration.zero,
        () => NativeVaultService.instance.getAccountConfig(accountId: accountId),
      ).timeout(
        const Duration(seconds: 5),
        onTimeout: () => throw TimeoutException('getAccountConfig timed out'),
      );
      SoloLog.d('AUTH', 'getAccountConfig returned: ${rustConfig != null}');

      String? salt;
      String? verifyHash;

      if (rustConfig != null && rustConfig.salt != null && rustConfig.verifyHash != null) {
        salt = rustConfig.salt;
        verifyHash = rustConfig.verifyHash;
      } else {
        // Fallback: read config.json directly from disk
        SoloLog.w('AUTH', 'Rust FFI returned null, trying direct file read...');
        final vaultRoot = RustVaultService.instance.vaultRoot;
        if (vaultRoot != null) {
          // Validate accountId format to prevent path traversal
          if (!RegExp(r'^acc_[a-f0-9\-]{36}$').hasMatch(accountId)) {
            SoloLog.e('AUTH', 'Invalid accountId format, possible path traversal attempt');
            return;
          }
          final configFile = File('$vaultRoot/$accountId/config.json');
          if (await configFile.exists()) {
            final configJson = jsonDecode(await configFile.readAsString()) as Map<String, dynamic>;
            salt = configJson['salt'] as String?;
            verifyHash = configJson['verify_hash'] as String?;
            SoloLog.d('AUTH', 'File config.json: salt=${salt != null}, verifyHash=${verifyHash != null}');
          } else {
            SoloLog.w('AUTH', 'config.json not found at ${configFile.path}');
          }
        } else {
          SoloLog.w('AUTH', 'vaultRoot is null — cannot read config.json');
        }
      }

      // Salt/verify_hash are no longer stored in Dart-side Keychain.
      // Rust vault is the single source of truth for sensitive credentials.
      // Only update crypto version marker.
      try {
        await _storage
            .updateAccountCryptoVersion(accountId, cryptoVersion)
            .timeout(
              const Duration(seconds: 5),
              onTimeout: () =>
                  throw TimeoutException('updateAccountCryptoVersion timed out'),
            );
      } on Exception catch (e, st) {
        SoloLog.e('AUTH', 'Failed to update crypto version', e, st);
      }
    } on Exception catch (e, st) {
      DebugLogger.instance
          .logError('AUTH', 'Migration error: $e\nStack trace: $st');
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

    // Step 4: Salt/verify_hash are no longer stored in Dart-side Keychain.
    // Rust vault is the single source of truth for sensitive credentials.

    // Step 5: Re-save profile if it exists (session key managed by Rust)
    if (currentProfile != null) {
      await profileStorage.saveProfile(accountId, currentProfile);
    }

    // Step 6: Migrate encrypted metadata (scan config, settings, field histories,
    // operation logs) to new session key. These were encrypted with the old key
    // and would fail to decrypt after password change without re-encryption.
    final migrated = await RustVaultService.instance.migrateMetadataAfterPasswordChange(accountId);
    if (!migrated) {
      SoloLog.w('PasswordService',
          'Some metadata entries could not be migrated after password change for $accountId');
    }

    // Step 7: Re-save biometric credential with new password if biometric was enabled.
    // This ensures users can still use Touch ID / Face ID after password change
    // without manually re-enabling it.
    final settings = SecurityService.instance.settings;
    if (settings.biometricsEnabled || settings.faceIdEnabled) {
      await BiometricCredentialService.instance.initialize();
      final saved = await BiometricCredentialService.instance.saveBiometricCredential(
        accountId,
        newPassword,
      );
      if (saved) {
        SoloLog.d('PasswordService',
            'Biometric credential re-saved with new password for $accountId');
      } else {
        SoloLog.w('PasswordService',
            'Failed to re-save biometric credential after password change for $accountId');
        // Clear stale credential so user isn't stuck with a broken biometric
        await BiometricCredentialService.instance.clearBiometricCredential(accountId);
      }
    } else {
      // Biometric not enabled — clear any stale credential
      await BiometricCredentialService.instance.clearBiometricCredential(accountId);
      SoloLog.d('PasswordService',
          'Biometric credential cleared for $accountId after password change (not enabled)');
    }

    // Step 8: Update password hint if provided
    if (newPasswordHint != null) {
      await _storage.updatePasswordHint(accountId, newPasswordHint);
    }

    return (success: true, error: null);
  }

  /// Update only the password hint (no password change).
  /// Verifies current password before updating.
  Future<({bool success, String? error})> updatePasswordHintOnly({
    required String accountId,
    required String currentPassword,
    required String newPasswordHint,
  }) async {
    // Step 1: Verify current password
    final verifyResult = await _storage.verifyPassword(accountId, currentPassword);
    if (!verifyResult) {
      return (success: false, error: 'Invalid current password');
    }

    // Step 2: Update password hint
    await _storage.updatePasswordHint(accountId, newPasswordHint);

    return (success: true, error: null);
  }
}

// ============================================================================
// AccountManager - Account CRUD operations
// ============================================================================

/// Service for account CRUD operations
class AccountManager {
  final SecureAccountStorage _storage;

  String? _selectedAccountId;
  AccountInfo? _selectedAccountInfo;
  int _accountsVersion = 0;

  AccountManager(this._storage);

  String? get selectedAccountId => _selectedAccountId;
  AccountInfo? get selectedAccount => _selectedAccountInfo;
  int get accountsVersion => _accountsVersion;

  /// Bump accounts version to trigger provider rebuild
  void bumpAccountsVersion() {
    _accountsVersion++;
  }

  /// Get all accounts sorted by most recent access
  Future<List<AccountInfo>> getAccountsSortedByRecent() async {
    SoloLog.d('AccountMgr', 'getAccountsSortedByRecent: Fetching accounts...');
    List<Map<String, dynamic>> rustAccounts;
    try {
      rustAccounts = await RustVaultService.instance.listAccountsFromRust();
    } on Object catch (e, st) {
      SoloLog.e('AccountMgr', 'listAccountsFromRust FAILED', e, st);
      rethrow;
    }
    List<AccountInfo> accounts;

    if (rustAccounts.isNotEmpty) {
      SoloLog.d('AccountMgr', 'Found ${rustAccounts.length} accounts in Rust');
      final rustMappedAccounts = rustAccounts
          .map((r) {
            final devicesRaw = r['recent_devices'] as List<dynamic>?;
            final devices = devicesRaw
                    ?.map((d) => DeviceInfo.fromJson(d as Map<String, dynamic>))
                    .toList() ??
                const <DeviceInfo>[];
            return AccountInfo(
                id: r['id'] as String? ?? '',
                name: r['name'] as String? ?? '',
                passwordHint: r['password_hint'] as String?,
                lastAccessed: r['last_accessed'] != null
                    ? DateTime.tryParse(r['last_accessed'] as String)
                    : null,
                createdAt: r['created_at'] != null
                    ? DateTime.tryParse(r['created_at'] as String)
                    : null,
                lastLoginAt: r['last_login_at'] != null
                    ? DateTime.tryParse(r['last_login_at'] as String)
                    : null,
                lastOperationAt: r['last_operation_at'] != null
                    ? DateTime.tryParse(r['last_operation_at'] as String)
                    : null,
                lastOperationDesc: r['last_operation_desc'] as String?,
                recentDevices: devices,
              );
          })
          .toList();

      final storageAccounts = await _storage.listAccounts();
      final storageById = {for (final a in storageAccounts) a.id: a};
      SoloLog.d('AccountMgr', 'Found ${storageAccounts.length} accounts in Keychain');

      accounts = rustMappedAccounts.map((rustAccount) {
        final storageAccount = storageById[rustAccount.id];
        if (storageAccount != null) {
          SoloLog.d('AccountMgr', 'Merging account ${rustAccount.id}: hasHint=${storageAccount.passwordHint != null}');
          // Prefer Rust values; fall back to Keychain only when Rust is null
          return rustAccount.copyWith(
            createdAt: rustAccount.createdAt ?? storageAccount.createdAt,
            lastLoginAt: rustAccount.lastLoginAt ?? storageAccount.lastLoginAt,
            lastOperationAt: rustAccount.lastOperationAt ?? storageAccount.lastOperationAt,
            lastOperationDesc: rustAccount.lastOperationDesc ?? storageAccount.lastOperationDesc,
            recentDevices: rustAccount.recentDevices.isNotEmpty
                ? rustAccount.recentDevices
                : storageAccount.recentDevices,
            passwordHint: rustAccount.passwordHint ?? storageAccount.passwordHint,
          );
        }
        return rustAccount;
      }).toList();
    } else {
      SoloLog.d('AccountMgr', 'No Rust accounts, using Keychain only');
      accounts = await _storage.listAccounts();
    }

    accounts.sort((a, b) {
      final aLast = a.lastAccessed;
      final bLast = b.lastAccessed;
      if (aLast == null && bLast == null) return 0;
      if (aLast == null) return 1;
      if (bLast == null) return -1;
      return bLast.compareTo(aLast);
    });
    SoloLog.d('AccountMgr', 'Returning ${accounts.length} accounts sorted by recent');
    return accounts;
  }

  /// Get all accounts
  Future<List<AccountInfo>> getAccounts() async {
    return _storage.listAccounts();
  }

  /// Select an account
  Future<void> selectAccount(String? accountId) async {
    SoloLog.d('AccountMgr', 'Selecting account: $accountId');
    _selectedAccountId = accountId;
    if (accountId != null) {
      final accounts = await _storage.listAccounts();
      _selectedAccountInfo = accounts.cast<AccountInfo?>().firstWhere(
            (a) => a?.id == accountId,
            orElse: () => null,
          );
      SoloLog.d('AccountMgr', 'Account selected: ${_selectedAccountInfo?.name}, hasHint=${_selectedAccountInfo?.passwordHint != null}');
    } else {
      _selectedAccountInfo = null;
      SoloLog.d('AccountMgr', 'Account deselected');
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
    final vaultResult = await RustVaultService.instance.createAccount(
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

      final account = result.account;
      final sessionKey = result.sessionKey;
      if (result.success && account != null && sessionKey != null) {
        _selectedAccountId = account.id;
        _selectedAccountInfo = account;
        _accountsVersion++;
        SecureAccountStorage.secureWipe(sessionKey);
        // Persist hint to Rust vault for reliable retrieval on next login
        if (passwordHint != null && passwordHint.isNotEmpty) {
          unawaited(_storage.updatePasswordHint(account.id, passwordHint));
        }
        return (success: true, error: null);
      } else if (result.error != null) {
        return (success: false, error: result.error);
      }
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('AUTH', 'Keychain createAccount failed, using Rust-only mode: $e\nStack: $st');
    }

    // If Keychain failed, we still have Rust account (session key managed by Rust)
    DebugLogger.instance.logInfo('AUTH', 'Using Rust-only mode (Keychain unavailable)');

    _selectedAccountId = vaultResult.accountId;
    _selectedAccountInfo = AccountInfo(
      id: vaultResult.accountId!,
      name: name,
      passwordHint: passwordHint,
      lastAccessed: DateTime.now(),
      createdAt: DateTime.now(),
    );
    // Persist hint to Rust vault for reliable retrieval on next login
    if (passwordHint != null && passwordHint.isNotEmpty) {
      unawaited(_storage.updatePasswordHint(_selectedAccountId!, passwordHint));
    }
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

    final rustDeleted = await RustVaultService.instance.deleteAccount(_selectedAccountId!);

    // Clean up Keychain if possible, but don't fail if Keychain is unavailable
    // Rust is the source of truth for account data
    final storageDeleted = await _storage.deleteAccount(_selectedAccountId!);
    if (!storageDeleted) {
      DebugLogger.instance.logError(
        'AUTH',
        'Rust delete succeeded but Keychain delete failed for $_selectedAccountId. '
        'Account list may be out of sync.',
      );
    }

    if (rustDeleted) {
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
