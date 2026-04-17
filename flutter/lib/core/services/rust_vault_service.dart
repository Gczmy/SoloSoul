import 'dart:convert';
import 'dart:typed_data';

import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';

/// Bridge profile summary returned from Rust FFI
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

  factory BridgeProfileSummary.fromJson(Map<String, dynamic> json) {
    return BridgeProfileSummary(
      id: json['id'] as String,
      name: json['name'] as String,
      createdAt: json['created_at'] as String,
      updatedAt: json['updated_at'] as String,
      version: json['version'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'created_at': createdAt,
    'updated_at': updatedAt,
    'version': version,
  };
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

  /// Clear the encryption key (on lock)
  void clearEncryptionKey() {
    _encryptionKey = null;
  }

  /// Encrypt profile data using AES-256-GCM
  ///
  /// Returns nonce + ciphertext combined, or null on failure
  Uint8List? _encryptData(Uint8List data) {
    if (_encryptionKey == null) return null;

    final nonce = NativeCryptoService.instance.generateSalt();
    if (nonce == null) return null;

    // Use first 12 bytes of 32-byte salt as nonce
    final nonce12 = Uint8List.fromList(nonce.sublist(0, 12));

    final encrypted = NativeCryptoService.instance.encrypt(
      data: data,
      key: _encryptionKey!,
      nonce: nonce12,
    );
    if (encrypted == null) return null;

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
    if (_encryptionKey == null) return null;
    if (combined.length < 13) return null;

    final nonce = combined.sublist(0, 12);
    final encryptedData = combined.sublist(12);

    return NativeCryptoService.instance.decrypt(
      encrypted: encryptedData,
      key: _encryptionKey!,
      nonce: Uint8List.fromList(nonce),
    );
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
    return result?.data;
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
    final result = NativeVaultService.instance.createAccount(name: name, password: password);
    return result ?? (success: false, error: 'Failed to create account', accountId: null, name: null, salt: null, verifyHash: null);
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

  /// Get vault statistics
  Map<String, dynamic>? getVaultStats() {
    return NativeVaultService.instance.getVaultStats();
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
    if (_encryptionKey == null) return null;

    final jsonBytes = Uint8List.fromList(utf8.encode(jsonData));
    final encryptedData = _encryptData(jsonBytes);
    if (encryptedData == null) return null;

    return saveProfile(name, encryptedData);
  }

  /// Load and decrypt a profile by ID
  ///
  /// [id] - Profile ID
  ///
  /// Returns decrypted JSON string, or null if not found/error
  Future<String?> loadProfileDecrypted(String id) async {
    final encryptedData = await loadProfile(id);
    if (encryptedData == null) return null;

    final decrypted = _decryptData(encryptedData);
    if (decrypted == null) return null;

    return utf8.decode(decrypted);
  }
}
