import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart'
    show DefaultPageIds, DefaultSectionIds, getItemTypeIdForSection;
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';

/// Convert a [UnifiedObject] to a flat String map from its properties.
Map<String, String> itemToMap(UnifiedObject item) {
  return <String, String>{
    for (final entry in item.properties.entries)
      entry.key: propValueToString(entry.value),
  };
}

/// Pure-data configuration for a preset section type.
///
/// Used by [DynamicSectionCard] to build the correct [ObjectCard] parameters
/// without hard-coding them in page files.
class PresetSectionConfig {
  final String typeId;

  /// Localized title builder.
  final String Function(AppLocalizations) l10nTitle;

  /// Section header icon.
  final IconData icon;

  /// Icon shown inside each item's [EntryCardWidget].
  final IconData itemIcon;

  /// Max items visible before "Show more" collapse.
  final int maxVisibleItems;

  /// History field identifier (e.g. 'identity', 'passport').
  final String historyFieldId;

  /// Field prefix for sensitivity lookup and auto-build (e.g. 'identity').
  final String fieldPrefix;

  /// The property key that maps to the title input field.
  final String titlePropertyKey;

  /// Field keys excluded from auto-built [EntryCardWidget] fields.
  final Set<String> excludeFields;

  /// Whether items in this section are restricted (require password).
  final bool isRestricted;

  /// Optional formatter for "copy all" action.
  /// Receives the [AppLocalizations] and the [UnifiedObject] item.
  final String Function(AppLocalizations, UnifiedObject)? formatAllFields;

  const PresetSectionConfig({
    required this.typeId,
    required this.l10nTitle,
    required this.icon,
    required this.itemIcon,
    this.maxVisibleItems = 3,
    required this.historyFieldId,
    required this.fieldPrefix,
    required this.titlePropertyKey,
    this.excludeFields = const {},
    this.isRestricted = false,
    this.formatAllFields,
  });
}

/// Registry that maps preset [typeId]s to their rendering configuration.
///
/// Custom sections (typeId: 'collection') have no entry here and fall back
/// to the generic [ObjectCard] rendering.
class SectionRendererRegistry {
  static final Map<String, PresetSectionConfig> _configs = {
    // ── Profile ──
    'profile_identity': PresetSectionConfig(
      typeId: 'profile_identity',
      l10nTitle: (l) => l.profileIdentity,
      icon: Icons.person_outlined,
      itemIcon: Icons.person,
      maxVisibleItems: 1,
      historyFieldId: 'identity',
      fieldPrefix: 'identity',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          '${l10n.profileIdentity}\n${item.toFormattedStringLocalized(l10n)}',
    ),
    'profile_contact': PresetSectionConfig(
      typeId: 'profile_contact',
      l10nTitle: (l) => l.profileContactInfo,
      icon: Icons.contact_mail_outlined,
      itemIcon: Icons.email_outlined,
      maxVisibleItems: 3,
      historyFieldId: 'contact',
      fieldPrefix: 'contact',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) {
        final map = itemToMap(item);
        final type = (map['type'] ?? 'contact').trim().isNotEmpty
            ? map['type']!
            : 'contact';
        return '$type: ${map['value'] ?? ''}';
      },
    ),
    'profile_id_card': PresetSectionConfig(
      typeId: 'profile_id_card',
      l10nTitle: (l) => l.profileIdentityDocuments,
      icon: Icons.badge_outlined,
      itemIcon: Icons.badge_outlined,
      maxVisibleItems: 3,
      historyFieldId: 'idCard',
      fieldPrefix: 'idCard',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          '${l10n.profileIdCard}\n${item.toFormattedStringLocalized(l10n)}',
    ),
    'profile_address': PresetSectionConfig(
      typeId: 'profile_address',
      l10nTitle: (l) => l.profileAddresses,
      icon: Icons.location_on_outlined,
      itemIcon: Icons.home_outlined,
      maxVisibleItems: 3,
      historyFieldId: 'address',
      fieldPrefix: 'address',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
    ),

    // ── Travel ──
    'travel_passport': PresetSectionConfig(
      typeId: 'travel_passport',
      l10nTitle: (l) => l.travelPassports,
      icon: Icons.flight_outlined,
      itemIcon: Icons.book,
      maxVisibleItems: 3,
      historyFieldId: 'passport',
      fieldPrefix: 'passport',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      isRestricted: true,
      formatAllFields: (l10n, item) =>
          l10n.travelFormatPassport(item.toFormattedStringLocalized(l10n)),
    ),
    'travel_visa': PresetSectionConfig(
      typeId: 'travel_visa',
      l10nTitle: (l) => l.travelVisas,
      icon: Icons.assignment_ind_outlined,
      itemIcon: Icons.article,
      maxVisibleItems: 3,
      historyFieldId: 'visa',
      fieldPrefix: 'visa',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      isRestricted: true,
      formatAllFields: (l10n, item) =>
          l10n.travelFormatVisa(item.toFormattedStringLocalized(l10n)),
    ),
    'travel_history': PresetSectionConfig(
      typeId: 'travel_history',
      l10nTitle: (l) => l.travelHistory,
      icon: Icons.history_outlined,
      itemIcon: Icons.place,
      maxVisibleItems: 3,
      historyFieldId: 'travel',
      fieldPrefix: 'travel',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.travelFormatHistory(item.toFormattedStringLocalized(l10n)),
    ),

