import 'dart:convert';
import 'dart:typed_data';

import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// Service for managing user UI preferences with optional encryption.
///
/// Data is stored via [FallbackSecureStorage] (Keychain primary, file fallback).
/// When the vault is unlocked, an additional layer of encryption is applied
/// via Rust FRB (SOLO blob format).
///
/// Stored JSON schema:
/// ```json
/// {
///   "quick_action_routes": ["/profile", "/travel", ...]
/// }
/// ```
class UserPreferencesService {
  static const _keyPrefix = 'user_preferences_v1';
  static UserPreferencesService? _instance;

  final _storage = FallbackSecureStorage();

  UserPreferencesService._();

  static UserPreferencesService get instance {
    _instance ??= UserPreferencesService._();
    return _instance!;
  }

  String _storageKey(String accountId) => '${_keyPrefix}_$accountId';

  // ---------------------------------------------------------------------------
  // Quick Actions
  // ---------------------------------------------------------------------------

  /// Persist the ordered list of quick-action route paths.
  /// [accountId] ensures data is isolated per-account.
  Future<void> saveQuickActions(List<String> routes, String accountId) async {
    final json = jsonEncode({'quick_action_routes': routes});
    final payload = await _encrypt(json);
    await _storage.write(key: _storageKey(accountId), value: payload);
  }

  /// Load the ordered list of quick-action route paths.
  /// [accountId] ensures data is isolated per-account.
  /// Returns an empty list when nothing has been saved yet.
  Future<List<String>> loadQuickActions(String accountId) async {
    final payload = await _storage.read(key: _storageKey(accountId));
    if (payload == null || payload.isEmpty) return [];

    final json = await _decrypt(payload, accountId);
    if (json == null || json.isEmpty) return [];

    try {
      final data = jsonDecode(json) as Map<String, dynamic>;
      return (data['quick_action_routes'] as List<dynamic>?)?.cast<String>() ??
          [];
    } on Object {
      return [];
    }
  }

  // ---------------------------------------------------------------------------
  // Encryption helpers (via Rust FRB when vault is unlocked)
  // ---------------------------------------------------------------------------

  /// Encrypts [plaintext] when the vault is unlocked.
  /// Falls back to returning the plaintext as-is when the vault is locked.
  Future<String> _encrypt(String plaintext) async {
    if (!RustVaultService.instance.isVaultUnlocked()) {
      return plaintext;
    }

    try {
      final data = Uint8List.fromList(utf8.encode(plaintext));
      final encrypted = await frb.frbEncryptBytes(data: data);
      if (encrypted.isEmpty) return plaintext;

      return 'enc:${base64Encode(encrypted)}';
    } on Object {
      return plaintext;
    }
  }

  /// Decrypts a payload produced by [_encrypt].
  /// Handles both encrypted ('enc:...') and plain JSON payloads.
  /// On failure, clears the stale data so it doesn't recur on every launch.
  Future<String?> _decrypt(String payload, String accountId) async {
    if (!payload.startsWith('enc:')) {
      return payload;
    }

    try {
      final combined = base64Decode(payload.substring(4));
      final decrypted = await frb.frbDecryptBytes(data: combined);
      return utf8.decode(decrypted);
    } on Object {
      // FRB handles both SOLO blob and legacy Dart formats.
      // If decryption fails (e.g. account switched or password changed),
      // clear the stale encrypted data to prevent repeated errors.
      await _storage.write(key: _storageKey(accountId), value: null);
      return null;
    }
  }
}
