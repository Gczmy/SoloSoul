
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section_helpers.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_document_button.dart';

class TravelPage extends ConsumerStatefulWidget {
  const TravelPage({super.key});

  @override
  ConsumerState<TravelPage> createState() => _TravelPageState();
}

class _TravelPageState extends ConsumerState<TravelPage> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return ObjectCategoryPage(
      title: AppLocalizations.of(context).travelTitle,
      sections: [
        const SizedBox(height: 8),
        const ScanDocumentButton(parentId: DefaultSectionIds.passport),
        const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.passport),
              sectionId: DefaultSectionIds.passport,
              typeId: 'travel_passport',
              title: AppLocalizations.of(context).travelPassports,
              icon: Icons.flight_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (passport, itemMap) => EntryCardWidget<UnifiedObject>(
                item: passport,
                title: passport.name,
                icon: Icons.book,
                itemId: passport.id,
                historyFieldId: 'passport',
                isRestricted: true,
                formatAllFields: (p) => 'Passport\n${p.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'passport',
                excludeFields: const {'title'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.travel,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'passport',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.visa),
              sectionId: DefaultSectionIds.visa,
              typeId: 'travel_visa',
              title: AppLocalizations.of(context).travelVisas,
              icon: Icons.assignment_ind_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (visa, itemMap) => EntryCardWidget<UnifiedObject>(
                item: visa,
                title: visa.name,
                icon: Icons.article,
                itemId: visa.id,
                historyFieldId: 'visa',
                isRestricted: true,
                formatAllFields: (v) => 'Visa\n${v.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'visa',
                excludeFields: const {'title'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.travel,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'visa',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              key: const ValueKey(DefaultSectionIds.travelHistory),
              sectionId: DefaultSectionIds.travelHistory,
              typeId: 'travel_history',
              title: AppLocalizations.of(context).travelHistory,
              icon: Icons.history_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.place,
                itemId: item.id,
                historyFieldId: 'travel',
                formatAllFields: (t) => 'Travel History\n${t.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'travel',
                excludeFields: const {'destination'},
              ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.travel,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'travel history',
              ),
              onCopyAll: buildOnCopyAll(context),
            )
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
      ],
    );
  }
}

