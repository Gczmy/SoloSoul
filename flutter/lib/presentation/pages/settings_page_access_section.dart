part of 'settings_page.dart';

/// Access settings section showing lock vault, change password, and biometric options.
/// Extracted from SettingsPage to reduce file length.
class _AccessSettingsSection extends ConsumerWidget {
  const _AccessSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return SectionCard(
      title: l10n.settingsAccess,
      icon: Icons.lock_outlined,
      children: [
        SettingsTile(
          icon: Icons.lock_open_outlined,
          title: l10n.settingsLockVault,
          subtitle: l10n.settingsLockVaultDesc,
          onTap: () async {
            final confirmed = await showLockVaultDialog(context);
            if (confirmed == true && context.mounted) {
              await ref.read(authNotifierProvider.notifier).lockVault();
              ref.read(sensitivePageAccessProvider.notifier).clear();
            }
          },
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.password_outlined,
          title: l10n.settingsChangePassword,
          subtitle: l10n.settingsChangePasswordDesc,
          onTap: () async {
            final result = await showChangePasswordDialog(
              context: context,
              ref: ref,
            );
            if (result != ChangePasswordDialogResult.cancelled && context.mounted) {
              final message = result == ChangePasswordDialogResult.hintOnlyChanged
                  ? AppLocalizations.of(context).settingsPasswordHintChangedSuccess
                  : AppLocalizations.of(context).settingsPasswordChangedSuccess;
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: Row(
                    children: [
                      const Icon(
                        Icons.check_circle,
                        color: Colors.white,
                        size: 20,
                      ),
                      const SizedBox(width: 12),
                      Text(message),
                    ],
                  ),
                  backgroundColor: AppTheme.successColor,
                  behavior: SnackBarBehavior.floating,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  margin: const EdgeInsets.all(16),
                ),
              );
            }
          },
        ),
        const Divider(height: 1),
        const BiometricSettingsWidget(),
      ],
    ).animate().fadeIn(delay: 100.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
