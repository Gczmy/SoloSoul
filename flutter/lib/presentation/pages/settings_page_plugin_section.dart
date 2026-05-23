part of 'settings_page.dart';

/// Plugin management section in SettingsPage.
/// Navigates to PluginDashboardPage on tap.
class _PluginSettingsSection extends ConsumerWidget {
  const _PluginSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);

    return SectionCard(
      title: l10n.pluginManagement,
      icon: Icons.extension_outlined,
      children: [
        SettingsTile(
          icon: Icons.extension_outlined,
          title: l10n.pluginManagement,
          subtitle: Platform.isIOS
              ? l10n.pluginManagementSubtitleIOS
              : l10n.pluginManagementSubtitle,
          onTap: Platform.isIOS ? null : () => context.push(AppRoutes.pluginDashboard),
        ),
      ],
    ).animate().fadeIn(delay: 150.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
