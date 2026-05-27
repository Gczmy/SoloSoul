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

  /// Material icon name string for storage in [UnifiedObject.iconName].
  final String iconName;

  /// Default English name used when no [AppLocalizations] is available.
  final String defaultName;

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
    required this.iconName,
    required this.defaultName,
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
    '__preset_identity': PresetSectionConfig(
      typeId: '__preset_identity',
      l10nTitle: (l) => l.builtinSectionIdentityTitle,
      icon: Icons.person_outlined,
      iconName: 'person',
      defaultName: 'Identity',
      itemIcon: Icons.person,
      maxVisibleItems: 1,
      historyFieldId: 'identity',
      fieldPrefix: 'identity',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          '${l10n.builtinSectionIdentityTitle}\n${item.toFormattedStringLocalized(l10n)}',
    ),
    '__preset_contact': PresetSectionConfig(
      typeId: '__preset_contact',
      l10nTitle: (l) => l.builtinSectionContactTitle,
      icon: Icons.contact_mail_outlined,
      iconName: 'contact_mail',
      defaultName: 'Contact Information',
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
    '__preset_identity_document': PresetSectionConfig(
      typeId: '__preset_identity_document',
      l10nTitle: (l) => l.builtinSectionIdentityDocumentTitle,
      icon: Icons.badge_outlined,
      iconName: 'badge',
      defaultName: 'ID Cards',
      itemIcon: Icons.badge_outlined,
      maxVisibleItems: 3,
      historyFieldId: 'idCard',
      fieldPrefix: 'idCard',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          '${l10n.builtinSectionIdentityDocumentTitle}\n${item.toFormattedStringLocalized(l10n)}',
    ),
    '__preset_address': PresetSectionConfig(
      typeId: '__preset_address',
      l10nTitle: (l) => l.builtinSectionAddressTitle,
      icon: Icons.location_on_outlined,
      iconName: 'home',
      defaultName: 'Addresses',
      itemIcon: Icons.home_outlined,
      maxVisibleItems: 3,
      historyFieldId: 'address',
      fieldPrefix: 'address',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
    ),

    // ── Travel ──
    '__preset_passport': PresetSectionConfig(
      typeId: '__preset_passport',
      l10nTitle: (l) => l.builtinSectionPassportTitle,
      icon: Icons.flight_outlined,
      iconName: 'flight',
      defaultName: 'Passports',
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
    '__preset_visa': PresetSectionConfig(
      typeId: '__preset_visa',
      l10nTitle: (l) => l.builtinSectionVisaTitle,
      icon: Icons.assignment_ind_outlined,
      iconName: 'description',
      defaultName: 'Visas',
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
    '__preset_travel_history': PresetSectionConfig(
      typeId: '__preset_travel_history',
      l10nTitle: (l) => l.builtinSectionTravelHistoryTitle,
      icon: Icons.history_outlined,
      iconName: 'history',
      defaultName: 'Travel History',
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
    '__preset_bank_account': PresetSectionConfig(
      typeId: '__preset_bank_account',
      l10nTitle: (l) => l.builtinSectionBankAccountTitle,
      icon: Icons.account_balance_outlined,
      iconName: 'account_balance',
      defaultName: 'Bank Accounts',
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
    '__preset_payment_card': PresetSectionConfig(
      typeId: '__preset_payment_card',
      l10nTitle: (l) => l.builtinSectionPaymentCardTitle,
      icon: Icons.credit_card_outlined,
      iconName: 'credit_card',
      defaultName: 'Cards',
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
    '__preset_tax_id': PresetSectionConfig(
      typeId: '__preset_tax_id',
      l10nTitle: (l) => l.builtinSectionTaxIdTitle,
      icon: Icons.receipt_long_outlined,
      iconName: 'receipt',
      defaultName: 'Tax IDs',
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
    '__preset_education': PresetSectionConfig(
      typeId: '__preset_education',
      l10nTitle: (l) => l.builtinSectionEducationTitle,
      icon: Icons.school_outlined,
      iconName: 'school',
      defaultName: 'Education',
      itemIcon: Icons.school,
      maxVisibleItems: 3,
      historyFieldId: 'education',
      fieldPrefix: 'education',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatEducation(item.toFormattedStringLocalized(l10n)),
    ),
    '__preset_employment': PresetSectionConfig(
      typeId: '__preset_employment',
      l10nTitle: (l) => l.builtinSectionEmploymentTitle,
      icon: Icons.work_outlined,
      iconName: 'work',
      defaultName: 'Employment',
      itemIcon: Icons.work,
      maxVisibleItems: 3,
      historyFieldId: 'employment',
      fieldPrefix: 'employment',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatEmployment(item.toFormattedStringLocalized(l10n)),
    ),
    '__preset_skill': PresetSectionConfig(
      typeId: '__preset_skill',
      l10nTitle: (l) => l.builtinSectionSkillTitle,
      icon: Icons.star_outline,
      iconName: 'stars',
      defaultName: 'Skills',
      itemIcon: Icons.star,
      maxVisibleItems: 3,
      historyFieldId: 'skill',
      fieldPrefix: 'skill',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatSkill(item.toFormattedStringLocalized(l10n)),
    ),
    '__preset_language': PresetSectionConfig(
      typeId: '__preset_language',
      l10nTitle: (l) => l.builtinSectionLanguageTitle,
      icon: Icons.translate,
      iconName: 'language',
      defaultName: 'Languages',
      itemIcon: Icons.translate,
      maxVisibleItems: 3,
      historyFieldId: 'language',
      fieldPrefix: 'language',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatLanguage(item.toFormattedStringLocalized(l10n)),
    ),
    '__preset_award': PresetSectionConfig(
      typeId: '__preset_award',
      l10nTitle: (l) => l.builtinSectionAwardTitle,
      icon: Icons.emoji_events_outlined,
      iconName: 'emoji_events',
      defaultName: 'Awards',
      itemIcon: Icons.emoji_events,
      maxVisibleItems: 3,
      historyFieldId: 'award',
      fieldPrefix: 'award',
      titlePropertyKey: 'Title',
      excludeFields: const {'Title'},
      formatAllFields: (l10n, item) =>
          l10n.professionalFormatAward(item.toFormattedStringLocalized(l10n)),
    ),
    '__preset_article': PresetSectionConfig(
      typeId: '__preset_article',
      l10nTitle: (l) => l.builtinSectionArticleTitle,
      icon: Icons.article_outlined,
      iconName: 'article',
      defaultName: 'Articles',
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

  /// 根据字段路径的 section key（即 [PresetSectionConfig.fieldPrefix]）
  /// 获取对应的显示标签。
  ///
  /// 这是页面/Sidebar 分区名称的唯一来源。
  /// 授权对话框通过此方法复用分区标题，确保与页面显示一致。
  ///
  /// 返回 `null` 表示无匹配的 preset（如 `financial`、`medical` 等
  /// page 级别的分组），调用方应 fallback 到 ARB 通用翻译。
  static String? getSectionLabelByFieldPrefix(
    String fieldPrefix,
    AppLocalizations l10n,
  ) {
    for (final config in _configs.values) {
      if (config.fieldPrefix == fieldPrefix) {
        return config.l10nTitle(l10n);
      }
    }
    return null;
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
