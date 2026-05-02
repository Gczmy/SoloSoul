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
  static const _key = 'user_preferences_v1';
  static UserPreferencesService? _instance;

  final _storage = FallbackSecureStorage();

  UserPreferencesService._();

  static UserPreferencesService get instance {
    _instance ??= UserPreferencesService._();
    return _instance!;
  }

  // ---------------------------------------------------------------------------
  // Quick Actions
  // ---------------------------------------------------------------------------

  /// Persist the ordered list of quick-action route paths.
  Future<void> saveQuickActions(List<String> routes) async {
    final json = jsonEncode({'quick_action_routes': routes});
    final payload = await _encrypt(json);
    await _storage.write(key: _key, value: payload);
  }

  /// Load the ordered list of quick-action route paths.
  /// Returns an empty list when nothing has been saved yet.
  Future<List<String>> loadQuickActions() async {
    final payload = await _storage.read(key: _key);
    if (payload == null || payload.isEmpty) return [];

    final json = await _decrypt(payload);
    if (json == null || json.isEmpty) return [];

    try {
      final data = jsonDecode(json) as Map<String, dynamic>;
      return (data['quick_action_routes'] as List<dynamic>?)?.cast<String>() ??
          [];
    } on Exception {
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
    } on Exception {
      return plaintext;
    }
  }

  /// Decrypts a payload produced by [_encrypt].
  /// Handles both encrypted ('enc:...') and plain JSON payloads.
  Future<String?> _decrypt(String payload) async {
    if (!payload.startsWith('enc:')) {
      return payload;
    }

    try {
      final combined = base64Decode(payload.substring(4));
      final decrypted = await frb.frbDecryptBytes(data: combined);
      return utf8.decode(decrypted);
    } on Exception {
      // FRB handles both SOLO blob and legacy Dart formats.
      // If it fails, the data is unrecoverable without the original key.
      return null;
    }
  }
}
