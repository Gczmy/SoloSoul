import 'dart:async';
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
  String? passwordHint,
  required Future<bool> Function(String password) onVerify,
}) async {
  final controller = TextEditingController();
  String? error;
  bool isVerifyingDialog = false;
  bool isPasswordEmpty = true;

  void showHintOverlay(BuildContext ctx, String hint) {
    final overlay = Overlay.of(ctx);
    late OverlayEntry entry;

    entry = OverlayEntry(
      builder: (overlayCtx) => Positioned(
        top: MediaQuery.of(ctx).padding.top + kToolbarHeight + 8,
        left: 16,
        right: 16,
        child: SafeArea(
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
              decoration: BoxDecoration(
                color: Colors.orange.shade700,
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.15),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Row(
                children: [
                  const Icon(Icons.lightbulb_outline, color: Colors.white, size: 22),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      'Password Hint: $hint',
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white70, size: 18),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    onPressed: () => entry.remove(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(entry);
    Timer(const Duration(seconds: 4), () {
      if (entry.mounted) {
        entry.remove();
      }
    });
  }

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
                    if (passwordHint != null) ...[
                      const SizedBox(width: 4),
                      InkWell(
                        onTap: () {
                          // Use overlay to show hint without changing dialog size
                          showHintOverlay(context, passwordHint);
                        },
                        borderRadius: BorderRadius.circular(4),
                        child: Padding(
                          padding: const EdgeInsets.all(4),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                Icons.help_outline,
                                color: Colors.orange.shade700,
                                size: 18,
                              ),
                              const SizedBox(width: 4),
                              Text(
                                'Hint',
                                style: TextStyle(
                                  fontSize: 12,
                                  color: Colors.orange.shade700,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
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
