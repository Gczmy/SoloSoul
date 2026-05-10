import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

/// Secure storage for account data using FlutterSecureStorage (Keychain on macOS)
class SecureAccountStorage {
  static const _accountsKey = 'solosoul_accounts';
  static const _accountDataPrefix = 'solosoul_account_';

  // Brute-force protection: track failed attempts per account
  static final Map<String, AttemptTracker> _attemptTrackers = {};

  static const int maxAttemptsBeforeLockout = 10;
  static const int backoffStartAfterAttempts = 5;
  static const Duration initialBackoff = Duration(seconds: 30);

  const SecureAccountStorage._();

  static const SecureAccountStorage _instance = SecureAccountStorage._();
  static SecureAccountStorage get instance => _instance;

  static final FallbackSecureStorage _fallbackSecureStorage =
      FallbackSecureStorage();

  FallbackSecureStorage get _secureStorage => _fallbackSecureStorage;

  /// Clears all attempt trackers. Used only in tests for isolation.
  void clearAttemptTrackersForTest() {
    _attemptTrackers.clear();
  }

  /// Best-effort secure wipe of a byte buffer (fills with zeros).
  static void secureWipe(Uint8List buffer) {
    for (var i = 0; i < buffer.length; i++) {
      buffer[i] = 0;
    }
  }

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
        !hasSufficientComplexity(password)) {
      return (
        success: false,
        error: 'Password must be at least 12 characters, '
            'or 8+ with uppercase, lowercase, digits, and special characters',
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

    Uint8List? sessionKey;

    if (salt != null && verifyHashFromRust != null) {
      sessionKey = await frb.frbDeriveKey(
        password: password,
        salt: base64Decode(salt),
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
    } else {
      final dartSalt = await frb.frbGenerateSalt(length: 32);
      // Step 1: Derive master_key from password (same as Rust)
      final masterKey = await frb.frbDeriveKey(
        password: password,
        salt: dartSalt,
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
      sessionKey = masterKey;
      // Wipe intermediate key material — sessionKey is returned for caller use
      secureWipe(dartSalt);
      secureWipe(verifyKey);
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

  /// Verify password against Rust AccountManager.
  /// Phase 2: delegates entirely to Rust — no more Dart-side Argon2id.
  /// Includes brute-force protection with exponential backoff.
  Future<bool> verifyPassword(String accountId, String password) async {
    // Brute-force protection: check rate limit
    final tracker = _attemptTrackers.putIfAbsent(accountId, AttemptTracker.new);
    if (tracker.isLockedOut) {
      final remaining = tracker.remainingLockout;
      SoloLog.w('AuthStorage', 'verifyPassword: account locked out, ${remaining.inSeconds}s remaining');
      return false;
    }
    if (tracker.shouldBackoff) {
      final delay = tracker.currentBackoff;
      SoloLog.w('AuthStorage', 'verifyPassword: backing off ${delay.inSeconds}s after ${tracker.attempts} failed attempts');
      await Future<void>.delayed(delay);
    }

    SoloLog.d('AuthStorage', 'verifyPassword: Starting for accountId=$accountId hasPassword=${password.isNotEmpty}');
    final timer = SoloLog.startTimer('AuthStorage', 'verifyPassword');
    try {
      final result = NativeVaultService.instance.request(
        'verify_password',
        {'account_id': accountId, 'password': password},
      );
      final data = result?['data'] as Map<String, dynamic>?;
      final success = data?['success'] == true;
      final error = data?['error'] as String?;
      final cv = data?['crypto_version'];
      SoloLog.d('AuthStorage', 'verifyPassword result: success=$success error=$error cv=$cv trackerAttempts=${tracker.attempts}');
      SoloLog.endTimer(timer);

      if (success) {
        tracker.reset();
      } else {
        tracker.recordFailure();
      }
      return success;
    } on Object catch (e, st) {
      SoloLog.e('AuthStorage', 'verifyPassword unexpected error', e, st);
      SoloLog.endTimer(timer);
      tracker.recordFailure();
      return false;
    }
  }

  Future<bool> deleteAccount(String accountId) async {
    var success = true;
    try {
      final accounts = await listAccounts();
      accounts.removeWhere((a) => a.id == accountId);
      await _saveAccounts(accounts);
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('STORAGE', 'deleteAccount _saveAccounts error: $e\nStack trace: $st');
      success = false;
    }

    try {
      await _secureStorage.delete(key: '$_accountDataPrefix$accountId');
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('STORAGE', 'deleteAccount _secureStorage.delete error: $e\nStack trace: $st');
      // Keychain cleanup failure is non-fatal — Rust vault is source of truth
    }

    return success;
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
    SoloLog.d('AuthStorage', 'updateAccountMetadata: START accountId=$accountId, lastLoginAt=$lastLoginAt, device=${device != null}');
    try {
      final response = NativeVaultService.instance.request(
        'update_account_metadata',
        {
          'account_id': accountId,
          if (lastLoginAt != null) 'last_login_at': lastLoginAt.toUtc().toIso8601String(),
          if (lastOperationAt != null) 'last_operation_at': lastOperationAt.toUtc().toIso8601String(),
          if (lastOperationDesc != null) 'last_operation_desc': lastOperationDesc,
          if (device != null) 'add_device': device,
        },
      );
      if (response == null || response['success'] != true) {
        SoloLog.e('AuthStorage', 'updateAccountMetadata FAILED: ${response?['error']}');
      } else {
        SoloLog.d('AuthStorage', 'updateAccountMetadata: SUCCESS');
      }
    } on Exception catch (e, st) {
      SoloLog.e('AuthStorage', 'updateAccountMetadata EXCEPTION', e, st);
    }
  }

  static bool hasSufficientComplexity(String password) {
    final hasUpper = password.contains(RegExp(r'[A-Z]'));
    final hasLower = password.contains(RegExp(r'[a-z]'));
    final hasDigit = password.contains(RegExp(r'[0-9]'));
    final hasSpecial = password.contains(RegExp(r'[!@#$%^&*()_+\-=\[\]{}|;:,.<>?]'));
    return hasUpper && hasLower && hasDigit && hasSpecial;
  }
}

/// Tracks failed password attempts for brute-force protection.
class AttemptTracker {
  int attempts = 0;
  DateTime? _lockoutUntil;

  static const int backoffStartAfterAttempts =
      SecureAccountStorage.backoffStartAfterAttempts;
  static const int maxAttempts =
      SecureAccountStorage.maxAttemptsBeforeLockout;
  static const Duration initialBackoff =
      SecureAccountStorage.initialBackoff;
  static const Duration lockoutDuration = Duration(minutes: 15);

  bool get isLockedOut {
    if (_lockoutUntil == null) return false;
    if (DateTime.now().isAfter(_lockoutUntil!)) {
      _lockoutUntil = null;
      return false;
    }
    return true;
  }

  Duration get remainingLockout {
    if (_lockoutUntil == null) return Duration.zero;
    final remaining = _lockoutUntil!.difference(DateTime.now());
    return remaining.isNegative ? Duration.zero : remaining;
  }

  bool get shouldBackoff =>
      attempts >= backoffStartAfterAttempts && !isLockedOut;

  Duration get currentBackoff {
    if (attempts < backoffStartAfterAttempts) return Duration.zero;
    final exponent = attempts - backoffStartAfterAttempts;
    final seconds = initialBackoff.inSeconds * (1 << exponent);
    return Duration(seconds: seconds.clamp(0, 300));
  }

  void recordFailure() {
    attempts++;
    if (attempts >= maxAttempts) {
      _lockoutUntil = DateTime.now().add(lockoutDuration);
    }
  }

  void reset() {
    attempts = 0;
    _lockoutUntil = null;
  }
}
