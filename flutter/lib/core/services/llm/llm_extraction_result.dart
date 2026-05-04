import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

// =============================================================================
// LLM Extraction Data Models
// =============================================================================

/// Schema definition for a field that LLM should extract.
class FieldSchema {
  final String propertyId;
  final String displayName;
  final String description;
  final SensitivityLevel expectedSensitivity;

  const FieldSchema({
    required this.propertyId,
    required this.displayName,
    this.description = '',
    this.expectedSensitivity = SensitivityLevel.public,
  });
}

/// Result of an LLM extraction operation.
class LlmExtractionResult {
  final List<ExtractedField> fields;

  /// Overall confidence of the extraction (0.0 ~ 1.0).
  final double confidence;

  /// LLM reasoning chain for debugging / user confirmation.
  final String? reasoning;

  const LlmExtractionResult({
    required this.fields,
    this.confidence = 0.0,
    this.reasoning,
  });

  /// Empty result when LLM is unavailable or declines to answer.
  static const LlmExtractionResult empty = LlmExtractionResult(fields: []);
}

/// A single field extracted by LLM.
class ExtractedField {
  final String propertyId;
  final String value;

  /// LLM-assigned confidence for this specific field.
  final double confidence;

  /// Sensitivity inferred by LLM (fallback to schema default if uncertain).
  final SensitivityLevel sensitivity;

  const ExtractedField({
    required this.propertyId,
    required this.value,
    this.confidence = 0.0,
    this.sensitivity = SensitivityLevel.public,
  });
}

/// Validation result from LLM sanity check.
class LlmValidationResult {
  final bool isValid;
  final List<LlmValidationWarning> warnings;

  const LlmValidationResult({
    this.isValid = true,
    this.warnings = const [],
  });
}

class LlmValidationWarning {
  final String fieldId;
  final String message;

  const LlmValidationWarning({
    required this.fieldId,
    required this.message,
  });
}
