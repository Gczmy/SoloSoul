import 'dart:typed_data';

import 'package:solosoul_flutter/core/services/native_crypto_service.dart';

void main() {
  _runAllCryptoBenchmarks();
}

void _runAllCryptoBenchmarks() {
  print('=' * 60);
  print('CRYPTO BENCHMARKS');
  print('=' * 60);

  _benchmarkArgon2idKeyDerivation();
  _benchmarkAESEncryptDecrypt();
}

void _benchmarkArgon2idKeyDerivation() {
  print('\n--- Argon2id Key Derivation ---');

  final crypto = NativeCryptoService.instance;
  const password = 'test_password_123';
  final salt = crypto.generateSalt()!;
  const iterations = 3;
  const memoryKib = 65536;
  const parallelism = 4;

  // Warm-up run
  crypto.deriveKey(
    password: password,
    salt: salt,
    memoryKib: memoryKib,
    iterations: iterations,
    parallelism: parallelism,
  );

  const runs = 5;
  final results = <int>[];

  for (var i = 0; i < runs; i++) {
    final sw = Stopwatch()..start();
    crypto.deriveKey(
      password: password,
      salt: salt,
      memoryKib: memoryKib,
      iterations: iterations,
      parallelism: parallelism,
    );
    sw.stop();
    results.add(sw.elapsedMicroseconds);
  }

  final avg = results.reduce((a, b) => a + b) / runs;
  final min = results.reduce((a, b) => a < b ? a : b);
  final max = results.reduce((a, b) => a > b ? a : b);

  print('Params: memory=${memoryKib}KiB, iterations=$iterations, parallelism=$parallelism');
  print('Runs: $runs');
  print('Avg: ${(avg / 1000).toStringAsFixed(2)} ms');
  print('Min: ${(min / 1000).toStringAsFixed(2)} ms');
  print('Max: ${(max / 1000).toStringAsFixed(2)} ms');
}

void _benchmarkAESEncryptDecrypt() {
  print('\n--- AES-256-GCM Encrypt/Decrypt Roundtrip ---');

  final crypto = NativeCryptoService.instance;
  final key = Uint8List(32)..fillRange(0, 32, 1);
  final nonce = Uint8List(12)..fillRange(0, 12, 2);

  // Test data sizes
  final sizes = [64, 1024, 10240]; // 64B, 1KB, 10KB

  for (final size in sizes) {
    final data = Uint8List(size)..fillRange(0, size, 3);

    // Warm-up
    final encrypted = crypto.encrypt(data: data, key: key, nonce: nonce);
    crypto.decrypt(encrypted: encrypted!, key: key, nonce: nonce);

    const runs = 20;
    final encryptTimes = <int>[];
    final decryptTimes = <int>[];
    final roundtripTimes = <int>[];

    for (var i = 0; i < runs; i++) {
      // Measure encrypt
      final swEncrypt = Stopwatch()..start();
      final enc = crypto.encrypt(data: data, key: key, nonce: nonce);
      swEncrypt.stop();
      encryptTimes.add(swEncrypt.elapsedMicroseconds);

      // Measure decrypt
      final swDecrypt = Stopwatch()..start();
      crypto.decrypt(encrypted: enc!, key: key, nonce: nonce);
      swDecrypt.stop();
      decryptTimes.add(swDecrypt.elapsedMicroseconds);

      roundtripTimes.add(swEncrypt.elapsedMicroseconds + swDecrypt.elapsedMicroseconds);
    }

    final avgEncrypt = encryptTimes.reduce((a, b) => a + b) / runs;
    final avgDecrypt = decryptTimes.reduce((a, b) => a + b) / runs;
    final avgRoundtrip = roundtripTimes.reduce((a, b) => a + b) / runs;

    print('\nData size: $size bytes');
    print('Encrypt avg: ${avgEncrypt.toStringAsFixed(2)} us');
    print('Decrypt avg: ${avgDecrypt.toStringAsFixed(2)} us');
    print('Roundtrip avg: ${avgRoundtrip.toStringAsFixed(2)} us');
  }
}
