import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

class VaultInfoCard extends StatelessWidget {
  final String vaultDataSize;

  const VaultInfoCard({
    super.key,
    required this.vaultDataSize,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Row(
        children: [
          Text(
            l10n.dataMgmtVaultSize,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          Text(
            vaultDataSize,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}
