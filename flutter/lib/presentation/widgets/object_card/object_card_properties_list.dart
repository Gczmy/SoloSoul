import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';

class ObjectCardPropertiesList extends StatelessWidget {
  final UnifiedObject item;
  final String titlePropertyKey;
  final Map<String, PropertyValue>? template;

  const ObjectCardPropertiesList({
    super.key,
    required this.item,
    this.titlePropertyKey = 'Title',
    this.template,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    // Only show properties defined in the schema (template). Properties
    // that exist in item data but not in the schema are deprecated and
    // hidden from view — they can be accessed via "Show Deprecated" in
    // edit mode.
    final tmpl = template;
    final visibleEntries = tmpl != null
        ? tmpl.keys
            .where((k) => item.properties.containsKey(k))
            .map((k) => MapEntry(k, item.properties[k]!))
        : item.properties.entries;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: visibleEntries
          .where((e) => e.key != titlePropertyKey)
          .map((entry) {
        final sensitivity = entry.value.sensitivity;
        final isSensitive = sensitivity == SensitivityLevel.sensitive ||
            sensitivity == SensitivityLevel.critical;
        final valueStr = propValueToString(entry.value);
        final isEmptyValue = valueStr.isEmpty;

        return Padding(
          padding: const EdgeInsets.only(left: 8, bottom: 2),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Flexible(
                child: SelectableText(
                  wrapEveryNChars(item.getDisplayLabelFor(entry.key, l10n), 12),
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              if (isEmptyValue)
                Flexible(
                  child: Text(
                    l10n.commonEmpty,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                )
              else if (isSensitive)
                Flexible(
                  child: SensitiveValueWidget(
                    fieldId: 'item.${item.id}.${entry.key}',
                    value: valueStr,
                    sensitivityLevel: sensitivity,
                  ),
                )
              else
                Flexible(
                  child: SelectableText(
                    valueStr,
                    style: theme.textTheme.bodyMedium,
                  ),
                ),
              const SizedBox(width: 6),
              SensitivityTag(level: sensitivity),
            ],
          ),
        );
      }).toList(),
    );
  }
}
