part of 'settings_page.dart';

/// App info section showing language, version, debug log, privacy policy, and terms of service.
/// Extracted from SettingsPage to reduce file length.
class _AppInfoSection extends ConsumerWidget {
  const _AppInfoSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return SectionCard(
      title: l10n.settingsAbout,
      icon: Icons.info_outlined,
      children: [
        Consumer(
          builder: (context, ref, _) {
            final locale = ref.watch(languageProvider).value;
            final languageLabel = locale?.languageCode == 'zh'
                ? l10n.settingsLanguageChinese
                : l10n.settingsLanguageEnglish;
            return SettingsTile(
              icon: Icons.language_outlined,
              title: l10n.settingsLanguage,
              subtitle: languageLabel,
              onTap: () => _showLanguagePicker(context, ref),
            );
          },
        ),
        const Divider(height: 1),
        Consumer(
          builder: (context, ref, _) {
            final packageInfo = ref.watch(packageInfoProvider);
            return SettingsTile(
              icon: Icons.code,
              title: l10n.settingsVersion,
              subtitle: packageInfo.when(
                data: (info) => kDebugMode ? '${info.version} (dev)' : info.version,
                loading: () => '...',
                error: (_, __) => '1.0.0',
              ),
              onTap: () => _showVersionSheet(context, ref),
            );
          },
        ),
        Consumer(
          builder: (context, ref, _) {
            final isDebugMode = ref.watch(debugModeProvider);
            if (!isDebugMode) return const SizedBox.shrink();
            return Column(
              children: [
                const Divider(height: 1),
                SettingsTile(
                  icon: Icons.bug_report_outlined,
                  title: l10n.settingsDebugLog,
                  subtitle: l10n.settingsDebugLogDesc,
                  onTap: () => _showDebugLogSheet(context, ref),
                ),
              ],
            );
          },
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.description_outlined,
          title: l10n.settingsPrivacyPolicy,
          subtitle: l10n.settingsPrivacyPolicyDesc,
          onTap: () {
            final locale = Localizations.localeOf(context);
            final isZh = locale.languageCode == 'zh';
            showLegalDocumentSheet(
              context: context,
              title: l10n.settingsPrivacyPolicy,
              assetPath: isZh
                  ? 'assets/docs/PRIVACY_POLICY_zh.md'
                  : 'assets/docs/PRIVACY_POLICY.md',
            );
          },
        ),
        const Divider(height: 1),
        SettingsTile(
          icon: Icons.article_outlined,
          title: l10n.settingsTermsOfService,
          subtitle: l10n.settingsTermsOfServiceDesc,
          onTap: () {
            final locale = Localizations.localeOf(context);
            final isZh = locale.languageCode == 'zh';
            showLegalDocumentSheet(
              context: context,
              title: l10n.settingsTermsOfService,
              assetPath: isZh
                  ? 'assets/docs/TERMS_OF_SERVICE_zh.md'
                  : 'assets/docs/TERMS_OF_SERVICE.md',
            );
          },
        ),
      ],
    ).animate().fadeIn(delay: 300.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}

void _showLanguagePicker(BuildContext context, WidgetRef ref) {
  final l10n = AppLocalizations.of(context);
  final currentCode = ref.read(languageProvider).value?.languageCode ?? 'en';

  showModalBottomSheet(
    context: context,
    backgroundColor: Colors.transparent,
    builder: (ctx) => Container(
      decoration: BoxDecoration(
        color: Theme.of(ctx).colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              margin: const EdgeInsets.only(top: 12, bottom: 8),
              width: 40,
              height: 4,
              decoration: BoxDecoration(
                color: Theme.of(ctx).colorScheme.outlineVariant,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
              child: Text(
                l10n.settingsLanguage,
                style: Theme.of(ctx).textTheme.titleMedium,
              ),
            ),
            const Divider(height: 1),
            ListTile(
              leading: const Icon(Icons.language, size: 24),
              title: Text(l10n.settingsLanguageEnglish),
              trailing: currentCode == 'en'
                  ? Icon(Icons.check, color: Theme.of(ctx).colorScheme.primary)
                  : null,
              onTap: () {
                ref.read(languageProvider.notifier).setLanguage('en');
                Navigator.pop(ctx);
              },
            ),
            const Divider(height: 1, indent: 56),
            ListTile(
              leading: const Icon(Icons.language, size: 24),
              title: Text(l10n.settingsLanguageChinese),
              trailing: currentCode == 'zh'
                  ? Icon(Icons.check, color: Theme.of(ctx).colorScheme.primary)
                  : null,
              onTap: () {
                ref.read(languageProvider.notifier).setLanguage('zh');
                Navigator.pop(ctx);
              },
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    ),
  );
}

void _showVersionSheet(BuildContext context, WidgetRef ref) {
  final packageInfo = ref.read(packageInfoProvider);
  showModalBottomSheet(
    context: context,
    backgroundColor: Colors.transparent,
    builder: (context) => VersionSheet(
      packageInfo: packageInfo,
      onDebugActivationRequested: () => _SettingsPageState._showDebugActivationDialog(context, ref),
    ),
  );
}

void _showDebugLogSheet(BuildContext context, WidgetRef ref) {
  showModalBottomSheet(
    context: context,
    backgroundColor: Colors.transparent,
    isScrollControlled: true,
    builder: (context) => DraggableScrollableSheet(
      initialChildSize: 0.7,
      minChildSize: 0.3,
      maxChildSize: 0.95,
      builder: (context, scrollController) => DebugLogSheet(
        scrollController: scrollController,
        onDisableDebugMode: () async {
          await ref.read(debugModeProvider.notifier).disableDebugMode();
        },
      ),
    ),
  );
}

void _showComingSoon(BuildContext context, String feature) {
  showDialog(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(feature),
      content: const Text(
        'This feature will be available in a future update.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(AppLocalizations.of(context).settingsOk),
        ),
      ],
    ),
  );
}
