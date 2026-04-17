import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart';

// Forward declaration for accounts provider
final accountsProvider = FutureProvider<List<AccountInfo>>((ref) async {
  final notifier = ref.read(authNotifierProvider.notifier);
  return notifier.getAccountsSortedByRecent();
});

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final accountsAsync = ref.watch(accountsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Account Section
            SectionCard(
              title: 'Account',
              icon: Icons.account_circle_outlined,
              children: [
                accountsAsync.when(
                  data: (accounts) {
                    final selectedId = authNotifier.selectedAccountId;
                    final currentAccount = accounts.cast<AccountInfo?>().firstWhere(
                      (a) => a?.id == selectedId,
                      orElse: () => null,
                    );
                    return Column(
                      children: [
                        _SettingsTile(
                          icon: Icons.person_outline,
                          title: 'Current Account',
                          subtitle: currentAccount?.name ?? 'Unknown',
                          trailing: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                            decoration: BoxDecoration(
                              color: AppTheme.successColor.withValues(alpha: 0.1),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'Active',
                              style: TextStyle(
                                color: AppTheme.successColor,
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                        ),
                        const Divider(height: 1),
                        _SettingsTile(
                          icon: Icons.history,
                          title: 'All Accounts',
                          subtitle: '${accounts.length} account(s)',
                          onTap: () => Navigator.pushReplacementNamed(context, '/login'),
                        ),
                      ],
                    );
                  },
                  loading: () => const Padding(
                    padding: EdgeInsets.all(16),
                    child: Center(child: CircularProgressIndicator()),
                  ),
                  error: (_, __) => const _SettingsTile(
                    icon: Icons.error_outline,
                    title: 'Error loading accounts',
                    subtitle: 'Please restart the app',
                  ),
                ),
              ],
            )
                .animate()
                .fadeIn(duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Security Section
            SectionCard(
              title: 'Security',
              icon: Icons.shield_outlined,
              children: [
                _SettingsTile(
                  icon: Icons.lock_outline,
                  title: 'Lock Vault',
                  subtitle: 'Lock now and require password',
                  onTap: () {
                    authNotifier.lockVault();
                    Navigator.of(context).pushNamedAndRemoveUntil('/login', (route) => false);
                  },
                ),
                const Divider(height: 1),
                _SettingsTile(
                  icon: Icons.password_outlined,
                  title: 'Change Master Password',
                  subtitle: 'Update your vault password',
                  onTap: () => _showComingSoon(context, 'Password change'),
                ),
                const Divider(height: 1),
                _SettingsTile(
                  icon: Icons.fingerprint,
                  title: 'Biometric Unlock',
                  subtitle: 'Use TouchID to unlock',
                  trailing: Switch(
                    value: false,
                    onChanged: (value) => _showComingSoon(context, 'Biometric setup'),
                  ),
                ),
              ],
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Sync Section
            SectionCard(
              title: 'Sync',
              icon: Icons.sync_outlined,
              children: [
                _SettingsTile(
                  icon: Icons.cloud_outlined,
                  title: 'Cloud Sync',
                  subtitle: 'Not configured',
                  trailing: Switch(
                    value: false,
                    onChanged: (value) => _showComingSoon(context, 'Cloud sync setup'),
                  ),
                ),
                const Divider(height: 1),
                _SettingsTile(
                  icon: Icons.wifi_off_outlined,
                  title: 'Offline Mode',
                  subtitle: 'Local data only',
                  trailing: Icon(
                    Icons.check_circle,
                    color: AppTheme.successColor,
                    size: 20,
                  ),
                ),
              ],
            )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // App Info Section
            SectionCard(
              title: 'About',
              icon: Icons.info_outlined,
              children: [
                _SettingsTile(
                  icon: Icons.code,
                  title: 'Version',
                  subtitle: '1.0.0 (dev)',
                ),
                const Divider(height: 1),
                _SettingsTile(
                  icon: Icons.description_outlined,
                  title: 'Privacy Policy',
                  subtitle: 'View our privacy policy',
                  onTap: () {},
                ),
                const Divider(height: 1),
                _SettingsTile(
                  icon: Icons.article_outlined,
                  title: 'Terms of Service',
                  subtitle: 'View terms of service',
                  onTap: () {},
                ),
              ],
            )
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 32),

            // Dev info
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.developer_mode,
                        size: 16,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        'Development Mode',
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Storage: File-based (insecure)\nKeychain: Not configured\nSQLCipher: Not implemented',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            )
                .animate()
                .fadeIn(delay: 400.ms, duration: 400.ms),
          ],
        ),
      ),
    );
  }

  void _showComingSoon(BuildContext context, String feature) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(feature),
        content: const Text('This feature will be available in a future update.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget? trailing;
  final VoidCallback? onTap;

  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Row(
          children: [
            Icon(
              icon,
              size: 20,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: theme.textTheme.bodyLarge,
                  ),
                  Text(
                    subtitle,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            if (trailing != null) trailing!,
            if (trailing == null && onTap != null)
              Icon(
                Icons.chevron_right,
                color: theme.colorScheme.onSurfaceVariant,
              ),
          ],
        ),
      ),
    );
  }
}
