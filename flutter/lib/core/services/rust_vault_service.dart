import 'dart:convert';
import 'dart:typed_data';

import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';

part 'rust_vault_service.g.dart';

/// Bridge profile summary returned from Rust FFI
@JsonSerializable()
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
  @deprecated
  factory BridgeProfileSummary.fromJson(Map<String, dynamic> json) =>
      _$BridgeProfileSummaryFromJson(json);

  /// Deprecated: using generated toJson
  @deprecated
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

  /// Encryption key derived from master password (set after unlock)
  Uint8List? _encryptionKey;

  /// Set the encryption key (derived from master password via Argon2id)
  void setEncryptionKey(Uint8List key) {
    _encryptionKey = key;
  }

  /// Get the encryption key
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

  /// Encrypt profile data using AES-256-GCM
  ///
  /// Returns nonce + ciphertext combined, or null on failure
  Uint8List? _encryptData(Uint8List data) {
    if (_encryptionKey == null) {
      return null;
    }

    final nonce = NativeCryptoService.instance.generateSalt();
    if (nonce == null) {
      return null;
    }

    // Use first 12 bytes of 32-byte salt as nonce
    final nonce12 = Uint8List.fromList(nonce.sublist(0, 12));

    final encrypted = NativeCryptoService.instance.encrypt(
      data: data,
      key: _encryptionKey!,
      nonce: nonce12,
    );
    if (encrypted == null) {
      return null;
    }

    // Combine nonce + ciphertext
    final combined = Uint8List(12 + encrypted.length);
    combined.setRange(0, 12, nonce12);
    combined.setRange(12, combined.length, encrypted);
    return combined;
  }

  /// Decrypt profile data using AES-256-GCM
  ///
  /// Expects nonce + ciphertext combined format
  Uint8List? _decryptData(Uint8List combined) {
    if (_encryptionKey == null) {
      return null;
    }
    if (combined.length < 13) {
      return null;
    }

    final nonce = combined.sublist(0, 12);
    final encryptedData = combined.sublist(12);

    final result = NativeCryptoService.instance.decrypt(
      encrypted: encryptedData,
      key: _encryptionKey!,
      nonce: Uint8List.fromList(nonce),
    );
    if (result == null) {
      return null;
    }
    return result;
  }

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

    if (_encryptionKey == null) {
      return null;
    }

    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));

    final encryptedData = _encryptData(jsonBytes);
    if (encryptedData == null) {
      return null;
    }

    final result = await saveProfile(name, encryptedData);
    return result;
  }

  /// Load and decrypt a profile by ID
  ///
  /// [id] - Profile ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadProfileDecrypted(String id) async {

    final encryptedData = await loadProfile(id);
    if (encryptedData == null) {
      return null;
    }

    final decrypted = _decryptData(encryptedData);
    if (decrypted == null) {
      return null;
    }

    final result = utf8.decode(decrypted);
    return result;
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

    final data = result!['data'];
    if (data == null) {
      return null;
    }

    // Handle both encrypted (base64 string) and already-decrypted (Map) formats
    if (data is String) {
      // data is base64 encoded encrypted data
      final encryptedBytes = base64Decode(data);
      final decrypted = _decryptData(encryptedBytes);
      if (decrypted == null) {
        return null;
      }
      return utf8.decode(decrypted);
    } else if (data is Map) {
      // data is already decrypted JSON
      return jsonEncode(data);
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

    final data = result!['data'];
    if (data == null) {
      return null;
    }

    // Handle both encrypted (base64 string) and already-decrypted (Map) formats
    if (data is String) {
      // data is base64 encoded encrypted data
      final encryptedBytes = base64Decode(data);
      final decrypted = _decryptData(encryptedBytes);
      if (decrypted == null) {
        return null;
      }
      return utf8.decode(decrypted);
    } else if (data is Map) {
      // data is already decrypted JSON
      return jsonEncode(data);
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
