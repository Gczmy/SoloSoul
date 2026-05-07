
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section_helpers.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_scanner_sheet.dart';

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

  Widget _buildScanButton(BuildContext context) {
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16),
      child: InkWell(
        onTap: () => _showMrzScanner(context),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  Icons.document_scanner_outlined,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Scan Passport',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Extract MRZ data from passport photo',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context)
                                .colorScheme
                                .onSurfaceVariant,
                          ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  Future<void> _showMrzScanner(BuildContext context) async {
    final result = await showModalBottomSheet<MrzData?>(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => const MrzScannerSheet(),
    );

    if (result != null && context.mounted) {
      await _createPassportFromMrz(result);
    }
  }

  Future<void> _createPassportFromMrz(MrzData mrz) async {
    final notifier = ref.read(unifiedObjectProvider.notifier);

    // 构建护照属性（键名必须与 ObjectTypeRegistry 中 travel_passport 的
    // PropertyDefinition.id 一致）
    final properties = <String, PropertyValue>{
      'title': TextProperty(
        text: mrz.documentType,
        sensitivity: SensitivityLevel.public,
      ),
      'number': TextProperty(
        text: mrz.documentNumber,
        sensitivity: SensitivityLevel.critical,
      ),
      'holderName': TextProperty(
        text: '${mrz.surname} ${mrz.givenNames}'.trim(),
        sensitivity: SensitivityLevel.public,
      ),
      'nationality': TextProperty(
        text: mrz.nationality,
        sensitivity: SensitivityLevel.public,
      ),
      'dateOfBirth': TextProperty(
        text: mrz.dateOfBirth,
        sensitivity: SensitivityLevel.sensitive,
      ),
      'sex': TextProperty(
        text: mrz.sex,
        sensitivity: SensitivityLevel.public,
      ),
      'expiryDate': TextProperty(
        text: mrz.expiryDate,
        sensitivity: SensitivityLevel.sensitive,
      ),
    };

    // 关键：parentId 必须设为 passport section，否则对象不会显示在 Travel 页面
    final success = await notifier.createObject(
      name: '${mrz.surname} ${mrz.givenNames}'.trim(),
      typeId: 'travel_passport',
      iconName: 'book',
      parentId: DefaultSectionIds.passport,
      properties: properties,
    );

    if (success && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Passport created: ${mrz.documentNumber}'),
          duration: const Duration(seconds: 2),
        ),
      );
    } else if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Failed to save passport'),
          backgroundColor: Colors.red,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return ObjectCategoryPage(
      title: 'Travel',
      sections: [
        const SizedBox(height: 8),
        _buildScanButton(context),
        const SizedBox(height: 16),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.passport,
              typeId: 'travel_passport',
              title: 'Passports',
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
              sectionId: DefaultSectionIds.visa,
              typeId: 'travel_visa',
              title: 'Visas',
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
              sectionId: DefaultSectionIds.travelHistory,
              typeId: 'travel_history',
              title: 'Travel History',
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

