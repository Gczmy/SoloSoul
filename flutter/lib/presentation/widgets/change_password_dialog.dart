import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';

String? _localizeError(String? error, AppLocalizations l10n) {
  if (error == null) return null;
  if (error == 'Invalid current password') {
    return l10n.errorInvalidCurrentPassword;
  }
  return error;
}

enum ChangePasswordDialogResult {
  cancelled,
  passwordChanged,
  hintOnlyChanged,
}

/// Dialog for changing the master password or password hint.
Future<ChangePasswordDialogResult> showChangePasswordDialog({
  required BuildContext context,
  required WidgetRef ref,
}) async {
  // Pre-fetch current password hint from Rust vault (source of truth)
  // to avoid Keychain/Rust sync issues.
  String? currentPasswordHint;
  try {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId != null) {
      final accounts = await RustVaultService.instance.listAccountsFromRust();
      final account = accounts.cast<Map<String, dynamic>?>().firstWhere(
        (a) => a?['id'] == accountId,
        orElse: () => null,
      );
      currentPasswordHint = account?['password_hint'] as String?;
    }
  } on Object {
    // Non-fatal — dialog will just show "no hint"
  }

  if (!context.mounted) return ChangePasswordDialogResult.cancelled;

  final result = await showDialog<ChangePasswordDialogResult>(
    context: context,
    barrierDismissible: false,
    builder: (dialogContext) => _ChangePasswordDialogContent(
      ref: ref,
      currentPasswordHint: currentPasswordHint,
    ),
  );

  return result ?? ChangePasswordDialogResult.cancelled;
}

class _ChangePasswordDialogContent extends ConsumerStatefulWidget {
  final WidgetRef ref;
  final String? currentPasswordHint;

  const _ChangePasswordDialogContent({
    required this.ref,
    required this.currentPasswordHint,
  });

  @override
  ConsumerState<_ChangePasswordDialogContent> createState() =>
      _ChangePasswordDialogContentState();
}

