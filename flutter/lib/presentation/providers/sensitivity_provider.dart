import 'dart:convert';
import 'package:collection/collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// Re-export so consumers can import from one place
export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart' show SensitivityLevel, SensitivityLevelExtension;

/// Sensitivity display mode
enum SensitivityDisplayMode {
  showAll,
  hidePrivate,
  hideAll,
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

    // Contact Section - personal contact info
    FieldSensitivity(
      fieldId: 'contact.email',
      fieldName: 'Email',
      fieldSection: 'contact',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'contact.phone',
      fieldName: 'Phone',
      fieldSection: 'contact',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'contact.mobile',
      fieldName: 'Mobile',
      fieldSection: 'contact',
      level: SensitivityLevel.sensitive,
    ),
    FieldSensitivity(
      fieldId: 'contact.address',
      fieldName: 'Address',
      fieldSection: 'contact',
      level: SensitivityLevel.sensitive,
    ),

    // ID Card Section
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
      fieldId: 'address.state',
      fieldName: 'State/Province',
      fieldSection: 'address',
      level: SensitivityLevel.public,
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
      fieldId: 'passport.issueDate',
      fieldName: 'Issue Date',
      fieldSection: 'passport',
      level: SensitivityLevel.internal,
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

    // Visa Section
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

    // Travel History
    FieldSensitivity(
      fieldId: 'travelHistory.destination',
      fieldName: 'Destination',
      fieldSection: 'travelHistory',
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
      fieldId: 'education.field',
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
      fieldId: 'skills.name',
      fieldName: 'Skill Name',
      fieldSection: 'skills',
      level: SensitivityLevel.public,
    ),
    FieldSensitivity(
      fieldId: 'skills.proficiency',
      fieldName: 'Proficiency Level',
      fieldSection: 'skills',
      level: SensitivityLevel.public,
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
        'travelHistory',
        'bankAccount',
        'card',
        'education',
        'employment',
        'skills',
      ];

  static String getSectionDisplayName(String section) {
    const names = {
      'identity': 'Identity',
      'contact': 'Contact',
      'idCard': 'ID Card',
      'address': 'Address',
      'passport': 'Passport',
      'visa': 'Visa',
      'travelHistory': 'Travel History',
      'bankAccount': 'Bank Account',
      'card': 'Card',
      'education': 'Education',
      'employment': 'Employment',
      'skills': 'Skills',
    };
    return names[section] ?? section;
  }

  /// Check if a field ID exists in the registry
  static bool isValidFieldId(String fieldId) {
    return defaultFields.any((f) => f.fieldId == fieldId);
  }
}

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

/// Sensitivity settings state
class SensitivitySettings {
  final SensitivityDisplayMode displayMode;
  final Set<String> revealedFields;
  final List<FieldSensitivity> fieldSettings;

  const SensitivitySettings({
    this.displayMode = SensitivityDisplayMode.hidePrivate,
    this.revealedFields = const {},
    this.fieldSettings = const [],
  });

  SensitivitySettings copyWith({
    SensitivityDisplayMode? displayMode,
    Set<String>? revealedFields,
    List<FieldSensitivity>? fieldSettings,
  }) {
    return SensitivitySettings(
      displayMode: displayMode ?? this.displayMode,
      revealedFields: revealedFields ?? this.revealedFields,
      fieldSettings: fieldSettings ?? this.fieldSettings,
    );
  }

  bool isFieldRevealed(String fieldId) => revealedFields.contains(fieldId);

  SensitivitySettings revealField(String fieldId) {
    return copyWith(revealedFields: {...revealedFields, fieldId});
  }

  SensitivitySettings hideField(String fieldId) {
    return copyWith(
      revealedFields: revealedFields.where((id) => id != fieldId).toSet(),
    );
  }

  SensitivityLevel? getFieldLevel(String fieldId) {
    return fieldSettings
        .firstWhereOrNull((f) => f.fieldId == fieldId)
        ?.level;
  }

  List<FieldSensitivity> getFieldsByLevel(SensitivityLevel level) {
    return fieldSettings.where((f) => f.level == level).toList();
  }

  Map<String, dynamic> toJson() => {
        'displayMode': displayMode.index,
        'revealedFields': revealedFields.toList(),
        'fieldSettings': fieldSettings.map((f) => f.toJson()).toList(),
      };

