import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Shared header action buttons for authenticated pages.
///
/// Contains:
/// - Lock Sensitivity Access (only shown when verified)
/// - Settings
/// - Lock Vault
class HeaderActionButtons extends ConsumerWidget {
  const HeaderActionButtons({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final sensitiveAccess = ref.watch(sensitivePageAccessProvider);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Search
        IconButton(
          icon: const Icon(Icons.search),
          onPressed: () {
            Navigator.pushNamed(context, '/search');
          },
          tooltip: 'Search',
        ),
        // Lock Sensitivity Access - only shown when verified
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
            tooltip: 'Lock Sensitivity Access',
          ),
        // Settings - do nothing if already on settings page
        IconButton(
          icon: const Icon(Icons.settings_outlined),
          onPressed: () {
            final currentRoute = ModalRoute.of(context)?.settings.name;
            if (currentRoute == '/settings') {
              return; // Already on settings page, do nothing
            }
            Navigator.pushNamed(context, '/settings');
          },
          tooltip: 'Settings',
        ),
        // Lock Vault
        IconButton(
          icon: const Icon(Icons.lock_outline),
          onPressed: () {
            ref.read(authNotifierProvider.notifier).lockVault();
            // Also clear sensitive access
            ref.read(sensitivePageAccessProvider.notifier).clear();
            // Clear entire route stack to prevent back-navigation to destroyed pages
            Navigator.of(
              context,
            ).pushNamedAndRemoveUntil('/login', (route) => false);
          },
          tooltip: 'Lock Vault',
        ),
      ],
    );
  }
}
