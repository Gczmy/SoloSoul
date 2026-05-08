import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Shows a confirmation dialog before locking the vault.
/// Returns `true` if user confirms, `null` or `false` if cancelled.
Future<bool?> showLockVaultDialog(BuildContext context) {
  return showDialog<bool>(
    context: context,
    barrierDismissible: true,
    builder: (ctx) {
      final l10n = AppLocalizations.of(ctx);
      return AlertDialog(
        title: Row(
          children: [
            const Icon(Icons.lock_outline),
            const SizedBox(width: 12),
            Text(l10n.sidebarLockVault),
          ],
        ),
        content: const Text(
          'Locking the vault will require your master password to unlock again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.dialogLock),
          ),
        ],
      );
    },
  );
}
