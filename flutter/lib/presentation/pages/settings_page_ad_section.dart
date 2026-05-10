part of 'settings_page.dart';

/// SoloSoul advertisement section showing app branding and tagline.
/// Extracted from SettingsPage to reduce file length.
class _SoloSoulAdSection extends StatelessWidget {
  const _SoloSoulAdSection();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            AppTheme.primaryColor.withValues(alpha: 0.15),
            AppTheme.primaryColor.withValues(alpha: 0.05),
          ],
        ),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: AppTheme.primaryColor.withValues(alpha: 0.2),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: AppTheme.primaryColor.withValues(alpha: 0.2),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: const Icon(
                  Icons.auto_awesome,
                  color: AppTheme.primaryColor,
                  size: 28,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      l10n.mainAppTitle,
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                        color: AppTheme.primaryColor,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      l10n.settingsTagline,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        height: 1.4,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  SloganChip(
                    icon: Icons.location_on_outlined,
                    label: l10n.settingsLocal,
                  ),
                  const SizedBox(width: 8),
                  SloganChip(
                    icon: Icons.lock_outline,
                    label: l10n.settingsPrivate,
                  ),
                  const SizedBox(width: 8),
                  SloganChip(
                    icon: Icons.person_outline,
                    label: l10n.settingsUniversal,
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    ).animate().fadeIn(delay: 400.ms, duration: 400.ms);
  }
}
