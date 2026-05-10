part of 'settings_page.dart';

/// LLM settings section showing LLM config and AI chat options.
/// Extracted from SettingsPage to reduce file length.
class _LLMSettingsSection extends ConsumerWidget {
  const _LLMSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return SectionCard(
      title: l10n.settingsAiAssistant,
      icon: Icons.psychology_outlined,
      children: [
        SettingsTile(
          icon: Icons.smart_toy_outlined,
          title: l10n.settingsLlmConfig,
          subtitle: l10n.settingsLlmConfigDesc,
          onTap: () => context.push(AppRoutes.llmConfig),
        ),
        SettingsTile(
          icon: Icons.chat_bubble_outline,
          title: l10n.settingsAiChat,
          subtitle: l10n.settingsAiChatSubtitle,
          onTap: () => context.push(AppRoutes.llmChat),
        ),
      ],
    ).animate().fadeIn(delay: 250.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
