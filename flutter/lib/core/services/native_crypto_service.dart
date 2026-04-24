import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:encrypt/encrypt.dart' as enc;
import 'package:ffi/ffi.dart';
import 'package:pointycastle/export.dart';

/// FFI bindings to Rust Argon2id implementation (iOS/macOS only)
/// Uses pure Dart implementation on Android
class NativeCryptoService {
  static NativeCryptoService? _instance;
  late DynamicLibrary _lib;
  bool _isAndroid = false;

  // FFI function types (iOS/macOS only)
  late int Function(Pointer<Uint8> salt, int saltLen) _generateSalt;
  late int Function(
    Pointer<Uint8> password,
    int passwordLen,
    Pointer<Uint8> salt,
    int saltLen,
    int memoryKib,
    int iterations,
    int parallelism,
    Pointer<Uint8> output,
    int outputLen,
  ) _deriveKey;

  // AES-256-GCM FFI function types
  late int Function(
    Pointer<Uint8> key,
    Pointer<Uint8> plaintext,
    int plaintextLen,
    Pointer<Uint8> nonce,
    Pointer<Uint8> ciphertext,
    Pointer<IntPtr> ciphertextLen,
  ) _aesEncrypt;

  late int Function(
    Pointer<Uint8> key,
    Pointer<Uint8> ciphertext,
    int ciphertextLen,
    Pointer<Uint8> nonce,
    Pointer<Uint8> plaintext,
    Pointer<IntPtr> plaintextLen,
  ) _aesDecrypt;

  // Dart implementations for Android
  final _random = Random.secure();

  NativeCryptoService._();

  static NativeCryptoService get instance {
    _instance ??= NativeCryptoService._().._initialize();
    return _instance!;
  }

  void _initialize() {
    _isAndroid = Platform.isAndroid;

    if (_isAndroid) {
      // Android: Use pure Dart implementation - no FFI needed
      return;
    }

    // iOS/macOS: Load the native library
    if (Platform.isMacOS || Platform.isIOS) {
      _lib = DynamicLibrary.process();
    } else {
      throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
    }

    // Bind argon2_generate_salt
    try {
      _generateSalt = _lib
          .lookup<NativeFunction<Int32 Function(Pointer<Uint8>, IntPtr)>>(
              'argon2_generate_salt')
          .asFunction();
    } catch (e, st) {
      throw Exception('Failed to bind argon2_generate_salt: $e\nStack: $st');
    }

    // Bind argon2_derive_key
    try {
      _deriveKey = _lib
          .lookup<
              NativeFunction<
                  Int32 Function(
                      Pointer<Uint8>,
                      IntPtr,
                      Pointer<Uint8>,
                      IntPtr,
                      Uint32,
                      Uint32,
                      Uint32,
                      Pointer<Uint8>,
                      IntPtr,
                  )>>('argon2_derive_key')
          .asFunction();
    } catch (e, st) {
      throw Exception('Failed to bind argon2_derive_key: $e\nStack: $st');
    }

    // Bind aes_256_gcm_encrypt
    try {
      _aesEncrypt = _lib
          .lookup<
              NativeFunction<
                  Int32 Function(
                      Pointer<Uint8>,
                      Pointer<Uint8>,
                      IntPtr,
                      Pointer<Uint8>,
                      Pointer<Uint8>,
                      Pointer<IntPtr>,
                  )>>('aes_256_gcm_encrypt')
          .asFunction();
    } catch (e, st) {
      throw Exception('Failed to bind aes_256_gcm_encrypt: $e\nStack: $st');
    }

    // Bind aes_256_gcm_decrypt
    try {
      _aesDecrypt = _lib
          .lookup<
              NativeFunction<
                  Int32 Function(
                      Pointer<Uint8>,
                      Pointer<Uint8>,
                      IntPtr,
                      Pointer<Uint8>,
                      Pointer<Uint8>,
                      Pointer<IntPtr>,
                  )>>('aes_256_gcm_decrypt')
          .asFunction();
    } catch (e, st) {
      throw Exception('Failed to bind aes_256_gcm_decrypt: $e\nStack: $st');
    }
  }

  /// Generate a cryptographically secure random 32-byte salt
  /// Returns null on failure
  Uint8List? generateSalt() {
    if (_isAndroid) {
      return _generateSaltDart();
    }

    final salt = calloc<Uint8>(32);
    try {
      final result = _generateSalt(salt, 32);
      if (result != 0) {
        return null;
      }
      final resultBytes = Uint8List(32);
      for (var i = 0; i < 32; i++) {
        resultBytes[i] = salt[i];
      }
      return resultBytes;
    } finally {
      calloc.free(salt);
    }
  }

