part of 'settings_page.dart';

/// Help & Guides section listing all user-facing feature guides.
/// Guides are loaded from `assets/docs/guides/` via [UserGuideService].
class _HelpSettingsSection extends ConsumerWidget {
  const _HelpSettingsSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final guides = UserGuideService.instance.guideList;
    final locale = Localizations.localeOf(context);
    final language = locale.languageCode;

    if (guides.isEmpty) {
      return const SizedBox.shrink();
    }

    return SectionCard(
      title: AppLocalizations.of(context).settingsHelpAndGuides,
      icon: Icons.menu_book_outlined,
      children: [
        for (var i = 0; i < guides.length; i++) ...[
          if (i > 0) const Divider(height: 1),
          SettingsTile(
            icon: Icons.article_outlined,
            title: _guideTitle(guides[i], language),
            subtitle: _guideSubtitle(guides[i], language),
            onTap: () => _showGuide(context, guides[i], language),
          ),
        ],
      ],
    ).animate().fadeIn(delay: 350.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }

  String _guideTitle(GuideIndexEntry guide, String language) {
    return language == 'zh' ? guide.title : guide.titleEn;
  }

  String _guideSubtitle(GuideIndexEntry guide, String language) {
    return language == 'zh' ? guide.description : guide.descriptionEn;
  }

  void _showGuide(BuildContext context, GuideIndexEntry guide, String language) {
    final title = _guideTitle(guide, language);
    final assetPath = _resolveAssetPath(guide, language);
    showLegalDocumentSheet(
      context: context,
      title: title,
      assetPath: assetPath,
    );
  }

  String _resolveAssetPath(GuideIndexEntry guide, String language) {
    final files = guide.files;
    if (files.containsKey(language)) {
      return files[language]!;
    }
    if (files.containsKey('en')) {
      return files['en']!;
    }
    return files.values.first;
  }
}
