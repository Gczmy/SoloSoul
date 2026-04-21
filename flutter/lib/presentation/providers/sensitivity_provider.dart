import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// Re-export SensitivityLevel from sensitivity_enums for backward compatibility
export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart' show SensitivityLevel, SensitivityLevelExtension;

// Re-export AccountStyle, SensitivityResolver, SensitivityDisplayMode, and helper
export 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show AccountStyle, AccountStyleNotifier, accountStyleProvider,
        SensitivityResolver, sensitivityResolver, SensitivityDisplayMode,
        firstWhereOrNull;

// Import accountStyleProvider for internal use within this file
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider;

/// Helper to find first matching element or null
FieldSensitivity? firstWhereOrNull(List<FieldSensitivity> list, bool Function(FieldSensitivity) test) {
  for (final item in list) {
    if (test(item)) return item;
  }
  return null;
}

/// Represents a single field's sensitivity configuration
class FieldSensitivity {
  final String fieldId;
  final String fieldName;
  final String fieldSection;
  final SensitivityLevel level;

  const FieldSensitivity({
    required this.fieldId,
    required this.fieldName,
    required this.fieldSection,
    required this.level,
  });

  FieldSensitivity copyWith({
    String? fieldId,
    String? fieldName,
    String? fieldSection,
    SensitivityLevel? level,
  }) {
    return FieldSensitivity(
      fieldId: fieldId ?? this.fieldId,
      fieldName: fieldName ?? this.fieldName,
      fieldSection: fieldSection ?? this.fieldSection,
      level: level ?? this.level,
    );
  }

  Map<String, dynamic> toJson() => {
        'fieldId': fieldId,
        'fieldName': fieldName,
        'fieldSection': fieldSection,
        'level': level.index,
      };

