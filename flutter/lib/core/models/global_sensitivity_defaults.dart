import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

/// Defines global default sensitivity levels for the application.
///
/// These defaults are used when no specific field or tag configuration
/// is available. They represent the baseline security posture for
/// the application.
class GlobalSensitivityDefaults {
  /// Default sensitivity level for fields with no specific configuration.
  ///
  /// This is the fallback level for any field that doesn't have:
  /// - An explicit field setting in SensitivityConfig.fieldSettings
  /// - A tag-based default from SensitivityConfig.tagDefaults
  final SensitivityLevel defaultFieldLevel;

  /// Default sensitivity level for newly created fields.
  ///
  /// When a new field is added without an explicit sensitivity level,
  /// this level is assigned by default.
  final SensitivityLevel newFieldDefault;

  /// Default sensitivity level for fields without tags.
  ///
  /// When a field has no tags and no explicit setting, this level is used.
  final SensitivityLevel untaggedFieldDefault;

  /// Default sensitivity level for fields marked with the 'work' tag.
  final SensitivityLevel workTagDefault;

  /// Default sensitivity level for fields marked with the 'personal' tag.
  final SensitivityLevel personalTagDefault;

  /// Default sensitivity level for fields marked with the 'financial' tag.
  final SensitivityLevel financialTagDefault;

  /// Default sensitivity level for fields marked with the 'health' tag.
  final SensitivityLevel healthTagDefault;

  const GlobalSensitivityDefaults({
    this.defaultFieldLevel = SensitivityLevel.internal,
    this.newFieldDefault = SensitivityLevel.internal,
    this.untaggedFieldDefault = SensitivityLevel.public,
    this.workTagDefault = SensitivityLevel.internal,
    this.personalTagDefault = SensitivityLevel.sensitive,
    this.financialTagDefault = SensitivityLevel.critical,
    this.healthTagDefault = SensitivityLevel.critical,
  });

  /// Default global settings for new accounts.
  factory GlobalSensitivityDefaults.standard() {
    return const GlobalSensitivityDefaults();
  }

  /// Strict settings for high-security environments.
  factory GlobalSensitivityDefaults.strict() {
    return const GlobalSensitivityDefaults(
      defaultFieldLevel: SensitivityLevel.sensitive,
      newFieldDefault: SensitivityLevel.sensitive,
      untaggedFieldDefault: SensitivityLevel.internal,
      workTagDefault: SensitivityLevel.sensitive,
      personalTagDefault: SensitivityLevel.critical,
      financialTagDefault: SensitivityLevel.critical,
      healthTagDefault: SensitivityLevel.critical,
    );
  }

  /// Permissive settings for testing or development.
  factory GlobalSensitivityDefaults.permissive() {
    return const GlobalSensitivityDefaults(
      defaultFieldLevel: SensitivityLevel.public,
      newFieldDefault: SensitivityLevel.public,
      untaggedFieldDefault: SensitivityLevel.public,
      workTagDefault: SensitivityLevel.public,
      personalTagDefault: SensitivityLevel.internal,
      financialTagDefault: SensitivityLevel.sensitive,
      healthTagDefault: SensitivityLevel.sensitive,
    );
  }

  /// Returns the effective sensitivity level for a field.
  ///
  /// This considers both the global defaults and optional per-account overrides.
  SensitivityLevel getEffectiveLevel({
    SensitivityLevel? explicitLevel,
    SensitivityLevel? tagBasedLevel,
    SensitivityLevel? accountDefault,
  }) {
    // Explicit field setting takes highest priority
    if (explicitLevel != null) return explicitLevel;

    // Tag-based level is second priority
    if (tagBasedLevel != null) return tagBasedLevel;

    // Account-specific default is third priority
    if (accountDefault != null) return accountDefault;

    // Fall back to global default
    return defaultFieldLevel;
  }

  GlobalSensitivityDefaults copyWith({
    SensitivityLevel? defaultFieldLevel,
    SensitivityLevel? newFieldDefault,
    SensitivityLevel? untaggedFieldDefault,
    SensitivityLevel? workTagDefault,
    SensitivityLevel? personalTagDefault,
    SensitivityLevel? financialTagDefault,
    SensitivityLevel? healthTagDefault,
  }) {
    return GlobalSensitivityDefaults(
      defaultFieldLevel: defaultFieldLevel ?? this.defaultFieldLevel,
      newFieldDefault: newFieldDefault ?? this.newFieldDefault,
      untaggedFieldDefault: untaggedFieldDefault ?? this.untaggedFieldDefault,
      workTagDefault: workTagDefault ?? this.workTagDefault,
      personalTagDefault: personalTagDefault ?? this.personalTagDefault,
      financialTagDefault: financialTagDefault ?? this.financialTagDefault,
      healthTagDefault: healthTagDefault ?? this.healthTagDefault,
    );
  }

  factory GlobalSensitivityDefaults.fromJson(Map<String, dynamic> json) {
    SensitivityLevel levelFromString(String? name, SensitivityLevel fallback) {
      if (name == null) return fallback;
      return SensitivityLevel.values.firstWhere(
        (e) => e.name == name,
        orElse: () => fallback,
      );
    }

    return GlobalSensitivityDefaults(
      defaultFieldLevel: levelFromString(
        json['default_field_level'] as String?,
        SensitivityLevel.internal,
      ),
      newFieldDefault: levelFromString(
        json['new_field_default'] as String?,
        SensitivityLevel.internal,
      ),
      untaggedFieldDefault: levelFromString(
        json['untagged_field_default'] as String?,
        SensitivityLevel.public,
      ),
      workTagDefault: levelFromString(
        json['work_tag_default'] as String?,
        SensitivityLevel.internal,
      ),
      personalTagDefault: levelFromString(
        json['personal_tag_default'] as String?,
        SensitivityLevel.sensitive,
      ),
      financialTagDefault: levelFromString(
        json['financial_tag_default'] as String?,
        SensitivityLevel.critical,
      ),
      healthTagDefault: levelFromString(
        json['health_tag_default'] as String?,
        SensitivityLevel.critical,
      ),
    );
  }

  Map<String, dynamic> toJson() => {
        'default_field_level': defaultFieldLevel.name,
        'new_field_default': newFieldDefault.name,
        'untagged_field_default': untaggedFieldDefault.name,
        'work_tag_default': workTagDefault.name,
        'personal_tag_default': personalTagDefault.name,
        'financial_tag_default': financialTagDefault.name,
        'health_tag_default': healthTagDefault.name,
      };
}