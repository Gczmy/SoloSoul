import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Language Service
// =============================================================================

/// 管理应用语言偏好的持久化存储。
///
/// 主存储使用 [FlutterSecureStorage]，iOS Keychain 不可用时降级到
/// [SharedPreferences]。语言偏好为应用级设置，不绑定特定账户。
class LanguageService {
  static const _key = 'app_language';
  static final LanguageService instance = LanguageService._();
  LanguageService._();

  String? _cached;

  /// 读取当前语言代码，默认 'en'。
  Future<String> getLanguage() async {
    if (_cached != null) return _cached!;
    try {
      const storage = FlutterSecureStorage();
      final value = await storage.read(key: _key);
      _cached = value ?? 'en';
    } on Exception catch (e) {
      SoloLog.w('LanguageService',
          'SecureStorage read failed, fallback to SharedPreferences', e);
      try {
        final prefs = await SharedPreferences.getInstance();
        _cached = prefs.getString(_key) ?? 'en';
      } on Exception catch (_) {
        _cached = 'en';
      }
    }
    return _cached!;
  }

  /// 检查用户是否曾手动设置过语言偏好。
  Future<bool> hasStoredPreference() async {
    try {
      const storage = FlutterSecureStorage();
      return await storage.read(key: _key) != null;
    } on Exception {
      try {
        final prefs = await SharedPreferences.getInstance();
        return prefs.containsKey(_key);
      } on Exception {
        return false;
      }
    }
  }

  /// 写入语言代码。
  Future<void> setLanguage(String languageCode) async {
    _cached = languageCode;
    try {
      const storage = FlutterSecureStorage();
      await storage.write(key: _key, value: languageCode);
    } on Exception catch (e) {
      SoloLog.w('LanguageService',
          'SecureStorage write failed, fallback to SharedPreferences', e);
      try {
        final prefs = await SharedPreferences.getInstance();
        await prefs.setString(_key, languageCode);
      } on Exception catch (fallbackErr) {
        SoloLog.e('LanguageService', 'All persistence failed', fallbackErr);
      }
    }
  }
}