  factory FieldSensitivity.fromJson(Map<String, dynamic> json) {
    return FieldSensitivity(
      fieldId: json['fieldId'] as String,
      fieldName: json['fieldName'] as String,
      fieldSection: json['fieldSection'] as String,
      level: SensitivityLevel.values[json['level'] as int],
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FieldSensitivity &&
          runtimeType == other.runtimeType &&
          fieldId == other.fieldId;

  @override
  int get hashCode => fieldId.hashCode;
}

/// All field definitions organized by section
class FieldRegistry {
  static const List<FieldSensitivity> defaultFields = [
    // Identity Section
    FieldSensitivity(
      fieldId: 'identity.fullName',
      fieldName: 'Full Name',
      fieldSection: 'identity',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'identity.givenName',
      fieldName: 'Given Name',
      fieldSection: 'identity',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'identity.familyName',
      fieldName: 'Family Name',
      fieldSection: 'identity',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'identity.dateOfBirth',
      fieldName: 'Date of Birth',
      fieldSection: 'identity',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'identity.gender',
      fieldName: 'Gender',
      fieldSection: 'identity',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'identity.nationality',
      fieldName: 'Nationality',
      fieldSection: 'identity',
      level: SensitivityLevel.sensitive,
    ),

    // Contact Section
    // Note: contact.title, contact.type, contact.value are registered by ContactForm
    // Legacy fields (contact.email, contact.phone, contact.mobile, contact.address)
    // were removed - they don't exist in the actual form definitions

    // ID Card Section
    FieldSensitivity(
      fieldId: 'idCard.title',
      fieldName: 'Title',
      fieldSection: 'idCard',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'idCard.number',
      fieldName: 'ID Card Number',
      fieldSection: 'idCard',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'idCard.holderName',
      fieldName: 'Holder Name',
      fieldSection: 'idCard',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'idCard.issueDate',
      fieldName: 'Issue Date',
      fieldSection: 'idCard',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'idCard.expiryDate',
      fieldName: 'Expiry Date',
      fieldSection: 'idCard',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'idCard.country',
      fieldName: 'Country',
      fieldSection: 'idCard',
      level: SensitivityLevel.public,
    ),

    // Address Section
    FieldSensitivity(
      fieldId: 'address.street',
      fieldName: 'Street',
      fieldSection: 'address',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'address.city',
      fieldName: 'City',
      fieldSection: 'address',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'address.postalCode',
      fieldName: 'Postal Code',
      fieldSection: 'address',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'address.country',
      fieldName: 'Country',
      fieldSection: 'address',
      level: SensitivityLevel.public,
    ),

    // Passport Section
    FieldSensitivity(
      fieldId: 'passport.title',
      fieldName: 'Title',
      fieldSection: 'passport',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'passport.number',
      fieldName: 'Passport Number',
      fieldSection: 'passport',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'passport.country',
      fieldName: 'Country',
      fieldSection: 'passport',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'passport.countryCode',
      fieldName: 'Country Code',
      fieldSection: 'passport',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'passport.issueDate',
      fieldName: 'Issue Date',
      fieldSection: 'passport',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'passport.placeOfIssue',
      fieldName: 'Place of Issue',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'passport.expiryDate',
      fieldName: 'Expiry Date',
      fieldSection: 'passport',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'passport.holderName',
      fieldName: 'Holder Name',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'passport.dateOfBirth',
      fieldName: 'Date of Birth',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'passport.placeOfBirth',
      fieldName: 'Place of Birth',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'passport.sex',
      fieldName: 'Sex',
      fieldSection: 'passport',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'passport.nationality',
      fieldName: 'Nationality',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'passport.authority',
      fieldName: 'Authority',
      fieldSection: 'passport',
      level: SensitivityLevel.sensitive,
    ),

    // Visa Section
    FieldSensitivity(
      fieldId: 'visa.title',
      fieldName: 'Title',
      fieldSection: 'visa',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'visa.number',
      fieldName: 'Visa Number',
      fieldSection: 'visa',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'visa.country',
      fieldName: 'Country',
      fieldSection: 'visa',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'visa.visaType',
      fieldName: 'Visa Type',
      fieldSection: 'visa',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'visa.issueDate',
      fieldName: 'Issue Date',
      fieldSection: 'visa',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'visa.expiryDate',
      fieldName: 'Expiry Date',
      fieldSection: 'visa',
      level: SensitivityLevel.internal,
    ),

    // Travel Section
    FieldSensitivity(
      fieldId: 'travel.destination',
      fieldName: 'Destination',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.travelType',
      fieldName: 'Travel Type',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.date',
      fieldName: 'Date',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.departureCity',
      fieldName: 'Departure City',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.departureTime',
      fieldName: 'Departure Time',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.arrivalTime',
      fieldName: 'Arrival Time',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.flightNumber',
      fieldName: 'Flight Number',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'travel.ticketPrice',
      fieldName: 'Ticket Price',
      fieldSection: 'travel',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'travel.airline',
      fieldName: 'Airline',
      fieldSection: 'travel',
      level: SensitivityLevel.public,
    ),

    // Bank Account Section
    FieldSensitivity(
      fieldId: 'bankAccount.accountNumber',
      fieldName: 'Account Number',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.accountHolderName',
      fieldName: 'Account Holder Name',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.routingNumber',
      fieldName: 'Routing Number',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.bankName',
      fieldName: 'Bank Name',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.swiftBic',
      fieldName: 'SWIFT/BIC',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.sortCode',
      fieldName: 'Sort Code',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'bankAccount.currency',
      fieldName: 'Currency',
      fieldSection: 'bankAccount',
      level: SensitivityLevel.public,
    ),

    // Card Section
    FieldSensitivity(
      fieldId: 'card.cardNumber',
      fieldName: 'Card Number',
      fieldSection: 'card',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'card.cardType',
      fieldName: 'Card Type',
      fieldSection: 'card',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'card.title',
      fieldName: 'Title',
      fieldSection: 'card',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'card.expiryDate',
      fieldName: 'Expiry Date',
      fieldSection: 'card',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'card.holderName',
      fieldName: 'Holder Name',
      fieldSection: 'card',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'card.cvv',
      fieldName: 'CVV',
      fieldSection: 'card',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'card.billingAddress',
      fieldName: 'Billing Address',
      fieldSection: 'card',
      level: SensitivityLevel.sensitive,
    ),

    // Tax ID Section
    FieldSensitivity(
      fieldId: 'taxId.title',
      fieldName: 'Title',
      fieldSection: 'taxId',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'taxId.taxIdNumber',
      fieldName: 'Tax ID Number',
      fieldSection: 'taxId',
      level: SensitivityLevel.critical,
    ),
    FieldSensitivity(
      fieldId: 'taxId.taxIdType',
      fieldName: 'Tax ID Type',
      fieldSection: 'taxId',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'taxId.issuingAuthority',
      fieldName: 'Issuing Authority',
      fieldSection: 'taxId',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'taxId.country',
      fieldName: 'Country',
      fieldSection: 'taxId',
      level: SensitivityLevel.public,
    ),

    // Education Section
    FieldSensitivity(
      fieldId: 'education.institution',
      fieldName: 'Institution',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.degree',
      fieldName: 'Degree',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.degreeCustom',
      fieldName: 'Custom Degree',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.field',
      fieldName: 'Field of Study',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.fieldOfStudy',
      fieldName: 'Field of Study',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.isCurrent',
      fieldName: 'Currently Enrolled',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.gpa',
      fieldName: 'GPA',
      fieldSection: 'education',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'education.startDate',
      fieldName: 'Start Date',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'education.endDate',
      fieldName: 'End Date',
      fieldSection: 'education',
      level: SensitivityLevel.public,
    ),

    // Employment Section
    FieldSensitivity(
      fieldId: 'employment.company',
      fieldName: 'Company',
      fieldSection: 'employment',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'employment.position',
      fieldName: 'Position',
      fieldSection: 'employment',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'employment.responsibilities',
      fieldName: 'Responsibilities',
      fieldSection: 'employment',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'employment.workAddress',
      fieldName: 'Work Address',
      fieldSection: 'employment',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'employment.supervisorName',
      fieldName: 'Supervisor Name',
      fieldSection: 'employment',
      level: SensitivityLevel.internal,
    ),
    FieldSensitivity(
      fieldId: 'employment.monthlySalary',
      fieldName: 'Monthly Salary',
      fieldSection: 'employment',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'employment.startDate',
      fieldName: 'Start Date',
      fieldSection: 'employment',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'employment.endDate',
      fieldName: 'End Date',
      fieldSection: 'employment',
      level: SensitivityLevel.public,
    ),

    // Skills Section
    FieldSensitivity(
      fieldId: 'skill.name',
      fieldName: 'Skill Name',
      fieldSection: 'skill',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'skill.level',
      fieldName: 'Proficiency Level',
      fieldSection: 'skill',
      level: SensitivityLevel.public,
    ),

    // Language Section
    FieldSensitivity(
      fieldId: 'language.name',
      fieldName: 'Language',
      fieldSection: 'language',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'language.proficiency',
      fieldName: 'Proficiency Level',
      fieldSection: 'language',
      level: SensitivityLevel.public,
    ),

    // Award Section
    FieldSensitivity(
      fieldId: 'award.title',
      fieldName: 'Title',
      fieldSection: 'award',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'award.issuer',
      fieldName: 'Issuer',
      fieldSection: 'award',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'award.date',
      fieldName: 'Date',
      fieldSection: 'award',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'award.description',
      fieldName: 'Description',
      fieldSection: 'award',
      level: SensitivityLevel.sensitive,
    ),
  ];

  static bool isFieldRestricted(String fieldId) {
    try {
      return defaultFields.firstWhere((f) => f.fieldId == fieldId).level ==
          SensitivityLevel.critical;
    } catch (_) {
      return false;
    }
  }

  static List<FieldSensitivity> getFieldsBySection(String section) {
    return defaultFields.where((f) => f.fieldSection == section).toList();
  }

  static List<String> get allSections => [
        'identity',
        'contact',
        'idCard',
        'address',
        'passport',
        'visa',
        'travel',
        'bankAccount',
        'card',
        'taxId',
        'education',
        'employment',
        'skill',
        'language',
        'award',
      ];

  static String getSectionDisplayName(String section) {
    const names = {
      'identity': 'Identity',
      'contact': 'Contact',
      'idCard': 'ID Card',
      'address': 'Address',
      'passport': 'Passport',
      'visa': 'Visa',
      'travel': 'Travel',
      'bankAccount': 'Bank Account',
      'card': 'Card',
      'taxId': 'Tax ID',
      'education': 'Education',
      'employment': 'Employment',
      'skill': 'Skills',
      'language': 'Language',
      'award': 'Award',
    };
    return names[section] ?? section;
  }

  /// Check if a field ID exists in the registry
  static bool isValidFieldId(String fieldId) {
    return defaultFields.any((f) => f.fieldId == fieldId);
  }
}

/// Dynamic field registry populated by forms at runtime.
/// This is the Single Source of Truth for field sensitivity definitions.
class FormFieldRegistry {
  static final Map<String, FieldSensitivity> _fields = {};

  /// Register a single field. Idempotent - calling twice replaces.
  static void register(FieldSensitivity field) {
    _fields[field.fieldId] = field;
  }

  /// Register multiple fields at once
  static void registerAll(List<FieldSensitivity> fields) {
    for (final field in fields) {
      register(field);
    }
  }

  /// Get all fields - merges FormFieldRegistry with FieldRegistry defaults.
  /// FormFieldRegistry fields override FieldRegistry defaults.
  static List<FieldSensitivity> getAllFields() {
    // Start with FieldRegistry defaults
    final Map<String, FieldSensitivity> merged = {
      for (final f in FieldRegistry.defaultFields) f.fieldId: f,
    };
    // Override with registered fields
    merged.addAll(_fields);
    // Sort by section then name
    final list = merged.values.toList()
      ..sort((a, b) {
        final sectionCompare = a.fieldSection.compareTo(b.fieldSection);
        if (sectionCompare != 0) return sectionCompare;
        return a.fieldName.compareTo(b.fieldName);
      });
    return list;
  }

  /// Get a single field by fieldId, or null if not found
  static FieldSensitivity? getField(String fieldId) {
    return _fields[fieldId] ??
        firstWhereOrNull(
            FieldRegistry.defaultFields, (f) => f.fieldId == fieldId);
  }

  /// Clear all registered fields (for testing)
  static void reset() {
    _fields.clear();
  }

  /// Check if a field is registered in FormFieldRegistry (not legacy)
  static bool isRegistered(String fieldId) => _fields.containsKey(fieldId);
}

// =============================================================================
// NEW: Reactive Field Registry (ADR-013)
// =============================================================================

/// Reactive field registry notifier using StateNotifier.
/// Replaces static FormFieldRegistry for declarative, data-flow based updates.
class FormFieldRegistryNotifier extends StateNotifier<Map<String, FieldSensitivity>> {
  FormFieldRegistryNotifier() : super({});

  /// Register a single field. Idempotent - calling twice replaces.
  void register(FieldSensitivity field) {
    debugPrint('[FormFieldRegistry] Registering field: ${field.fieldId}');
    state = {...state, field.fieldId: field};
  }

  /// Register multiple fields at once
  void registerAll(List<FieldSensitivity> fields) {
    debugPrint('[FormFieldRegistry] Registering ${fields.length} fields: '
        '${fields.map((f) => f.fieldId).join(', ')}');
    state = {...state, for (var f in fields) f.fieldId: f};
  }

  /// Get a single field by fieldId, or null if not found
  FieldSensitivity? getField(String fieldId) => state[fieldId];

  /// Get all fields as a sorted list (deduplicated)
  List<FieldSensitivity> getAllFields() {
    final deduped = state.values.toSet().toList();
    deduped.sort((a, b) {
      final sec = a.fieldSection.compareTo(b.fieldSection);
      return sec != 0 ? sec : a.fieldName.compareTo(b.fieldName);
    });
    return deduped;
  }

  /// Clear all registered fields (for testing or app lock)
  void reset() {
    debugPrint('[FormFieldRegistry] Reset called');
    state = {};
  }
}

/// Provider for reactive field registry.
/// Forms register fields via this provider, settings page watches it.
final formFieldRegistryProvider =
    StateNotifierProvider<FormFieldRegistryNotifier, Map<String, FieldSensitivity>>((ref) {
  return FormFieldRegistryNotifier();
});

/// OPTIMIZED: Effective sensitivity level for a specific field.
/// Uses select() to narrow watch scope - only rebuilds when THIS fieldId changes.
final effectiveSensitivityProvider =
    Provider.family<SensitivityLevel, String>((ref, fieldId) {
  // Only watch this specific fieldId's registry entry
  final fieldDef = ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
  // Only watch this specific fieldId's user override
  final userOverride = ref.watch(
    accountStyleProvider.select((s) => s.fieldSettings[fieldId]),
  );
  // Watch revealed fields set
  final revealedFields = ref.watch(
    accountStyleProvider.select((s) => s.revealedFields),
  );

  // 1. Temporary reveal
  if (revealedFields.contains(fieldId)) {
    return SensitivityLevel.public;
  }

  // 2. User override
  if (userOverride != null) {
    return userOverride;
  }

  // 3. Registry default
  if (fieldDef != null) {
    return fieldDef.level;
  }

  // 4. Legacy FieldRegistry fallback
  final legacyField = firstWhereOrNull(
    FieldRegistry.defaultFields,
    (f) => f.fieldId == fieldId,
  );
  if (legacyField != null) {
    return legacyField.level;
  }

  // 5. Fallback to public
  return SensitivityLevel.public;
});

/// Provider for field metadata (name, section, etc.) for settings page display.
final fieldMetadataProvider =
    Provider.family<FieldSensitivity?, String>((ref, fieldId) {
  return ref.watch(
    formFieldRegistryProvider.select((s) => s[fieldId]),
  );
});

/// Field ID constants to prevent typos
class FieldIds {
  FieldIds._();

  // Identity
  static const String dateOfBirth = 'identity.dateOfBirth';
  static const String nationality = 'identity.nationality';

  // Contact
  static const String email = 'contact.email';
  static const String phone = 'contact.phone';
  static const String mobile = 'contact.mobile';
  static const String address = 'contact.address';

  // ID Card
  static const String idCardNumber = 'idCard.number';
  static const String idCardHolderName = 'idCard.holderName';
  static const String idCardIssueDate = 'idCard.issueDate';
  static const String idCardExpiryDate = 'idCard.expiryDate';
  static const String idCardCountry = 'idCard.country';

  // Address
  static const String street = 'address.street';
  static const String city = 'address.city';
  static const String postalCode = 'address.postalCode';
  static const String country = 'address.country';

  // Passport
  static const String passportNumber = 'passport.number';
  static const String passportCountry = 'passport.country';
  static const String passportExpiryDate = 'passport.expiryDate';
  static const String passportHolderName = 'passport.holderName';

  // Visa
  static const String visaNumber = 'visa.number';
  static const String visaCountry = 'visa.country';
  static const String visaType = 'visa.visaType';
  static const String visaExpiryDate = 'visa.expiryDate';

  // Bank Account
  static const String accountNumber = 'bankAccount.accountNumber';
  static const String bankName = 'bankAccount.bankName';
  static const String swiftBic = 'bankAccount.swiftBic';

  // Card
  static const String cardNumber = 'card.cardNumber';
  static const String cardExpiryDate = 'card.expiryDate';
  static const String cardHolderName = 'card.holderName';

  // Education
  static const String gpa = 'education.gpa';

  // Employment
  static const String workAddress = 'employment.workAddress';
  static const String supervisorName = 'employment.supervisorName';
  static const String monthlySalary = 'employment.monthlySalary';
}
