import 'dart:typed_data';

import 'package:solosoul_flutter/frb/api.dart' as frb;

/// Convert bytes to hex string (for Rust-compatible verification hashes)
String bytesToHex(List<int> bytes) {
  return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
}

/// Convert hex string to bytes
Uint8List hexToBytes(String hex) {
  final result = <int>[];
  for (var i = 0; i < hex.length; i += 2) {
    result.add(int.parse(hex.substring(i, i + 2), radix: 16));
  }
  return Uint8List.fromList(result);
}

/// Constant-time string comparison to prevent timing attacks.
/// Delegates to Rust FFI for guaranteed constant-time execution.
Future<bool> constantTimeEquals(String a, String b) async {
  return frb.frbConstantTimeCompare(
    a: Uint8List.fromList(a.codeUnits),
    b: Uint8List.fromList(b.codeUnits),
  );
}

/// Synchronous fallback for contexts where async is not available.
/// WARNING: This uses Dart's runtime which cannot guarantee constant-time.
/// Prefer the async `constantTimeEquals` when possible.
bool constantTimeEqualsSync(String a, String b) {
  final lenA = a.length;
  final lenB = b.length;
  final maxLen = lenA > lenB ? lenA : lenB;

  final paddedA = a.padRight(maxLen, '\x00');
  final paddedB = b.padRight(maxLen, '\x00');

  var result = 0;
  for (var i = 0; i < maxLen; i++) {
    result |= paddedA.codeUnitAt(i) ^ paddedB.codeUnitAt(i);
  }
  result |= lenA ^ lenB;
  return result == 0;
}
