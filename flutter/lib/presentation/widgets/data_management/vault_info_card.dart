import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 展示 Vault 数据大小和附件存储大小的信息卡片。
class VaultInfoCard extends StatelessWidget {
  final String vaultDataSize;
  final String attachmentSize;
  final int attachmentCount;
  final String totalSize;

  const VaultInfoCard({
    super.key,
    required this.vaultDataSize,
    this.attachmentSize = '0 B',
    this.attachmentCount = 0,
    this.totalSize = '0 B',
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _SizeRow(
            label: l10n.dataMgmtVaultSize,
            value: vaultDataSize,
            theme: theme,
          ),
          const SizedBox(height: 4),
          _SizeRow(
            label: l10n.dataMgmtAttachmentSize,
            value: '$attachmentSize（$attachmentCount ${l10n.dataMgmtAttachmentCountUnit}）',
            theme: theme,
          ),
          const SizedBox(height: 4),
          const Divider(height: 1),
          const SizedBox(height: 4),
          _SizeRow(
            label: l10n.dataMgmtTotalSize,
            value: totalSize,
            theme: theme,
            isBold: true,
          ),
        ],
      ),
    );
  }
}

class _SizeRow extends StatelessWidget {
  final String label;
  final String value;
  final ThemeData theme;
  final bool isBold;

  const _SizeRow({
    required this.label,
    required this.value,
    required this.theme,
    this.isBold = false,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Text(
          label,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const Spacer(),
        Text(
          value,
          style: theme.textTheme.bodyMedium?.copyWith(
            fontWeight: isBold ? FontWeight.w700 : FontWeight.w600,
            color: isBold ? theme.colorScheme.primary : null,
          ),
        ),
      ],
    );
  }
}
