import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:local_auth/local_auth.dart';

/// Keychain service for secure storage and biometric authentication
/// - macOS: uses native Keychain via MethodChannel
/// - Windows: uses flutter_secure_storage (Windows Credential Manager) + local_auth (Windows Hello)
class KeychainService {
  static const _channel = MethodChannel('com.solosoul/keychain');
  static final _localAuth = LocalAuthentication();

  // flutter_secure_storage instance for non-macOS platforms
  static final _secureStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
    wOptions: WindowsOptions(),
  );

  /// Authenticate with biometrics (TouchID/FaceID on macOS, Windows Hello on Windows)
  static Future<Map<String, dynamic>> authenticateWithBiometrics() async {
    if (Platform.isMacOS) {
      try {
        final result = await _channel.invokeMethod('authenticateWithBiometrics');
        return Map<String, dynamic>.from(result);
      } on PlatformException catch (e) {
        return {'success': false, 'error': e.message};
      }
    }

    // Windows and other platforms: use local_auth
    try {
      final isAvailable = await _localAuth.canCheckBiometrics;
      final isDeviceSupported = await _localAuth.isDeviceSupported();

      if (!isAvailable || !isDeviceSupported) {
        return {'success': false, 'error': 'Biometrics not available'};
      }

      final biometryType = await _localAuth.getAvailableBiometrics();
      String biometryTypeStr = 'none';
      if (biometryType.contains(BiometricType.face)) {
        biometryTypeStr = 'faceID';
      } else if (biometryType.contains(BiometricType.fingerprint)) {
        biometryTypeStr = 'touchID';
      } else if (biometryType.contains(BiometricType.iris)) {
        biometryTypeStr = 'opticID';
      } else if (biometryType.contains(BiometricType.strong)) {
        biometryTypeStr = 'windowsHello';
      }

      final didAuthenticate = await _localAuth.authenticate(
        localizedReason: 'Authenticate to access secure data',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );

      return {
        'success': didAuthenticate,
        'biometryType': biometryTypeStr,
      };
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Save value to secure storage (Keychain on macOS, Credential Manager on Windows)
  static Future<Map<String, dynamic>> save(String key, String value) async {
    if (Platform.isMacOS) {
      try {
        final result = await _channel.invokeMethod('saveToKeychain', {
          'key': key,
          'value': value,
        });
        return Map<String, dynamic>.from(result);
      } on PlatformException catch (e) {
        return {'success': false, 'error': e.message};
      }
    }

    // Windows and other platforms: use flutter_secure_storage
    try {
      await _secureStorage.write(key: key, value: value);
      return {'success': true};
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Read value from secure storage (Keychain on macOS, Credential Manager on Windows)
  static Future<Map<String, dynamic>> read(String key) async {
    if (Platform.isMacOS) {
      try {
        final result = await _channel.invokeMethod('readFromKeychain', {
          'key': key,
        });
        return Map<String, dynamic>.from(result);
      } on PlatformException catch (e) {
        return {'success': false, 'error': e.message};
      }
    }

    // Windows and other platforms: use flutter_secure_storage
    try {
      final value = await _secureStorage.read(key: key);
      if (value != null) {
        return {'success': true, 'value': value};
      } else {
        return {'success': false, 'error': 'Key not found'};
      }
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Delete value from secure storage (Keychain on macOS, Credential Manager on Windows)
  static Future<Map<String, dynamic>> delete(String key) async {
    if (Platform.isMacOS) {
      try {
        final result = await _channel.invokeMethod('deleteFromKeychain', {
          'key': key,
        });
        return Map<String, dynamic>.from(result);
      } on PlatformException catch (e) {
        return {'success': false, 'error': e.message};
      }
    }

    // Windows and other platforms: use flutter_secure_storage
    try {
      await _secureStorage.delete(key: key);
      return {'success': true};
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Check if biometrics are available
  static Future<bool> isBiometricsAvailable() async {
    final result = await authenticateWithBiometrics();
    return result['success'] == true;
  }

  /// Get biometry type (touchID, faceID, opticID, windowsHello, none)
  static Future<String> getBiometryType() async {
    if (Platform.isMacOS) {
      final result = await authenticateWithBiometrics();
      return result['biometryType'] ?? 'none';
    }

    // Windows: check available biometrics
    try {
      final biometryType = await _localAuth.getAvailableBiometrics();
      if (biometryType.contains(BiometricType.face)) {
        return 'faceID';
      } else if (biometryType.contains(BiometricType.fingerprint)) {
        return 'touchID';
      } else if (biometryType.contains(BiometricType.iris)) {
        return 'opticID';
      } else if (biometryType.contains(BiometricType.strong)) {
        return 'windowsHello';
      }
      return 'none';
    } on PlatformException {
      return 'none';
    }
  }
}
