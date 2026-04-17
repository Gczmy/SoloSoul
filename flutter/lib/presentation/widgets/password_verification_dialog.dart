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
  bool showHintBanner = false;

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
                          setDialogState(() => showHintBanner = !showHintBanner);
                        },
                        borderRadius: BorderRadius.circular(4),
                        child: Padding(
                          padding: const EdgeInsets.all(4),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                showHintBanner ? Icons.visibility_off : Icons.help_outline,
                                color: Colors.orange.shade700,
                                size: 18,
                              ),
                              const SizedBox(width: 4),
                              Text(
                                showHintBanner ? 'Hide Hint' : 'Hint',
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
              if (showHintBanner && passwordHint != null) ...[
                const SizedBox(height: 8),
                _HintBanner(hint: passwordHint),
              ],
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

/// Overlay hint banner that shows password hint, auto-hides after 4 seconds
class _HintBanner extends StatefulWidget {
  final String hint;

  const _HintBanner({required this.hint});

  @override
  State<_HintBanner> createState() => _HintBannerState();
}

class _HintBannerState extends State<_HintBanner> {
  @override
  void initState() {
    super.initState();
    // Auto-hide after 4 seconds
    Timer(const Duration(seconds: 4), () {
      if (mounted) {
        Navigator.of(context).maybePop();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.amber.shade100,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Row(
          children: [
            Icon(Icons.lightbulb_outline, color: Colors.amber.shade800, size: 18),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                widget.hint,
                style: TextStyle(
                  color: Colors.amber.shade900,
                  fontSize: 13,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
