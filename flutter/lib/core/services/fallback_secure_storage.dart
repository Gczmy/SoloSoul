import 'dart:convert';
import 'dart:io';
import 'package:flutter/services.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

/// Keychain-aware secure storage with transparent file-based fallback.
///
/// On macOS/iOS, FlutterSecureStorage uses Keychain. In sandboxed apps
/// without proper entitlements/codesigning, Keychain access may return -34018.
/// This wrapper detects Keychain failures and transparently falls back to file
/// storage in the app's support directory.
///
/// **SECURITY WARNING**: Fallback files are stored as plaintext JSON with 0600
/// permissions. This is less secure than Keychain and **should not be relied
/// upon for production use**. Users should be encouraged to configure proper
/// Keychain entitlements. A future version will add AES-256-GCM encryption
/// to fallback files using the Rust crypto backend.
class FallbackSecureStorage {
  static const _fallbackDirName = 'solosoul_fallback_storage';

  final FlutterSecureStorage _secureStorage;
  Directory? _fallbackDir;
  bool _fallbackWarningEmitted = false;

  FallbackSecureStorage({FlutterSecureStorage? secureStorage})
      : _secureStorage = secureStorage ??
          const FlutterSecureStorage(
            mOptions: MacOsOptions(
              accessibility: KeychainAccessibility.first_unlock,
            ),
          );

  Future<Directory> _getFallbackDir() async {
    final existing = _fallbackDir;
    if (existing != null) return existing;
    final supportDir = await getApplicationSupportDirectory();
    final dir = Directory('${supportDir.path}/$_fallbackDirName');
    _fallbackDir = dir;
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  String _fallbackFilePath(Directory dir, String key) {
    final sanitized = base64Url.encode(utf8.encode(key));
    return '${dir.path}/$sanitized.json';
  }

  bool _isKeychainError(Object e) {
    if (Platform.isMacOS || Platform.isIOS) {
      return e.toString().contains('-34018') ||
          e.toString().contains('errSecMissingEntitlement');
    }
    return false;
  }

  Future<void> _writeFallback(String key, String? value) async {
    if (!_fallbackWarningEmitted) {
      _fallbackWarningEmitted = true;
      DebugLogger.instance.logWarning(
        'FALLBACK_STORAGE',
        'SECURITY: Writing sensitive data to plaintext fallback storage. '
        'Configure Keychain entitlements for production use.',
      );
    }
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
    try {
      final result = await Process.run('chmod', ['600', path]);
      if (result.exitCode != 0) {
        DebugLogger.instance.logWarning(
          'FALLBACK_STORAGE',
          'chmod 600 failed for fallback file (exit ${result.exitCode}). '
          'File may be world-readable.',
        );
      }
    } on Exception {
      DebugLogger.instance.logWarning(
        'FALLBACK_STORAGE',
        'chmod not available on this platform. '
        'Fallback storage files use default permissions.',
      );
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
    try {
      final value = await _secureStorage
          .read(key: key)
          .timeout(const Duration(seconds: 30));
      return value;
    } on PlatformException catch (e) {
      if (_isKeychainError(e)) {
        DebugLogger.instance.logWarning(
            'FALLBACK_STORAGE', 'Keychain unavailable ($e). Using fallback.');
      } else {
        DebugLogger.instance.logError(
            'FALLBACK_STORAGE', 'Keychain read error for $key: $e');
      }
    } on Exception catch (e) {
      DebugLogger.instance.logWarning(
          'FALLBACK_STORAGE', 'Keychain read timeout/other error for $key: $e. Using fallback.');
    }
    return _readFallback(key);
  }

  /// Write value. Tries Keychain first, falls back to file storage.
  Future<void> write({required String key, required String? value}) async {
    try {
      await _secureStorage
          .write(key: key, value: value)
          .timeout(const Duration(seconds: 30));
      return;
    } on PlatformException catch (e) {
      if (_isKeychainError(e)) {
        DebugLogger.instance.logWarning(
            'FALLBACK_STORAGE', 'Keychain unavailable ($e). Using fallback.');
      } else {
        DebugLogger.instance.logError(
            'FALLBACK_STORAGE', 'Keychain write error for $key: $e');
      }
    } on Exception catch (e) {
      DebugLogger.instance.logWarning(
          'FALLBACK_STORAGE', 'Keychain write timeout/other error for $key: $e. Using fallback.');
    }
    await _writeFallback(key, value);
  }

  /// Delete value from both Keychain and fallback storage.
  Future<void> delete({required String key}) async {
    try {
      await _secureStorage.delete(key: key).timeout(const Duration(seconds: 30));
    } on PlatformException catch (e) {
      if (_isKeychainError(e)) {
        DebugLogger.instance.logDebug(
            'FALLBACK_STORAGE', 'Keychain delete unavailable ($e).');
      } else {
        DebugLogger.instance.logError(
            'FALLBACK_STORAGE', 'Keychain delete error for $key: $e');
      }
    } on Exception catch (e) {
      DebugLogger.instance.logWarning(
          'FALLBACK_STORAGE', 'Keychain delete timeout/other error for $key: $e');
    }
    await _deleteFallback(key);
  }

  /// Delete all values from both Keychain and fallback storage.
  Future<void> deleteAll() async {
    try {
      await _secureStorage.deleteAll().timeout(const Duration(seconds: 30));
    } on PlatformException catch (e) {
      if (_isKeychainError(e)) {
        DebugLogger.instance.logDebug(
            'FALLBACK_STORAGE', 'Keychain deleteAll unavailable ($e).');
      } else {
        DebugLogger.instance
            .logError('FALLBACK_STORAGE', 'Keychain deleteAll error: $e');
      }
    } on Exception catch (e) {
      DebugLogger.instance.logWarning(
          'FALLBACK_STORAGE', 'Keychain deleteAll timeout/other error: $e');
    }
    final dir = await _getFallbackDir();
    if (await dir.exists()) {
      await dir.delete(recursive: true);
      await dir.create();
    }
  }
}
