import 'dart:convert';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';

/// Security settings for the app including auto-lock and clipboard behavior.
class SecuritySettings {
  final int autoLockDelayMinutes;
  final int clipboardClearDelaySeconds;
  final bool lockOnWindowBlur;
  final bool biometricsEnabled;
  final bool faceIdEnabled;
  final bool privacyScreenEnabled;

  const SecuritySettings({
    this.autoLockDelayMinutes = 5,
    this.clipboardClearDelaySeconds = 60,
    this.lockOnWindowBlur = true,
    this.biometricsEnabled = false,
    this.faceIdEnabled = false,
    this.privacyScreenEnabled = true,
  });

  SecuritySettings copyWith({
    int? autoLockDelayMinutes,
    int? clipboardClearDelaySeconds,
    bool? lockOnWindowBlur,
    bool? biometricsEnabled,
    bool? faceIdEnabled,
    bool? privacyScreenEnabled,
  }) {
    return SecuritySettings(
      autoLockDelayMinutes: autoLockDelayMinutes ?? this.autoLockDelayMinutes,
      clipboardClearDelaySeconds: clipboardClearDelaySeconds ?? this.clipboardClearDelaySeconds,
      lockOnWindowBlur: lockOnWindowBlur ?? this.lockOnWindowBlur,
      biometricsEnabled: biometricsEnabled ?? this.biometricsEnabled,
      faceIdEnabled: faceIdEnabled ?? this.faceIdEnabled,
      privacyScreenEnabled: privacyScreenEnabled ?? this.privacyScreenEnabled,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'autoLockDelayMinutes': autoLockDelayMinutes,
      'clipboardClearDelaySeconds': clipboardClearDelaySeconds,
      'lockOnWindowBlur': lockOnWindowBlur,
      'biometricsEnabled': biometricsEnabled,
      'faceIdEnabled': faceIdEnabled,
      'privacyScreenEnabled': privacyScreenEnabled,
    };
  }

  factory SecuritySettings.fromJson(Map<String, dynamic> json) {
    return SecuritySettings(
      autoLockDelayMinutes: json['autoLockDelayMinutes'] as int? ?? 5,
      clipboardClearDelaySeconds: json['clipboardClearDelaySeconds'] as int? ?? 60,
      lockOnWindowBlur: json['lockOnWindowBlur'] as bool? ?? true,
      biometricsEnabled: json['biometricsEnabled'] as bool? ?? false,
      faceIdEnabled: json['faceIdEnabled'] as bool? ?? false,
      privacyScreenEnabled: json['privacyScreenEnabled'] as bool? ?? true,
    );
  }

  static const List<int> autoLockDelayOptions = [1, 5, 15, 30, -1];
  static const List<int> clipboardClearDelayOptions = [30, 60, 120, -1];

  String get autoLockDelayLabel {
    if (autoLockDelayMinutes == -1) return 'Never';
    return '$autoLockDelayMinutes min';
  }

  String get clipboardClearDelayLabel {
    if (clipboardClearDelaySeconds == -1) return 'Never';
    return '$clipboardClearDelaySeconds sec';
  }
}

/// Service for managing security settings with persistent storage.
class SecurityService {
  SecurityService._();

  static SecurityService? _instance;
  // NOTE: Using first_unlock_this_device ensures data is accessible after first device unlock
  // but NOT before device is unlocked. This is appropriate for passwords that should persist
  // after initial device setup. Consider after_first_unlock_this_device if data must be
  // accessible even before any unlock (requires device passcode setup).
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
  );

  static const _keySettings = 'security_settings';

  final FallbackSecureStorage _fallbackStorage = FallbackSecureStorage();

  SecuritySettings _settings = const SecuritySettings();
  bool _initialized = false;

  /// Singleton instance
  static SecurityService get instance {
    _instance ??= SecurityService._();
    return _instance!;
  }

  /// Current security settings
  SecuritySettings get settings => _settings;

  /// Whether settings have been loaded from storage
  bool get isInitialized => _initialized;

  /// Load settings from secure storage.
  /// Should be called at app startup.
  Future<void> loadSettings() async {
    try {
      final data = await _storage.read(key: _keySettings);
      if (data != null && data.isNotEmpty) {
        final json = Map<String, dynamic>.from(jsonDecode(data) as Map);
        _settings = SecuritySettings.fromJson(json);
      }
      _initialized = true;
    } on Exception catch (e) {
      DebugLogger.instance.logWarning('SECURITY', 'Failed to load security settings: $e');
      _initialized = true;
    }
  }

  /// Save current settings to secure storage.
  Future<void> saveSettings() async {
    final data = jsonEncode(_settings.toJson());
    try {
      await _storage.write(key: _keySettings, value: data);
    } on Exception {
      // Fallback to file-based storage if Keychain is unavailable
      await _fallbackStorage.write(key: _keySettings, value: data);
    }
  }

  /// Update auto lock delay.
  Future<void> setAutoLockDelay(int minutes) async {
    _settings = _settings.copyWith(autoLockDelayMinutes: minutes);
    await saveSettings();
  }

  /// Update clipboard clear delay.
  Future<void> setClipboardClearDelay(int seconds) async {
    _settings = _settings.copyWith(clipboardClearDelaySeconds: seconds);
    await saveSettings();
  }

  /// Update lock on window blur setting.
  Future<void> setLockOnWindowBlur(bool enabled) async {
    _settings = _settings.copyWith(lockOnWindowBlur: enabled);
    await saveSettings();
  }

  /// Update biometric unlock setting (Touch ID / fingerprint).
  Future<void> setBiometricsEnabled(bool enabled) async {
    _settings = _settings.copyWith(biometricsEnabled: enabled);
    await saveSettings();
  }

  /// Update Face ID unlock setting.
  Future<void> setFaceIdEnabled(bool enabled) async {
    _settings = _settings.copyWith(faceIdEnabled: enabled);
    await saveSettings();
  }

  /// Reset settings to defaults.
  Future<void> resetToDefaults() async {
    _settings = const SecuritySettings();
    await saveSettings();
    await BiometricCredentialService.instance.clearAllBiometricCredentials();
  }
}