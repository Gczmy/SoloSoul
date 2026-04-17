import 'package:flutter/services.dart';

/// Keychain service for macOS native Keychain access
class KeychainService {
  static const _channel = MethodChannel('com.solosoul/keychain');

  /// Authenticate with biometrics (TouchID/FaceID)
  static Future<Map<String, dynamic>> authenticateWithBiometrics() async {
    try {
      final result = await _channel.invokeMethod('authenticateWithBiometrics');
      return Map<String, dynamic>.from(result);
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Save value to Keychain
  static Future<Map<String, dynamic>> save(String key, String value) async {
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

  /// Read value from Keychain
  static Future<Map<String, dynamic>> read(String key) async {
    try {
      final result = await _channel.invokeMethod('readFromKeychain', {
        'key': key,
      });
      return Map<String, dynamic>.from(result);
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Delete value from Keychain
  static Future<Map<String, dynamic>> delete(String key) async {
    try {
      final result = await _channel.invokeMethod('deleteFromKeychain', {
        'key': key,
      });
      return Map<String, dynamic>.from(result);
    } on PlatformException catch (e) {
      return {'success': false, 'error': e.message};
    }
  }

  /// Check if biometrics are available
  static Future<bool> isBiometricsAvailable() async {
    final result = await authenticateWithBiometrics();
    return result['success'] == true;
  }

  /// Get biometry type (touchID, faceID, opticID, none)
  static Future<String> getBiometryType() async {
    final result = await authenticateWithBiometrics();
    return result['biometryType'] ?? 'none';
  }
}
