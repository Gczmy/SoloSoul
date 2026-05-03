import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

/// Dialog for changing the master password
/// Returns true if password was changed successfully
Future<bool> showChangePasswordDialog({
  required BuildContext context,
  required WidgetRef ref,
}) async {
  final currentPasswordController = TextEditingController();
  final newPasswordController = TextEditingController();
  final confirmPasswordController = TextEditingController();
  final newPasswordHintController = TextEditingController();

  bool obscureCurrent = true;
  bool obscureNew = true;
  bool obscureConfirm = true;
  bool isLoading = false;
  String? error;

  final result = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => StatefulBuilder(
      builder: (dialogContext, setDialogState) {
        return AlertDialog(
          title: Row(
            children: [
              Icon(Icons.lock_outline, color: Colors.indigo.shade700),
              const SizedBox(width: 8),
              const Text('Change Master Password'),
            ],
          ),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Colors.indigo.shade50,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.indigo.shade200),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.warning_amber, color: Colors.indigo.shade700, size: 20),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'Changing your password will re-encrypt all your data with the new key.',
                          style: TextStyle(fontSize: 12),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 20),

                // Current Password
                TextField(
                  controller: currentPasswordController,
                  obscureText: obscureCurrent,
                  decoration: InputDecoration(
                    labelText: 'Current Password',
                    prefixIcon: const Icon(Icons.key),
                    suffixIcon: IconButton(
                      icon: Icon(obscureCurrent ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                      onPressed: () => setDialogState(() => obscureCurrent = !obscureCurrent),
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.grey.shade400),
                    ),
                    errorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade300),
                    ),
                    focusedErrorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade500, width: 2),
                    ),
                  ),
                ),
                const SizedBox(height: 16),

                // New Password
                TextField(
                  controller: newPasswordController,
                  obscureText: obscureNew,
                  decoration: InputDecoration(
                    labelText: 'New Password',
                    prefixIcon: const Icon(Icons.vpn_key),
                    hintText: 'Minimum 8 characters',
                    suffixIcon: IconButton(
                      icon: Icon(obscureNew ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                      onPressed: () => setDialogState(() => obscureNew = !obscureNew),
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.grey.shade400),
                    ),
                    errorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade300),
                    ),
                    focusedErrorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade500, width: 2),
                    ),
                  ),
                ),
                const SizedBox(height: 16),

                // Confirm New Password
                TextField(
                  controller: confirmPasswordController,
                  obscureText: obscureConfirm,
                  decoration: InputDecoration(
                    labelText: 'Confirm New Password',
                    prefixIcon: const Icon(Icons.vpn_key),
                    suffixIcon: IconButton(
                      icon: Icon(obscureConfirm ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                      onPressed: () => setDialogState(() => obscureConfirm = !obscureConfirm),
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.grey.shade400),
                    ),
                    errorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade300),
                    ),
                    focusedErrorBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide(color: Colors.red.shade500, width: 2),
                    ),
                  ),
                ),
                const SizedBox(height: 16),

                // New Password Hint (Optional)
                TextField(
                  controller: newPasswordHintController,
                  decoration: const InputDecoration(
                    labelText: 'New Password Hint (Optional)',
                    prefixIcon: Icon(Icons.help_outline),
                    hintText: 'A hint to help you remember',
                  ),
                ),

                if (error != null) ...[
                  const SizedBox(height: 16),
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.red.shade50,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.red.shade200),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.error_outline, color: Colors.red.shade700, size: 20),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            error!,
                            style: TextStyle(color: Colors.red.shade700, fontSize: 13),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: isLoading ? null : () => Navigator.pop(dialogContext, false),
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: isLoading
                  ? null
                  : () async {
                      final current = currentPasswordController.text;
                      final newPwd = newPasswordController.text;
                      final confirm = confirmPasswordController.text;

                      // Validation
                      if (current.isEmpty) {
                        setDialogState(() => error = 'Current password is required');
                        return;
                      }
                      if (newPwd.isEmpty) {
                        setDialogState(() => error = 'New password is required');
                        return;
                      }
                      if (newPwd.length < 8) {
                        setDialogState(() => error = 'Password must be at least 8 characters');
                        return;
                      }
                      if (newPwd != confirm) {
                        setDialogState(() => error = 'Passwords do not match');
                        return;
                      }
                      if (current == newPwd) {
                        setDialogState(() => error = 'New password must be different');
                        return;
                      }

                      setDialogState(() {
                        isLoading = true;
                        error = null;
                      });

                      final authNotifier = ref.read(authNotifierProvider.notifier);
                      final result = await authNotifier.changePassword(
                        currentPassword: current,
                        newPassword: newPwd,
                        newPasswordHint: newPasswordHintController.text.isNotEmpty
                            ? newPasswordHintController.text
                            : null,
                      );

                      if (result.success && dialogContext.mounted) {
                        Navigator.pop(dialogContext, true);
                      } else if (dialogContext.mounted) {
                        setDialogState(() {
                          isLoading = false;
                          error = result.error ?? 'Failed to change password';
                        });
                      }
                    },
              child: isLoading
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Change'),
            ),
          ],
        );
      },
    ),
  );

  currentPasswordController.text = '';
  currentPasswordController.dispose();
  newPasswordController.text = '';
  newPasswordController.dispose();
  confirmPasswordController.text = '';
  confirmPasswordController.dispose();
  newPasswordHintController.text = '';
  newPasswordHintController.dispose();

  return result ?? false;
}
