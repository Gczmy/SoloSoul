import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Shared password verification dialog for sensitive operations.
/// Returns the password if verified, null if cancelled.
///
/// Usage:
/// ```dart
/// final password = await showPasswordVerificationDialog(
///   context: context,
///   ref: ref,
///   onVerify: (password) => authNotifier.verifyPasswordForSensitiveData(password),
/// );
/// ```
Future<String?> showPasswordVerificationDialog({
  required BuildContext context,
  required WidgetRef ref,
  String message = 'Restricted field. Enter your master password to proceed.',
  required Future<bool> Function(String password) onVerify,
}) async {
  final controller = TextEditingController();
  String? error;
  bool isVerifyingDialog = false;
  bool isPasswordEmpty = true;

  return showDialog<String>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => StatefulBuilder(
      builder: (dialogContext, setDialogState) {
        isPasswordEmpty = controller.text.isEmpty;

        return AlertDialog(
          title: Row(
            children: [
              Icon(Icons.lock_outline, color: Colors.orange.shade700),
              const SizedBox(width: 8),
              const Text('Verify Identity'),
            ],
          ),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Colors.orange.shade50,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: Colors.orange.shade200),
                ),
                child: Row(
                  children: [
                    Icon(Icons.info_outline, color: Colors.orange.shade700, size: 20),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        message,
                        style: const TextStyle(fontSize: 13),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              TextField(
                controller: controller,
                obscureText: true,
                autofocus: true,
                decoration: const InputDecoration(
                  labelText: 'Master Password',
                  prefixIcon: Icon(Icons.key),
                ),
                onChanged: (value) {
                  if (error != null) {
                    setDialogState(() => error = null);
                  }
                  setDialogState(() => isPasswordEmpty = value.isEmpty);
                },
                onSubmitted: (_) async {
                  if (isPasswordEmpty) return;
                  setDialogState(() => isVerifyingDialog = true);
                  final success = await onVerify(controller.text);
                  if (success && dialogContext.mounted) {
                    Navigator.of(dialogContext).pop(controller.text);
                  } else if (dialogContext.mounted) {
                    setDialogState(() {
                      isVerifyingDialog = false;
                      error = 'Invalid password';
                    });
                  }
                },
              ),
              if (error != null) ...[
                const SizedBox(height: 8),
                Text(
                  error!,
                  style: TextStyle(
                    color: Colors.red.shade700,
                    fontSize: 12,
                  ),
                ),
              ],
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialogContext, null),
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: isPasswordEmpty
                  ? null
                  : () async {
                      if (controller.text.isEmpty) return;
                      setDialogState(() => isVerifyingDialog = true);
                      final success = await onVerify(controller.text);
                      if (success && dialogContext.mounted) {
                        Navigator.of(dialogContext).pop(controller.text);
                      } else if (dialogContext.mounted) {
                        setDialogState(() {
                          isVerifyingDialog = false;
                          error = 'Invalid password';
                        });
                      }
                    },
              child: isVerifyingDialog
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Verify'),
            ),
          ],
        );
      },
    ),
  );
}
