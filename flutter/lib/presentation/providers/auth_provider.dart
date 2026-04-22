import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

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
}

/// Auth state
enum AuthState { initial, locked, unlocked, loading }

/// Secure storage for account data using FlutterSecureStorage (Keychain on macOS)
class SecureAccountStorage {
  static const _accountsKey = 'solosoul_accounts';
  static const _accountDataPrefix = 'solosoul_account_';

  const SecureAccountStorage._();

  static const SecureAccountStorage _instance = SecureAccountStorage._();
  static SecureAccountStorage get instance => _instance;

  FlutterSecureStorage get _secureStorage {
    // FlutterSecureStorage uses Keychain on macOS automatically
    return const FlutterSecureStorage();
  }

  /// Write to secure storage with a timeout to prevent indefinite blocking
  Future<void> _writeSecure(String key, String? value) async {
    try {
      await _secureStorage.write(key: key, value: value).timeout(
        const Duration(seconds: 5),
        onTimeout: () {
          throw Exception('Keychain write timed out for key: $key');
        },
      );
    } catch (e) {
      // Log but don't throw - this helps diagnose Keychain issues
      final traceLog = File('${Platform.environment['HOME']}/Library/Logs/flutter_native_vault.log');
      traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] Keychain write error: $e\n', mode: FileMode.append);
    }
  }

  /// List all accounts
  Future<List<AccountInfo>> listAccounts() async {
    final data = await _secureStorage.read(key: _accountsKey);

    if (data == null || data.isEmpty) {
      return [];
    }

    final decoded = jsonDecode(data) as List<dynamic>;
    final accounts = decoded.map((e) => AccountInfo.fromJson(e as Map<String, dynamic>)).toList();

    return accounts;
  }

  /// Save accounts list with timeout
  Future<void> _saveAccounts(List<AccountInfo> accounts) async {
    final jsonData = jsonEncode(accounts.map((e) => e.toJson()).toList());
    await _writeSecure(_accountsKey, jsonData);
  }

  /// Get account data (includes password hash)
  Future<Map<String, dynamic>?> getAccountData(String id) async {
    final data = await _secureStorage.read(key: '$_accountDataPrefix$id');
    if (data == null) {
      return null;
    }
    return jsonDecode(data) as Map<String, dynamic>;
  }

  /// Save account data with timeout
  Future<void> saveAccountData(String id, Map<String, dynamic> data) async {
    await _writeSecure('$_accountDataPrefix$id', jsonEncode(data));
  }

  /// Create a new account using Argon2id from Rust
  /// [salt] and [verifyHashFromRust] - if provided, store them directly instead of deriving
  /// This ensures Dart and Rust store identical verification data
  Future<({bool success, String? error, AccountInfo? account, Uint8List? sessionKey})> createAccount(
    String name,
    String password, {
    String? passwordHint,
    String? accountId,
    String? salt,               // Optional: Rust-generated salt (Base64)
    String? verifyHashFromRust, // Optional: Rust-generated verify_hash (Hex)
  }) async {
    final traceLog = File('${Platform.environment['HOME']}/Library/Logs/flutter_native_vault.log');
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] createAccount start\n', mode: FileMode.append);

    // Validation
    if (name.trim().isEmpty) {
      traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] name empty, returning error\n', mode: FileMode.append);
      return (success: false, error: 'Account name is required', account: null, sessionKey: null);
    }
    if (password.length < 8) {
      return (
        success: false,
        error: 'Password must be at least 8 characters',
        account: null,
        sessionKey: null,
      );
    }

    // Check if name is available
    final accounts = await listAccounts();
    if (accounts.any((a) => a.name.toLowerCase() == name.toLowerCase())) {
      return (
        success: false,
        error: 'This account name is already taken',
        account: null,
        sessionKey: null,
      );
    }

    // Use provided accountId or generate one
    final effectiveAccountId = accountId ?? 'acc_${DateTime.now().millisecondsSinceEpoch}';

    // Use provided salt/verifyHash or generate using Dart's algorithm
    String saltToStore;
    String hashToStore;
    Uint8List? sessionKey;

    if (salt != null && verifyHashFromRust != null) {
      // Use Rust-provided values (consistent with Rust vault)
      saltToStore = salt;
      hashToStore = verifyHashFromRust;
      // Also derive session key for profile encryption
      sessionKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: base64Decode(salt),
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
    } else {
      // Fallback: generate using Dart's algorithm (for backwards compat)
      final dartSalt = NativeCryptoService.instance.generateSalt();
      if (dartSalt == null) {
        return (success: false, error: 'Failed to generate salt', account: null, sessionKey: null);
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
        return (success: false, error: 'Failed to derive key', account: null, sessionKey: null);
      }
      hashToStore = base64Encode(verifyKey);
      sessionKey = verifyKey;
    }

    // Create account metadata
    final now = DateTime.now();
    final account = AccountInfo(
      id: effectiveAccountId,
      name: name.trim(),
      passwordHint: passwordHint,
      lastAccessed: now,
      createdAt: now,
      lastLoginAt: now,
    );

    // Save account data (salt + verify hash) to Keychain
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] calling saveAccountData\n', mode: FileMode.append);
    await saveAccountData(effectiveAccountId, {
      'salt': saltToStore,
      'verify_hash': hashToStore,
      'crypto_version': 2,
    });
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] saveAccountData done\n', mode: FileMode.append);

    // Add to accounts list
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] calling _saveAccounts\n', mode: FileMode.append);
    accounts.add(account);
    await _saveAccounts(accounts);
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [STORAGE] _saveAccounts done, returning success\n', mode: FileMode.append);

    return (success: true, error: null, account: account, sessionKey: sessionKey);
  }

  /// Unlock account with password using Argon2id from Rust
  Future<({bool success, String? error, Uint8List? sessionKey})> unlockAccount(
    String accountId,
    String password,
  ) async {
    final accountData = await getAccountData(accountId);
    if (accountData == null) {
      return (success: false, error: 'Account not found', sessionKey: null);
    }

    final salt = base64Decode(accountData['salt'] as String);
    final storedHash = accountData['verify_hash'] as String;

    // Derive key from provided password using Argon2id
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

    // Support both base64 (Dart-generated) and hex (Rust-generated) verify hashes
    if (derivedHashBase64 != storedHash && derivedHashHex != storedHash) {
      return (success: false, error: 'Invalid password', sessionKey: null);
    }

    // Update last accessed and last login
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

  /// Verify password before deletion
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

    // Support both base64 (Dart-generated) and hex (Rust-generated) verify hashes
    final derivedHashBase64 = base64Encode(derivedKey);
    final derivedHashHex = _bytesToHex(derivedKey);
    return derivedHashBase64 == storedHash || derivedHashHex == storedHash;
  }

  /// Delete an account and all its data
  Future<bool> deleteAccount(String accountId) async {
    try {
      // Remove from accounts list
      final accounts = await listAccounts();
      accounts.removeWhere((a) => a.id == accountId);
      await _saveAccounts(accounts);

      // Delete account data from Keychain
      await _secureStorage.delete(key: '$_accountDataPrefix$accountId');

      return true;
    } catch (e) {
      return false;
    }
  }

  /// Update account salt and verification hash (for password change)
  Future<void> updateAccountSalt(String accountId, Uint8List salt, Uint8List verifyHash) async {
    await saveAccountData(accountId, {
      'salt': base64Encode(salt),
      'verify_hash': base64Encode(verifyHash),
    });
  }

  /// Update crypto version marker for account migration
  /// Returns true if update was successful
  Future<bool> updateAccountCryptoVersion(String accountId, int cryptoVersion) async {
    final data = await getAccountData(accountId);
    if (data == null) return false;
    data['crypto_version'] = cryptoVersion;
    await saveAccountData(accountId, data);
    return true;
  }

  /// Update account operation metadata (last operation time and description)
  Future<void> updateAccountOperation(String accountId, String operationDesc) async {
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

  /// Update password hint for an account
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

  /// Update account metadata (lastLoginAt, lastOperationAt, device info)
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
      final existingIdx = recentDevices.indexWhere(
        (d) => d.deviceName == device['device_name'],
      );
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
class AuthNotifier extends StateNotifier<AuthState> {
  AuthNotifier() : super(AuthState.initial);

  final SecureAccountStorage _storage = SecureAccountStorage.instance;
  final ProfileStorageService _profileStorage = ProfileStorageService.instance;
  String? _selectedAccountId;
  AccountInfo? _selectedAccountInfo;
  String? get selectedAccountId => _selectedAccountId;
  AccountInfo? get selectedAccount => _selectedAccountInfo;
  bool get isUnlocked => state == AuthState.unlocked;

  /// Get all accounts sorted by most recent access
  /// Uses Rust vault as single source of truth to ensure consistency
  Future<List<AccountInfo>> getAccountsSortedByRecent() async {
    final rustAccounts = RustVaultService.instance.listAccountsFromRust();
    if (rustAccounts == null) {
      return [];
    }
    final accounts = <AccountInfo>[];
    for (final r in rustAccounts) {
      accounts.add(AccountInfo(
        id: r['id'] as String? ?? '',
        name: r['name'] as String? ?? '',
        lastAccessed: r['last_accessed'] != null ? DateTime.tryParse(r['last_accessed'] as String) : null,
        createdAt: DateTime.now(),
      ));
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
    // NOTE: Do NOT change auth state here!
    // If vault is unlocked, keep it unlocked - selectAccount is for account selection, not auth state management.
    // The unlockVault and lockVault methods are the only places that should change auth state.
  }

  /// Create a new account
  Future<({bool success, String? error})> createAccount(
    String name,
    String password, {
    String? passwordHint,
  }) async {
    // Sync trace to detect exactly where hang occurs
    final traceLog = File('${Platform.environment['HOME']}/Library/Logs/flutter_native_vault.log');
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: createAccount start\n', mode: FileMode.append);

    state = AuthState.loading;
    // First create account in Rust vault (this also auto-unlocks)
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: calling RustVaultService.createAccount\n', mode: FileMode.append);
    final vaultResult = RustVaultService.instance.createAccount(
      name: name,
      password: password,
    );
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: RustVaultService.createAccount returned, success=${vaultResult.success}\n', mode: FileMode.append);

    if (!vaultResult.success) {
      traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: vaultResult failed, returning error\n', mode: FileMode.append);
      state = AuthState.locked;
      return (success: false, error: vaultResult.error ?? 'Failed to create vault account');
    }

    // Also create account in SecureAccountStorage (Dart Keychain)
    // Use the SAME accountId, salt, and verify_hash that Rust generated so both are in sync
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: calling _storage.createAccount\n', mode: FileMode.append);
    final result = await _storage.createAccount(
      name,
      password,
      passwordHint: passwordHint,
      accountId: vaultResult.accountId,
      salt: vaultResult.salt,
      verifyHashFromRust: vaultResult.verifyHash,
    );
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: _storage.createAccount returned, success=${result.success}\n', mode: FileMode.append);

    if (result.success && result.account != null && result.sessionKey != null) {
      _selectedAccountId = result.account!.id;
      _selectedAccountInfo = result.account;
      // Set encryption key for profile storage
      _profileStorage.setEncryptionKey(result.sessionKey!);
      // Keep vault locked after account creation - user must explicitly unlock
      // This ensures the home page shows "Locked" state on first entry
      state = AuthState.locked;
      return (success: true, error: null);
    } else {
      state = AuthState.locked;
      return (success: false, error: result.error);
    }
  }

  /// Unlock vault with master password
  /// Handles automatic migration from V1 to V2 crypto on successful login
  Future<bool> unlockVault(String password) async {
    // Sync trace to detect exactly where hang occurs
    final traceLog = File('${Platform.environment['HOME']}/Library/Logs/flutter_native_vault.log');
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: unlockVault start, selectedAccountId=$_selectedAccountId\n', mode: FileMode.append);

    if (_selectedAccountId == null) {
      traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: _selectedAccountId is null, returning false\n', mode: FileMode.append);
      return false;
    }
    if (password.isEmpty) {
      traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: password is empty, returning false\n', mode: FileMode.append);
      state = AuthState.locked;
      return false;
    }

    state = AuthState.loading;

    // Step 1: Unlock Rust vault (source of truth for authentication)
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: calling RustVaultService.unlockVault\n', mode: FileMode.append);
    final vaultResult = RustVaultService.instance.unlockVault(
      accountId: _selectedAccountId!,
      password: password,
    );
    traceLog.writeAsStringSync('${DateTime.now().toIso8601String()} [AUTH] CHECKPOINT: RustVaultService.unlockVault returned, success=${vaultResult.success}\n', mode: FileMode.append);

    DebugLogger.instance.logInfo('AUTH', 'Rust unlock result: success=${vaultResult.success}, error=${vaultResult.error}, cryptoVersion=${vaultResult.cryptoVersion}');

    if (!vaultResult.success) {
      // Rust unlock failed - password incorrect or account not found in Rust
      state = AuthState.locked;
      return false;
    }

    DebugLogger.instance.logInfo('AUTH', 'Rust unlock succeeded, checking Keychain...');

    // Step 2: Rust unlock succeeded - get session key for profile encryption
    // Derive session key from password using the same params Rust used
    try {
      final accountData = await _storage.getAccountData(_selectedAccountId!).timeout(
        const Duration(seconds: 5),
        onTimeout: () => null,
      );
      DebugLogger.instance.logInfo('AUTH', 'Keychain accountData: ${accountData != null ? "found" : "null"}');
      if (accountData == null) {
        // Account not in Keychain but in Rust - migrate it
        DebugLogger.instance.logInfo('AUTH', 'Account not in Keychain, migrating from Rust...');
        await _migrateAccountFromRust(
          _selectedAccountId!,
          vaultResult.cryptoVersion ?? 2,
        );
      } else if ((accountData['crypto_version'] as int? ?? 1) < 2) {
        // V1 account - migrate to V2 after successful login
        DebugLogger.instance.logInfo('AUTH', 'V1 account detected, migrating to V2...');
        await _migrateAccountToV2(
          _selectedAccountId!,
          password,
          vaultResult.cryptoVersion ?? 2,
        );
      }
    } catch (e) {
      DebugLogger.instance.logError('AUTH', 'Migration error: $e');
      // Migration failed - continue anyway since Rust unlock succeeded
    }

    // Step 3: Get fresh account data after potential migration
    DebugLogger.instance.logInfo('AUTH', 'Getting fresh account data after migration...');
    final freshData = await _storage.getAccountData(_selectedAccountId!).timeout(
      const Duration(seconds: 5),
      onTimeout: () => null,
    );
    DebugLogger.instance.logInfo('AUTH', 'freshData: ${freshData != null ? "found" : "null"}');

    Uint8List salt;
    if (freshData == null) {
      // Keychain doesn't have this account yet - get salt from Rust directly
      DebugLogger.instance.logInfo('AUTH', 'freshData is null, getting salt from Rust...');
      final rustConfig = await Future.delayed(
        Duration.zero,
        () => NativeVaultService.instance.getAccountConfig(accountId: _selectedAccountId!),
      ).timeout(const Duration(seconds: 5), onTimeout: () => null);
      if (rustConfig?.salt == null) {
        DebugLogger.instance.logError('AUTH', 'Cannot get salt from Rust - returning false');
        state = AuthState.locked;
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
      state = AuthState.locked;
      return false;
    }

    _profileStorage.setEncryptionKey(sessionKey);

    // Reload account info
    final accounts = await _storage.listAccounts();
    _selectedAccountInfo = accounts.cast<AccountInfo?>().firstWhere(
      (a) => a?.id == _selectedAccountId,
      orElse: () => null,
    );

    state = AuthState.unlocked;

    // NOTE: Do NOT call loadProfile here directly!
    // The ref.listen in profile_provider will trigger loadProfile when state becomes unlocked.
    // Calling it directly here causes a double-load race condition with the listener.

    return true;
  }

  /// Migrate a V1 account to V2: re-derive credentials using Rust's salt/verify_hash
  Future<void> _migrateAccountToV2(
    String accountId,
    String password,
    int cryptoVersion,
  ) async {
    try {
      // Get salt from Rust vault (it's stored in config.json there)
      // For now, we re-derive using Dart's algorithm but store with V2 version marker
      final salt = NativeCryptoService.instance.generateSalt();
      if (salt == null) {
        return;
      }

      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: password,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      );
      if (verifyKey == null) {
        return;
      }

      // Update salt and verify hash
      await _storage.updateAccountSalt(accountId, salt, verifyKey);

      // Clear sensitive data from memory immediately after use
      for (var i = 0; i < salt.length; i++) {
        salt[i] = 0;
      }
      for (var i = 0; i < verifyKey.length; i++) {
        verifyKey[i] = 0;
      }

      // Update crypto version marker with retry logic
      bool versionUpdated = await _storage.updateAccountCryptoVersion(accountId, cryptoVersion);
      if (!versionUpdated) {
        // Retry once after brief delay
        await Future.delayed(const Duration(milliseconds: 100));
        versionUpdated = await _storage.updateAccountCryptoVersion(accountId, cryptoVersion);
      }
    } catch (e) {
      // Migration errors are handled silently - will retry on next login
    }
  }

  /// Migrate an account that exists in Rust but not in Dart Keychain
  /// This syncs account credentials (salt, verify_hash) from Rust to Dart Keychain
  Future<void> _migrateAccountFromRust(String accountId, int cryptoVersion) async {
    try {
      // Get account config from Rust (salt, verify_hash, name) with timeout protection
      DebugLogger.instance.logInfo('AUTH', 'Migrating account from Rust, calling getAccountConfig...');
      final rustConfig = await Future.delayed(
        Duration.zero,
        () => NativeVaultService.instance.getAccountConfig(accountId: accountId),
      ).timeout(const Duration(seconds: 5), onTimeout: () {
        DebugLogger.instance.logError('AUTH', 'getAccountConfig timed out during migration');
        return null;
      });
      DebugLogger.instance.logInfo('AUTH', 'getAccountConfig returned: ${rustConfig != null}');

      if (rustConfig == null || rustConfig.salt == null || rustConfig.verifyHash == null) {
        // Fallback: just update crypto version if we can't get full config
        try {
          await _storage.updateAccountCryptoVersion(accountId, cryptoVersion).timeout(
            const Duration(seconds: 5),
            onTimeout: () => false,
          );
        } catch (_) {}
        return;
      }

      // Decode the credentials from Rust
      // Salt is base64 (from Rust), verify_hash is hex (from Rust)
      final saltBytes = base64Decode(rustConfig.salt!);
      final verifyHashBytes = _hexToBytes(rustConfig.verifyHash!);

      // Check if account already exists in Keychain with timeout
      final accounts = await _storage.listAccounts().timeout(
        const Duration(seconds: 5),
        onTimeout: () => <AccountInfo>[],
      );
      final existingAccount = accounts.cast<AccountInfo?>().firstWhere(
        (a) => a?.id == accountId,
        orElse: () => null,
      );

      if (existingAccount == null) {
        // Account doesn't exist in Keychain at all - create it
        try {
          await _storage.saveAccountData(accountId, {
            'salt': rustConfig.salt,
            'verify_hash': rustConfig.verifyHash,
            'crypto_version': cryptoVersion,
          }).timeout(const Duration(seconds: 5), onTimeout: () {});
        } catch (_) {}
      } else {
        // Account exists but credentials are stale - update them
        try {
          await _storage.updateAccountSalt(
            accountId,
            Uint8List.fromList(saltBytes),
            Uint8List.fromList(verifyHashBytes),
          ).timeout(const Duration(seconds: 5), onTimeout: () {});
        } catch (_) {}
        try {
          await _storage.updateAccountCryptoVersion(accountId, cryptoVersion).timeout(
            const Duration(seconds: 5),
            onTimeout: () => false,
          );
        } catch (_) {}
      }
    } catch (e) {
      // Migration errors are handled silently - continue with default behavior
    }
  }

  /// Verify password for sensitive data access (does NOT change auth state)
  Future<bool> verifyPasswordForSensitiveData(String password) async {
    if (_selectedAccountId == null) return false;
    if (password.isEmpty) return false;

    // Just verify password without changing any state
    return await _storage.verifyPassword(_selectedAccountId!, password);
  }

  /// Lock the vault
  void lockVault() {
    // Lock Rust vault (clears session key, closes database)
    RustVaultService.instance.lockVault();
    // Clear encryption key
    _profileStorage.clearEncryptionKey();
    // Keep _selectedAccountId and _selectedAccountInfo so user can re-unlock
    state = AuthState.locked;
  }

  /// Check if vault exists
  Future<bool> vaultExists() async {
    final accounts = await _storage.listAccounts();
    return accounts.isNotEmpty;
  }

  /// Delete the current account
  /// Requires password verification first
  Future<bool> deleteAccount(String password) async {
    if (_selectedAccountId == null) return false;

    // Verify password first
    final isValid = await _storage.verifyPassword(_selectedAccountId!, password);
    if (!isValid) return false;

    // Delete the account from Rust vault first (removes from accounts.json and deletes account directory)
    RustVaultService.instance.deleteAccount(_selectedAccountId!);

    // Delete the account from Dart's Keychain
    final success = await _storage.deleteAccount(_selectedAccountId!);
    if (success) {
      _profileStorage.clearEncryptionKey();
      _selectedAccountId = null;
      _selectedAccountInfo = null;
      state = AuthState.locked;
    }
    return success;
  }

  /// Change master password for current account
  /// 1. Unlock vault with current password to verify
  /// 2. Load profile data with current encryption key
  /// 3. Call Rust's change_password to update config.json and get new credentials
  /// 4. Update Dart's Keychain with new salt/verify_hash
  /// 5. Derive new session key and re-save profile with new encryption
  Future<({bool success, String? error})> changePassword({
    required String currentPassword,
    required String newPassword,
    String? newPasswordHint,
  }) async {
    if (_selectedAccountId == null) {
      return (success: false, error: 'No account selected');
    }

    // Step 1: Unlock vault with current password to verify identity
    final isUnlocked = await unlockVault(currentPassword);
    if (!isUnlocked) {
      return (success: false, error: 'Invalid current password');
    }

    // Step 2: Load profile data with current encryption key BEFORE password change
    final currentProfile = await _profileStorage.loadProfile(_selectedAccountId!);

    // Step 3: Call Rust to change password (updates Rust's config.json)
    final rustResult = NativeVaultService.instance.changePassword(
      accountId: _selectedAccountId!,
      oldPassword: currentPassword,
      newPassword: newPassword,
    );

    if (rustResult == null || !rustResult.success) {
      return (success: false, error: rustResult?.error ?? 'Failed to change password in vault');
    }

    // Step 4: Update Dart's Keychain with new salt/verify_hash from Rust
    // Rust has already updated config.json with the new credentials
    Uint8List saltBytes;
    if (rustResult.salt != null && rustResult.verifyHash != null) {
      saltBytes = base64Decode(rustResult.salt!);
      final verifyHashBytes = base64Decode(rustResult.verifyHash!);
      await _storage.updateAccountSalt(
        _selectedAccountId!,
        saltBytes,
        Uint8List.fromList(verifyHashBytes),
      );
    } else {
      return (success: false, error: 'Failed to get new credentials from vault');
    }

    // Step 5: Derive new session key from new password and update encryption key
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
    _profileStorage.setEncryptionKey(newSessionKey);
    if (currentProfile != null) {
      await _profileStorage.saveProfile(_selectedAccountId!, currentProfile);
    }

    // Step 7: Update password hint if provided
    if (newPasswordHint != null) {
      await _updateAccountPasswordHint(_selectedAccountId!, newPasswordHint);
    }

    return (success: true, error: null);
  }

  /// Update the password hint for an account
  Future<void> _updateAccountPasswordHint(String accountId, String hint) async {
    await _storage.updatePasswordHint(accountId, hint);
    // Refresh local cache
    final accounts = await _storage.listAccounts();
    _selectedAccountInfo = accounts.cast<AccountInfo?>().firstWhere(
      (a) => a?.id == accountId,
      orElse: () => null,
    );
  }

  /// Update operation metadata for the current account
  Future<void> updateOperation(String operationDesc) async {
    if (_selectedAccountId == null) return;
    await _storage.updateAccountOperation(_selectedAccountId!, operationDesc);
  }
}

/// Auth state provider
final authNotifierProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  return AuthNotifier();
});

