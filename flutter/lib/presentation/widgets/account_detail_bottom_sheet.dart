import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

/// Shows a modal bottom sheet with detailed account information.
///
/// Call [showAccountDetailBottomSheet] to display this sheet.
Future<void> showAccountDetailBottomSheet(
  BuildContext context,
  String accountId,
) async {
  // Reload fresh account data from storage so last operation times etc. are up to date
  final accounts = await SecureAccountStorage.instance.listAccounts();
  final account = accounts.cast<AccountInfo?>().firstWhere(
    (a) => a?.id == accountId,
    orElse: () => null,
  );
  if (account == null) return;

  if (!context.mounted) return;

  return showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
    ),
    builder: (context) => AccountDetailBottomSheet(account: account),
  );
}

/// Modal bottom sheet widget displaying detailed account information.
class AccountDetailBottomSheet extends StatelessWidget {
  const AccountDetailBottomSheet({super.key, required this.account});

  final AccountInfo account;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return DraggableScrollableSheet(
      initialChildSize: 0.62,
      minChildSize: 0.3,
      maxChildSize: 0.9,
      expand: false,
      builder: (context, scrollController) => SingleChildScrollView(
        controller: scrollController,
        child: Padding(
          padding: EdgeInsets.only(
            left: 24,
            right: 24,
            top: 24,
            bottom: MediaQuery.of(context).viewInsets.bottom + 24,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header with account name and person icon
              Row(
                children: [
                  Container(
                    width: 48,
                    height: 48,
                    decoration: BoxDecoration(
                      color: AppTheme.primaryColor.withValues(alpha: 0.1),
                      shape: BoxShape.circle,
                    ),
                    child: const Icon(
                      Icons.person,
                      color: AppTheme.primaryColor,
                      size: 28,
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Text(
                      account.name,
                      style: theme.textTheme.headlineSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(context),
                  ),
                ],
              ),

              const SizedBox(height: 24),

              // Info section
              const _SectionTitle(title: 'Account Info'),
              const SizedBox(height: 12),
              _InfoRow(
                label: 'Created',
                value: _formatDate(account.createdAt),
              ),
              _InfoRow(
                label: 'Last Login',
                value: _formatDateTime(account.lastLoginAt),
              ),
              _InfoRow(
                label: 'Last Operation',
                value: _formatDateTime(account.lastOperationAt),
              ),
              _InfoRow(
                label: 'Operation Details',
                value: account.lastOperationDesc ?? 'No recent operations',
              ),

              const SizedBox(height: 24),

              // Recent devices section
              const _SectionTitle(title: 'Recent Devices'),
              const SizedBox(height: 12),
              if (account.recentDevices.isEmpty)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  child: Center(
                    child: Text(
                      'No device data',
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                )
              else
                ...account.recentDevices.map(
                  (device) => _DeviceRow(device: device),
                ),

              const SizedBox(height: 8),
            ],
          ),
        ),
      ),
    );
  }

  String _formatDate(DateTime? date) {
    if (date == null) return 'Unknown';
    return '${date.day}/${date.month}/${date.year}';
  }

  String _formatDateTime(DateTime? date) {
    if (date == null) return 'Unknown';
    return '${date.day}/${date.month}/${date.year} ${_pad(date.hour)}:${_pad(date.minute)}';
  }

  String _pad(int value) => value.toString().padLeft(2, '0');
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: Theme.of(context).textTheme.titleMedium?.copyWith(
        color: AppTheme.primaryColor,
        fontWeight: FontWeight.w600,
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              value,
              style: theme.textTheme.bodyMedium,
            ),
          ),
        ],
      ),
    );
  }
}

class _DeviceRow extends StatelessWidget {
  const _DeviceRow({required this.device});

  final DeviceInfo device;

  IconData get _deviceIcon {
    final name = device.deviceName.toLowerCase();
    if (name.contains('mac') || name.contains('darwin')) return Icons.laptop_mac;
    if (name.contains('iphone') || name.contains('ios')) return Icons.phone_iphone;
    if (name.contains('android')) return Icons.phone_android;
    if (name.contains('linux')) return Icons.computer;
    if (name.contains('windows')) return Icons.desktop_windows;
    if (name.contains('flutter')) return Icons.developer_mode;
    return Icons.devices;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        children: [
          Icon(
            _deviceIcon,
            size: 20,
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
            _formatDate(device.lastUsed),
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  String _formatDate(DateTime date) {
    return '${date.day}/${date.month}/${date.year} ${_pad(date.hour)}:${_pad(date.minute)}';
  }

  String _pad(int value) => value.toString().padLeft(2, '0');
}
