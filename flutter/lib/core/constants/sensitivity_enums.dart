import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:json_annotation/json_annotation.dart';

/// Sensitivity levels for data classification.
///
/// Levels are ordered from least restrictive to most restrictive.
/// Higher levels require stricter handling (masking, restricted access, etc.).
enum SensitivityLevel {
  /// Data that can be freely shared and displayed.
  /// Examples: public profile fields, general preferences.
  @JsonValue('public')
  public,

  /// Data intended for internal use only.
  /// Not publicly visible but no special protection needed.
  /// Examples: internal notes, non-sensitive metadata.
  @JsonValue('internal')
  internal,

  /// Sensitive personal information requiring protection.
  /// Examples: email addresses, phone numbers, physical addresses.
  @JsonValue('sensitive')
  sensitive,

  /// Highly sensitive data requiring maximum protection.
  /// Examples: financial accounts, identity documents, master passwords.
  @JsonValue('critical')
  critical,
}

extension SensitivityLevelExtension on SensitivityLevel {
  /// Returns true if this level is at or above the given threshold.
  bool isAtLeast(SensitivityLevel threshold) {
    return index >= threshold.index;
  }

  /// Returns the numeric rank of this level (0-3).
  int get rank => index;

  /// Human-readable label for UI display.
  String get label {
    switch (this) {
      case SensitivityLevel.public:
        return 'Public';
      case SensitivityLevel.internal:
        return 'Internal';
      case SensitivityLevel.sensitive:
        return 'Sensitive';
      case SensitivityLevel.critical:
        return 'Critical';
    }
  }

  /// Description for help text and tooltips.
  String get description {
    switch (this) {
      case SensitivityLevel.public:
        return 'Freely visible and shareable';
      case SensitivityLevel.internal:
        return 'Internal use only, not publicly visible';
      case SensitivityLevel.sensitive:
        return 'Requires protection, may be masked';
      case SensitivityLevel.critical:
        return 'Maximum protection required, always masked';
    }
  }

  /// Localized human-readable label for UI display.
  String localizedLabel(AppLocalizations l10n) {
    return switch (this) {
      SensitivityLevel.public => l10n.sensitivityPublic,
      SensitivityLevel.internal => l10n.sensitivityInternal,
      SensitivityLevel.sensitive => l10n.sensitivitySensitive,
      SensitivityLevel.critical => l10n.sensitivityCritical,
    };
  }

  /// Localized description for help text and tooltips.
  String localizedDescription(AppLocalizations l10n) {
    return switch (this) {
      SensitivityLevel.public => l10n.sensitivityPublicDesc,
      SensitivityLevel.internal => l10n.sensitivityInternalDesc,
      SensitivityLevel.sensitive => l10n.sensitivitySensitiveDesc,
      SensitivityLevel.critical => l10n.sensitivityCriticalDesc,
    };
  }
}