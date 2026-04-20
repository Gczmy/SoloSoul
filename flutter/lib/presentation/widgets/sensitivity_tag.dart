import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';

/// Helper to get sensitivity level color
Color getSensitivityColor(SensitivityLevel level) {
  switch (level) {
    case SensitivityLevel.critical:
      return Colors.red.shade900;
    case SensitivityLevel.sensitive:
      return Colors.orange;
    case SensitivityLevel.internal:
      return Colors.blue;
    case SensitivityLevel.public:
      return Colors.green;
  }
}

/// Helper to get sensitivity level label
String getSensitivityLabel(SensitivityLevel level) {
  switch (level) {
    case SensitivityLevel.critical:
      return 'Critical';
    case SensitivityLevel.sensitive:
      return 'Sensitive';
    case SensitivityLevel.internal:
      return 'Internal';
    case SensitivityLevel.public:
      return 'Public';
  }
}

/// Widget that shows a sensitivity level tag
class SensitivityTag extends StatelessWidget {
  final SensitivityLevel level;

  const SensitivityTag({super.key, required this.level});

  @override
  Widget build(BuildContext context) {
    final color = getSensitivityColor(level);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withValues(alpha: 0.3)),
      ),
      child: Text(
        getSensitivityLabel(level),
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
