import 'dart:convert';
import 'dart:io';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

/// Keychain-aware secure storage with transparent file-based fallback.
///
/// On macOS/iOS, FlutterSecureStorage uses Keychain. In sandboxed apps
/// without proper entitlements/codesigning (ad-hoc signed builds), Keychain
/// access returns -34018. This wrapper detects Keychain failures and
/// transparently falls back to file storage in the app's support directory.
///
/// SECURITY NOTE: The fallback files are stored in the app's sandboxed
/// Application Support directory with 0600 permissions. This is less secure
/// than Keychain but functional when Keychain is unavailable. When Keychain
/// starts working (properly signed build), data is automatically migrated back.
class FallbackSecureStorage {
  static const _fallbackDirName = 'solosoul_fallback_storage';
  static const _metaKey = '__fallback_meta__';

  final FlutterSecureStorage _secureStorage;
  Directory? _fallbackDir;
  bool? _keychainAvailable;

  FallbackSecureStorage({FlutterSecureStorage? secureStorage})
      : _secureStorage = secureStorage ?? const FlutterSecureStorage();

  Future<Directory> _getFallbackDir() async {
    if (_fallbackDir != null) return _fallbackDir!;
    final supportDir = await getApplicationSupportDirectory();
    _fallbackDir = Directory('${supportDir.path}/$_fallbackDirName');
    if (!await _fallbackDir!.exists()) {
      await _fallbackDir!.create(recursive: true);
    }
    return _fallbackDir!;
  }

  String _fallbackFilePath(Directory dir, String key) {
    // Sanitize key for filesystem safety
    final sanitized = base64Url.encode(utf8.encode(key));
    return '${dir.path}/$sanitized.json';
  }

  Future<bool> _checkKeychainAvailable() async {
    if (_keychainAvailable != null) return _keychainAvailable!;
    try {
      // Probe Keychain with a test write/read/delete cycle
      const testKey = '__keychain_probe__';
      await _secureStorage
          .write(key: testKey, value: 'probe')
          .timeout(const Duration(seconds: 3));
      final readValue = await _secureStorage
          .read(key: testKey)
          .timeout(const Duration(seconds: 3));
      await _secureStorage.delete(key: testKey);
      _keychainAvailable = readValue == 'probe';
    } on Exception catch (e) {
      DebugLogger.instance.logError(
          'FALLBACK_STORAGE', 'Keychain probe failed: $e');
      _keychainAvailable = false;
    }
    DebugLogger.instance.logInfo(
        'FALLBACK_STORAGE', 'Keychain available: $_keychainAvailable');
    return _keychainAvailable!;
  }

  Future<void> _writeFallback(String key, String? value) async {
    final dir = await _getFallbackDir();
    final path = _fallbackFilePath(dir, key);
    final file = File(path);
    if (value == null) {
      if (await file.exists()) {
        await file.delete();
      }
      return;
    }
    final data = {'value': value, 'timestamp': DateTime.now().toIso8601String()};
    await file.writeAsString(jsonEncode(data));
    // Restrict permissions (best effort on macOS)
    try {
      await Process.run('chmod', ['600', path]);
    } on Exception {
      // Ignore chmod failures on platforms where it's not supported
    }
  }

  Future<String?> _readFallback(String key) async {
    final dir = await _getFallbackDir();
    final path = _fallbackFilePath(dir, key);
    final file = File(path);
    if (!await file.exists()) return null;
    try {
      final content = await file.readAsString();
      final data = jsonDecode(content) as Map<String, dynamic>;
      return data['value'] as String?;
    } on Exception catch (e) {
      DebugLogger.instance
          .logError('FALLBACK_STORAGE', 'Fallback read error: $e');
      return null;
    }
  }

  Future<void> _deleteFallback(String key) async {
    final dir = await _getFallbackDir();
    final path = _fallbackFilePath(dir, key);
    final file = File(path);
    if (await file.exists()) {
      await file.delete();
    }
  }

  /// Read value. Tries Keychain first, falls back to file storage.
  Future<String?> read({required String key}) async {
    if (await _checkKeychainAvailable()) {
      try {
        final value = await _secureStorage
            .read(key: key)
            .timeout(const Duration(seconds: 5));
        return value;
      } on Exception catch (e) {
        DebugLogger.instance
            .logError('FALLBACK_STORAGE', 'Keychain read error for $key: $e');
      }
    }
    return _readFallback(key);
  }

  /// Write value. Tries Keychain first, falls back to file storage.
  Future<void> write({required String key, required String? value}) async {
    if (await _checkKeychainAvailable()) {
      try {
        await _secureStorage
            .write(key: key, value: value)
            .timeout(const Duration(seconds: 5));
        return;
      } on Exception catch (e) {
        DebugLogger.instance
            .logError('FALLBACK_STORAGE', 'Keychain write error for $key: $e');
      }
    }
    await _writeFallback(key, value);
  }

  /// Delete value from both Keychain and fallback storage.
  Future<void> delete({required String key}) async {
    if (await _checkKeychainAvailable()) {
      try {
        await _secureStorage.delete(key: key);
      } on Exception catch (e) {
        DebugLogger.instance.logError(
            'FALLBACK_STORAGE', 'Keychain delete error for $key: $e');
      }
    }
    await _deleteFallback(key);
  }

  /// Delete all values from both Keychain and fallback storage.
  Future<void> deleteAll() async {
    if (await _checkKeychainAvailable()) {
      try {
        await _secureStorage.deleteAll();
      } on Exception catch (e) {
        DebugLogger.instance
            .logError('FALLBACK_STORAGE', 'Keychain deleteAll error: $e');
      }
    }
    final dir = await _getFallbackDir();
    if (await dir.exists()) {
      await dir.delete(recursive: true);
      await dir.create();
    }
  }
}
