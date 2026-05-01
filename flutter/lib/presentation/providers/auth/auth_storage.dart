import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

/// Secure storage for account data using FlutterSecureStorage (Keychain on macOS)
class SecureAccountStorage {
  static const _accountsKey = 'solosoul_accounts';
  static const _accountDataPrefix = 'solosoul_account_';

  const SecureAccountStorage._();

  static const SecureAccountStorage _instance = SecureAccountStorage._();
  static SecureAccountStorage get instance => _instance;

  static final FallbackSecureStorage _fallbackSecureStorage =
      FallbackSecureStorage();

  FallbackSecureStorage get _secureStorage => _fallbackSecureStorage;

  Future<void> _writeSecure(String key, String? value) async {
    SoloLog.d('AuthStorage', 'Writing to Keychain: key=$key');
    try {
      await _secureStorage.write(key: key, value: value).timeout(
            const Duration(seconds: 5),
            onTimeout: () {
              throw Exception('Keychain write timed out for key: $key');
            },
          );
      SoloLog.d('AuthStorage', 'Keychain write successful: $key');
    } on Exception catch (e, st) {
      SoloLog.e('AuthStorage', 'Keychain write failed: $key', e, st);
      rethrow;
    }
  }

  Future<List<AccountInfo>> listAccounts() async {
    SoloLog.d('AuthStorage', 'Reading accounts list from Keychain...');
    final data = await _secureStorage.read(key: _accountsKey);

    if (data == null || data.isEmpty) {
      SoloLog.d('AuthStorage', 'No accounts found in Keychain');
      return [];
    }
    SoloLog.d('AuthStorage', 'Found ${(jsonDecode(data) as List).length} accounts');

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
    SoloLog.d('AuthStorage', 'Reading account data from Keychain: id=$id');
    try {
      final data = await _secureStorage
          .read(key: '$_accountDataPrefix$id')
          .timeout(const Duration(seconds: 5));
      if (data == null) {
        SoloLog.w('AuthStorage', 'No account data found in Keychain: id=$id');
        return null;
      }
      SoloLog.d('AuthStorage', 'Successfully read account data from Keychain: id=$id');
      return jsonDecode(data) as Map<String, dynamic>;
    } on TimeoutException {
      SoloLog.w('AuthStorage', 'Keychain read timed out for id=$id');
      return null;
    } on Exception catch (e, st) {
      SoloLog.e('AuthStorage', 'Keychain read error for id=$id', e, st);
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
    if (password.length < 12 &&
        !_hasSufficientComplexity(password)) {
      return (
        success: false,
        error: 'Password must be at least 12 characters, '
            'or 8+ with uppercase, lowercase, and digits',
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
        accountId ?? 'acc_${const Uuid().v4()}';

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

  /// Verify password against Rust AccountManager.
  /// Phase 2: delegates entirely to Rust — no more Dart-side Argon2id.
  Future<bool> verifyPassword(String accountId, String password) async {
    SoloLog.d('AuthStorage', 'verifyPassword: Starting for accountId=$accountId');
    final timer = SoloLog.startTimer('AuthStorage', 'verifyPassword');
    try {
      final result = NativeVaultService.instance.request(
        'verify_password',
        {'account_id': accountId, 'password': password},
      );
      final success = result?['data']?['success'] == true;
      SoloLog.d('AuthStorage', 'verifyPassword result: $success');
      SoloLog.endTimer(timer);
      return success;
    } on Object catch (e, st) {
      SoloLog.e('AuthStorage', 'verifyPassword unexpected error', e, st);
      SoloLog.endTimer(timer);
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

  /// Update last operation — delegates to Rust.
  Future<void> updateAccountOperation(
      String accountId, String operationDesc) async {
    NativeVaultService.instance.request(
      'update_account_metadata',
      {
        'account_id': accountId,
        'last_operation_at': DateTime.now().toUtc().toIso8601String(),
        'last_operation_desc': operationDesc,
      },
    );
  }

  /// Update password hint — delegates to Rust.
  Future<void> updatePasswordHint(String accountId, String hint) async {
    NativeVaultService.instance.request(
      'update_account_metadata',
      {'account_id': accountId, 'password_hint': hint},
    );
  }

  /// Update account metadata — delegates to Rust `update_account_metadata`.
  /// Phase 2: Rust is the single source of truth for account metadata.
  Future<void> updateAccountMetadata(
    String accountId, {
    DateTime? lastLoginAt,
    DateTime? lastOperationAt,
    String? lastOperationDesc,
    Map<String, dynamic>? device,
  }) async {
    NativeVaultService.instance.request(
      'update_account_metadata',
      {
        'account_id': accountId,
        if (lastLoginAt != null) 'last_login_at': lastLoginAt.toUtc().toIso8601String(),
        if (lastOperationAt != null) 'last_operation_at': lastOperationAt.toUtc().toIso8601String(),
        if (lastOperationDesc != null) 'last_operation_desc': lastOperationDesc,
        if (device != null) 'add_device': device,
      },
    );
  }

  static bool _hasSufficientComplexity(String password) {
    final hasUpper = password.contains(RegExp(r'[A-Z]'));
    final hasLower = password.contains(RegExp(r'[a-z]'));
    final hasDigit = password.contains(RegExp(r'[0-9]'));
    return hasUpper && hasLower && hasDigit;
  }
}