  /// Dart implementation of salt generation
  Uint8List _generateSaltDart() {
    final salt = Uint8List(32);
    for (var i = 0; i < 32; i++) {
      salt[i] = _random.nextInt(256);
    }
    return salt;
  }

  /// Derive a key using Argon2id
  ///
  /// [password] - The password to derive from
  /// [salt] - 32-byte salt
  /// [memoryKib] - Memory in KiB (default 16384 = 16MB)
  /// [iterations] - Number of iterations (default 1)
  /// [parallelism] - Number of parallel threads (default 4)
  ///
  /// Returns 32-byte derived key, or null on failure
  Uint8List? deriveKey({
    required String password,
    required Uint8List salt,
    int memoryKib = defaultMemoryKib,
    int iterations = defaultIterations,
    int parallelism = 4,
  }) {
    // Note: salt length is not enforced to be 32 bytes here.
    // The Rust FFI derive_key accepts any salt length, and some derivation
    // steps (e.g., verify derivation) use shorter fixed phrases.
    // generateSalt() still produces 32 bytes for primary salt generation.

    if (_isAndroid) {
      return _deriveKeyDart(password, salt, memoryKib, iterations, parallelism);
    }

    final passwordBytes = Uint8List.fromList(utf8.encode(password));
    final passwordPtr = calloc<Uint8>(passwordBytes.length);
    for (var i = 0; i < passwordBytes.length; i++) {
      passwordPtr[i] = passwordBytes[i];
    }

    final saltPtr = calloc<Uint8>(salt.length);
    for (var i = 0; i < salt.length; i++) {
      saltPtr[i] = salt[i];
    }

    final outputPtr = calloc<Uint8>(32);

    try {
      final result = _deriveKey(
        passwordPtr,
        passwordBytes.length,
        saltPtr,
        salt.length,
        memoryKib,
        iterations,
        parallelism,
        outputPtr,
        32,
      );

      // Securely zero the password buffer after FFI call
      for (var i = 0; i < passwordBytes.length; i++) {
        passwordBytes[i] = 0;
      }

      if (result != 0) {
        return null;
      }

      final output = Uint8List(32);
      for (var i = 0; i < 32; i++) {
        output[i] = outputPtr[i];
      }
      return output;
    } finally {
      calloc.free(passwordPtr);
      calloc.free(saltPtr);
      calloc.free(outputPtr);
    }
  }

  /// Dart implementation of key derivation using PBKDF2-HMAC-SHA256
  /// Note: This is a fallback for Android. For production, use Argon2id.
  // TODO: [P1] Android uses PBKDF2 instead of Argon2id - less secure on Android
  // Argon2id provides better protection against GPU/ASIC attacks
  Uint8List? _deriveKeyDart(
    String password,
    Uint8List salt,
    int memoryKib,
    int iterations,
    int parallelism,
  ) {
    try {
      // Using PBKDF2 with HMAC-SHA256 as a fallback
      // memoryKib is approximated via iteration count
      final pbkdf2 = PBKDF2KeyDerivator(HMac(SHA256Digest(), 64));
      pbkdf2.init(Pbkdf2Parameters(salt, iterations, 32));

      final passwordBytes = Uint8List.fromList(utf8.encode(password));
      final derivedKey = pbkdf2.process(passwordBytes);

      return Uint8List.fromList(derivedKey);
    } on Exception catch (_) {
      return null;
    }
  }

  /// Encrypt data using AES-256-GCM
  ///
  /// [data] - The plaintext data to encrypt
  /// [key] - 32-byte encryption key
  /// [nonce] - 12-byte nonce (will be used as-is, not generated)
  ///
  /// Returns the ciphertext (includes auth tag), or null on failure
  Uint8List? encrypt({
    required Uint8List data,
    required Uint8List key,
    required Uint8List nonce,
  }) {
    if (key.length != 32) {
      throw ArgumentError('Key must be 32 bytes');
    }
    if (nonce.length != 12) {
      throw ArgumentError('Nonce must be 12 bytes');
    }

    if (_isAndroid) {
      return _encryptDart(data, key, nonce);
    }

    final keyPtr = calloc<Uint8>(32);
    for (var i = 0; i < 32; i++) {
      keyPtr[i] = key[i];
    }

    final noncePtr = calloc<Uint8>(12);
    for (var i = 0; i < 12; i++) {
      noncePtr[i] = nonce[i];
    }

    final plaintextPtr = calloc<Uint8>(data.length);
    for (var i = 0; i < data.length; i++) {
      plaintextPtr[i] = data[i];
    }

    // Ciphertext output buffer (max possible size: plaintext + 16 byte tag)
    final maxCiphertextLen = data.length + 16;
    final ciphertextPtr = calloc<Uint8>(maxCiphertextLen);
    final ciphertextLenPtr = calloc<IntPtr>();
    ciphertextLenPtr.value = maxCiphertextLen;

    try {
      final result = _aesEncrypt(
        keyPtr,
        plaintextPtr,
        data.length,
        noncePtr,
        ciphertextPtr,
        ciphertextLenPtr,
      );

      if (result != 0) {
        return null;
      }

      final actualLen = ciphertextLenPtr.value;
      final ciphertext = Uint8List(actualLen);
      for (var i = 0; i < actualLen; i++) {
        ciphertext[i] = ciphertextPtr[i];
      }
      return ciphertext;
    } finally {
      calloc.free(keyPtr);
      calloc.free(noncePtr);
      calloc.free(plaintextPtr);
      calloc.free(ciphertextPtr);
      calloc.free(ciphertextLenPtr);
    }
  }

