import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

/// Service for managing user UI preferences with optional AES-256-GCM encryption.
///
/// Data is stored via [FallbackSecureStorage] (Keychain primary, file fallback).
/// When the vault is unlocked, an additional layer of AES-256-GCM encryption
/// is applied using the vault-derived encryption key.
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
  // Encryption helpers (AES-256-GCM via vault key when available)
  // ---------------------------------------------------------------------------

  /// Encrypts [plaintext] when the vault encryption key is available.
  /// Falls back to returning the plaintext as-is when the key is unavailable.
  Future<String> _encrypt(String plaintext) async {
    final key = RustVaultService.instance.encryptionKey;
    if (key == null) {
      // Vault not unlocked — store as plain JSON.
      // FallbackSecureStorage already uses Keychain (encrypted) when possible.
      return plaintext;
    }

    final nonce = _randomNonce();
    final data = Uint8List.fromList(utf8.encode(plaintext));
    final encrypted = NativeCryptoService.instance.encrypt(
      data: data,
      key: key,
      nonce: nonce,
    );
    if (encrypted == null) return plaintext;

    final combined = Uint8List(12 + encrypted.length);
    combined.setRange(0, 12, nonce);
    combined.setRange(12, combined.length, encrypted);

    // Prefix with 'enc:' so we can distinguish encrypted from plain payloads.
    return 'enc:${base64Encode(combined)}';
  }

  /// Decrypts a payload produced by [_encrypt].
  /// Handles both encrypted ('enc:...') and plain JSON payloads.
  Future<String?> _decrypt(String payload) async {
    if (!payload.startsWith('enc:')) {
      // Plain JSON (vault was locked at save time).
      return payload;
    }

    final key = RustVaultService.instance.encryptionKey;
    if (key == null) {
      // Cannot decrypt without the vault key.
      return null;
    }

    try {
      final combined = base64Decode(payload.substring(4));
      if (combined.length < 13) return null;

      final nonce = Uint8List.fromList(combined.sublist(0, 12));
      final encrypted = combined.sublist(12);

      final decrypted = NativeCryptoService.instance.decrypt(
        encrypted: encrypted,
        key: key,
        nonce: nonce,
      );
      if (decrypted == null) return null;

      return utf8.decode(decrypted);
    } on Exception {
      return null;
    }
  }

  Uint8List _randomNonce() {
    final rand = Random.secure();
    return Uint8List(12)..setAll(0, List.generate(12, (_) => rand.nextInt(256)));
  }
}