class _ChangePasswordDialogContentState
    extends ConsumerState<_ChangePasswordDialogContent> {
  late final TextEditingController _currentPasswordController;
  late final TextEditingController _newPasswordController;
  late final TextEditingController _confirmPasswordController;
  late final TextEditingController _newPasswordHintController;

  bool _obscureCurrent = true;
  bool _obscureNew = true;
  bool _obscureConfirm = true;
  bool _isLoading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _currentPasswordController = TextEditingController();
    _newPasswordController = TextEditingController();
    _confirmPasswordController = TextEditingController();
    _newPasswordHintController = TextEditingController();
  }

  @override
  void dispose() {
    _currentPasswordController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    _newPasswordHintController.dispose();
    super.dispose();
  }

  void _setError(String? error) {
    setState(() => _error = error);
  }

  Future<void> _handleChange() async {
    final l10n = AppLocalizations.of(context);
    final current = _currentPasswordController.text;
    final newPwd = _newPasswordController.text;
    final confirm = _confirmPasswordController.text;
    final hint = _newPasswordHintController.text;

    // Validation: current password is always required
    if (current.isEmpty) {
      _setError(l10n.changePasswordCurrentRequired);
      return;
    }

    // Case 1: Only updating password hint
    final isHintOnly = newPwd.isEmpty && confirm.isEmpty && hint.isNotEmpty;
    if (isHintOnly) {
      setState(() {
        _isLoading = true;
        _error = null;
      });

      final authNotifier = ref.read(authNotifierProvider.notifier);
      final result = await authNotifier.updatePasswordHintOnly(
        currentPassword: current,
        newPasswordHint: hint,
      );

      if (result.success && mounted) {
        Navigator.pop(context, ChangePasswordDialogResult.hintOnlyChanged);
      } else if (mounted) {
        setState(() {
          _isLoading = false;
          _error = _localizeError(result.error, l10n) ?? l10n.changePasswordFailed;
        });
      }
      return;
    }

    // Case 2: Changing password (with optional hint)
    if (newPwd.isEmpty) {
      _setError(l10n.changePasswordNewRequired);
      return;
    }
    if (newPwd.length < 8) {
      _setError(l10n.loginPasswordMinLength);
      return;
    }
    if (newPwd != confirm) {
      _setError(l10n.loginPasswordsDoNotMatch);
      return;
    }
    if (current == newPwd) {
      _setError(l10n.changePasswordMustDiffer);
      return;
    }

    setState(() {
      _isLoading = true;
      _error = null;
    });

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final result = await authNotifier.changePassword(
      currentPassword: current,
      newPassword: newPwd,
      newPasswordHint: hint.isNotEmpty ? hint : null,
    );

    if (result.success && mounted) {
      Navigator.pop(context, ChangePasswordDialogResult.passwordChanged);
    } else if (mounted) {
      setState(() {
        _isLoading = false;
        _error = _localizeError(result.error, l10n) ?? l10n.changePasswordFailed;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final currentHint = widget.currentPasswordHint;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.lock_outline, color: Colors.indigo.shade700),
          const SizedBox(width: 8),
          Text(l10n.settingsChangePassword),
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
                  Expanded(
                    child: Text(
                      l10n.changePasswordWarning,
                      style: const TextStyle(fontSize: 12),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 20),

            // Current Password
            TextField(
              controller: _currentPasswordController,
              obscureText: _obscureCurrent,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                labelText: l10n.dialogCurrentPassword,
                prefixIcon: const Icon(Icons.key),
                suffixIcon: IconButton(
                  icon: Icon(_obscureCurrent ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                  onPressed: () => setState(() => _obscureCurrent = !_obscureCurrent),
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
            const SizedBox(height: 8),

            // Current password hint display
            if (currentHint == null || currentHint.isEmpty)
              Padding(
                padding: const EdgeInsets.only(left: 4),
                child: Text(
                  '${l10n.loginPasswordHintOptional.replaceAll(' (Optional)', '')}: ${l10n.loginNoPasswordHint}',
                  style: TextStyle(
                    fontSize: 12,
                    color: Colors.grey.shade500,
                    fontStyle: FontStyle.italic,
                  ),
                ),
              )
            else
              Padding(
                padding: const EdgeInsets.only(left: 4),
                child: Row(
                  children: [
                    Icon(Icons.help_outline, size: 14, color: Colors.grey.shade600),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        '${l10n.loginPasswordHintOptional.replaceAll(' (Optional)', '')}: $currentHint',
                        style: TextStyle(
                          fontSize: 12,
                          color: Colors.grey.shade700,
                        ),
                      ),
                    ),
                  ],
                ),
              ),

            const SizedBox(height: 16),

            // New Password
            TextField(
              controller: _newPasswordController,
              obscureText: _obscureNew,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                labelText: l10n.dialogNewPassword,
                prefixIcon: const Icon(Icons.vpn_key),
                hintText: l10n.changePasswordMinLength,
                suffixIcon: IconButton(
                  icon: Icon(_obscureNew ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                  onPressed: () => setState(() => _obscureNew = !_obscureNew),
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
              controller: _confirmPasswordController,
              obscureText: _obscureConfirm,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                labelText: l10n.dialogConfirmNewPassword,
                prefixIcon: const Icon(Icons.vpn_key),
                suffixIcon: IconButton(
                  icon: Icon(_obscureConfirm ? Icons.visibility_outlined : Icons.visibility_off_outlined),
                  onPressed: () => setState(() => _obscureConfirm = !_obscureConfirm),
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
              controller: _newPasswordHintController,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                labelText: l10n.loginPasswordHintOptional,
                prefixIcon: const Icon(Icons.help_outline),
                hintText: l10n.loginPasswordHintHelp,
              ),
            ),

            if (_error != null) ...[
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
                        _error!,
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
          onPressed: _isLoading ? null : () => Navigator.pop(context, ChangePasswordDialogResult.cancelled),
          child: Text(l10n.commonCancel),
        ),
        ElevatedButton(
          onPressed: _isLoading || _currentPasswordController.text.isEmpty
              ? null
              : _handleChange,
          child: _isLoading
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l10n.dialogChange),
        ),
      ],
    );
  }
}
