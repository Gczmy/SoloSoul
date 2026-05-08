import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/entry_configs.dart';
export 'package:solosoul_flutter/core/models/entry_configs.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider, isSensitiveAccessGrantedProvider;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

/// Generates standard action buttons based on context and permissions.
class EntryActionBuilder {
  /// Build all standard action buttons with password verification for sensitive fields.
  static List<Widget> buildActions({
    required BuildContext context,
    required WidgetRef ref,
    required VoidCallback onCopy,
    required VoidCallback onEdit,
    required VoidCallback onDelete,
    EntryActionsConfig config = const EntryActionsConfig(),
    bool isSensitive = false,
    Widget? historyAction,
    Widget? attachmentAction,
  }) {
    final l10n = AppLocalizations.of(context);
    final actions = <Widget>[];

    if (config.showCopy) {
      actions.add(
        buildButton(
          icon: Icons.copy_all,
          tooltip: l10n.entryCopyAll,
          onPressed: isSensitive
              ? () => _handleWithVerification(
                  context: context,
                  ref: ref,
                  onSuccess: onCopy,
                )
              : onCopy,
        ),
      );
    }

    if (config.showEdit) {
      actions.add(
        buildButton(
          icon: Icons.edit_outlined,
          tooltip: l10n.commonEdit,
          onPressed: isSensitive
              ? () => _handleWithVerification(
                  context: context,
                  ref: ref,
                  onSuccess: onEdit,
                )
              : onEdit,
        ),
      );
    }

    if (historyAction != null) {
      actions.add(historyAction);
      actions.add(const SizedBox(width: 8));
    }

    if (config.showDelete) {
      actions.add(
        buildButton(
          icon: Icons.delete_outline,
          tooltip: l10n.commonDelete,
          onPressed: isSensitive
              ? () => _handleWithVerification(
                  context: context,
                  ref: ref,
                  onSuccess: onDelete,
                )
              : onDelete,
        ),
      );
    }

    if (attachmentAction != null) {
      actions.add(const SizedBox(width: 8));
      actions.add(attachmentAction);
    }

    return actions;
  }

  /// Build a single action button.
  static Widget buildButton({
    required IconData icon,
    required String tooltip,
    required VoidCallback? onPressed,
  }) {
    return IconButton(
      icon: Icon(icon, size: 20),
      tooltip: tooltip,
      onPressed: onPressed,
      visualDensity: VisualDensity.compact,
    );
  }

  /// Handle action with password verification for sensitive fields.
  static Future<void> _handleWithVerification({
    required BuildContext context,
    required WidgetRef ref,
    required VoidCallback onSuccess,
  }) async {
    // Check if user was verified within the valid duration (password cache)
    if (ref.read(isSensitiveAccessGrantedProvider)) {
      onSuccess();
      return;
    }

    // Show password dialog
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return;

    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    onSuccess();
  }
}
