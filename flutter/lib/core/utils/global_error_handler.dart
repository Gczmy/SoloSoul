import 'package:flutter/material.dart';

/// Vault error types for specific error messaging
enum VaultErrorType {
  databaseCorrupted,
  pathPermissionDenied,
  keyDerivationFailed,
  invalidPassword,
  vaultNotUnlocked,
  unknown,
}

/// Global error handler for vault operations
/// Provides consistent error UI across the app via SnackBar
class GlobalErrorHandler {
  /// Show a SnackBar with vault error information
  static void showSnackBar(
    BuildContext context,
    String message, {
    VaultErrorType? errorType,
    bool isWarning = false,
    Duration? duration,
  }) {
    final color = isWarning
        ? Colors.orange
        : errorType == VaultErrorType.databaseCorrupted
            ? Colors.red
            : errorType == VaultErrorType.pathPermissionDenied
                ? Colors.orange
                : Colors.red;

    final icon = isWarning
        ? Icons.warning_amber_rounded
        : errorType == VaultErrorType.databaseCorrupted
            ? Icons.broken_image_outlined
            : errorType == VaultErrorType.pathPermissionDenied
                ? Icons.lock_outline
                : errorType == VaultErrorType.keyDerivationFailed
                    ? Icons.key_off
                    : errorType == VaultErrorType.invalidPassword
                        ? Icons.password
                        : errorType == VaultErrorType.vaultNotUnlocked
                            ? Icons.lock_outline
                            : Icons.error_outline;

    ScaffoldMessenger.of(context).hideCurrentSnackBar();
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(icon, color: color),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                message,
                style: const TextStyle(color: Colors.white),
              ),
            ),
          ],
        ),
        backgroundColor: color.withValues(alpha: 0.9),
        duration: duration ?? const Duration(seconds: 4),
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
        margin: const EdgeInsets.all(16),
      ),
    );
  }

  /// Show vault error with specific type
  static void showVaultError(
    BuildContext context,
    String message,
    VaultErrorType type,
  ) {
    showSnackBar(context, message, errorType: type);
  }

  /// Parse vault error message to determine error type
  static VaultErrorType parseErrorType(String? errorMessage) {
    if (errorMessage == null) return VaultErrorType.unknown;

    final msg = errorMessage.toLowerCase();

    if (msg.contains('database') && (msg.contains('corrupt') || msg.contains('damage'))) {
      return VaultErrorType.databaseCorrupted;
    }
    if (msg.contains('permission') || msg.contains('denied') || msg.contains('access')) {
      return VaultErrorType.pathPermissionDenied;
    }
    if (msg.contains('key') && (msg.contains('deriv') || msg.contains('fail'))) {
      return VaultErrorType.keyDerivationFailed;
    }
    if (msg.contains('password') && (msg.contains('invalid') || msg.contains('wrong'))) {
      return VaultErrorType.invalidPassword;
    }
    if (msg.contains('unlock') && msg.contains('not')) {
      return VaultErrorType.vaultNotUnlocked;
    }

    return VaultErrorType.unknown;
  }

  /// Show error based on Rust vault response
  static void showFromVaultResponse(
    BuildContext context,
    String? errorMessage,
  ) {
    final type = parseErrorType(errorMessage);
    final message = errorMessage ?? 'An unknown error occurred';

    showSnackBar(
      context,
      _friendlyMessage(type, message),
      errorType: type,
    );
  }

  /// Get user-friendly error message
  static String _friendlyMessage(VaultErrorType type, String original) {
    switch (type) {
      case VaultErrorType.databaseCorrupted:
        return 'Database file appears to be corrupted. Please contact support.';
      case VaultErrorType.pathPermissionDenied:
        return 'Unable to access storage. Please check app permissions.';
      case VaultErrorType.keyDerivationFailed:
        return 'Failed to derive encryption key. Please try again.';
      case VaultErrorType.invalidPassword:
        return 'Incorrect password. Please try again.';
      case VaultErrorType.vaultNotUnlocked:
        return 'Vault is locked. Please unlock first.';
      case VaultErrorType.unknown:
        return original;
    }
  }
}

/// Error codes from Rust vault for programmatic handling
class VaultErrorCodes {
  static const invalidPassword = 'INVALID_PASSWORD';
  static const databaseCorrupted = 'DATABASE_CORRUPTED';
  static const pathPermissionDenied = 'PATH_PERMISSION_DENIED';
  static const keyDerivationFailed = 'KEY_DERIVATION_FAILED';
  static const vaultNotUnlocked = 'VAULT_NOT_UNLOCKED';
}
