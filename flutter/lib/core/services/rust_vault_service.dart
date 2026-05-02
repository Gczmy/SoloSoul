import 'dart:convert';
import 'dart:typed_data';

import 'package:json_annotation/json_annotation.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
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

  /// Base path for vault storage (set during init)
  String? _vaultRoot;
  String? get vaultRoot => _vaultRoot;

  // ===========================================================================
  // Public encryption helpers for external services (e.g. BackupService)
  // ===========================================================================

  /// Encrypt arbitrary bytes via Rust FRB (AES-256-GCM, SOLO blob format).
  /// Vault must be unlocked. Returns encrypted SOLO blob bytes.
  Future<Uint8List?> encryptBytes(Uint8List data) async {
    final result = await frb.frbEncryptBytes(data: data);
    return result.isEmpty ? null : result;
  }

  /// Decrypt SOLO blob bytes via Rust FRB.
  /// Vault must be unlocked. Returns plaintext bytes.
  Future<Uint8List?> decryptBytes(Uint8List combined) async {
    try {
      return await frb.frbDecryptBytes(data: combined);
    } on Exception {
      return null;
    }
  }

  // ===========================================================================
  // FFI Bridge calls via NativeVaultService (JSON Relay Pattern)
  // ===========================================================================

  /// Initialize account manager with base path
  Future<bool> initAccountManager(String basePath) async {
    try {
      await frb.frbInitAccountManager(basePath: basePath);
      _vaultRoot = basePath;
      SoloLog.d('RustVault', 'initAccountManager succeeded: $basePath');
      return true;
    } on Exception catch (e, st) {
      SoloLog.e('RustVault', 'initAccountManager failed', e, st);
      return false;
    }
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
  Future<({bool success, String? error, String? accountId, String? name, String? salt, String? verifyHash})> createAccount({
    required String name,
    required String password,
  }) async {
    final result = await frb.frbCreateAccount(name: name, password: password);
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
  Future<({bool success, String? error, int? cryptoVersion})> unlockVault({
    required String accountId,
    required String password,
  }) async {
    final result = await frb.frbUnlockVault(accountId: accountId, password: password);
    return (
      success: result.success,
      error: result.error,
      cryptoVersion: result.cryptoVersion,
    );
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
  Future<void> lockVault() async {
    await frb.frbLockVault();
  }

  /// Delete an account and all its data from Rust vault
  Future<bool> deleteAccount(String accountId) async {
    return frb.frbDeleteAccount(accountId: accountId);
  }

  /// Get vault statistics
  Future<frb.VaultStats?> getVaultStats() async {
    return frb.frbGetVaultStats();
  }

  /// List all accounts from Rust vault (single source of truth)
  Future<List<Map<String, dynamic>>> listAccountsFromRust() async {
    final accounts = await frb.frbListAccounts();
    return accounts.map((a) => {
      'id': a.id,
      'name': a.name,
      if (a.lastAccessed != null) 'last_accessed': a.lastAccessed!,
      if (a.passwordHint != null) 'password_hint': a.passwordHint!,
      if (a.lastLoginAt != null) 'last_login_at': a.lastLoginAt!,
      if (a.lastOperationAt != null) 'last_operation_at': a.lastOperationAt!,
      if (a.lastOperationDesc != null) 'last_operation_desc': a.lastOperationDesc!,
    }).toList();
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
    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = await frb.frbEncryptBytes(data: jsonBytes);
    if (encryptedData.isEmpty) {
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

    final responseData = result!['data'] as Map<String, dynamic>?;
    if (responseData == null) {
      return null;
    }

    final data = responseData['data'];
    if (data == null) {
      return null;
    }

    if (data is String) {
      final encryptedBytes = base64Decode(data);
      try {
        final decrypted = await frb.frbDecryptBytes(data: encryptedBytes);
        return utf8.decode(decrypted);
      } on Exception {
        return null;
      }
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
    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = await frb.frbEncryptBytes(data: jsonBytes);
    if (encryptedData.isEmpty) {
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

    final responseData = result!['data'] as Map<String, dynamic>?;
    if (responseData == null) {
      return null;
    }

    final data = responseData['data'];
    if (data == null) {
      return null;
    }

    if (data is String) {
      final encryptedBytes = base64Decode(data);
      try {
        final decrypted = await frb.frbDecryptBytes(data: encryptedBytes);
        return utf8.decode(decrypted);
      } on Exception {
        return null;
      }
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
