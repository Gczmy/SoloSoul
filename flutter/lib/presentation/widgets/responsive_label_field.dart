import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';

/// Data class for a single label-value field.
class LabelValueField {
  final String label;
  final String value;
  final String? fieldId;
  final bool isSensitive;

  const LabelValueField({
    required this.label,
    required this.value,
    this.fieldId,
    this.isSensitive = false,
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

    return Row(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        // Label with colon suffix
        SelectableText(
          '${field.label}: ',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        SizedBox(width: labelValueSpacing),
        // Value (sensitive or plain)
        if (field.isSensitive)
          Flexible(
            child: SensitiveValueWidget(
              fieldId: effectiveFieldId,
              value: field.value,
            ),
          )
        else
          Flexible(
            child: SelectableText(
              field.value,
              style: theme.textTheme.bodyMedium,
            ),
          ),
      ],
    );
  }
}
