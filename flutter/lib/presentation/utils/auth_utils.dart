import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

/// Show password verification dialog and mark sensitive access as granted
/// on success. Returns true if verification succeeded.
Future<bool> verifyPasswordAndGrantAccess({
  required BuildContext context,
  required WidgetRef ref,
  required String message,
}) async {
  final authNotifier = ref.read(authNotifierProvider.notifier);
  final selectedAccount = authNotifier.selectedAccount;
  final result = await showPasswordVerificationDialog(
    context: context,
    ref: ref,
    message: message,
    passwordHint: selectedAccount?.passwordHint,
    onVerify: authNotifier.verifyPasswordForSensitiveData,
  );

  if (!context.mounted) return false;

  if (result != null) {
    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    return true;
  }
  return false;
}
