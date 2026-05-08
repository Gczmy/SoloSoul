import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

class QuickAction {
  final IconData icon;
  final String label;
  final String route;
  final Color color;
  final bool isCustom;
  const QuickAction({
    required this.icon,
    required this.label,
    required this.route,
    required this.color,
    this.isCustom = false,
  });

  /// Returns the localized display label for this action.
  /// For default pages, uses the route→l10n mapping.
  /// For custom pages, uses the stored [label] directly.
  String localizedLabel(AppLocalizations l10n) {
    if (isCustom) return label;
    return _routeToLocalizedLabel(route, label, l10n);
  }

  /// Maps default page routes to localized labels.
  static String _routeToLocalizedLabel(String route, String fallback, AppLocalizations l10n) {
    return switch (route) {
      '/profile' => l10n.sidebarProfile,
      '/travel' => l10n.sidebarTravel,
      '/financial' => l10n.sidebarFinancial,
      '/professional' => l10n.sidebarProfessional,
      '/trash' => l10n.sidebarTrash,
      '/settings' => l10n.sidebarSettings,
      '/security' => l10n.sidebarSecurity,
      '/operation_log' => l10n.sidebarOperationLog,
      '/sensitivity' => l10n.sidebarSensitivity,
      '/search' => l10n.sidebarSearch,
      _ => fallback,
    };
  }
}
