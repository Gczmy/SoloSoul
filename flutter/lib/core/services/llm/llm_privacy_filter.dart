import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// =============================================================================
// LLM Privacy Filter
// =============================================================================

/// Pre-processes a batch of field data before sending to a cloud LLM.
///
/// Implements the "all-or-nothing" rule agreed in design:
/// any [SensitivityLevel.critical] field causes the **entire batch**
/// to be rejected.  Lower levels ([public], [internal], [sensitive])
/// are allowed through after optional redaction.
class LlmPrivacyFilter {
  const LlmPrivacyFilter();

  /// Checks whether [batch] is safe to send to a cloud LLM.
  ///
  /// Returns `null` if the batch is safe, or a [PrivacyBlockReason]
  /// explaining why it was blocked.
  PrivacyBlockReason? checkBatch(Map<String, dynamic> batch) {
    for (final entry in batch.entries) {
      final level = _extractSensitivity(entry.value);
      if (level == SensitivityLevel.critical) {
        return PrivacyBlockReason(
          fieldKey: entry.key,
          sensitivity: level,
          message: '字段 "${entry.key}" 敏感度为 ${level.label}，'
              '根据隐私策略禁止上传至云端 LLM。',
        );
      }
    }
    return null;
  }

  /// Redacts values that are marked [sensitive] (but not critical).
  ///
  /// Replaces the value with a generic placeholder so the LLM still sees
  /// the field exists (useful for schema-aware prompts) without leaking data.
  Map<String, dynamic> redactSensitive(Map<String, dynamic> batch) {
    final result = <String, dynamic>{};
    for (final entry in batch.entries) {
      final level = _extractSensitivity(entry.value);
      if (level == SensitivityLevel.sensitive) {
        result[entry.key] = '[REDACTED_SENSITIVE]';
      } else {
        result[entry.key] = entry.value;
      }
    }
    return result;
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  SensitivityLevel _extractSensitivity(dynamic value) {
    if (value is Map && value['sensitivity'] is String) {
      try {
        return SensitivityLevel.values.byName(value['sensitivity'] as String);
      } on ArgumentError {
        return SensitivityLevel.internal;
      }
    }
    return SensitivityLevel.internal;
  }
}

// =============================================================================
// Privacy Block Reason
// =============================================================================

class PrivacyBlockReason {
  final String fieldKey;
  final SensitivityLevel sensitivity;
  final String message;

  const PrivacyBlockReason({
    required this.fieldKey,
    required this.sensitivity,
    required this.message,
  });
}
