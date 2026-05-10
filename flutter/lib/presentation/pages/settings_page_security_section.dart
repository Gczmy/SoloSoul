part of 'settings_page.dart';

/// Security settings section showing auto-lock, sensitivity, and operation log options.
/// Extracted from SettingsPage to reduce file length.
class _SecuritySettingsSection extends ConsumerWidget {
  const _SecuritySettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return SectionCard(
      title: l10n.settingsSecurity,
      icon: Icons.shield_outlined,
      children: [
        SettingsTile(
          icon: Icons.lock_clock_outlined,
          title: l10n.settingsAutoLockPrivacy,
          subtitle: l10n.settingsAutoLockPrivacyDesc,
          onTap: () async {
            final authNotifier = ref.read(authNotifierProvider.notifier);
            final selectedAccount = authNotifier.selectedAccount;
            final result = await showPasswordVerificationDialog(
              context: context,
              ref: ref,
              message: l10n.settingsVerifyPassword,
              passwordHint: selectedAccount?.passwordHint,
              onVerify: authNotifier.verifyPasswordForSensitiveData,
            );
            if (!context.mounted) return;
            if (result != null) {
              ref.read(sensitivePageAccessProvider.notifier).markVerified();
              if (context.mounted) {
                await context.push(AppRoutes.securitySettings);
              }
            }
          },
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.security_outlined,
          title: l10n.settingsSensitivity,
          subtitle: l10n.settingsSensitivityDesc,
          onTap: () => context.push(AppRoutes.sensitivitySettings),
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.history,
          title: l10n.settingsOperationLog,
          subtitle: l10n.settingsOperationLogDesc,
          onTap: () => context.push(AppRoutes.operationLog),
        ),
      ],
    ).animate().fadeIn(delay: 100.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
