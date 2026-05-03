import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/native_vault_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart'
    show bytesToHex, constantTimeEquals;

/// Biometric credential using double-envelope encryption.
///
/// Architecture:
///   deviceKey (Secure Storage) → encryptedBioToken (Fallback)
///     → encryptedSessionKey (Fallback)
///
/// - deviceKey: 32-byte random, stored ONLY in raw FlutterSecureStorage (no fallback).
///   If secure storage is unavailable, biometric unlock is disabled.
/// - bioToken: 32-byte random, encrypted by deviceKey via AES-256-GCM.
/// - sessionKey: 32-byte Argon2id-derived vault master key, encrypted by bioToken.
///
/// The master password is NEVER stored in any form.
class BiometricCredentialService {
  BiometricCredentialService._();

  static BiometricCredentialService? _instance;
  static BiometricCredentialService get instance =>
      _instance ??= BiometricCredentialService._();

  // Raw secure storage for deviceKey — NO fallback.
  // If Keychain/Keystore is unavailable, deviceKey cannot be stored,
  // and biometric unlock is disabled.
  static const _rawSecureStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
    wOptions: WindowsOptions(),
  );

  final FallbackSecureStorage _fallbackStorage = FallbackSecureStorage();
  final SecureAccountStorage _accountStorage = SecureAccountStorage.instance;

  static const _deviceKeyId = 'biometric_device_key_v1';
  static const _credentialKeyPrefix = 'biometric_credential_';
  static const _oldBiometricPasswordKey = 'biometric_unlock_password';
  static const _migrationFlagKey = 'biometric_migration_v1_done';

  bool _didPurgeOldCredential = false;
  bool _initialized = false;

  /// Whether the old plaintext credential was purged during this session.
  bool get didPurgeOldCredential => _didPurgeOldCredential;

  /// Initialize the service. Generates deviceKey if absent.
  /// Purges old plaintext password on first run (once per device).
  Future<void> initialize() async {
    if (_initialized) return;

    await _ensureDeviceKey();
    await _purgeOldCredentialIfNeeded();
    _initialized = true;
  }

  // ==========================================================================
  // Device Key Management
  // ==========================================================================

  Future<void> _ensureDeviceKey() async {
    final existing = await _rawSecureStorage.read(key: _deviceKeyId);
    if (existing != null && existing.isNotEmpty) return;

    final deviceKey = await frb.frbGenerateSalt(length: 32);

    await _rawSecureStorage.write(
      key: _deviceKeyId,
      value: base64Encode(deviceKey),
    );
    _secureWipe(deviceKey);
    SoloLog.d('BioCred', 'deviceKey generated and stored');
  }

  Future<Uint8List?> _readDeviceKey() async {
    final encoded = await _rawSecureStorage.read(key: _deviceKeyId);
    if (encoded == null || encoded.isEmpty) return null;
    try {
      return base64Decode(encoded);
    } on FormatException {
      return null;
    }
  }

  /// Whether a deviceKey exists in secure storage.
  Future<bool> isDeviceKeyAvailable() async {
    final key = await _rawSecureStorage.read(key: _deviceKeyId);
    return key != null && key.isNotEmpty;
  }

  // ==========================================================================
  // Credential Lifecycle
  // ==========================================================================

  /// Save a biometric credential for an account.
  ///
  /// [password] is the master password. It is used to derive the session key,
  /// which is then encrypted and stored. The password itself is never stored.
  ///
  /// Returns true if the credential was saved successfully.
  Future<bool> saveBiometricCredential(
    String accountId,
    String password,
  ) async {
    try {
      // 1. Derive and verify session key
      final sessionKey = await _deriveAndVerifySessionKey(accountId, password);
      if (sessionKey == null) {
        SoloLog.w('BioCred', 'Password verification failed for $accountId');
        return false;
      }

      // 2. Read deviceKey
      final deviceKey = await _readDeviceKey();
      if (deviceKey == null || deviceKey.length != 32) {
        _secureWipe(sessionKey);
        SoloLog.w('BioCred', 'deviceKey unavailable');
        return false;
      }

      // 3. Generate bioToken
      final bioToken = await frb.frbGenerateSalt(length: 32);

      // 4. Encrypt sessionKey with bioToken (SOLO format — nonce embedded)
      final encryptedSessionKey = await frb.frbEncryptWithKey(
        key: bioToken,
        plaintext: sessionKey,
      );

      // 5. Encrypt bioToken with deviceKey (SOLO format — nonce embedded)
      final encryptedBioToken = await frb.frbEncryptWithKey(
        key: deviceKey,
        plaintext: bioToken,
      );

      // 6. Store credential envelope (v2: SOLO format, no separate nonces)
      final envelope = {
        'version': 2,
        'credentialVersion': 2,
        'encryptedSessionKey': base64Encode(encryptedSessionKey),
        'encryptedBioToken': base64Encode(encryptedBioToken),
        'createdAt': DateTime.now().toIso8601String(),
      };

      await _rawSecureStorage.write(
        key: '$_credentialKeyPrefix$accountId',
        value: jsonEncode(envelope),
      );

      SoloLog.d('BioCred', 'Credential saved for $accountId');

      // 7. Secure wipe
      _secureWipe(sessionKey);
      _secureWipe(deviceKey);
      _secureWipe(bioToken);

      return true;
    } on Exception catch (e, st) {
      SoloLog.e('BioCred', 'saveBiometricCredential error', e, st);
      return false;
    }
  }

  /// Retrieve the session key for biometric unlock.
  ///
  /// Returns the 32-byte session key if the credential exists and can be
  /// decrypted. Returns null otherwise.
  ///
  /// The caller is responsible for securely wiping the returned key after use.
  Future<Uint8List?> unlockWithBiometric(String accountId) async {
    try {
      // 1. Read deviceKey
      final deviceKey = await _readDeviceKey();
      if (deviceKey == null || deviceKey.length != 32) {
        SoloLog.w('BioCred', 'deviceKey unavailable for unlock');
        return null;
      }

      // 2. Read credential envelope
      final envelopeJson = await _rawSecureStorage.read(
        key: '$_credentialKeyPrefix$accountId',
      );
      if (envelopeJson == null || envelopeJson.isEmpty) {
        _secureWipe(deviceKey);
        return null;
      }

      final envelope = jsonDecode(envelopeJson) as Map<String, dynamic>;
      final credentialVersion = envelope['credentialVersion'] as int? ??
          envelope['version'] as int?;

      final encryptedBioToken = base64Decode(envelope['encryptedBioToken'] as String);
      final encryptedSessionKey = base64Decode(envelope['encryptedSessionKey'] as String);

      Uint8List? bioToken;
      Uint8List? sessionKey;

      if (credentialVersion != null && credentialVersion >= 2) {
        // v2: SOLO format — nonce embedded in blob
        bioToken = await frb.frbDecryptWithKey(
          key: deviceKey,
          ciphertext: encryptedBioToken,
        );
        _secureWipe(deviceKey);
        if (bioToken.length != 32) {
          SoloLog.w('BioCred', 'Failed to decrypt bioToken (v2)');
          return null;
        }

        sessionKey = await frb.frbDecryptWithKey(
          key: bioToken,
          ciphertext: encryptedSessionKey,
        );
        _secureWipe(bioToken);
        if (sessionKey.length != 32) {
          SoloLog.w('BioCred', 'Failed to decrypt sessionKey (v2)');
          return null;
        }
      } else {
        // v1: legacy format — Nonce(12B) + Ciphertext + Tag(16B)
        final bioTokenNonce = base64Decode(envelope['bioTokenNonce'] as String);
        final sessionKeyNonce = base64Decode(envelope['sessionKeyNonce'] as String);

        final legacyBioToken = Uint8List(bioTokenNonce.length + encryptedBioToken.length);
        legacyBioToken.setRange(0, bioTokenNonce.length, bioTokenNonce);
        legacyBioToken.setRange(bioTokenNonce.length, legacyBioToken.length, encryptedBioToken);

        bioToken = await frb.frbDecryptWithKey(
          key: deviceKey,
          ciphertext: legacyBioToken,
        );
        _secureWipe(deviceKey);
        if (bioToken.length != 32) {
          SoloLog.w('BioCred', 'Failed to decrypt bioToken (v1)');
          return null;
        }

        final legacySessionKey = Uint8List(sessionKeyNonce.length + encryptedSessionKey.length);
        legacySessionKey.setRange(0, sessionKeyNonce.length, sessionKeyNonce);
        legacySessionKey.setRange(sessionKeyNonce.length, legacySessionKey.length, encryptedSessionKey);

        sessionKey = await frb.frbDecryptWithKey(
          key: bioToken,
          ciphertext: legacySessionKey,
        );
        _secureWipe(bioToken);
        if (sessionKey.length != 32) {
          SoloLog.w('BioCred', 'Failed to decrypt sessionKey (v1)');
          return null;
        }
      }

      SoloLog.d('BioCred', 'Credential decrypted for $accountId (v$credentialVersion)');
      return sessionKey;
    } on Exception catch (e, st) {
      SoloLog.e('BioCred', 'unlockWithBiometric error', e, st);
      return null;
    }
  }

  /// Whether a biometric credential exists for the account.
  Future<bool> hasBiometricCredential(String accountId) async {
    final value = await _rawSecureStorage.read(
      key: '$_credentialKeyPrefix$accountId',
    );
    return value != null && value.isNotEmpty;
  }

  /// Clear the biometric credential for an account.
  Future<void> clearBiometricCredential(String accountId) async {
    await _rawSecureStorage.delete(key: '$_credentialKeyPrefix$accountId');
    SoloLog.d('BioCred', 'Credential cleared for $accountId');
  }

  /// Clear all biometric credentials (e.g. on reset to defaults).
  Future<void> clearAllBiometricCredentials() async {
    // Delete deviceKey from raw secure storage
    await _rawSecureStorage.delete(key: _deviceKeyId);
    // We can't enumerate all account credentials, but we can clear the deviceKey
    // which renders all existing credentials useless.
    // For a full cleanup, callers should iterate accounts and call clearBiometricCredential.
    SoloLog.d('BioCred', 'deviceKey cleared; all credentials invalidated');
  }

  // ==========================================================================
  // Session Key Derivation & Verification
  // ==========================================================================

  /// Derive the session key from password and verify it against stored hash.
  ///
  /// Returns the 32-byte master key on success, null on failure.
  /// This mirrors the derivation chain in AuthStorage.unlockAccount and
  /// Rust AccountManager.unlock.
  Future<Uint8List?> _deriveAndVerifySessionKey(
    String accountId,
    String password,
  ) async {
    // Read account data (salt and verify_hash)
    final accountData = await _accountStorage.getAccountData(accountId);
    String saltStr;
    String storedHash;

    if (accountData != null) {
      saltStr = accountData['salt'] as String;
      storedHash = accountData['verify_hash'] as String;
    } else {
      // Fallback to Rust config
      final rustConfig = NativeVaultService.instance.getAccountConfig(accountId: accountId);
      if (rustConfig == null ||
          rustConfig.salt == null ||
          rustConfig.verifyHash == null) {
        SoloLog.w('BioCred', 'No account data found for $accountId');
        return null;
      }
      saltStr = rustConfig.salt!;
      storedHash = rustConfig.verifyHash!;
    }

    final salt = base64Decode(saltStr);

    // Step 1: Derive masterKey from password
    final masterKey = await frb.frbDeriveKey(
      password: password,
      salt: Uint8List.fromList(salt),
      memoryKib: 16384,
      iterations: 1,
      parallelism: 4,
    );

    // Step 2: Derive verifyKey from masterKey hex
    final masterKeyHex = bytesToHex(masterKey);
    const verifyData = 'SOLOSOUL_VAULT_VERIFY_v1';
    final verifyKey = await frb.frbDeriveKey(
      password: masterKeyHex,
      salt: Uint8List.fromList(utf8.encode(verifyData)),
      memoryKib: 8192,
      iterations: 1,
      parallelism: 1,
    );

    // Step 3: Compare verify hash
    final derivedHashHex = bytesToHex(verifyKey);
    _secureWipe(verifyKey);

    if (!await constantTimeEquals(derivedHashHex, storedHash)) {
      _secureWipe(masterKey);
      return null;
    }

    return masterKey;
  }

  // ==========================================================================
  // Migration: Clear Old Plaintext Password
  // ==========================================================================

  Future<void> _purgeOldCredentialIfNeeded() async {
    // Check if migration already done
    final migrated = await _fallbackStorage.read(key: _migrationFlagKey);
    if (migrated != null) return;

    // Check for old plaintext password
    final oldPassword = await _fallbackStorage.read(key: _oldBiometricPasswordKey);
    if (oldPassword != null) {
      await _fallbackStorage.delete(key: _oldBiometricPasswordKey);
      SoloLog.d('BioCred', 'Purged old plaintext biometric password');
      _didPurgeOldCredential = true;
    }

    // Mark migration done
    await _fallbackStorage.write(key: _migrationFlagKey, value: 'done');
  }

  // ==========================================================================
  // Utilities
  // ==========================================================================

  /// Best-effort secure wipe of a byte buffer.
  void _secureWipe(Uint8List buffer) {
    for (var i = 0; i < buffer.length; i++) {
      buffer[i] = 0;
    }
  }
}
