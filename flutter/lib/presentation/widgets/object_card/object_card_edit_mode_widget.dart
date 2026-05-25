import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_edit_field.dart';

/// Widget for editing an existing item in edit mode.
class ObjectCardEditModeWidget extends StatelessWidget {
  final UnifiedObject item;
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
  final bool showDeprecated;
  final List<String> deprecatedKeys;
  final VoidCallback onToggleDeprecated;

  const ObjectCardEditModeWidget({
    super.key,
    required this.item,
    required this.template,
    required this.editControllers,
    required this.hasChanges,
    required this.onSave,
    required this.onCancel,
    required this.titlePropertyKey,
    this.customFormBuilder,
    required this.onCheckboxChanged,
    this.showDeprecated = false,
    this.deprecatedKeys = const [],
    this.onToggleDeprecated = _noop,
  });

  static void _noop() {}

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
        'edit',
        onSave,
        onCancel,
        sensitivities,
      );
    }

    final hasTitleField = item.properties.containsKey(titlePropertyKey);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title input (only when item has a title property)
          if (hasTitleField) ...[
            ObjectCardEditField(
              propertyKey: '__name__',
              value: item.properties[titlePropertyKey],
              controller: editControllers['__name__'],
              isTitle: true,
              displayLabel: item.getDisplayLabelFor(titlePropertyKey, l10n),
            ),
            const SizedBox(height: 12),
          ],
          // Property inputs - only render active (template) keys.
          // Deprecated keys are handled by the toggle section below.
          ...editControllers.keys
              .where((k) => k != '__name__' && k != titlePropertyKey && template.containsKey(k))
              .map((key) {
            final value = item.properties[key] ?? template[key];
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: ObjectCardEditField(
                propertyKey: key,
                value: value!,
                controller: editControllers[key],
                displayLabel: item.getDisplayLabelFor(key, l10n),
                onCheckboxChanged: (newValue) {
                  onCheckboxChanged(key, newValue);
                },
              ),
            );
          }),
          // Deprecated properties toggle (item-only keys not in schema)
          if (deprecatedKeys.isNotEmpty) ...[
            const SizedBox(height: 8),
            InkWell(
              onTap: onToggleDeprecated,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 4),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      showDeprecated
                          ? Icons.visibility_off_outlined
                          : Icons.visibility_outlined,
                      size: 16,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      showDeprecated
                          ? l10n.objectEditorHideDeprecated
                          : l10n.objectEditorShowDeprecated(deprecatedKeys.length),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            // Render deprecated fields when toggled on (read-only via AbsorbPointer)
            if (showDeprecated)
              ...deprecatedKeys.map((key) {
                final value = item.properties[key];
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: AbsorbPointer(
                    child: Opacity(
                      opacity: 0.55,
                      child: ObjectCardEditField(
                        propertyKey: key,
                        value: value,
                        controller: editControllers[key],
                      ),
                    ),
                  ),
                );
              }),
          ],
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
                child: Text(l10n.commonSave),
              ),
            ],
          ),
          const Divider(height: 16),
        ],
      ),
    );
  }
}
