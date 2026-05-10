import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/utils/device_utils.dart';
import 'settings_common.dart';

/// Current account info bottom sheet.
class CurrentAccountSheet extends StatelessWidget {
  final AccountInfo account;

  const CurrentAccountSheet({super.key, required this.account});

  String _formatDateTime(DateTime? dt, {required String naFallback}) {
    if (dt == null) return naFallback;
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  /// Translates stored English operation descriptions to the current locale.
  String _translateOpDesc(String desc, AppLocalizations l10n) {
    return switch (desc) {
      'Created account' => l10n.operationCreatedAccount,
      'Deleted account' => l10n.operationDeletedAccount,
      'Changed password' => l10n.operationChangedPassword,
      'Created backup' => l10n.dataMgmtOperationCreatedBackup,
      'Restored backup' => l10n.dataMgmtOperationRestoredBackup,
      'Deleted backup' => l10n.dataMgmtOperationDeletedBackup,
      'Promoted backup to special' => l10n.dataMgmtOperationPromotedBackup,
      'Created special backup' => l10n.dataMgmtOperationCreatedSpecial,
      'Renamed special backup' => l10n.dataMgmtOperationRenamedSpecial,
      'Restored special backup' => l10n.dataMgmtOperationRestoredSpecial,
      'Deleted special backup' => l10n.dataMgmtOperationDeletedSpecial,
      _ => desc,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 20),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Column(
              children: [
                // Account name header
                Row(
                  children: [
                    Container(
                      width: 56,
                      height: 56,
                      decoration: BoxDecoration(
                        color: AppTheme.primaryColor.withValues(alpha: 0.1),
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: const Icon(
                        Icons.account_circle,
                        size: 32,
                        color: AppTheme.primaryColor,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            account.name,
                            style: theme.textTheme.titleLarge?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          Text(
                            l10n.accountIdLabel(account.id),
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 24),

                // Account info items with icons
                InfoTile(
                  icon: Icons.calendar_today_outlined,
                  title: AppLocalizations.of(context).accountCreated,
                  value: _formatDateTime(account.createdAt, naFallback: l10n.commonNA),
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.login_outlined,
                  title: AppLocalizations.of(context).accountLastLogin,
                  value: _formatDateTime(account.lastLoginAt, naFallback: l10n.commonNA),
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.update_outlined,
                  title: AppLocalizations.of(context).accountLastOperation,
                  value: account.lastOperationDesc != null
                      ? _translateOpDesc(account.lastOperationDesc!, l10n)
                      : l10n.accountNoRecentOps,
                  subtitle: account.lastOperationAt != null
                      ? _formatDateTime(account.lastOperationAt, naFallback: l10n.commonNA)
                      : null,
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.devices_outlined,
                  title: AppLocalizations.of(context).accountLoginDevices,
                  value: account.recentDevices.isEmpty
                      ? l10n.accountNoDevices
                      : l10n.accountDeviceCount(account.recentDevices.length),
                ),
              ],
            ),
          ),

          // Login devices list
          if (account.recentDevices.isNotEmpty) ...[
            const SizedBox(height: 16),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.accountRecentDevices,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 8),
                  ...account.recentDevices.map(
                    (device) => Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Row(
                        children: [
                          Icon(
                            getDeviceIcon(device.deviceName),
                            size: 18,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text.rich(
                              TextSpan(
                                children: [
                                  TextSpan(
                                    text: '${getDevicePlatformLabel(device.deviceName)} ',
                                    style: theme.textTheme.bodyMedium?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant,
                                    ),
                                  ),
                                  TextSpan(
                                    text: device.deviceName,
                                    style: theme.textTheme.bodyMedium,
                                  ),
                                ],
                              ),
                            ),
                          ),
                          Text(
                            _formatDateTime(device.lastUsed, naFallback: l10n.commonNA),
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],

          const SizedBox(height: 32),
        ],
      ),
    );
  }
}
