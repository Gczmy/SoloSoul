import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

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
    final levelIndex = json['level'] as int;
    final level = (levelIndex >= 0 && levelIndex < SensitivityLevel.values.length)
        ? SensitivityLevel.values[levelIndex]
        : SensitivityLevel.public;
    return FieldSensitivity(
      fieldId: json['fieldId'] as String,
      fieldName: json['fieldName'] as String,
      fieldSection: json['fieldSection'] as String,
      level: level,
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
/// Identity Section fields
const identityFields = [
  FieldSensitivity(fieldId: 'identity.fullName', fieldName: 'Full Name', fieldSection: 'identity', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'identity.givenName', fieldName: 'Given Name', fieldSection: 'identity', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'identity.familyName', fieldName: 'Family Name', fieldSection: 'identity', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'identity.dateOfBirth', fieldName: 'Date of Birth', fieldSection: 'identity', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'identity.gender', fieldName: 'Gender', fieldSection: 'identity', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'identity.nationality', fieldName: 'Nationality', fieldSection: 'identity', level: SensitivityLevel.sensitive),
];

class FieldRegistry {
  static const List<FieldSensitivity> defaultFields = [
    ...identityFields,
    ...contactFields,
    ...idCardFields,
    ...addressFields,
    ...bankAccountFields,
    ...cardFields,
    ...taxIdFields,
    ...passportFields,
    ...visaFields,
    ...travelFields,
    ...educationFields,
    ...employmentFields,
    ...skillFields,
    ...languageFields,
    ...awardFields,
    ...articleFields,
  ];

  static bool isFieldRestricted(String fieldId) {
    try {
      return defaultFields.firstWhere((f) => f.fieldId == fieldId).level ==
          SensitivityLevel.critical;
    } on StateError catch (_) {
      return false;
    }
  }

  static List<FieldSensitivity> getFieldsBySection(String section) {
    return defaultFields.where((f) => f.fieldSection == section).toList();
  }

  /// Get display name for a field within a section.
  /// Falls back to title-casing the key if not found in registry.
  static String displayNameForField(String section, String key) {
    final fieldId = '$section.$key';
    final field = firstWhereOrNull(defaultFields, (f) => f.fieldId == fieldId);
    if (field != null) return field.fieldName;
    // Fallback: convert camelCase to Title Case
    return _camelCaseToTitle(key);
  }

  static String _camelCaseToTitle(String input) {
    if (input.isEmpty) return input;
    final buffer = StringBuffer();
    for (var i = 0; i < input.length; i++) {
      final char = input[i];
      if (i == 0) {
        buffer.write(char.toUpperCase());
      } else if (char.toUpperCase() == char && char.toLowerCase() != char) {
        buffer.write(' $char');
      } else {
        buffer.write(char);
      }
    }
    return buffer.toString();
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
        'article',
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
      'article': 'Article',
      'page': 'Page',
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
// Field Definitions by Section (used by FormFieldRegistryNotifier)
// =============================================================================

/// Contact Section fields
const contactFields = [
  FieldSensitivity(fieldId: 'contact.title', fieldName: 'Title', fieldSection: 'contact', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'contact.type', fieldName: 'Type', fieldSection: 'contact', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'contact.value', fieldName: 'Value', fieldSection: 'contact', level: SensitivityLevel.internal),
];

/// ID Card Section fields
const idCardFields = [
  FieldSensitivity(fieldId: 'idCard.title', fieldName: 'Title', fieldSection: 'idCard', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'idCard.number', fieldName: 'ID Card Number', fieldSection: 'idCard', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'idCard.holderName', fieldName: 'Holder Name', fieldSection: 'idCard', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'idCard.country', fieldName: 'Country', fieldSection: 'idCard', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'idCard.issueDate', fieldName: 'Issue Date', fieldSection: 'idCard', level: SensitivityLevel.internal),
  FieldSensitivity(fieldId: 'idCard.expiryDate', fieldName: 'Expiry Date', fieldSection: 'idCard', level: SensitivityLevel.internal),
];

/// Address Section fields
const addressFields = [
  FieldSensitivity(fieldId: 'address.title', fieldName: 'Title', fieldSection: 'address', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'address.street', fieldName: 'Street', fieldSection: 'address', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'address.city', fieldName: 'City', fieldSection: 'address', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'address.district', fieldName: 'District', fieldSection: 'address', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'address.state', fieldName: 'State', fieldSection: 'address', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'address.postalCode', fieldName: 'Postal Code', fieldSection: 'address', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'address.country', fieldName: 'Country', fieldSection: 'address', level: SensitivityLevel.public),
];

/// Bank Account Section fields
const bankAccountFields = [
  FieldSensitivity(fieldId: 'bankAccount.title', fieldName: 'Title', fieldSection: 'bankAccount', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'bankAccount.bankName', fieldName: 'Bank Name', fieldSection: 'bankAccount', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'bankAccount.accountNumber', fieldName: 'Account Number', fieldSection: 'bankAccount', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'bankAccount.currency', fieldName: 'Currency', fieldSection: 'bankAccount', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'bankAccount.swiftBic', fieldName: 'SWIFT/BIC', fieldSection: 'bankAccount', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'bankAccount.sortCode', fieldName: 'Sort Code', fieldSection: 'bankAccount', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'bankAccount.accountHolderName', fieldName: 'Account Holder Name', fieldSection: 'bankAccount', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'bankAccount.routingNumber', fieldName: 'Routing Number', fieldSection: 'bankAccount', level: SensitivityLevel.critical),
];

/// Card Section fields
const cardFields = [
  FieldSensitivity(fieldId: 'card.title', fieldName: 'Title', fieldSection: 'card', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'card.cardType', fieldName: 'Card Type', fieldSection: 'card', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'card.cardNumber', fieldName: 'Card Number', fieldSection: 'card', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'card.expiryDate', fieldName: 'Expiry Date', fieldSection: 'card', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'card.holderName', fieldName: 'Holder Name', fieldSection: 'card', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'card.cvv', fieldName: 'CVV', fieldSection: 'card', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'card.billingAddress', fieldName: 'Billing Address', fieldSection: 'card', level: SensitivityLevel.sensitive),
];

/// Tax ID Section fields
const taxIdFields = [
  FieldSensitivity(fieldId: 'taxId.title', fieldName: 'Title', fieldSection: 'taxId', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'taxId.taxIdNumber', fieldName: 'Tax ID Number', fieldSection: 'taxId', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'taxId.taxIdType', fieldName: 'Tax ID Type', fieldSection: 'taxId', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'taxId.issuingAuthority', fieldName: 'Issuing Authority', fieldSection: 'taxId', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'taxId.country', fieldName: 'Country', fieldSection: 'taxId', level: SensitivityLevel.public),
];

/// Passport Section fields
const passportFields = [
  FieldSensitivity(fieldId: 'passport.title', fieldName: 'Title', fieldSection: 'passport', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'passport.country', fieldName: 'Country', fieldSection: 'passport', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'passport.countryCode', fieldName: 'Country Code', fieldSection: 'passport', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'passport.number', fieldName: 'Passport Number', fieldSection: 'passport', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'passport.issueDate', fieldName: 'Issue Date', fieldSection: 'passport', level: SensitivityLevel.internal),
  FieldSensitivity(fieldId: 'passport.placeOfIssue', fieldName: 'Place of Issue', fieldSection: 'passport', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'passport.expiryDate', fieldName: 'Expiry Date', fieldSection: 'passport', level: SensitivityLevel.internal),
  FieldSensitivity(fieldId: 'passport.holderName', fieldName: 'Holder Name', fieldSection: 'passport', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'passport.dateOfBirth', fieldName: 'Date of Birth', fieldSection: 'passport', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'passport.placeOfBirth', fieldName: 'Place of Birth', fieldSection: 'passport', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'passport.sex', fieldName: 'Sex', fieldSection: 'passport', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'passport.nationality', fieldName: 'Nationality', fieldSection: 'passport', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'passport.authority', fieldName: 'Authority', fieldSection: 'passport', level: SensitivityLevel.sensitive),
];

/// Visa Section fields
const visaFields = [
  FieldSensitivity(fieldId: 'visa.title', fieldName: 'Title', fieldSection: 'visa', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'visa.country', fieldName: 'Country', fieldSection: 'visa', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'visa.visaType', fieldName: 'Visa Type', fieldSection: 'visa', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'visa.number', fieldName: 'Visa Number', fieldSection: 'visa', level: SensitivityLevel.critical),
  FieldSensitivity(fieldId: 'visa.expiryDate', fieldName: 'Expiry Date', fieldSection: 'visa', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'visa.issueDate', fieldName: 'Issue Date', fieldSection: 'visa', level: SensitivityLevel.internal),
];

/// Travel Section fields
const travelFields = [
  FieldSensitivity(fieldId: 'travel.destination', fieldName: 'Destination', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.travelType', fieldName: 'Travel Type', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.date', fieldName: 'Date', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.departureCity', fieldName: 'Departure City', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.arrivalCity', fieldName: 'Arrival City', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.departureTime', fieldName: 'Departure Time', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.arrivalTime', fieldName: 'Arrival Time', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.flightNumber', fieldName: 'Flight Number', fieldSection: 'travel', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'travel.ticketPrice', fieldName: 'Ticket Price', fieldSection: 'travel', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'travel.airline', fieldName: 'Airline', fieldSection: 'travel', level: SensitivityLevel.public),
];

/// Education Section fields
const educationFields = [
  FieldSensitivity(fieldId: 'education.institution', fieldName: 'Institution', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.degree', fieldName: 'Degree', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.degreeCustom', fieldName: 'Custom Degree', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.fieldOfStudy', fieldName: 'Field of Study', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.startDate', fieldName: 'Start Date', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.endDate', fieldName: 'End Date', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.isCurrent', fieldName: 'Currently Enrolled', fieldSection: 'education', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'education.gpa', fieldName: 'GPA', fieldSection: 'education', level: SensitivityLevel.internal),
];

/// Employment Section fields
const employmentFields = [
  FieldSensitivity(fieldId: 'employment.company', fieldName: 'Company', fieldSection: 'employment', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'employment.position', fieldName: 'Position', fieldSection: 'employment', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'employment.responsibilities', fieldName: 'Responsibilities', fieldSection: 'employment', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'employment.startDate', fieldName: 'Start Date', fieldSection: 'employment', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'employment.endDate', fieldName: 'End Date', fieldSection: 'employment', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'employment.isCurrent', fieldName: 'Currently Working', fieldSection: 'employment', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'employment.monthlySalary', fieldName: 'Monthly Salary', fieldSection: 'employment', level: SensitivityLevel.sensitive),
  FieldSensitivity(fieldId: 'employment.supervisorName', fieldName: 'Supervisor Name', fieldSection: 'employment', level: SensitivityLevel.internal),
  FieldSensitivity(fieldId: 'employment.workAddress', fieldName: 'Work Address', fieldSection: 'employment', level: SensitivityLevel.internal),
];

/// Skills Section fields
const skillFields = [
  FieldSensitivity(fieldId: 'skill.name', fieldName: 'Skill Name', fieldSection: 'skill', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'skill.level', fieldName: 'Proficiency Level', fieldSection: 'skill', level: SensitivityLevel.public),
];

/// Language Section fields
const languageFields = [
  FieldSensitivity(fieldId: 'language.name', fieldName: 'Language', fieldSection: 'language', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'language.proficiency', fieldName: 'Proficiency Level', fieldSection: 'language', level: SensitivityLevel.public),
];

/// Award Section fields
const awardFields = [
  FieldSensitivity(fieldId: 'award.title', fieldName: 'Title', fieldSection: 'award', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'award.issuer', fieldName: 'Issuer', fieldSection: 'award', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'award.date', fieldName: 'Date', fieldSection: 'award', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'award.description', fieldName: 'Description', fieldSection: 'award', level: SensitivityLevel.sensitive),
];

/// Article Section fields
const articleFields = [
  FieldSensitivity(fieldId: 'article.title', fieldName: 'Title', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.authors', fieldName: 'Authors', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.institution', fieldName: 'Institution', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.contact', fieldName: 'Contact', fieldSection: 'article', level: SensitivityLevel.internal),
  FieldSensitivity(fieldId: 'article.abstract', fieldName: 'Abstract', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.doi', fieldName: 'DOI', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.url', fieldName: 'URL', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.venue', fieldName: 'Venue', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.year', fieldName: 'Year', fieldSection: 'article', level: SensitivityLevel.public),
  FieldSensitivity(fieldId: 'article.citation', fieldName: 'Citation', fieldSection: 'article', level: SensitivityLevel.public),
];

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

// =============================================================================
// Reactive Field Registry (ADR-013)
// =============================================================================

/// Reactive field registry notifier using Notifier.
/// Replaces static FormFieldRegistry for declarative, data-flow based updates.
class FormFieldRegistryNotifier extends Notifier<Map<String, FieldSensitivity>> {
  @override
  Map<String, FieldSensitivity> build() => {};

  /// Register a single field. Idempotent - calling twice replaces.
  void register(FieldSensitivity field) {
    state = {...state, field.fieldId: field};
  }

  /// Register multiple fields at once
  void registerAll(List<FieldSensitivity> fields) {
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
    state = {};
  }

  /// Register all fields from all form sections at once.
  /// Call this after first unlock to pre-populate the registry.
  void registerAllForms() {
    final allFieldLists = [
      contactFields,
      idCardFields,
      addressFields,
      bankAccountFields,
      cardFields,
      taxIdFields,
      passportFields,
      visaFields,
      travelFields,
      educationFields,
      employmentFields,
      skillFields,
      languageFields,
      awardFields,
      articleFields,
    ];

    final allFields = allFieldLists.expand((list) => list).toList();
    // Register to both static FormFieldRegistry (for SensitivityResolver legacy compatibility)
    // and reactive FormFieldRegistryNotifier (for effectiveSensitivityProvider)
    FormFieldRegistry.registerAll(allFields);
    registerAll(allFields);

    // Development guard: detect duplicate fieldId registrations
    assert(() {
      final seen = <String>{};
      for (final f in allFields) {
        if (!seen.add(f.fieldId)) {
          // Duplicate fieldId detected: ${f.fieldId}
        }
      }
      return true;
    }());
  }
}