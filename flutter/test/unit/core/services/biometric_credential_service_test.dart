import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_storage.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;
  final secureStorageData = <String, String?>{};

  setUpAll(() async {
    tempDir = await Directory.systemTemp.createTemp('solosoul_test_');

    // Mock path_provider
    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    pathProviderChannel.setMockMethodCallHandler((call) async {
      if (call.method == 'getApplicationSupportDirectory') {
        return tempDir.path;
      }
      return null;
    });

    // Mock flutter_secure_storage
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    secureStorageChannel.setMockMethodCallHandler((call) async {
      final args = call.arguments as Map<dynamic, dynamic>?;
      final key = args?['key'] as String?;
      switch (call.method) {
        case 'read':
          return secureStorageData[key];
        case 'write':
          if (key != null) {
            secureStorageData[key] = args?['value'] as String?;
          }
          return null;
        case 'delete':
          if (key != null) {
            secureStorageData.remove(key);
          }
          return null;
        case 'deleteAll':
          secureStorageData.clear();
          return null;
      }
      return null;
    });
  });

  tearDownAll(() async {
    await tempDir.delete(recursive: true);
    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    pathProviderChannel.setMockMethodCallHandler(null);
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    secureStorageChannel.setMockMethodCallHandler(null);
  });
  group('BiometricCredentialService', () {
    const testAccountId = 'test_account_123';
    const testPassword = 'test_password_secure_123';

    setUp(() async {
      final service = BiometricCredentialService.instance;
      await service.initialize();
      await service.clearBiometricCredential(testAccountId);
    });

    tearDown(() async {
      final service = BiometricCredentialService.instance;
      await service.clearBiometricCredential(testAccountId);
    });

    test('device key is generated on initialize', () async {
      final service = BiometricCredentialService.instance;
      final available = await service.isDeviceKeyAvailable();
      expect(available, isTrue);
    });

    test('save and retrieve biometric credential roundtrip', () async {
      final service = BiometricCredentialService.instance;

      // Create a fake account with known salt/verify_hash
      final salt = NativeCryptoService.instance.generateSalt()!;
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: testPassword,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      final masterKeyHex = bytesToHex(masterKey);
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode('SOLOSOUL_VAULT_VERIFY_v1')),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      )!;
      final verifyHash = bytesToHex(verifyKey);

      // Store fake account data in secure storage
      await SecureAccountStorage.instance.saveAccountData(
        testAccountId,
        {
          'salt': base64Encode(salt),
          'verify_hash': verifyHash,
          'crypto_version': 2,
        },
      );

      // Save biometric credential
      final saved = await service.saveBiometricCredential(testAccountId, testPassword);
      expect(saved, isTrue);

      // Check credential exists
      final hasCredential = await service.hasBiometricCredential(testAccountId);
      expect(hasCredential, isTrue);

      // Retrieve and verify session key
      final sessionKey = await service.unlockWithBiometric(testAccountId);
      expect(sessionKey, isNotNull);
      expect(sessionKey!.length, equals(32));

      // Verify the session key matches the expected master key
      expect(sessionKey, equals(masterKey));

      // Clean up secure wipe
      for (var i = 0; i < sessionKey.length; i++) {
        sessionKey[i] = 0;
      }
    });

    test('wrong password fails to save credential', () async {
      final service = BiometricCredentialService.instance;

      // Create a fake account
      final salt = NativeCryptoService.instance.generateSalt()!;
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: testPassword,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      final masterKeyHex = bytesToHex(masterKey);
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode('SOLOSOUL_VAULT_VERIFY_v1')),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      )!;
      final verifyHash = bytesToHex(verifyKey);

      await SecureAccountStorage.instance.saveAccountData(
        testAccountId,
        {
          'salt': base64Encode(salt),
          'verify_hash': verifyHash,
          'crypto_version': 2,
        },
      );

      // Try saving with wrong password
      final saved = await service.saveBiometricCredential(testAccountId, 'wrong_password');
      expect(saved, isFalse);

      // Credential should not exist
      final hasCredential = await service.hasBiometricCredential(testAccountId);
      expect(hasCredential, isFalse);
    });

    test('clear biometric credential removes data', () async {
      final service = BiometricCredentialService.instance;

      // Create a fake account
      final salt = NativeCryptoService.instance.generateSalt()!;
      final masterKey = NativeCryptoService.instance.deriveKey(
        password: testPassword,
        salt: salt,
        memoryKib: 16384,
        iterations: 1,
        parallelism: 4,
      )!;
      final masterKeyHex = bytesToHex(masterKey);
      final verifyKey = NativeCryptoService.instance.deriveKey(
        password: masterKeyHex,
        salt: Uint8List.fromList(utf8.encode('SOLOSOUL_VAULT_VERIFY_v1')),
        memoryKib: 8192,
        iterations: 1,
        parallelism: 1,
      )!;
      final verifyHash = bytesToHex(verifyKey);

      await SecureAccountStorage.instance.saveAccountData(
        testAccountId,
        {
          'salt': base64Encode(salt),
          'verify_hash': verifyHash,
          'crypto_version': 2,
        },
      );

      await service.saveBiometricCredential(testAccountId, testPassword);
      expect(await service.hasBiometricCredential(testAccountId), isTrue);

      await service.clearBiometricCredential(testAccountId);
      expect(await service.hasBiometricCredential(testAccountId), isFalse);

      final sessionKey = await service.unlockWithBiometric(testAccountId);
      expect(sessionKey, isNull);
    });

    test('unlock with missing credential returns null', () async {
      final service = BiometricCredentialService.instance;
      final sessionKey = await service.unlockWithBiometric('nonexistent_account');
      expect(sessionKey, isNull);
    });

    test('nonce generation produces unique 12-byte values', () {
      final nonce1 = _generateNonceForTest();
      final nonce2 = _generateNonceForTest();
      expect(nonce1.length, equals(12));
      expect(nonce2.length, equals(12));
      expect(nonce1, isNot(equals(nonce2)));
    });

    test('device key is 32 bytes', () async {
      final key = NativeCryptoService.instance.generateSalt()!;
      expect(key.length, equals(32));

      final nonce = _generateNonceForTest();
      final plaintext = Uint8List.fromList([1, 2, 3, 4, 5]);

      final encrypted = NativeCryptoService.instance.encrypt(
        data: plaintext,
        key: key,
        nonce: nonce,
      );
      expect(encrypted, isNotNull);

      final decrypted = NativeCryptoService.instance.decrypt(
        encrypted: encrypted!,
        key: key,
        nonce: nonce,
      );
      expect(decrypted, equals(plaintext));
    });
  });
}

// Helper for testing nonce generation (mirrors BiometricCredentialService._generateNonce)
Uint8List _generateNonceForTest() {
  final nonce = Uint8List(12);
  final random = Random.secure();
  for (var i = 0; i < 12; i++) {
    nonce[i] = random.nextInt(256);
  }
  return nonce;
}
