import 'package:flutter/material.dart';

/// Shows a confirmation dialog before locking the vault.
/// Returns `true` if user confirms, `null` or `false` if cancelled.
Future<bool?> showLockVaultDialog(BuildContext context) {
  return showDialog<bool>(
    context: context,
    barrierDismissible: true,
    builder: (ctx) => AlertDialog(
      title: const Row(
        children: [
          Icon(Icons.lock_outline),
          SizedBox(width: 12),
          Text('Lock Vault?'),
        ],
      ),
      content: const Text(
        'Locking the vault will require your master password to unlock again.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, true),
          child: const Text('Lock'),
        ),
      ],
    ),
  );
}
