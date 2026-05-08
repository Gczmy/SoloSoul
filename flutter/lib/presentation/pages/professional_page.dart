
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section_helpers.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_document_button.dart';

class ProfessionalPage extends ConsumerStatefulWidget {
  const ProfessionalPage({super.key});

  @override
  ConsumerState<ProfessionalPage> createState() => _ProfessionalPageState();
}

class _ProfessionalPageState extends ConsumerState<ProfessionalPage> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return ObjectCategoryPage(
      title: l10n.professionalTitle,
      sections: [
            const ScanDocumentButton(parentId: DefaultSectionIds.employment),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.education),
              sectionId: DefaultSectionIds.education,
              typeId: 'professional_education',
              title: l10n.professionalEducation,
              icon: Icons.school_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.school,
                itemId: item.id,
                historyFieldId: 'education',
                formatAllFields: (e) => l10n.professionalFormatEducation(e.toFormattedString()),
                itemData: itemMap,
                fieldPrefix: 'education',
                excludeFields: const {'institution'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.professional,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'education',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.employment),
              sectionId: DefaultSectionIds.employment,
              typeId: 'professional_employment',
              title: l10n.professionalEmployment,
              icon: Icons.work_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.work,
                itemId: item.id,
                historyFieldId: 'employment',
                formatAllFields: (e) => l10n.professionalFormatEmployment(e.toFormattedString()),
                itemData: itemMap,
                fieldPrefix: 'employment',
                excludeFields: const {'company'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.professional,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'employment',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.award),
              sectionId: DefaultSectionIds.award,
              typeId: 'professional_award',
              title: l10n.professionalAwards,
              icon: Icons.emoji_events_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.emoji_events,
                itemId: item.id,
                historyFieldId: 'award',
                formatAllFields: (e) => l10n.professionalFormatAward(e.toFormattedString()),
                itemData: itemMap,
                fieldPrefix: 'award',
                excludeFields: const {'title'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.professional,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'award',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.skill),
              sectionId: DefaultSectionIds.skill,
              typeId: 'professional_skill',
              title: l10n.professionalSkills,
              icon: Icons.star_outline,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.star,
                itemId: item.id,
                historyFieldId: 'skill',
                formatAllFields: (e) => l10n.professionalFormatSkill(e.toFormattedString()),
                itemData: itemMap,
                fieldPrefix: 'skill',
                excludeFields: const {'name'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.professional,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'skill',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 400.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.language),
              sectionId: DefaultSectionIds.language,
              typeId: 'professional_language',
              title: l10n.professionalLanguages,
              icon: Icons.translate,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.translate,
                itemId: item.id,
                historyFieldId: 'language',
                formatAllFields: (e) => l10n.professionalFormatLanguage(e.toFormattedString()),
                itemData: itemMap,
                fieldPrefix: 'language',
                excludeFields: const {'name'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.professional,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'language',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 500.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
      ],
    );
  }

}
