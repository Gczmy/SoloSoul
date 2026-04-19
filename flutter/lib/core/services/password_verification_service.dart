import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show sensitivitySettingsProvider, SensitivityLevel;

final passwordVerificationServiceProvider = Provider((ref) => PasswordVerificationService(ref));

class PasswordVerificationService {
  final Ref _ref;
  PasswordVerificationService(this._ref);

  /// Verify password for sensitive actions. Returns true if verified or not required.
  Future<bool> verifyForSensitiveData(
    BuildContext context, {
    required WidgetRef ref,
    String? fieldId,
  }) async {
    if (fieldId == null) return true;

    final settings = _ref.read(sensitivitySettingsProvider);
    final level = settings.getFieldLevel(fieldId);

    if (level != SensitivityLevel.restricted) return true;

    // Check if user was verified within the last 1 minute
    final sensitiveAccess = _ref.read(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    final hasRecentVerification = sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);
    if (hasRecentVerification) return true;

    // Show password dialog
    final authNotifier = _ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return false;

    _ref.read(sensitivePageAccessProvider.notifier).markVerified();
    return true;
  }
}
