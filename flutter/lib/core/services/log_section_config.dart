import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';

/// Centralized configuration for operation log section mapping
/// To add new form categories, update this file only
class LogSectionConfig {
  /// Maps (section, itemType) tuples to their corresponding LogSection
  /// Add new entries here when adding new form categories
  static const Map<(String, String), LogSection> itemTypeToSection = {
    // Profile section
    ('profile', 'contact'): LogSection.contactInformation,
    ('profile', 'idCard'): LogSection.idCard,
    ('profile', 'address'): LogSection.address,

    // Travel section
    ('travel', 'passport'): LogSection.passport,
    ('travel', 'visa'): LogSection.visa,
    ('travel', 'travel_history'): LogSection.travelHistory,

    // Financial section
    ('financial', 'bank_account'): LogSection.bankAccount,
    ('financial', 'card'): LogSection.card,

    // Professional section
    ('professional', 'education'): LogSection.education,
    ('professional', 'employment'): LogSection.employment,
    ('professional', 'skill'): LogSection.skill,
    ('professional', 'language'): LogSection.language,
  };

  /// Default section to LogSection mapping for whole sections
  static const Map<String, LogSection> sectionToLogSection = {
    'identity': LogSection.identity,
    'profile': LogSection.identity,
    'travel': LogSection.travel,
    'financial': LogSection.financial,
    'professional': LogSection.professional,
  };

  /// Get LogSection for a given section and itemType
  /// Falls back to section-based mapping if no itemType-specific mapping exists
  static LogSection getLogSection(String section, String itemType) {
    return itemTypeToSection[(section, itemType)] ??
        sectionToLogSection[section] ??
        LogSection.identity;
  }

  /// Get item label for logging based on itemType
  static String getItemLabel(String section, String itemType, dynamic item) {
    switch (itemType) {
      // Travel items
      case 'passport':
        return (item as PassportData?)?.country ?? 'Passport';
      case 'visa':
        return (item as VisaData?)?.country ?? 'Visa';

      // Financial items
      case 'bank_account':
        return (item as BankAccountData?)?.bankName ?? 'Bank Account';
      case 'card':
        return (item as CardData?)?.cardType ?? 'Card';

      // Professional items
      case 'education':
        return (item as EducationData?)?.institution ?? 'Education';
      case 'employment':
        return (item as EmploymentData?)?.company ?? 'Employment';

      // Profile items
      case 'contact':
        final entry = item as ContactEntry?;
        return entry?.title.isNotEmpty == true
            ? '${entry!.title} - ${entry.value}'
            : entry?.value ?? 'Contact';
      case 'idCard':
        final idCard = item as IdCardData?;
        return idCard?.title ?? idCard?.number ?? 'ID Card';
      case 'address':
        return (item as AddressData?)?.title ?? 'Address';

      // Simple string items
      case 'travel_history':
        return (item as TravelHistoryData?)?.destination ?? itemType;
      case 'skill':
        return (item as SkillData?)?.toString() ?? itemType;
      case 'language':
        return (item as LanguageData?)?.toString() ?? itemType;

      default:
        return itemType;
    }
  }

  /// Get all supported itemTypes for a given section
  static List<String> getItemTypesForSection(String section) {
    return itemTypeToSection.entries
        .where((e) => e.key.$1 == section)
        .map((e) => e.key.$2)
        .toList();
  }

  /// Get all supported sections
  static List<String> get allSections => sectionToLogSection.keys.toList();
}

/// Extension to add helper methods to LogSection
extension LogSectionExtension on LogSection {
  /// Create OperationEntry with this section
  OperationEntry createEntry({
    required LogAction action,
    required String description,
    String? fieldPath,
  }) {
    return OperationEntry(
      timestamp: DateTime.now(),
      action: action.value,
      section: value,
      description: description,
      fieldPath: fieldPath,
    );
  }
}
