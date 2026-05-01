import 'dart:convert';
import 'dart:typed_data';

import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

part 'rust_vault_service.g.dart';

/// Bridge profile summary returned from Rust FFI
@JsonSerializable(fieldRename: FieldRename.snake)
class BridgeProfileSummary {
  final String id;
  final String name;
  final String createdAt;
  final String updatedAt;
  final int version;

  const BridgeProfileSummary({
    required this.id,
    required this.name,
    required this.createdAt,
    required this.updatedAt,
    required this.version,
  });

  /// Deprecated: using generated fromJson
  @Deprecated('Use the generated fromJson instead')
  factory BridgeProfileSummary.fromJson(Map<String, dynamic> json) =>
      _$BridgeProfileSummaryFromJson(json);

  /// Deprecated: using generated toJson
  @Deprecated('Use the generated toJson instead')
  Map<String, dynamic> toJson() => _$BridgeProfileSummaryToJson(this);
}

/// Rust Vault Service - Flutter wrapper around Rust VaultStore via FFI
///
/// This service provides CRUD operations for profiles stored in the SQLCipher-
/// encrypted vault through the native FFI layer.
class RustVaultService {
  static RustVaultService? _instance;

  RustVaultService._();

  static RustVaultService get instance {
    _instance ??= RustVaultService._();
    return _instance!;
  }

  /// Encryption key derived from master password (set after unlock).
  /// Phase 1: kept for backward compatibility during migration.
  /// The key now lives in Rust's AccountManager session — Dart only holds
  /// a copy for the transition period. Will be removed in Phase 3.
  Uint8List? _encryptionKey;

  /// Set the encryption key (derived from master password via Argon2id).
  /// Makes a defensive copy so callers can safely wipe their original buffer.
  /// DEPRECATED: Key is now managed by Rust. This will be removed in Phase 3.
  void setEncryptionKey(Uint8List key) {
    _encryptionKey = Uint8List.fromList(key);
  }

  /// Get the encryption key
  /// DEPRECATED: Key is now managed by Rust. This will be removed in Phase 3.
  Uint8List? get encryptionKey => _encryptionKey;

  /// Clear the encryption key (on lock) - securely zero the buffer
  void clearEncryptionKey() {
    if (_encryptionKey != null) {
      for (var i = 0; i < _encryptionKey!.length; i++) {
        _encryptionKey![i] = 0;
      }
    }
    _encryptionKey = null;
  }

  /// Encrypt data via Rust FFI (AES-256-GCM, SOLO blob format).
  /// Returns base64-encoded SOLO blob, or null on failure.
  String? _encryptViaRust(Uint8List data) {
    final result = NativeVaultService.instance.request(
      'encrypt_data',
      {'data': base64Encode(data)},
    );
    if (result?['success'] != true) return null;
    return result!['data']?['data'] as String?;
  }

  /// Decrypt data via Rust FFI (auto-detects SOLO blob or legacy format).
  /// Returns raw bytes, or null on failure.
  Uint8List? _decryptViaRust(Uint8List combined) {
    final result = NativeVaultService.instance.request(
      'decrypt_data',
      {'data': base64Encode(combined)},
    );
    if (result?['success'] != true) return null;
    final dataB64 = result!['data']?['data'] as String?;
    if (dataB64 == null) return null;
    return base64Decode(dataB64);
  }

  /// Encrypt profile data — delegates to Rust.
  /// Returns encrypted bytes (SOLO blob format), or null on failure.
  Uint8List? _encryptData(Uint8List data) {
    final b64 = _encryptViaRust(data);
    if (b64 == null) return null;
    return base64Decode(b64);
  }

  /// Decrypt profile data — delegates to Rust (auto-detects format).
  /// Returns raw plaintext bytes, or null on failure.
  Uint8List? _decryptData(Uint8List combined) {
    return _decryptViaRust(combined);
  }

  // ===========================================================================
  // Public encryption helpers for external services (e.g. BackupService)
  // ===========================================================================

  /// Encrypt arbitrary bytes using the current encryption key.
  /// Returns nonce(12B) + ciphertext combined, or null if key not set.
  Uint8List? encryptBytes(Uint8List data) => _encryptData(data);

  /// Decrypt bytes that were encrypted with [encryptBytes].
  /// Expects nonce + ciphertext combined format.
  Uint8List? decryptBytes(Uint8List combined) => _decryptData(combined);

  // ===========================================================================
  // FFI Bridge calls via NativeVaultService (JSON Relay Pattern)
  // ===========================================================================

  /// Initialize account manager with base path
  bool initAccountManager(String basePath) {
    return NativeVaultService.instance.initAccountManager(basePath);
  }