  factory SensitivitySettings.fromJson(Map<String, dynamic> json) {
    return SensitivitySettings(
      displayMode: SensitivityDisplayMode.values[json['displayMode'] as int? ?? 0],
      revealedFields: Set<String>.from(json['revealedFields'] as List? ?? []),
      fieldSettings: (json['fieldSettings'] as List?)
              ?.map((f) => FieldSensitivity.fromJson(f as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }

  String toJsonString() => jsonEncode(toJson());

  factory SensitivitySettings.fromJsonString(String jsonString) {
    try {
      return SensitivitySettings.fromJson(
          jsonDecode(jsonString) as Map<String, dynamic>);
    } catch (_) {
      return const SensitivitySettings();
    }
  }
}

/// Sensitivity settings notifier
class SensitivitySettingsNotifier extends StateNotifier<SensitivitySettings> {
  SensitivitySettingsNotifier() : super(const SensitivitySettings()) {
    _initializeDefaults();
  }

  void _initializeDefaults() {
    // Initialize with default field settings
    state = state.copyWith(fieldSettings: FieldRegistry.defaultFields);
  }

  void setDisplayMode(SensitivityDisplayMode mode) {
    state = state.copyWith(displayMode: mode);
  }

  void toggleField(String fieldId) {
    if (state.isFieldRevealed(fieldId)) {
      state = state.hideField(fieldId);
    } else {
      state = state.revealField(fieldId);
    }
  }

  void revealField(String fieldId) {
    state = state.revealField(fieldId);
  }

  void hideField(String fieldId) {
    state = state.hideField(fieldId);
  }

  void hideAllPrivate() {
    state = state.copyWith(revealedFields: {});
  }

  /// Upgrade field to higher sensitivity (public -> internal -> sensitive -> critical)
  void upgradeField(String fieldId) {
    _moveField(fieldId, 1);
  }

  /// Downgrade field to lower sensitivity (critical -> sensitive -> internal -> public)
  void downgradeField(String fieldId) {
    _moveField(fieldId, -1);
  }

  void _moveField(String fieldId, int direction) {
    final fieldIndex = state.fieldSettings.indexWhere((f) => f.fieldId == fieldId);
    if (fieldIndex == -1) return;

    final field = state.fieldSettings[fieldIndex];
    final newLevel = SensitivityLevel.values[field.level.index + direction];

    if (newLevel.index < 0 || newLevel.index > 3) return;

    final updatedFields = List<FieldSensitivity>.from(state.fieldSettings);
    updatedFields[fieldIndex] = field.copyWith(level: newLevel);

    state = state.copyWith(fieldSettings: updatedFields);
  }

  /// Set a field's sensitivity level directly
  void setFieldLevel(String fieldId, SensitivityLevel level) {
    final fieldIndex = state.fieldSettings.indexWhere((f) => f.fieldId == fieldId);
    if (fieldIndex == -1) return;

    final field = state.fieldSettings[fieldIndex];
    final updatedFields = List<FieldSensitivity>.from(state.fieldSettings);
    updatedFields[fieldIndex] = field.copyWith(level: level);

    state = state.copyWith(fieldSettings: updatedFields);
  }

  /// Load settings from JSON string (for persistence)
  void loadFromJson(String jsonString) {
    try {
      final loaded = SensitivitySettings.fromJsonString(jsonString);
      // Ensure all default fields are present
      final loadedFieldIds = loaded.fieldSettings.map((f) => f.fieldId).toSet();
      final missingFields = FieldRegistry.defaultFields
          .where((f) => !loadedFieldIds.contains(f.fieldId))
          .toList();

      state = loaded.copyWith(
        fieldSettings: [...loaded.fieldSettings, ...missingFields],
      );
    } catch (_) {
      _initializeDefaults();
    }
  }

  /// Get current settings as JSON string (for persistence)
  String toJsonString() {
    return state.toJsonString();
  }

  /// Check if a field with given sensitivity should be visible
  bool shouldShowField(SensitivityLevel level) {
    switch (state.displayMode) {
      case SensitivityDisplayMode.showAll:
        return true;
      case SensitivityDisplayMode.hidePrivate:
        return level == SensitivityLevel.public;
      case SensitivityDisplayMode.hideAll:
        return false;
    }
  }

  /// Check if a specific field should be shown based on display mode
  bool shouldShowFieldById(String fieldId) {
    final level = state.getFieldLevel(fieldId);
    if (level == null) return true;
    return shouldShowField(level);
  }
}

/// Sensitivity settings provider
final sensitivitySettingsProvider =
    StateNotifierProvider<SensitivitySettingsNotifier, SensitivitySettings>((ref) {
  return SensitivitySettingsNotifier();
});

/// Provider family for getting effective sensitivity level of a field.
/// Returns: user override > FieldRegistry default > public fallback
final fieldLevelProvider = Provider.family<SensitivityLevel, String>((ref, fieldId) {
  final settings = ref.watch(sensitivitySettingsProvider);

  // Check user override first
  final override = settings.fieldSettings.firstWhereOrNull((f) => f.fieldId == fieldId);
  if (override != null) return override.level;

  // Fall back to FieldRegistry default
  return FieldRegistry.defaultFields
      .firstWhereOrNull((f) => f.fieldId == fieldId)
      ?.level ?? SensitivityLevel.public;
});
