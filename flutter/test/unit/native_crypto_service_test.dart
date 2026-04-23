import 'dart:convert';
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'package:encrypt/encrypt.dart' as enc;
import 'package:flutter_test/flutter_test.dart';
import 'package:pointycastle/export.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';

// NativeCryptoService only supports Android/macOS/iOS - skip on Linux
final _isSupported = Platform.isAndroid || Platform.isMacOS || Platform.isIOS;

void main() {
  // NativeCryptoService uses singleton pattern, we test its Dart fallback behavior
  // by directly invoking the static instance methods

  group('NativeCryptoService Dart Fallback - Salt Generation', () {
    test(', skip: !_isSupported,generateSalt returns 32 bytes on Android', () {
      // On Android (_isAndroid=true), it uses _generateSaltDart
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt();

      expect(salt, isNotNull);
      expect(salt!.length, 32);
    });

    test(', skip: !_isSupported,generateSalt returns unique values (randomness)', () {
      final service = NativeCryptoService.instance;
      final salt1 = service.generateSalt();
      final salt2 = service.generateSalt();

      expect(salt1, isNotNull);
      expect(salt2, isNotNull);
      // Very high probability of uniqueness
      expect(salt1, isNot(equals(salt2)));
    });
  });

  group('NativeCryptoService Dart Fallback - PBKDF2 Key Derivation', () {
    // Note: Our implementation requires 32-byte salt (matching Argon2id requirement)
    // RFC 6070 uses 4-byte salt, so we test our implementation differently

    test(', skip: !_isSupported,deriveKey produces consistent output for same inputs', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key1 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, equals(key2));
    });

    test(', skip: !_isSupported,deriveKey returns null for invalid salt length', () {
      final service = NativeCryptoService.instance;

      // Salt must be 32 bytes (as enforced by the service)
      final invalidSalt = Uint8List.fromList(utf8.encode('short'));

      expect(
        () => service.deriveKey(
          password: 'password',
          salt: invalidSalt,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test(', skip: !_isSupported,deriveKey returns different keys for different passwords', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key1 = service.deriveKey(
        password: 'password1',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'password2',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, isNot(equals(key2)));
    });

    test(', skip: !_isSupported,deriveKey returns different keys for different salts', () {
      final service = NativeCryptoService.instance;
      final salt1 = service.generateSalt()!;
      final salt2 = service.generateSalt()!;

      final key1 = service.deriveKey(
        password: 'samepassword',
        salt: salt1,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'samepassword',
        salt: salt2,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, isNot(equals(key2)));
    });

    test(', skip: !_isSupported,deriveKey returns same key for same inputs (deterministic)', () {
      final service = NativeCryptoService.instance;
      final salt = Uint8List.fromList(List.filled(32, 0x42));

      final key1 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      final key2 = service.deriveKey(
        password: 'testpassword',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key1, isNotNull);
      expect(key2, isNotNull);
      expect(key1, equals(key2));
    });

    test(', skip: !_isSupported,deriveKey returns 32-byte key', () {
      final service = NativeCryptoService.instance;
      final salt = service.generateSalt()!;

      final key = service.deriveKey(
        password: 'password',
        salt: salt,
        memoryKib: 4096,
        iterations: 1,
        parallelism: 1,
      );

      expect(key, isNotNull);
      expect(key!.length, 32);
    });
  });

  group('NativeCryptoService Dart Fallback - AES-256-GCM Encryption', () {
    late Uint8List testKey;
    late Uint8List testNonce;
    late Uint8List testPlaintext;

    setUp(() {
      // Generate a valid 32-byte key and 12-byte nonce
      testKey = Uint8List.fromList(List.filled(32, 0x42));
      testNonce = Uint8List.fromList(List.filled(12, 0x24));
      testPlaintext = Uint8List.fromList(utf8.encode('Hello, World!'));
    });

    test(', skip: !_isSupported,encrypt and decrypt roundtrip succeeds', () {
      final service = NativeCryptoService.instance;

      final ciphertext = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);
      expect(ciphertext!.length, greaterThan(testPlaintext.length));

      final decrypted = service.decrypt(
        encrypted: ciphertext,
        key: testKey,
        nonce: testNonce,
      );

      expect(decrypted, isNotNull);
      expect(utf8.decode(decrypted!), equals('Hello, World!'));
    });

    test(', skip: !_isSupported,encrypt produces different ciphertext for same plaintext (due to random IV behavior)', () {
      // Note: With GCM mode, the encrypt package uses the provided nonce as IV
      // If we use different nonces, we get different ciphertext
      final service = NativeCryptoService.instance;

      final nonce1 = Uint8List.fromList(List.filled(12, 0x11));
      final nonce2 = Uint8List.fromList(List.filled(12, 0x22));

      final ciphertext1 = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: nonce1,
      );

      final ciphertext2 = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: nonce2,
      );

      expect(ciphertext1, isNot(equals(ciphertext2)));
    });

    test(', skip: !_isSupported,decrypt fails with wrong key', () {
      final service = NativeCryptoService.instance;

      final ciphertext = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);

      // Use a different key for decryption
      final wrongKey = Uint8List.fromList(List.filled(32, 0x99));

      final decrypted = service.decrypt(
        encrypted: ciphertext!,
        key: wrongKey,
        nonce: testNonce,
      );

      // GCM authentication should fail with wrong key
      expect(decrypted, isNull);
    });

    test(', skip: !_isSupported,decrypt fails with wrong nonce', () {
      final service = NativeCryptoService.instance;

      final ciphertext = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);

      // Use a different nonce for decryption
      final wrongNonce = Uint8List.fromList(List.filled(12, 0x99));

      final decrypted = service.decrypt(
        encrypted: ciphertext!,
        key: testKey,
        nonce: wrongNonce,
      );

      // GCM authentication should fail with wrong nonce
      expect(decrypted, isNull);
    });

    test(', skip: !_isSupported,decrypt fails with tampered ciphertext', () {
      final service = NativeCryptoService.instance;

      final ciphertext = service.encrypt(
        data: testPlaintext,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);

      // Tamper with the ciphertext
      final tampered = Uint8List.fromList(ciphertext!);
      tampered[0] ^= 0xFF; // Flip bits in first byte

      final decrypted = service.decrypt(
        encrypted: tampered,
        key: testKey,
        nonce: testNonce,
      );

      // GCM authentication should fail with tampered ciphertext
      expect(decrypted, isNull);
    });

    test(', skip: !_isSupported,encrypt throws ArgumentError for invalid key length', () {
      final service = NativeCryptoService.instance;

      final invalidKey = Uint8List.fromList(List.filled(16, 0x42)); // 16 bytes instead of 32

      expect(
        () => service.encrypt(
          data: testPlaintext,
          key: invalidKey,
          nonce: testNonce,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test(', skip: !_isSupported,encrypt throws ArgumentError for invalid nonce length', () {
      final service = NativeCryptoService.instance;

      final invalidNonce = Uint8List.fromList(List.filled(16, 0x24)); // 16 bytes instead of 12

      expect(
        () => service.encrypt(
          data: testPlaintext,
          key: testKey,
          nonce: invalidNonce,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test(', skip: !_isSupported,decrypt throws ArgumentError for invalid key length', () {
      final service = NativeCryptoService.instance;

      final invalidKey = Uint8List.fromList(List.filled(16, 0x42));

      expect(
        () => service.decrypt(
          encrypted: testPlaintext,
          key: invalidKey,
          nonce: testNonce,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test(', skip: !_isSupported,decrypt throws ArgumentError for invalid nonce length', () {
      final service = NativeCryptoService.instance;

      final invalidNonce = Uint8List.fromList(List.filled(16, 0x24));

      expect(
        () => service.decrypt(
          encrypted: testPlaintext,
          key: testKey,
          nonce: invalidNonce,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test(', skip: !_isSupported,encrypt and decrypt empty data', () {
      final service = NativeCryptoService.instance;
      final emptyData = Uint8List(0);

      final ciphertext = service.encrypt(
        data: emptyData,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);

      final decrypted = service.decrypt(
        encrypted: ciphertext!,
        key: testKey,
        nonce: testNonce,
      );

      expect(decrypted, isNotNull);
      expect(decrypted!.length, 0);
    });

    test(', skip: !_isSupported,encrypt and decrypt large data', () {
      final service = NativeCryptoService.instance;
      // 1 MB of data
      final largeData = Uint8List.fromList(List.filled(1024 * 1024, 0xAB));

      final ciphertext = service.encrypt(
        data: largeData,
        key: testKey,
        nonce: testNonce,
      );

      expect(ciphertext, isNotNull);

      final decrypted = service.decrypt(
        encrypted: ciphertext!,
        key: testKey,
        nonce: testNonce,
      );

      expect(decrypted, isNotNull);
      expect(decrypted!.length, largeData.length);
      // Verify content
      for (var i = 0; i < decrypted.length; i++) {
        expect(decrypted[i], largeData[i]);
      }
    });
  });

  group('NativeCryptoService Base64 Helpers', () {
    test(', skip: !_isSupported,base64 encoding roundtrip', () {
      final original = Uint8List.fromList(utf8.encode('Hello, World!'));
      final encoded = base64Encode(original);
      final decoded = base64Decode(encoded);

      expect(decoded, equals(original));
      expect(utf8.decode(decoded), equals('Hello, World!'));
    });

    test(', skip: !_isSupported,base64 encoding of binary data', () {
      final binary = Uint8List.fromList([0x00, 0xFF, 0x42, 0x7F, 0x80]);
      final encoded = base64Encode(binary);

      // Verify it's valid base64
      final decoded = base64Decode(encoded);
      expect(decoded, equals(binary));
    });
  });

  group('Default Constants', () {
    test(', skip: !_isSupported,defaultMemoryKib is 65536 (64MB)', () {
      expect(defaultMemoryKib, 65536);
    });

    test(', skip: !_isSupported,defaultIterations is 3', () {
      expect(defaultIterations, 3);
    });

    test(', skip: !_isSupported,defaultParallelism is 4', () {
      expect(defaultParallelism, 4);
    });
  });
}

// Helper function to convert bytes to hex string (matches _bytesToHex in auth_provider)
String _bytesToHex(List<int> bytes) {
  return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
}
