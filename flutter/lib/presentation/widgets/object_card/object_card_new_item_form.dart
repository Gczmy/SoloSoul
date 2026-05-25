import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_edit_field.dart';

/// Form widget for adding a new item to the ObjectCard.
class ObjectCardNewItemForm extends StatelessWidget {
  final Map<String, PropertyValue> template;
  final Map<String, TextEditingController> editControllers;
  final bool hasChanges;
  final VoidCallback onSave;
  final VoidCallback onCancel;
  final String titlePropertyKey;
  final Widget Function(
    BuildContext,
    ThemeData,
    Map<String, TextEditingController>,
    String,
    VoidCallback,
    VoidCallback,
    Map<String, SensitivityLevel>,
  )? customFormBuilder;
  final void Function(String key, bool? value) onCheckboxChanged;

  const ObjectCardNewItemForm({
    super.key,
    required this.template,
    required this.editControllers,
    required this.hasChanges,
    required this.onSave,
    required this.onCancel,
    required this.titlePropertyKey,
    this.customFormBuilder,
    required this.onCheckboxChanged,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    if (customFormBuilder != null) {
      final sensitivities = <String, SensitivityLevel>{
        for (final entry in template.entries)
          entry.key: entry.value.sensitivity,
      };
      return customFormBuilder!(
        context,
        theme,
        editControllers,
        'add',
        onSave,
        onCancel,
        sensitivities,
      );
    }

    final hasTitleField = template.containsKey(titlePropertyKey);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input (only when template defines a title property)
          if (hasTitleField) ...[
            ObjectCardEditField(
              propertyKey: '__name__',
              value: template[titlePropertyKey],
              controller: editControllers['__name__'],
              isTitle: true,
              displayLabel: translateFieldLabel(titlePropertyKey, l10n),
            ),
            const SizedBox(height: 12),
          ],
          // Property inputs (skip the title property - already shown above)
          ...template.keys.where((k) => k != titlePropertyKey).map((key) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: ObjectCardEditField(
                propertyKey: key,
                value: template[key]!,
                controller: editControllers[key],
                onCheckboxChanged: (newValue) {
                  onCheckboxChanged(key, newValue);
                },
              ),
            );
          }),
          const SizedBox(height: 12),
          // Action buttons
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: onCancel,
                child: Text(l10n.commonCancel),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: hasChanges ? onSave : null,
                child: Text(l10n.commonAdd),
              ),
            ],
          ),
          const Divider(height: 16),
        ],
      ),
    );
  }
}
