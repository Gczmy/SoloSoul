import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

part 'auth_provider.g.dart';

/// Convert bytes to hex string (for Rust-compatible verification hashes)
String _bytesToHex(List<int> bytes) {
  return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
}

/// Convert hex string to bytes
Uint8List _hexToBytes(String hex) {
  final result = <int>[];
  for (var i = 0; i < hex.length; i += 2) {
    result.add(int.parse(hex.substring(i, i + 2), radix: 16));
  }
  return Uint8List.fromList(result);
}

/// Constant-time string comparison to prevent timing attacks
bool _constantTimeEquals(String a, String b) {
  final lenA = a.length;
  final lenB = b.length;
  final maxLen = lenA > lenB ? lenA : lenB;

  final paddedA = a.padRight(maxLen, '\x00');
  final paddedB = b.padRight(maxLen, '\x00');

  var result = 0;
  for (var i = 0; i < maxLen; i++) {
    result |= paddedA.codeUnitAt(i) ^ paddedB.codeUnitAt(i);
  }
  result |= lenA ^ lenB;
  return result == 0;
}

/// Device info for tracking recent device logins
class DeviceInfo {
  final String deviceName;
  final DateTime lastUsed;
  const DeviceInfo({required this.deviceName, required this.lastUsed});

  factory DeviceInfo.fromJson(Map<String, dynamic> json) {
    return DeviceInfo(
      deviceName: json['device_name'] as String,
      lastUsed: DateTime.parse(json['last_used'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
        'device_name': deviceName,
        'last_used': lastUsed.toIso8601String(),
      };
}

/// Account info
class AccountInfo {
  final String id;
  final String name;
  final String? passwordHint;
  final DateTime? lastAccessed;
  final DateTime? createdAt;
  final DateTime? lastLoginAt;
  final DateTime? lastOperationAt;
  final String? lastOperationDesc;
  final List<DeviceInfo> recentDevices;

  const AccountInfo({
    required this.id,
    required this.name,
    this.passwordHint,
    this.lastAccessed,
    this.createdAt,
    this.lastLoginAt,
    this.lastOperationAt,
    this.lastOperationDesc,
    this.recentDevices = const [],
  });

  factory AccountInfo.fromJson(Map<String, dynamic> json) {
    return AccountInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      passwordHint: json['password_hint'] as String?,
      lastAccessed: json['last_accessed'] != null
          ? DateTime.parse(json['last_accessed'] as String)
          : null,
      createdAt: json['created_at'] != null
          ? DateTime.parse(json['created_at'] as String)
          : null,
      lastLoginAt: json['last_login_at'] != null
          ? DateTime.parse(json['last_login_at'] as String)
          : null,
      lastOperationAt: json['last_operation_at'] != null
          ? DateTime.parse(json['last_operation_at'] as String)
          : null,
      lastOperationDesc: json['last_operation_desc'] as String?,
      recentDevices: (json['recent_devices'] as List<dynamic>?)
              ?.map((e) => DeviceInfo.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'password_hint': passwordHint,
        'last_accessed': lastAccessed?.toIso8601String(),
        'created_at': createdAt?.toIso8601String(),
        'last_login_at': lastLoginAt?.toIso8601String(),
        'last_operation_at': lastOperationAt?.toIso8601String(),
        'last_operation_desc': lastOperationDesc,
        'recent_devices': recentDevices.map((e) => e.toJson()).toList(),
      };

  AccountInfo copyWith({
    String? id,
    String? name,
    String? passwordHint,
    DateTime? lastAccessed,
    DateTime? createdAt,
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    List<DeviceInfo>? recentDevices,
  }) {
    return AccountInfo(
      id: id ?? this.id,
      name: name ?? this.name,
      passwordHint: passwordHint ?? this.passwordHint,
      lastAccessed: lastAccessed ?? this.lastAccessed,
      createdAt: createdAt ?? this.createdAt,
      lastLoginAt: lastLoginAt ?? this.lastLoginAt,
      lastOperationAt: lastOperationAt ?? this.lastOperationAt,
      lastOperationDesc: lastOperationDesc ?? this.lastOperationDesc,
      recentDevices: recentDevices ?? this.recentDevices,
    );
  }
}

/// Auth state
enum AuthState { initial, locked, unlocked, loading }

// ============================================================================
// SecureAccountStorage - unchanged, kept at bottom for reference
// ============================================================================

/// Secure storage for account data using FlutterSecureStorage (Keychain on macOS)
class SecureAccountStorage {
  static const _accountsKey = 'solosoul_accounts';
  static const _accountDataPrefix = 'solosoul_account_';

  const SecureAccountStorage._();

  static const SecureAccountStorage _instance = SecureAccountStorage._();
  static SecureAccountStorage get instance => _instance;

  FlutterSecureStorage get _secureStorage {
    return const FlutterSecureStorage();
  }

  Future<void> _writeSecure(String key, String? value) async {
    try {
      await _secureStorage.write(key: key, value: value).timeout(
            const Duration(seconds: 5),
            onTimeout: () {
              throw Exception('Keychain write timed out for key: $key');
            },
          );
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('STORAGE', 'Keychain write error: $e\nStack trace: $st');
      rethrow;
    }
  }

  Future<List<AccountInfo>> listAccounts() async {
    final data = await _secureStorage.read(key: _accountsKey);

    if (data == null || data.isEmpty) {
      return [];
    }

    final decoded = jsonDecode(data) as List<dynamic>;
    final accounts = decoded
        .map((e) => AccountInfo.fromJson(e as Map<String, dynamic>))
        .toList();

    return accounts;
  }

  Future<void> _saveAccounts(List<AccountInfo> accounts) async {
    final jsonData = jsonEncode(accounts.map((e) => e.toJson()).toList());
    await _writeSecure(_accountsKey, jsonData);
  }

  Future<Map<String, dynamic>?> getAccountData(String id) async {
    final data = await _secureStorage.read(key: '$_accountDataPrefix$id');
    if (data == null) {
      return null;
    }
    return jsonDecode(data) as Map<String, dynamic>;
  }

  Future<void> saveAccountData(String id, Map<String, dynamic> data) async {
    await _writeSecure('$_accountDataPrefix$id', jsonEncode(data));
  }

  Future<(
      {bool success,
      String? error,
      AccountInfo? account,
      Uint8List? sessionKey})>
      createAccount(
    String name,
    String password, {
    String? passwordHint,
    String? accountId,
    String? salt,
    String? verifyHashFromRust,
  }) async {
    DebugLogger.instance.logInfo('STORAGE', 'createAccount start');

    if (name.trim().isEmpty) {
      DebugLogger.instance.logInfo('STORAGE', 'name empty, returning error');
      return (
        success: false,
        error: 'Account name is required',
        account: null,
        sessionKey: null
      );
    }
    if (password.length < 8) {
      return (
        success: false,
        error: 'Password must be at least 8 characters',
        account: null,
        sessionKey: null
      );
    }

    final accounts = await listAccounts();
    if (accounts.any((a) => a.name.toLowerCase() == name.toLowerCase())) {
      return (
        success: false,
        error: 'This account name is already taken',
        account: null,
        sessionKey: null
      );
    }

    final effectiveAccountId =
        accountId ?? 'acc_${DateTime.now().millisecondsSinceEpoch}';

    String saltToStore;
    String hashToStore;
    Uint8List? sessionKey;

    if (salt != null && verifyHashFromRust != null) {
      saltToStore = salt;
      hashToStore = verifyHashFromRust;
      sessionKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: base64Decode(salt),
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
    } else {
      final dartSalt = NativeCryptoService.instance.generateSalt();
      if (dartSalt == null) {
        return (
          success: false,
          error: 'Failed to generate salt',
          account: null,
          sessionKey: null
        );
      }
      saltToStore = base64Encode(dartSalt);
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: dartSalt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (verifyKey == null) {
        return (
          success: false,
          error: 'Failed to derive key',
          account: null,
          sessionKey: null
        );
      }
      hashToStore = base64Encode(verifyKey);
      sessionKey = verifyKey;
    }

    final now = DateTime.now();
    final account = AccountInfo(
      id: effectiveAccountId,
      name: name.trim(),
      passwordHint: passwordHint,
      lastAccessed: now,
      createdAt: now,
      lastLoginAt: now,
    );

    DebugLogger.instance.logInfo('STORAGE', 'calling saveAccountData');
    await saveAccountData(effectiveAccountId, {
      'salt': saltToStore,
      'verify_hash': hashToStore,
      'crypto_version': 2,
    });
    DebugLogger.instance.logInfo('STORAGE', 'saveAccountData done');

    DebugLogger.instance.logInfo('STORAGE', 'calling _saveAccounts');
    accounts.add(account);
    await _saveAccounts(accounts);
    DebugLogger.instance.logInfo('STORAGE', '_saveAccounts done, returning success');

    return (
      success: true,
      error: null,
      account: account,
      sessionKey: sessionKey
    );
  }

  Future<({bool success, String? error, Uint8List? sessionKey})>
      unlockAccount(
    String accountId,
    String password,
  ) async {
    final accountData = await getAccountData(accountId);
    if (accountData == null) {
      return (success: false, error: 'Account not found', sessionKey: null);
    }

    final salt = base64Decode(accountData['salt'] as String);
    final storedHash = accountData['verify_hash'] as String;

    final derivedKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );

    if (derivedKey == null) {
      return (success: false, error: 'Key derivation failed', sessionKey: null);
    }

    final derivedHashBase64 = base64Encode(derivedKey);
    final derivedHashHex = _bytesToHex(derivedKey);

    if (!_constantTimeEquals(derivedHashBase64, storedHash) &&
        !_constantTimeEquals(derivedHashHex, storedHash)) {
      return (success: false, error: 'Invalid password', sessionKey: null);
    }

    final accounts = await listAccounts();
    final idx = accounts.indexWhere((a) => a.id == accountId);
    if (idx >= 0) {
      final existing = accounts[idx];
      accounts[idx] = AccountInfo(
        id: existing.id,
        name: existing.name,
        passwordHint: existing.passwordHint,
        lastAccessed: DateTime.now(),
        createdAt: existing.createdAt,
        lastLoginAt: DateTime.now(),
        lastOperationAt: existing.lastOperationAt,
        lastOperationDesc: existing.lastOperationDesc,
        recentDevices: existing.recentDevices,
      );
      await _saveAccounts(accounts);
    }

    return (success: true, error: null, sessionKey: derivedKey);
  }

  Future<bool> verifyPassword(String accountId, String password) async {
    final accountData = await getAccountData(accountId);
    if (accountData == null) return false;

    final salt = base64Decode(accountData['salt'] as String);
    final storedHash = accountData['verify_hash'] as String;

    final derivedKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );

    if (derivedKey == null) return false;

    final derivedHashBase64 = base64Encode(derivedKey);
    final derivedHashHex = _bytesToHex(derivedKey);
    return _constantTimeEquals(derivedHashBase64, storedHash) ||
        _constantTimeEquals(derivedHashHex, storedHash);
  }

  Future<bool> deleteAccount(String accountId) async {
    try {
      final accounts = await listAccounts();
      accounts.removeWhere((a) => a.id == accountId);
      await _saveAccounts(accounts);

      await _secureStorage.delete(key: '$_accountDataPrefix$accountId');

      return true;
    } on Exception {
      return false;
    }
  }

  Future<void> updateAccountSalt(
      String accountId, Uint8List salt, Uint8List verifyHash) async {
    await saveAccountData(accountId, {
      'salt': base64Encode(salt),
      'verify_hash': base64Encode(verifyHash),
    });
  }

  Future<bool> updateAccountCryptoVersion(
      String accountId, int cryptoVersion) async {
    final data = await getAccountData(accountId);
    if (data == null) return false;
    data['crypto_version'] = cryptoVersion;
    await saveAccountData(accountId, data);
    return true;
  }

  Future<void> updateAccountOperation(
      String accountId, String operationDesc) async {
    final accounts = await listAccounts();
    final idx = accounts.indexWhere((a) => a.id == accountId);
    if (idx >= 0) {
      final existing = accounts[idx];
      accounts[idx] = AccountInfo(
        id: existing.id,
        name: existing.name,
        passwordHint: existing.passwordHint,
        lastAccessed: existing.lastAccessed,
        createdAt: existing.createdAt,
        lastLoginAt: existing.lastLoginAt,
        lastOperationAt: DateTime.now(),
        lastOperationDesc: operationDesc,
        recentDevices: existing.recentDevices,
      );
      await _saveAccounts(accounts);
    }
  }

  Future<void> updatePasswordHint(String accountId, String hint) async {
    final accounts = await listAccounts();
    final idx = accounts.indexWhere((a) => a.id == accountId);
    if (idx >= 0) {
      final existing = accounts[idx];
      accounts[idx] = AccountInfo(
        id: existing.id,
        name: existing.name,
        passwordHint: hint,
        lastAccessed: existing.lastAccessed,
        createdAt: existing.createdAt,
        lastLoginAt: existing.lastLoginAt,
        lastOperationAt: existing.lastOperationAt,
        lastOperationDesc: existing.lastOperationDesc,
        recentDevices: existing.recentDevices,
      );
      await _saveAccounts(accounts);
    }
  }

  Future<void> updateAccountMetadata(
    String accountId, {
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    Map<String, dynamic>? device,
  }) async {
    final accounts = await listAccounts();
    final idx = accounts.indexWhere((a) => a.id == accountId);
    if (idx < 0) return;

    final existing = accounts[idx];
    final recentDevices = List<DeviceInfo>.from(existing.recentDevices);

    if (device != null) {
      final existingIdx =
          recentDevices.indexWhere((d) => d.deviceName == device['device_name']);
      if (existingIdx >= 0) {
        recentDevices[existingIdx] = DeviceInfo(
          deviceName: device['device_name'] as String,
          lastUsed: DateTime.parse(device['last_used'] as String),
        );
      } else {
        if (recentDevices.length >= 5) {
          recentDevices.removeAt(0);
        }
        recentDevices.add(DeviceInfo(
          deviceName: device['device_name'] as String,
          lastUsed: DateTime.parse(device['last_used'] as String),
        ));
      }
    }

    accounts[idx] = AccountInfo(
      id: existing.id,
      name: existing.name,
      passwordHint: existing.passwordHint,
      lastAccessed: lastLoginAt ?? existing.lastAccessed,
      createdAt: existing.createdAt,
      lastLoginAt: lastLoginAt ?? existing.lastLoginAt,
      lastOperationAt: lastOperationAt ?? existing.lastOperationAt,
      lastOperationDesc: lastOperationDesc ?? existing.lastOperationDesc,
      recentDevices: recentDevices,
    );
    await _saveAccounts(accounts);
  }
}

// ============================================================================
// Service 1: AuthStateNotifier - Pure state machine
// ============================================================================

/// Pure state machine for authentication state (locked/unlocked/loading)
class AuthStateNotifier extends StateNotifier<AuthState> {
  AuthStateNotifier() : super(AuthState.initial);

  void setInitial() => state = AuthState.initial;
  void setLoading() => state = AuthState.loading;
  void setLocked() => state = AuthState.locked;
  void setUnlocked() => state = AuthState.unlocked;

  bool get isUnlocked => state == AuthState.unlocked;
}

// ============================================================================
// Service 2: VaultUnlockService - Rust FFI unlock/lock operations
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
// Service 3: MigrationService - V1→V2 and Rust→Keychain migrations
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

      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (verifyKey == null) return;

      await _storage.updateAccountSalt(accountId, salt, verifyKey);

      // Clear sensitive data from memory
      for (var i = 0; i < salt.length; i++) {
        salt[i] = 0;
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
      final verifyHashBytes = _hexToBytes(rustConfig.verifyHash!);

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
// Service 4: PasswordService - Password change flow
// ============================================================================

/// Service for password modification operations
class PasswordService {
  final SecureAccountStorage _storage;
  final VaultUnlockService _vaultUnlockService;

  const PasswordService(this._storage, this._vaultUnlockService);

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
    // Step 1: Verify current password via Rust
    final vaultResult = _vaultUnlockService.unlockVault(
      accountId: accountId,
      password: currentPassword,
    );

    if (!vaultResult.success) {
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
// Service 5: AccountManager - Account CRUD operations
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
    DebugLogger.instance
        .logInfo('AUTH', 'CHECKPOINT: calling _storage.createAccount');
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
    } else {
      return (success: false, error: result.error);
    }
  }

  /// Delete the current account
  Future<bool> deleteAccount(String password) async {
    if (_selectedAccountId == null) return false;

    final isValid = await _storage.verifyPassword(_selectedAccountId!, password);
    if (!isValid) return false;

    RustVaultService.instance.deleteAccount(_selectedAccountId!);

    final success = await _storage.deleteAccount(_selectedAccountId!);
    if (success) {
      _profileStorage.clearEncryptionKey();
      _selectedAccountId = null;
      _selectedAccountInfo = null;
      _accountsVersion++;
    }
    return success;
  }

  /// Update operation metadata
  Future<void> updateOperation(String operationDesc) async {
    if (_selectedAccountId == null) return;
    await _storage.updateAccountOperation(_selectedAccountId!, operationDesc);
  }
}

// ============================================================================
// AuthNotifier - Facade that delegates to services (AsyncNotifier)
// ============================================================================

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
  Future<bool> verifyPasswordForSensitiveData(String password) async {
    if (_accountManager.selectedAccountId == null) return false;
    if (password.isEmpty) return false;
    return await _storage.verifyPassword(_accountManager.selectedAccountId!, password);
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

/// Sensitive data access validation timeout (unique constant)
const kSensitiveAccessTimeout = Duration(minutes: 1);

/// Sensitive page access state
class SensitivePageAccessState {
  final DateTime? lastVerified;

  const SensitivePageAccessState({this.lastVerified});

  bool get isValid {
    if (lastVerified == null) return false;
    return DateTime.now().difference(lastVerified!) < kSensitiveAccessTimeout;
  }

  SensitivePageAccessState copyWith({DateTime? lastVerified}) {
    return SensitivePageAccessState(lastVerified: lastVerified ?? this.lastVerified);
  }
}

/// Notifier for sensitive page access
class SensitivePageAccessNotifier extends StateNotifier<SensitivePageAccessState> {
  Timer? _timer;

  SensitivePageAccessNotifier() : super(const SensitivePageAccessState());

  void markVerified() {
    _timer?.cancel();
    state = SensitivePageAccessState(lastVerified: DateTime.now());
    _timer = Timer(kSensitiveAccessTimeout, () {
      state = state.copyWith(lastVerified: state.lastVerified);
    });
  }

  void clear() {
    _timer?.cancel();
    _timer = null;
    state = const SensitivePageAccessState();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}

/// Provider for sensitive page access
final sensitivePageAccessProvider =
    StateNotifierProvider<SensitivePageAccessNotifier, SensitivePageAccessState>(
        (ref) {
  return SensitivePageAccessNotifier();
});

/// Provider that checks if sensitive access is currently granted
@riverpod
class IsSensitiveAccessGranted extends _$IsSensitiveAccessGranted {
  @override
  bool build() {
    final access = ref.watch(sensitivePageAccessProvider);
    return access.isValid;
  }
}
