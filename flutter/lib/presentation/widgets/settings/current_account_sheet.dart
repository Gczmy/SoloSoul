import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'settings_common.dart';

/// Current account info bottom sheet.
class CurrentAccountSheet extends StatelessWidget {
  final AccountInfo account;

  const CurrentAccountSheet({super.key, required this.account});

  String _formatDateTime(DateTime? dt) {
    if (dt == null) return 'N/A';
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  IconData _getDeviceIcon(String deviceName) {
    final lower = deviceName.toLowerCase();
    if (lower.contains('iphone') || lower.contains('ios')) {
      return Icons.phone_iphone;
    }
    if (lower.contains('android')) return Icons.phone_android;
    if (lower.contains('mac') || lower.contains('darwin')) {
      return Icons.laptop_mac;
    }
    if (lower.contains('windows')) return Icons.desktop_windows;
    if (lower.contains('linux')) return Icons.computer;
    if (lower.contains('web') || lower.contains('browser')) return Icons.web;
    return Icons.devices;
  }

  @override
  Widget build(BuildContext context) {
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
                            'Account ID: ${account.id}',
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
                  title: 'Created',
                  value: _formatDateTime(account.createdAt),
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.login_outlined,
                  title: 'Last Login',
                  value: _formatDateTime(account.lastLoginAt),
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.update_outlined,
                  title: 'Last Operation',
                  value: account.lastOperationDesc ?? 'No recent operations',
                  subtitle: account.lastOperationAt != null
                      ? _formatDateTime(account.lastOperationAt)
                      : null,
                ),
                const Divider(height: 1),
                InfoTile(
                  icon: Icons.devices_outlined,
                  title: 'Login Devices',
                  value: account.recentDevices.isEmpty
                      ? 'No devices recorded'
                      : '${account.recentDevices.length} device(s)',
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
                    'Recent Devices',
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
                            _getDeviceIcon(device.deviceName),
                            size: 18,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              device.deviceName,
                              style: theme.textTheme.bodyMedium,
                            ),
                          ),
                          Text(
                            _formatDateTime(device.lastUsed),
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
