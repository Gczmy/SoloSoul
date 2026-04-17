import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

class HomePage extends ConsumerWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final authState = ref.watch(authNotifierProvider);
    final sensitiveAccess = ref.watch(sensitivePageAccessProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('SoloSoul'),
        actions: [
          // Sensitive access indicator
          if (sensitiveAccess.isVerified)
            IconButton(
              icon: const Icon(
                Icons.lock_open_outlined,
                color: AppTheme.successColor,
              ),
              onPressed: () {
                // Manual lock - clear sensitive page access
                ref.read(sensitivePageAccessProvider.notifier).clear();
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Sensitive access locked'),
                    behavior: SnackBarBehavior.floating,
                  ),
                );
              },
              tooltip: 'Lock Sensitive Access',
            ),
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            onPressed: () => Navigator.pushNamed(context, '/settings'),
            tooltip: 'Settings',
          ),
          IconButton(
            icon: const Icon(Icons.lock_outline),
            onPressed: () {
              ref.read(authNotifierProvider.notifier).lockVault();
              // Also clear sensitive access
              ref.read(sensitivePageAccessProvider.notifier).clear();
              Navigator.of(context).pushReplacementNamed('/login');
            },
            tooltip: 'Lock Vault',
          ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Status card
            Card(
              child: Padding(
                padding: const EdgeInsets.all(20),
                child: Row(
                  children: [
                    Container(
                      width: 48,
                      height: 48,
                      decoration: BoxDecoration(
                        color: AppTheme.successColor.withValues(alpha: 0.1),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: const Icon(
                        Icons.shield,
                        color: AppTheme.successColor,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Vault Unlocked',
                            style: theme.textTheme.titleMedium,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            ref
                                    .watch(authNotifierProvider.notifier)
                                    .selectedAccount
                                    ?.name ??
                                'Account',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: AppTheme.primaryColor,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ],
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        color: authState == AuthState.unlocked
                            ? AppTheme.successColor.withValues(alpha: 0.1)
                            : Colors.blue.withValues(alpha: 0.1),
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            authState == AuthState.unlocked
                                ? Icons.shield
                                : Icons.lock,
                            size: 14,
                            color: authState == AuthState.unlocked
                                ? AppTheme.successColor
                                : Colors.blue,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            authState == AuthState.unlocked
                                ? 'Online'
                                : 'Offline',
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: authState == AuthState.unlocked
                                  ? AppTheme.successColor
                                  : Colors.blue,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 32),

            // Quick actions
            Text('Quick Actions', style: theme.textTheme.titleLarge),
            const SizedBox(height: 16),

            _QuickActionCard(
              icon: Icons.person_outline,
              title: 'Profile',
              subtitle: 'Manage your identity information',
              onTap: () => Navigator.pushNamed(context, '/profile'),
            ),

            _QuickActionCard(
              icon: Icons.flight_outlined,
              title: 'Travel',
              subtitle: 'Passports, visas, travel history',
              onTap: () => Navigator.pushNamed(context, '/travel'),
            ),

            _QuickActionCard(
              icon: Icons.account_balance_outlined,
              title: 'Financial',
              subtitle: 'Bank accounts, cards, tax IDs',
              onTap: () => Navigator.pushNamed(context, '/financial'),
            ),

            _QuickActionCard(
              icon: Icons.work_outline,
              title: 'Professional',
              subtitle: 'Education, employment, skills',
              onTap: () => Navigator.pushNamed(context, '/professional'),
            ),

            _QuickActionCard(
              icon: Icons.delete_outline,
              title: 'Trash',
              subtitle: 'View and restore deleted items',
              onTap: () => Navigator.pushNamed(context, '/trash'),
            ),

            _QuickActionCard(
              icon: Icons.settings_outlined,
              title: 'Settings',
              subtitle: 'Account, security, sync',
              onTap: () => Navigator.pushNamed(context, '/settings'),
            ),

            const SizedBox(height: 32),

            // Security status
            Text('Security Status', style: theme.textTheme.titleLarge),
            const SizedBox(height: 16),

            Card(
              child: Padding(
                padding: const EdgeInsets.all(20),
                child: Column(
                  children: [
                    _SecurityItem(
                      icon: Icons.check_circle,
                      color: AppTheme.successColor,
                      title: 'End-to-End Encrypted',
                      subtitle: 'AES-256-GCM + Argon2id',
                    ),
                    const Divider(height: 24),
                    _SecurityItem(
                      icon: Icons.check_circle,
                      color: AppTheme.successColor,
                      title: 'Local Storage',
                      subtitle: 'Data encrypted and stored locally',
                    ),
                    const Divider(height: 24),
                    _SecurityItem(
                      icon: Icons.check_circle,
                      color: AppTheme.successColor,
                      title: 'Zero Knowledge',
                      subtitle: 'Master password never stored',
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _QuickActionCard extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _QuickActionCard({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(16),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(icon, color: AppTheme.primaryColor),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: theme.textTheme.titleMedium),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SecurityItem extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;

  const _SecurityItem({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      children: [
        Icon(icon, color: color, size: 24),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: theme.textTheme.titleSmall),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
