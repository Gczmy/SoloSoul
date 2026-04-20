import '../constants/sensitivity_config.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'global_sensitivity_defaults.dart';

/// Complete style settings for an account.
///
/// This aggregates the sensitivity configuration, global defaults,
/// and other style-related settings for a single account.
/// Each account can have its own customized sensitivity behavior.
class AccountStyleSettings {
  /// The unique identifier for this account.
  final String accountId;

  /// The display name for this account.
  final String accountName;

  /// The sensitivity configuration for this account.
  /// Defines field-level and tag-level sensitivity mappings.
  final SensitivityConfig sensitivityConfig;

  /// The global sensitivity defaults for this account.
  /// Provides fallback values when no specific configuration exists.
  final GlobalSensitivityDefaults globalDefaults;

  /// Whether biometric authentication is required to view sensitive data.
  final bool requireBiometricForSensitive;

  /// Whether to show sensitive data by default (before verification).
  final bool showSensitiveByDefault;

  /// Timeout in seconds before re-verification is required for sensitive data.
  final int verificationTimeoutSeconds;

  /// Custom field tags supported by this account.
  /// This allows accounts to define their own field categorization.
  final List<String> customTags;

  const AccountStyleSettings({
    required this.accountId,
    required this.accountName,
    required this.sensitivityConfig,
    required this.globalDefaults,
    this.requireBiometricForSensitive = false,
    this.showSensitiveByDefault = false,
    this.verificationTimeoutSeconds = 60,
    this.customTags = const [],
  });

  /// Creates default style settings for a new account.
  factory AccountStyleSettings.defaults({
    required String accountId,
    required String accountName,
  }) {
    return AccountStyleSettings(
      accountId: accountId,
      accountName: accountName,
      sensitivityConfig: const SensitivityConfig.empty(),
      globalDefaults: GlobalSensitivityDefaults.standard(),
      requireBiometricForSensitive: false,
      showSensitiveByDefault: false,
      verificationTimeoutSeconds: 60,
      customTags: const ['work', 'personal', 'financial', 'health'],
    );
  }

  /// Returns the effective sensitivity level for a field in this account.
  ///
  /// This combines the account's sensitivity config with its global defaults
  /// to produce the final sensitivity level for a field.
  SensitivityLevel getEffectiveLevel({
    required String fieldId,
    List<String> tags = const [],
    SensitivityLevel? explicitOverride,
  }) {
    final configLevel = sensitivityConfig.getFieldLevel(fieldId, tags: tags);
    return globalDefaults.getEffectiveLevel(
      explicitLevel: explicitOverride,
      tagBasedLevel: configLevel,
    );
  }

  /// Returns true if the given sensitivity level requires masking in this account.
  bool shouldMask(SensitivityLevel level) {
    return level.isAtLeast(SensitivityLevel.sensitive);
  }

  /// Returns true if the given sensitivity level requires biometric auth.
  bool requiresBiometric(SensitivityLevel level) {
    if (!requireBiometricForSensitive) return false;
    return level.isAtLeast(SensitivityLevel.sensitive);
  }

  AccountStyleSettings copyWith({
    String? accountId,
    String? accountName,
    SensitivityConfig? sensitivityConfig,
    GlobalSensitivityDefaults? globalDefaults,
    bool? requireBiometricForSensitive,
    bool? showSensitiveByDefault,
    int? verificationTimeoutSeconds,
    List<String>? customTags,
  }) {
    return AccountStyleSettings(
      accountId: accountId ?? this.accountId,
      accountName: accountName ?? this.accountName,
      sensitivityConfig: sensitivityConfig ?? this.sensitivityConfig,
      globalDefaults: globalDefaults ?? this.globalDefaults,
      requireBiometricForSensitive:
          requireBiometricForSensitive ?? this.requireBiometricForSensitive,
      showSensitiveByDefault:
          showSensitiveByDefault ?? this.showSensitiveByDefault,
      verificationTimeoutSeconds:
          verificationTimeoutSeconds ?? this.verificationTimeoutSeconds,
      customTags: customTags ?? this.customTags,
    );
  }

  factory AccountStyleSettings.fromJson(Map<String, dynamic> json) {
    return AccountStyleSettings(
      accountId: json['account_id'] as String? ?? '',
      accountName: json['account_name'] as String? ?? '',
      sensitivityConfig: json['sensitivity_config'] != null
          ? SensitivityConfig.fromJson(
              json['sensitivity_config'] as Map<String, dynamic>)
          : const SensitivityConfig.empty(),
      globalDefaults: json['global_defaults'] != null
          ? GlobalSensitivityDefaults.fromJson(
              json['global_defaults'] as Map<String, dynamic>)
          : GlobalSensitivityDefaults.standard(),
      requireBiometricForSensitive:
          json['require_biometric_for_sensitive'] as bool? ?? false,
      showSensitiveByDefault:
          json['show_sensitive_by_default'] as bool? ?? false,
      verificationTimeoutSeconds:
          json['verification_timeout_seconds'] as int? ?? 60,
      customTags: (json['custom_tags'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
    );
  }

  Map<String, dynamic> toJson() => {
        'account_id': accountId,
        'account_name': accountName,
        'sensitivity_config': sensitivityConfig.toJson(),
        'global_defaults': globalDefaults.toJson(),
        'require_biometric_for_sensitive': requireBiometricForSensitive,
        'show_sensitive_by_default': showSensitiveByDefault,
        'verification_timeout_seconds': verificationTimeoutSeconds,
        'custom_tags': customTags,
      };
}