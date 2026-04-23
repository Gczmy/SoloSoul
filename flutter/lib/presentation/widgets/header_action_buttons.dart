import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme, SnackBarType, showOverlaySnackBar;

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
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Search
        ExcludeSemantics(
          child: IconButton(
            icon: const Icon(Icons.search),
            onPressed: () {
              context.push(AppRoutes.search);
            },
            tooltip: 'Search',
          ),
        ),
        // Lock Sensitivity Access - only shown when access is granted
        if (ref.watch(isSensitiveAccessGrantedProvider))
          ExcludeSemantics(
            child: IconButton(
              icon: const Icon(
                Icons.lock_open_outlined,
                color: AppTheme.successColor,
              ),
              onPressed: () {
                // Manual lock - clear sensitive page access
                ref.read(sensitivePageAccessProvider.notifier).clear();
                showOverlaySnackBar(
                  context,
                  content: 'Sensitive access locked',
                  type: SnackBarType.info,
                );
              },
              tooltip: 'Lock Sensitivity Access',
            ),
          ),
        // Settings - do nothing if already on settings page
        ExcludeSemantics(
          child: IconButton(
            icon: const Icon(Icons.settings_outlined),
            onPressed: () {
              final currentRoute = GoRouterState.of(context).matchedLocation;
              if (currentRoute == AppRoutes.settings) {
                return; // Already on settings page, do nothing
              }
              context.push(AppRoutes.settings);
            },
            tooltip: 'Settings',
          ),
        ),
        // Lock Vault
        ExcludeSemantics(
          child: IconButton(
            icon: const Icon(Icons.lock_outline),
            onPressed: () {
              // Lock vault first (synchronously sets AuthState.locked)
              ref.read(authNotifierProvider.notifier).lockVault();
              // Clear sensitive access after navigation completes
              ref.read(sensitivePageAccessProvider.notifier).clear();
              // GoRouter redirect will handle navigation to login based on auth state
            },
            tooltip: 'Lock Vault',
          ),
        ),
      ],
    );
  }
}
