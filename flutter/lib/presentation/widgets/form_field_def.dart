import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';

/// Field definition for a single form field
class FormFieldDef {
  final String fieldId;
  final String label;
  final String? hintText;
  final SensitivityLevel sensitivity;
  final String? initialValue;

  const FormFieldDef({
    required this.fieldId,
    required this.label,
    this.hintText,
    this.sensitivity = SensitivityLevel.public,
    this.initialValue,
  });
}
