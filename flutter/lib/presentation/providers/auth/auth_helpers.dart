import 'dart:typed_data';

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

/// Constant-time string comparison to prevent timing attacks
bool constantTimeEquals(String a, String b) {
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
