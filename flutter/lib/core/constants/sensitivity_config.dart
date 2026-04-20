import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

/// Configuration mapping that defines sensitivity levels for fields and tags.
///
/// This is the core configuration structure for the configurable sensitivity system.
/// It maps field identifiers and tag identifiers to their sensitivity levels,
/// allowing fine-grained control over how different data is handled.
class SensitivityConfig {
  /// Map of field identifiers to their sensitivity levels.
  ///
  /// Key: field identifier (e.g., 'email', 'phone', 'address')
  /// Value: the sensitivity level for that field.
  ///
  /// Example:
  /// ```dart
  /// fieldSettings: {
  ///   'email': SensitivityLevel.sensitive,
  ///   'phone': SensitivityLevel.sensitive,
  ///   'name': SensitivityLevel.public,
  /// }
  /// ```
  final Map<String, SensitivityLevel> fieldSettings;

  /// Map of tag identifiers to their default sensitivity levels.
  ///
  /// Key: tag identifier (e.g., 'work', 'personal', 'financial')
  /// Value: the default sensitivity level for fields with that tag.
  ///
  /// Tags provide a secondary way to assign sensitivity - if a field has no
  /// explicit setting, its tags' defaults are used.
  ///
  /// Example:
  /// ```dart
  /// tagDefaults: {
  ///   'work': SensitivityLevel.internal,
  ///   'personal': SensitivityLevel.sensitive,
  ///   'financial': SensitivityLevel.critical,
  /// }
  /// ```
  final Map<String, SensitivityLevel> tagDefaults;

  const SensitivityConfig({
    required this.fieldSettings,
    required this.tagDefaults,
  });

  /// Creates an empty configuration with no field or tag mappings.
  const SensitivityConfig.empty()
      : fieldSettings = const {},
        tagDefaults = const {};

  /// Creates a configuration with preset field settings only.
  const SensitivityConfig.withFieldDefaults(Map<String, SensitivityLevel> fields)
      : fieldSettings = fields,
        tagDefaults = const {};

  /// Returns the sensitivity level for a specific field.
  ///
  /// First checks if the field has an explicit setting.
  /// If not, falls back to checking the field's tags.
  /// Returns `null` if no level can be determined (use global defaults).
  SensitivityLevel? getFieldLevel(String fieldId, {List<String> tags = const []}) {
    // First priority: explicit field setting
    final fieldLevel = fieldSettings[fieldId];
    if (fieldLevel != null) return fieldLevel;

    // Second priority: tag-based defaults
    for (final tag in tags) {
      final tagLevel = tagDefaults[tag];
      if (tagLevel != null) return tagLevel;
    }

    return null;
  }

  /// Returns true if this config has any field settings defined.
  bool get hasFieldSettings => fieldSettings.isNotEmpty;

  /// Returns true if this config has any tag defaults defined.
  bool get hasTagDefaults => tagDefaults.isNotEmpty;

  /// Returns true if this config has any settings at all.
  bool get isEmpty => fieldSettings.isEmpty && tagDefaults.isEmpty;

  SensitivityConfig copyWith({
    Map<String, SensitivityLevel>? fieldSettings,
    Map<String, SensitivityLevel>? tagDefaults,
  }) {
    return SensitivityConfig(
      fieldSettings: fieldSettings ?? this.fieldSettings,
      tagDefaults: tagDefaults ?? this.tagDefaults,
    );
  }

  factory SensitivityConfig.fromJson(Map<String, dynamic> json) {
    final fieldJson = json['field_settings'] as Map<String, dynamic>? ?? {};
    final fieldSettings = fieldJson.map(
      (k, v) => MapEntry(k, SensitivityLevel.values.firstWhere(
        (e) => e.name == v,
        orElse: () => SensitivityLevel.public,
      )),
    );

    final tagJson = json['tag_defaults'] as Map<String, dynamic>? ?? {};
    final tagDefaults = tagJson.map(
      (k, v) => MapEntry(k, SensitivityLevel.values.firstWhere(
        (e) => e.name == v,
        orElse: () => SensitivityLevel.public,
      )),
    );

    return SensitivityConfig(
      fieldSettings: fieldSettings,
      tagDefaults: tagDefaults,
    );
  }

  Map<String, dynamic> toJson() => {
        'field_settings': fieldSettings.map((k, v) => MapEntry(k, v.name)),
        'tag_defaults': tagDefaults.map((k, v) => MapEntry(k, v.name)),
      };
}