  /// Dart implementation of AES-256-GCM encryption using encrypt package
  Uint8List? _encryptDart(Uint8List data, Uint8List key, Uint8List nonce) {
    try {
      final keyObj = enc.Key(key);
      final ivObj = enc.IV(nonce);

      final encrypter = enc.Encrypter(
        enc.AES(keyObj, mode: enc.AESMode.gcm),
      );

      final encrypted = encrypter.encryptBytes(data, iv: ivObj);

      // encrypt package already includes the GCM tag in the ciphertext
      return Uint8List.fromList(encrypted.bytes);
    } on Exception catch (_) {
      return null;
    }
  }

  /// Decrypt data using AES-256-GCM
  ///
  /// [encrypted] - The ciphertext data to decrypt
  /// [key] - 32-byte encryption key
  /// [nonce] - 12-byte nonce used during encryption
  ///
  /// Returns the plaintext, or null on failure
  Uint8List? decrypt({
    required Uint8List encrypted,
    required Uint8List key,
    required Uint8List nonce,
  }) {
    if (key.length != 32) {
      throw ArgumentError('Key must be 32 bytes');
    }
    if (nonce.length != 12) {
      throw ArgumentError('Nonce must be 12 bytes');
    }

    if (_isAndroid) {
      return _decryptDart(encrypted, key, nonce);
    }

    final keyPtr = calloc<Uint8>(32);
    for (var i = 0; i < 32; i++) {
      keyPtr[i] = key[i];
    }

    final noncePtr = calloc<Uint8>(12);
    for (var i = 0; i < 12; i++) {
      noncePtr[i] = nonce[i];
    }

    final ciphertextPtr = calloc<Uint8>(encrypted.length);
    for (var i = 0; i < encrypted.length; i++) {
      ciphertextPtr[i] = encrypted[i];
    }

    // Plaintext output buffer (max possible size: ciphertext - 16 byte tag)
    final maxPlaintextLen = encrypted.length > 16 ? encrypted.length - 16 : 0;
    final plaintextPtr = calloc<Uint8>(maxPlaintextLen);
    final plaintextLenPtr = calloc<IntPtr>();
    plaintextLenPtr.value = maxPlaintextLen;

    try {
      final result = _aesDecrypt(
        keyPtr,
        ciphertextPtr,
        encrypted.length,
        noncePtr,
        plaintextPtr,
        plaintextLenPtr,
      );

      if (result != 0) {
        return null;
      }

      final actualLen = plaintextLenPtr.value;
      if (actualLen == 0) {
        return Uint8List(0);
      }

      final plaintext = Uint8List(actualLen);
      for (var i = 0; i < actualLen; i++) {
        plaintext[i] = plaintextPtr[i];
      }
      return plaintext;
    } finally {
      calloc.free(keyPtr);
      calloc.free(noncePtr);
      calloc.free(ciphertextPtr);
      calloc.free(plaintextPtr);
      calloc.free(plaintextLenPtr);
    }
  }

  /// Dart implementation of AES-256-GCM decryption using encrypt package
  Uint8List? _decryptDart(Uint8List encrypted, Uint8List key, Uint8List nonce) {
    try {
      final keyObj = enc.Key(key);
      final ivObj = enc.IV(nonce);

      final encrypter = enc.Encrypter(
        enc.AES(keyObj, mode: enc.AESMode.gcm),
      );

      final encryptedObj = enc.Encrypted(encrypted);
      final decrypted = encrypter.decryptBytes(encryptedObj, iv: ivObj);

      return Uint8List.fromList(decrypted);
    } on Exception catch (_) {
      return null;
    }
  }
}

// Default Argon2id parameters (64MB memory, 3 iterations, 4 parallelism)
const int defaultMemoryKib = 65536;
const int defaultIterations = 3;
const int defaultParallelism = 4;
