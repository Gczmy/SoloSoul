import 'dart:convert';

import 'package:flutter/foundation.dart';
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

  factory BridgeProfileSummary.fromJson(Map<String, dynamic> json) =>
      _$BridgeProfileSummaryFromJson(json);

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
    } on Object {
      return null;
    }
  }

  // ===========================================================================
  // FFI Bridge calls via NativeVaultService (JSON Relay Pattern)
  // ===========================================================================

  /// Initialize account manager with base path.
  /// Initializes BOTH the FRB AccountManager and the C FFI AccountManager
  /// (JSON relay) so that all code paths can access account data.
  Future<bool> initAccountManager(String basePath) async {
    try {
      await frb.frbInitAccountManager(basePath: basePath);
      _vaultRoot = basePath;
      SoloLog.d('RustVault', 'initAccountManager (FRB) succeeded: $basePath');
    } on Object catch (e, st) {
      SoloLog.e('RustVault', 'initAccountManager (FRB) failed', e, st);
      return false;
    }

    // Also initialize the C FFI AccountManager for JSON relay operations
    final cResult = NativeVaultService.instance.initAccountManager(basePath);
    SoloLog.d('RustVault', 'initAccountManager (C FFI) result: $cResult');
    if (!cResult) {
      throw Exception('C FFI AccountManager init failed');
    }

    return true;
  }

  /// Check if vault is unlocked
  bool isVaultUnlocked() {
    return NativeVaultService.instance.isVaultUnlocked();
  }

  // ---------------------------------------------------------------------------
  // Generic encrypted JSON helpers
  // ---------------------------------------------------------------------------

  Future<bool> _saveEncryptedJson(
    String accountId,
    String jsonData,
    String operation,
  ) async {
    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = await frb.frbEncryptBytes(data: jsonBytes);
    if (encryptedData.isEmpty) {
      return false;
    }

    final result = NativeVaultService.instance.request(
      operation,
      {
        'account_id': accountId,
        'data': base64Encode(encryptedData),
      },
    );

    return result?['success'] == true;
  }

  Future<String?> _loadDecryptedJson(
    String accountId,
    String operation,
    String logLabel,
  ) async {
    final result = NativeVaultService.instance.request(
      operation,
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
      } on Object catch (e) {
        SoloLog.w('RustVault', '$logLabel decryption failed (likely stale key after password change): $e');
        return null;
      }
    }

    return null;
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
  ///
  /// Merges FRB account data (id, name, timestamps) with C FFI data
  /// (recent_devices) since FRB AccountInfo doesn't carry device entries.
  Future<List<Map<String, dynamic>>> listAccountsFromRust() async {
    // FRB path — typed account fields
    final accounts = await frb.frbListAccounts();

    // C FFI path — get recent_devices from JSON relay
    final Map<String, List<dynamic>> devicesByAccount = {};
    try {
      final cResult = NativeVaultService.instance.request(
        'list_accounts',
        {},
      );
      if (cResult?['success'] == true) {
        final data = cResult!['data'] as Map<String, dynamic>?;
        final accountList = data?['accounts'] as List<dynamic>?;
        if (accountList != null) {
          for (final entry in accountList) {
            final m = entry as Map<String, dynamic>;
            final id = m['id'] as String? ?? '';
            final devs = m['recent_devices'] as List<dynamic>? ?? [];
            if (id.isNotEmpty) {
              devicesByAccount[id] = devs;
            }
          }
        }
      }
    } on Object {
      // Non-fatal — devices may just be empty
    }

    return accounts.map((a) => {
      'id': a.id,
      'name': a.name,
      if (a.createdAt != null) 'created_at': a.createdAt!,
      if (a.lastAccessed != null) 'last_accessed': a.lastAccessed!,
      if (a.passwordHint != null) 'password_hint': a.passwordHint!,
      if (a.lastLoginAt != null) 'last_login_at': a.lastLoginAt!,
      if (a.lastOperationAt != null) 'last_operation_at': a.lastOperationAt!,
      if (a.lastOperationDesc != null) 'last_operation_desc': a.lastOperationDesc!,
      'recent_devices': devicesByAccount[a.id] ?? [],
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
    return _saveEncryptedJson(accountId, jsonData, 'save_field_histories');
  }

  /// Load and decrypt field histories by account ID
  ///
  /// [accountId] - Account ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadFieldHistoriesDecrypted(String accountId) async {
    return _loadDecryptedJson(accountId, 'load_field_histories', 'Field histories');
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
  // Scan config - encrypted storage (SCAN_CONFIG_{accountId} pattern)
  // ===========================================================================

  /// Save scan configuration with encryption
  ///
  /// [accountId] - Account ID
  /// [jsonData] - JSON string of scan configuration
  ///
  /// Returns true on success
  Future<bool> saveScanConfigEncrypted(
    String accountId,
    String jsonData,
  ) async {
    return _saveEncryptedJson(accountId, jsonData, 'save_scan_config');
  }

  /// Load and decrypt scan configuration by account ID
  ///
  /// [accountId] - Account ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadScanConfigDecrypted(String accountId) async {
    return _loadDecryptedJson(accountId, 'load_scan_config', 'Scan config');
  }

  /// Delete scan configuration for an account
  ///
  /// [accountId] - Account ID
  ///
  /// Returns true on success
  // ===========================================================================
  // Metadata migration helpers (called after password change)
  // ===========================================================================

  /// Re-encrypt all metadata entries for an account after password change.
  /// Loads each encrypted metadata block, decrypts with old key, re-encrypts
  /// with current (new) key, and saves back.
  ///
  /// Returns true if all entries were migrated successfully.
  Future<bool> migrateMetadataAfterPasswordChange(String accountId) async {
    final migratedScanConfig = await _migrateScanConfig(accountId);
    final migratedSettings = await _migrateAccountSettings(accountId);
    final migratedFieldHistories = await _migrateFieldHistories(accountId);

    SoloLog.d('RustVault',
        'Metadata migration: scanConfig=$migratedScanConfig, '
        'settings=$migratedSettings, fieldHistories=$migratedFieldHistories');

    return migratedScanConfig && migratedSettings && migratedFieldHistories;
  }

  Future<bool> _migrateScanConfig(String accountId) async {
    final decrypted = await loadScanConfigDecrypted(accountId);
    if (decrypted == null) return true; // nothing to migrate
    return saveScanConfigEncrypted(accountId, decrypted);
  }

  Future<bool> _migrateAccountSettings(String accountId) async {
    final decrypted = await loadSettingDecrypted(accountId);
    if (decrypted == null) return true;
    try {
      await saveSettingEncrypted(accountId, decrypted);
      return true;
    } on Object catch (e) {
      SoloLog.w('RustVault', 'Failed to re-encrypt settings for $accountId: $e');
      return false;
    }
  }

  Future<bool> _migrateFieldHistories(String accountId) async {
    final decrypted = await loadFieldHistoriesDecrypted(accountId);
    if (decrypted == null) return true;
    return saveFieldHistoriesEncrypted(accountId, decrypted);
  }

  Future<bool> deleteScanConfig(String accountId) async {
    final result = NativeVaultService.instance.request(
      'delete_scan_config',
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
  Future<void> saveSettingEncrypted(
    String accountId,
    String jsonData,
  ) async {
    // 诊断：对比 FRB 和 C FFI 的 vault 状态（仅在 debug 模式）
    if (kDebugMode) {
      try {
        await frb.frbEncryptBytes(data: Uint8List.fromList([0]));
        SoloLog.d('RustVault', 'DIAG: FRB encrypt success (vault unlocked in FRB)');
      } on Object catch (e) {
        SoloLog.d('RustVault', 'DIAG: FRB encrypt failed: $e');
      }
      final cffiUnlocked = isVaultUnlocked();
      SoloLog.d('RustVault', 'DIAG: C FFI isVaultUnlocked=$cffiUnlocked');
    }

    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = await frb.frbEncryptBytes(data: jsonBytes);

    final result = NativeVaultService.instance.request(
      'save_setting',
      {
        'account_id': accountId,
        'data': base64Encode(encryptedData),
      },
    );

    if (result == null) {
      throw Exception('save_setting returned null response');
    }
    if (result['success'] != true) {
      final error = result['error'] as String? ?? 'Unknown error';
      throw Exception('save_setting failed: $error');
    }
  }

  /// Load and decrypt account settings by account ID
  ///
  /// [accountId] - Account ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadSettingDecrypted(String accountId) async {
    return _loadDecryptedJson(accountId, 'load_setting', 'Setting');
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
