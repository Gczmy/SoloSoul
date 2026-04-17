import 'package:flutter/services.dart';
import 'package:local_auth/local_auth.dart';

export 'package:local_auth/local_auth.dart' show BiometricType;

/// Biometric authentication service for password-less unlock
/// Uses platform-native biometric (Face ID / Touch ID on iOS, Fingerprint on Android)
class BiometricService {
  static BiometricService? _instance;
  static BiometricService get instance => _instance ??= BiometricService._();
  BiometricService._();

  final LocalAuthentication _localAuth = LocalAuthentication();

  /// Check if biometric authentication is available
  Future<bool> isAvailable() async {
    try {
      final canCheck = await _localAuth.canCheckBiometrics;
      final isSupported = await _localAuth.isDeviceSupported();
      return canCheck && isSupported;
    } on PlatformException {
      return false;
    }
  }

  /// Authenticate using biometrics
  /// Returns true if successful, false otherwise
  Future<bool> authenticate({String reason = 'Unlock SoloSoul'}) async {
    try {
      final result = await _localAuth.authenticate(
        localizedReason: reason,
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );
      return result;
    } on PlatformException {
      return false;
    }
  }

  /// Get available biometric types from local_auth
  Future<List<BiometricType>> getAvailableBiometrics() async {
    try {
      final types = await _localAuth.getAvailableBiometrics();
      return types;
    } on PlatformException {
      return [];
    }
  }
}
