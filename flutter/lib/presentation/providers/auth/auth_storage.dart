import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

/// Secure storage for account data using FlutterSecureStorage (Keychain on macOS)
class SecureAccountStorage {
  static const _accountsKey = 'solosoul_accounts';
  static const _accountDataPrefix = 'solosoul_account_';

  const SecureAccountStorage._();

  static const SecureAccountStorage _instance = SecureAccountStorage._();
  static SecureAccountStorage get instance => _instance;

  FallbackSecureStorage get _secureStorage {
    return FallbackSecureStorage();
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
    try {
      final data = await _secureStorage
          .read(key: '$_accountDataPrefix$id')
          .timeout(const Duration(seconds: 5));
      if (data == null) {
        return null;
      }
      return jsonDecode(data) as Map<String, dynamic>;
    } on TimeoutException {
      DebugLogger.instance.logInfo('AUTH', 'getAccountData timed out for id=$id');
      return null;
    }
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
      // Step 1: Derive master_key from password (same as Rust)
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: dartSalt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (masterKey == null) {
        return (
          success: false,
          error: 'Failed to derive key',
          account: null,
          sessionKey: null
        );
      }
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
      if (verifyKey == null) {
        return (
          success: false,
          error: 'Failed to derive verify key',
          account: null,
          sessionKey: null
        );
      }
      // Step 3: Hex-encode verify_key (same as Rust)
      hashToStore = bytesToHex(verifyKey);
      sessionKey = masterKey;
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

    // Step 1: Derive master_key from password (same as Rust)
    final masterKey = NativeCryptoService.instance.deriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );
    if (masterKey == null) {
      return (success: false, error: 'Key derivation failed', sessionKey: null);
    }

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
    if (verifyKey == null) {
      return (success: false, error: 'Verify failed', sessionKey: null);
    }

    // Step 3: Hex-encode verify_key and compare (same as Rust)
    final derivedHashHex = bytesToHex(verifyKey);
    if (!constantTimeEquals(derivedHashHex, storedHash)) {
      return (success: false, error: 'Invalid password', sessionKey: null);
    }

    // Session key is masterKey (used for profile encryption)
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

    return (success: true, error: null, sessionKey: masterKey);
  }

  Future<bool> verifyPassword(String accountId, String password) async {
    try {
      final accountData = await getAccountData(accountId);

      String saltStr;
      String storedHash;

      if (accountData == null) {
        // Keychain has no data (migration may have failed) - fall back to Rust
        DebugLogger.instance.logInfo('AUTH', 'verifyPassword: Keychain has no data, falling back to Rust');
        final rustConfig = NativeVaultService.instance.getAccountConfig(accountId: accountId);
        if (rustConfig == null || rustConfig.salt == null || rustConfig.verifyHash == null) {
          DebugLogger.instance.logInfo('AUTH', 'verifyPassword: Rust also has no data');
          return false;
        }
        saltStr = rustConfig.salt!;
        storedHash = rustConfig.verifyHash!;
        DebugLogger.instance.logInfo('AUTH', 'verifyPassword: Rust saltStr=$saltStr (len=${saltStr.length}), verifyHash=$storedHash (len=${storedHash.length})');
      } else {
        saltStr = accountData['salt'] as String;
        storedHash = accountData['verify_hash'] as String;
        // If Keychain salt is corrupted (wrong length), fall back to Rust
        try {
          final decoded = base64Decode(saltStr);
          if (decoded.length != 32) {
            DebugLogger.instance.logInfo('AUTH', 'verifyPassword: Keychain salt length=${decoded.length}, falling back to Rust');
            final rustConfig = NativeVaultService.instance.getAccountConfig(accountId: accountId);
            if (rustConfig?.salt != null && rustConfig?.verifyHash != null) {
              saltStr = rustConfig!.salt!;
              storedHash = rustConfig.verifyHash!;
            }
          }
        } on Exception catch (_) {
          DebugLogger.instance.logInfo('AUTH', 'verifyPassword: Keychain salt decode failed, falling back to Rust');
          final rustConfig = NativeVaultService.instance.getAccountConfig(accountId: accountId);
          if (rustConfig?.salt != null && rustConfig?.verifyHash != null) {
            saltStr = rustConfig!.salt!;
            storedHash = rustConfig.verifyHash!;
          } else {
            return false;
          }
        }
      }

      final salt = base64Decode(saltStr);
      if (salt.length != 32) {
        DebugLogger.instance.logError('AUTH', 'verifyPassword: Invalid salt length ${salt.length}, expected 32');
        return false;
      }

      DebugLogger.instance.logInfo('AUTH', 'verifyPassword: salt=${base64Encode(salt)}, storedHash length=${storedHash.length}');

      // Step 1: Derive master_key from password (same as Rust)
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: Uint8List.fromList(salt),
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (masterKey == null) return false;

      // Step 2: Hex-encode master_key and use as password for verify derivation (same as Rust)
      final masterKeyHex = bytesToHex(masterKey);
      DebugLogger.instance.logInfo('AUTH', 'verifyPassword: masterKeyHex length=${masterKeyHex.length}');
      const verifyData = 'SOLOSOUL_VAULT_VERIFY_v1';
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode(verifyData)),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      );
      if (verifyKey == null) return false;

      // Step 3: Hex-encode verify_key and compare (same as Rust)
      final derivedHashHex = bytesToHex(verifyKey);
      DebugLogger.instance.logInfo('AUTH', 'verifyPassword: derivedHashHex=$derivedHashHex, storedHash=$storedHash');
      final result = constantTimeEquals(derivedHashHex, storedHash);
      DebugLogger.instance.logInfo('AUTH', 'verifyPassword: result=$result');
      return result;
    } on Object catch (e, st) {
      // Catch both Exception and Error (e.g., ArgumentError from deriveKey)
      DebugLogger.instance.logError('AUTH', 'verifyPassword error: $e\nStack trace: $st');
      return false;
    }
  }

  Future<bool> deleteAccount(String accountId) async {
    try {
      final accounts = await listAccounts();
      accounts.removeWhere((a) => a.id == accountId);
      await _saveAccounts(accounts);
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('STORAGE', 'deleteAccount _saveAccounts error: $e\nStack trace: $st');
    }

    try {
      await _secureStorage.delete(key: '$_accountDataPrefix$accountId');
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('STORAGE', 'deleteAccount _secureStorage.delete error: $e\nStack trace: $st');
    }

    // Return true regardless of Keychain errors - Rust is the source of truth
    return true;
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