  /// Check if vault is unlocked
  bool isVaultUnlocked() {
    return NativeVaultService.instance.isVaultUnlocked();
  }

  /// Save a profile (create or update)
  ///
  /// [name] - Profile name
  /// [data] - Encrypted profile data
  ///
  /// Returns the profile summary on success
  Future<BridgeProfileSummary?> saveProfile(String name, Uint8List data) async {
    final result = NativeVaultService.instance.saveProfile(name, data);
    if (result == null) return null;
    return BridgeProfileSummary.fromJson(result);
  }

  /// Load a profile by ID
  ///
  /// [id] - Profile ID
  ///
  /// Returns the encrypted profile data (to be decrypted by caller), or null if not found
  Future<Uint8List?> loadProfile(String id) async {
    final result = NativeVaultService.instance.loadProfile(id);
    if (result == null) {
      return null;
    }
    return result.data;
  }

  /// Delete a profile by ID
  ///
  /// [id] - Profile ID
  ///
  /// Returns true if deleted successfully
  Future<bool> deleteProfile(String id) async {
    return NativeVaultService.instance.deleteProfile(id);
  }

  /// List all profile summaries
  ///
  /// Returns list of profile summaries (without encrypted data)
  Future<List<BridgeProfileSummary>> listProfiles() async {
    final result = NativeVaultService.instance.listProfiles();
    if (result == null) return [];
    return result.map((json) => BridgeProfileSummary.fromJson(json)).toList();
  }

  /// Create a new account in the Rust vault
  /// This must be called before unlockVault for new accounts
  ({bool success, String? error, String? accountId, String? name, String? salt, String? verifyHash}) createAccount({
    required String name,
    required String password,
  }) {
    // NOTE: DebugLogger removed - synchronous file I/O may cause hangs
    final result = NativeVaultService.instance.createAccount(name: name, password: password);
    if (result == null) {
      return (success: false, error: 'Failed to create account', accountId: null, name: null, salt: null, verifyHash: null);
    }
    return (
      success: result.success,
      error: result.error,
      accountId: result.accountId,
      name: result.name,
      salt: result.salt,
      verifyHash: result.verifyHash,
    );
  }

  /// Unlock the vault with account credentials
  /// This opens the encrypted SQLCipher database
  ({bool success, String? error, int? cryptoVersion}) unlockVault({
    required String accountId,
    required String password,
  }) {
    final result = NativeVaultService.instance.unlockVault(accountId: accountId, password: password);
    return result ?? (success: false, error: 'Failed to unlock vault', cryptoVersion: null);
  }

  /// Unlock the vault with a pre-derived session key (for biometric unlock)
  /// This opens the encrypted SQLCipher database without password derivation
  ({bool success, String? error, int? cryptoVersion}) unlockVaultWithKey({
    required String accountId,
    required Uint8List sessionKey,
  }) {
    final payload = {
      'account_id': accountId,
      'session_key': base64Encode(sessionKey),
    };
    final response = NativeVaultService.instance.request(
      NativeVaultService.actionUnlockVaultWithKey,
      payload,
    );
    if (response == null || response['success'] != true) {
      return (
        success: false,
        error: response?['error'] as String? ?? 'Failed to unlock vault with key',
        cryptoVersion: null,
      );
    }
    final data = response['data'] as Map<String, dynamic>?;
    return (
      success: data?['success'] == true,
      error: data?['error'] as String?,
      cryptoVersion: data?['crypto_version'] as int?,
    );
  }

  /// Lock the vault - clears session key and closes database connection
  void lockVault() {
    clearEncryptionKey();
    NativeVaultService.instance.lockVault();
  }

  /// Delete an account and all its data from Rust vault
  bool deleteAccount(String accountId) {
    return NativeVaultService.instance.deleteAccount(accountId: accountId);
  }

  /// Get vault statistics
  Map<String, dynamic>? getVaultStats() {
    return NativeVaultService.instance.getVaultStats();
  }

  /// List all accounts from Rust vault (single source of truth)
  /// Uses JSON relay through NativeVaultService
  List<Map<String, dynamic>>? listAccountsFromRust() {
    return NativeVaultService.instance.listAccounts();
  }

  // ===========================================================================
  // High-level operations with encryption/decryption
  // ===========================================================================

