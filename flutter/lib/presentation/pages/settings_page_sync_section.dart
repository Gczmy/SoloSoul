part of 'settings_page.dart';

/// Sync settings section showing cloud sync and offline mode options.
/// Extracted from SettingsPage to reduce file length.
class _SyncSettingsSection extends ConsumerWidget {
  const _SyncSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return SectionCard(
      title: l10n.settingsSync,
      icon: Icons.sync_outlined,
      children: [
        SettingsTile(
          icon: Icons.cloud_outlined,
          title: l10n.settingsCloudSync,
          subtitle: l10n.settingsNotConfigured,
          trailing: Switch(
            value: false,
            onChanged: (value) => _showComingSoon(context, 'Cloud sync setup'),
          ),
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.wifi_off_outlined,
          title: l10n.settingsOfflineMode,
          subtitle: l10n.settingsOfflineModeDesc,
          trailing: const Icon(
            Icons.check_circle,
            color: AppTheme.successColor,
            size: 20,
          ),
        ),
      ],
    ).animate().fadeIn(delay: 200.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