    // ── Financial ──
    'financial_bank_account': PresetSectionConfig(
      typeId: 'financial_bank_account',
      l10nTitle: (l) => l.financialBankAccounts,
      icon: Icons.account_balance_outlined,
      itemIcon: Icons.account_balance,
      maxVisibleItems: 3,
      historyFieldId: 'bankAccount',
      fieldPrefix: 'bankAccount',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      isRestricted: true,
      formatAllFields: (l10n, item) =>
          l10n.financialFormatBankAccount(item.toFormattedStringLocalized(l10n)),
    ),
    'financial_card': PresetSectionConfig(
      typeId: 'financial_card',
      l10nTitle: (l) => l.financialCards,
      icon: Icons.credit_card_outlined,
      itemIcon: Icons.credit_card,
      maxVisibleItems: 3,
      historyFieldId: 'card',
      fieldPrefix: 'card',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      isRestricted: true,
      formatAllFields: (l10n, item) =>
          l10n.financialFormatCard(item.toFormattedStringLocalized(l10n)),
    ),
    'financial_tax_id': PresetSectionConfig(
      typeId: 'financial_tax_id',
      l10nTitle: (l) => l.financialTaxIdentification,
      icon: Icons.receipt_long_outlined,
      itemIcon: Icons.badge,
      maxVisibleItems: 3,
      historyFieldId: 'taxId',
      fieldPrefix: 'taxId',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      isRestricted: true,
      formatAllFields: (l10n, item) =>
          l10n.financialFormatTaxId(item.toFormattedStringLocalized(l10n)),
    ),

    // ── Professional ──
    'professional_education': PresetSectionConfig(
      typeId: 'professional_education',
      l10nTitle: (l) => l.professionalEducation,
      icon: Icons.school_outlined,
      itemIcon: Icons.school,
      maxVisibleItems: 3,
      historyFieldId: 'education',
      fieldPrefix: 'education',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatEducation(item.toFormattedStringLocalized(l10n)),
    ),
    'professional_employment': PresetSectionConfig(
      typeId: 'professional_employment',
      l10nTitle: (l) => l.professionalEmployment,
      icon: Icons.work_outlined,
      itemIcon: Icons.work,
      maxVisibleItems: 3,
      historyFieldId: 'employment',
      fieldPrefix: 'employment',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatEmployment(item.toFormattedStringLocalized(l10n)),
    ),
    'professional_skill': PresetSectionConfig(
      typeId: 'professional_skill',
      l10nTitle: (l) => l.professionalSkills,
      icon: Icons.star_outline,
      itemIcon: Icons.star,
      maxVisibleItems: 3,
      historyFieldId: 'skill',
      fieldPrefix: 'skill',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatSkill(item.toFormattedStringLocalized(l10n)),
    ),
    'professional_language': PresetSectionConfig(
      typeId: 'professional_language',
      l10nTitle: (l) => l.professionalLanguages,
      icon: Icons.translate,
      itemIcon: Icons.translate,
      maxVisibleItems: 3,
      historyFieldId: 'language',
      fieldPrefix: 'language',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatLanguage(item.toFormattedStringLocalized(l10n)),
    ),
    'professional_award': PresetSectionConfig(
      typeId: 'professional_award',
      l10nTitle: (l) => l.professionalAwards,
      icon: Icons.emoji_events_outlined,
      itemIcon: Icons.emoji_events,
      maxVisibleItems: 3,
      historyFieldId: 'award',
      fieldPrefix: 'award',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatAward(item.toFormattedStringLocalized(l10n)),
    ),
    'professional_article': PresetSectionConfig(
      typeId: 'professional_article',
      l10nTitle: (l) => l.professionalArticles,
      icon: Icons.article_outlined,
      itemIcon: Icons.article,
      maxVisibleItems: 3,
      historyFieldId: 'article',
      fieldPrefix: 'article',
      titlePropertyKey: 'title',
      excludeFields: const {},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatArticle(item.toFormattedStringLocalized(l10n)),
    ),
  };

  /// Get the config for a preset [typeId], or `null` for generic types.
  static PresetSectionConfig? getConfig(String typeId) => _configs[typeId];

  /// Whether the given [typeId] has a preset config.
  static bool isPreset(String typeId) => _configs.containsKey(typeId);

  /// All registered preset type IDs.
  static Iterable<String> get presetTypeIds => _configs.keys;

  /// Get the config for a preset section by its [sectionId] (e.g.
  /// [DefaultSectionIds.passport]). Returns `null` for custom sections.
  static PresetSectionConfig? getConfigBySectionId(String sectionId) {
    final itemTypeId = getItemTypeIdForSection(sectionId);
    if (itemTypeId == null) return null;
    return _configs[itemTypeId];
  }
}

/// Returns a localized display name for a [UnifiedObject].
///
/// Default pages (Profile, Travel, Financial, Professional) and preset
/// sections (identity, passport, bank account, etc.) are mapped through
/// [AppLocalizations]. Custom pages/sections fall back to [object.name].
String getLocalizedObjectName(AppLocalizations? l10n, UnifiedObject object) {
  if (l10n == null) return object.name;

  // Default pages
  if (object.typeId == 'page') {
    return switch (object.id) {
      DefaultPageIds.profile => l10n.profileTitle,
      DefaultPageIds.travel => l10n.travelTitle,
      DefaultPageIds.financial => l10n.financialTitle,
      DefaultPageIds.professional => l10n.professionalTitle,
      _ => object.name,
    };
  }

  // Preset sections — look up by section ID first (stored typeId is 'collection')
  final config = SectionRendererRegistry.getConfigBySectionId(object.id) ??
      SectionRendererRegistry.getConfig(object.typeId ?? '');
  if (config != null) return config.l10nTitle(l10n);

  // Custom sections / fallback
  return object.name;
}