  /// Save a profile with encryption
  ///
  /// [name] - Profile name
  /// [jsonData] - Profile data as JSON string
  ///
  /// Returns the profile summary on success
  Future<BridgeProfileSummary?> saveProfileEncrypted(
    String name,
    String jsonData,
  ) async {
    // Use FRB: encrypt via Rust, then save to vault
    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));

    final encryptedData = await frb.frbEncryptBytes(data: jsonBytes);
    if (encryptedData.isEmpty) {
      return null;
    }

    final summary = await frb.frbSaveProfile(name: name, data: encryptedData);
    return BridgeProfileSummary(
      id: summary.id,
      name: summary.name,
      createdAt: summary.createdAt,
      updatedAt: summary.updatedAt,
      version: summary.version,
    );
  }

  /// Load and decrypt a profile by ID
  ///
  /// [id] - Profile ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadProfileDecrypted(String id) async {
    // Use FRB: load from vault, then decrypt via Rust
    final loaded = await frb.frbLoadProfile(id: id);
    if (loaded == null) {
      return null;
    }

    final decrypted = await frb.frbDecryptBytes(data: loaded.data);
    return utf8.decode(decrypted);
  }

  // ===========================================================================
  // Field histories - encrypted storage
  // ===========================================================================

  /// Save field histories with encryption
  ///
  /// [accountId] - Account ID
  /// [jsonData] - Field histories data as JSON string
  ///
  /// Returns true on success
  Future<bool> saveFieldHistoriesEncrypted(
    String accountId,
    String jsonData,
  ) async {
    if (_encryptionKey == null) {
      return false;
    }

    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = _encryptData(jsonBytes);
    if (encryptedData == null) {
      return false;
    }

    final result = NativeVaultService.instance.request(
      'save_field_histories',
      {
        'account_id': accountId,
        'data': base64Encode(encryptedData),
      },
    );

    return result?['success'] == true;
  }

  /// Load and decrypt field histories by account ID
  ///
  /// [accountId] - Account ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadFieldHistoriesDecrypted(String accountId) async {
    final result = NativeVaultService.instance.request(
      'load_field_histories',
      {'account_id': accountId},
    );

    if (result?['success'] != true) {
      return null;
    }

    // Rust returns: {"success": true, "data": {"data": "base64..."}}
    // Need to unwrap one layer like loadProfile does.
    final responseData = result!['data'] as Map<String, dynamic>?;
    if (responseData == null) {
      return null;
    }

    final data = responseData['data'];
    if (data == null) {
      return null;
    }

    if (data is String) {
      // data is base64 encoded encrypted data
      final encryptedBytes = base64Decode(data);
      final decrypted = _decryptData(encryptedBytes);
      if (decrypted == null) {
        return null;
      }
      return utf8.decode(decrypted);
    }

    return null;
  }

  /// Delete field histories for an account
  ///
  /// [accountId] - Account ID
  ///
  /// Returns true on success
  Future<bool> deleteFieldHistories(String accountId) async {
    final result = NativeVaultService.instance.request(
      'delete_field_histories',
      {'account_id': accountId},
    );

    return result?['success'] == true;
  }

  // ===========================================================================
  // Account settings - encrypted storage (SETTING_{accountId} pattern)
  // ===========================================================================

  /// Save account settings with encryption
  ///
  /// [accountId] - Account ID
  /// [jsonData] - Settings data as JSON string
  ///
  /// Returns true on success
  Future<bool> saveSettingEncrypted(
    String accountId,
    String jsonData,
  ) async {
    if (_encryptionKey == null) {
      return false;
    }

    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = _encryptData(jsonBytes);
    if (encryptedData == null) {
      return false;
    }

    final result = NativeVaultService.instance.request(
      'save_setting',
      {
        'account_id': accountId,
        'data': base64Encode(encryptedData),
      },
    );

    return result?['success'] == true;
  }

  /// Load and decrypt account settings by account ID
  ///
  /// [accountId] - Account ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadSettingDecrypted(String accountId) async {
    final result = NativeVaultService.instance.request(
      'load_setting',
      {'account_id': accountId},
    );

    if (result?['success'] != true) {
      return null;
    }

    // Rust returns: {"success": true, "data": {"data": "base64..."}}
    // Need to unwrap one layer like loadProfile does.
    final responseData = result!['data'] as Map<String, dynamic>?;
    if (responseData == null) {
      return null;
    }

    final data = responseData['data'];
    if (data == null) {
      return null;
    }

    if (data is String) {
      // data is base64 encoded encrypted data
      final encryptedBytes = base64Decode(data);
      final decrypted = _decryptData(encryptedBytes);
      if (decrypted == null) {
        return null;
      }
      return utf8.decode(decrypted);
    }

    return null;
  }

  /// Delete account settings for an account
  ///
  /// [accountId] - Account ID
  ///
  /// Returns true on success
  Future<bool> deleteSetting(String accountId) async {
    final result = NativeVaultService.instance.request(
      'delete_setting',
      {'account_id': accountId},
    );

    return result?['success'] == true;
  }
}