/// Accounts provider - lists all accounts sorted by recent access
final accountsProvider = FutureProvider<List<AccountInfo>>((ref) async {
  final notifier = ref.read(authNotifierProvider.notifier);
  // Watch the notifier object - it only changes when account selection changes,
  // NOT when lock/unlock state changes
  ref.watch(authNotifierProvider.notifier);
  return notifier.getAccountsSortedByRecent();
});

/// 敏感数据访问验证的有效期（唯一常量）
const kSensitiveAccessTimeout = Duration(minutes: 1);

/// Sensitive page access state - tracks password verification for sensitive pages
/// 唯一真理来源：内部决定验证是否仍然有效
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

  /// 标记为已验证，并启动定时器在超时后主动失效
  void markVerified() {
    // 取消之前的定时器
    _timer?.cancel();
    state = SensitivePageAccessState(lastVerified: DateTime.now());
    // 启动新的定时器，在超时后主动触发状态更新
    _timer = Timer(kSensitiveAccessTimeout, () {
      // 时间到，通过 copyWith 重新创建状态（触发 notifyListeners）
      // 这确保所有订阅者在超时那一秒立即重新渲染
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

/// Provider for sensitive page access (shared between Operation Log and Sensitivity Settings)
final sensitivePageAccessProvider =
    StateNotifierProvider<SensitivePageAccessNotifier, SensitivePageAccessState>((ref) {
  return SensitivePageAccessNotifier();
});

/// 单一真理来源：UI 只需关注"我现在能不能看"敏感数据
/// 当 1 分钟超时发生时，这个 Provider 会自动通知所有订阅者
final isSensitiveAccessGrantedProvider = Provider<bool>((ref) {
  final access = ref.watch(sensitivePageAccessProvider);
  return access.isValid;
});
