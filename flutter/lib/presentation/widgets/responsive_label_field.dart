import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

/// Data class for a single label-value field.
class LabelValueField {
  final String label;
  final String value;
  final String? fieldId;
  final bool isSensitive;
  final SensitivityLevel? sensitivityLevel;

  const LabelValueField({
    required this.label,
    required this.value,
    this.fieldId,
    this.isSensitive = false,
    this.sensitivityLevel,
  });
}

/// Widget that displays form fields with a clear label prefix and responsive wrapping.
/// - Uses Wrap widget for horizontal wrapping when fields would exceed screen width
/// - Integrates with SensitiveValueWidget for sensitive fields
/// - Supports CrossAxisAlignment.start for proper alignment
class ResponsiveLabelField extends ConsumerWidget {
  final List<LabelValueField> fields;
  final double labelValueSpacing;
  final Axis layoutAxis;

  const ResponsiveLabelField({
    super.key,
    required this.fields,
    this.labelValueSpacing = 4,
    this.layoutAxis = Axis.horizontal,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (fields.isEmpty) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);

    if (layoutAxis == Axis.vertical) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (int i = 0; i < fields.length; i++) ...[
            _FieldRow(
              field: fields[i],
              theme: theme,
              labelValueSpacing: labelValueSpacing,
            ),
            if (i < fields.length - 1)
              SizedBox(height: labelValueSpacing * 2),
          ],
        ],
      );
    }

    return Wrap(
      spacing: labelValueSpacing * 2,
      runSpacing: labelValueSpacing * 2,
      crossAxisAlignment: WrapCrossAlignment.start,
      children: [
        for (final field in fields)
          _FieldRow(
            field: field,
            theme: theme,
            labelValueSpacing: labelValueSpacing,
          ),
      ],
    );
  }
}

class _FieldRow extends ConsumerWidget {
  final LabelValueField field;
  final ThemeData theme;
  final double labelValueSpacing;

  const _FieldRow({
    required this.field,
    required this.theme,
    required this.labelValueSpacing,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final effectiveFieldId = field.fieldId ?? field.label.toLowerCase().replaceAll(' ', '.');

    // Fall back to registry lookup if sensitivityLevel not provided
    final SensitivityLevel sensitivityLevel = field.sensitivityLevel ??
        ref.watch(effectiveSensitivityProvider(effectiveFieldId));
    final isSensitive = field.isSensitive ||
        sensitivityLevel == SensitivityLevel.sensitive ||
        sensitivityLevel == SensitivityLevel.critical;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        // Label with colon suffix — flexible so it can shrink when space is tight
        Flexible(
          child: SelectableText(
            '${field.label}: ',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        SizedBox(width: labelValueSpacing),
        // Value (sensitive or plain) — expanded to force tight fit and prevent overflow
        if (isSensitive)
          Expanded(
            child: SensitiveValueWidget(
              fieldId: effectiveFieldId,
              value: field.value,
            ),
          )
        else
          Expanded(
            child: SelectableText(
              field.value,
              style: theme.textTheme.bodyMedium,
            ),
          ),
        const SizedBox(width: 6),
        SensitivityTag(level: sensitivityLevel),
      ],
    );
  }
}
