import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

const kMaxPropertyLength = 100;

class ObjectCardEditField extends StatelessWidget {
  final String propertyKey;
  final PropertyValue? value;
  final TextEditingController? controller;
  final bool isTitle;
  final ValueChanged<bool?>? onCheckboxChanged;

  const ObjectCardEditField({
    super.key,
    required this.propertyKey,
    this.value,
    required this.controller,
    this.isTitle = false,
    this.onCheckboxChanged,
  });

  static final _dummyController = TextEditingController();

  @override
  Widget build(BuildContext context) {
    if (!isTitle && value is CheckboxProperty) {
      final checked = (value as CheckboxProperty).checked;
      return Row(
        children: [
          Checkbox(
            value: checked,
            onChanged: onCheckboxChanged,
          ),
          Text(formatLabel(propertyKey)),
          const SizedBox(width: 8),
          SensitivityTag(level: value!.sensitivity),
        ],
      );
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: TextField(
            controller: controller,
            maxLength: kMaxPropertyLength,
            buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
            decoration: InputDecoration(
              labelText: isTitle ? 'Title' : formatLabel(propertyKey),
              border: const OutlineInputBorder(),
              suffixIcon: !isTitle && value != null
                  ? Padding(
                      padding: const EdgeInsets.only(right: 12),
                      child: Align(
                        alignment: Alignment.centerRight,
                        widthFactor: 1,
                        child: SensitivityTag(level: value!.sensitivity),
                      ),
                    )
                  : null,
            ),
            keyboardType: (!isTitle && value is NumberProperty)
                ? TextInputType.number
                : null,
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 64,
          child: ValueListenableBuilder<TextEditingValue>(
            valueListenable: controller ?? _dummyController,
            builder: (context, val, child) {
              final len = val.text.length;
              const max = kMaxPropertyLength;
              return Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  SizedBox(
                    width: 28,
                    child: Text(
                      '$len',
                      textAlign: TextAlign.right,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: len >= max
                            ? Theme.of(context).colorScheme.error
                            : Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  Text(
                    '/',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: len >= max
                          ? Theme.of(context).colorScheme.error
                          : Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  SizedBox(
                    width: 28,
                    child: Text(
                      '$max',
                      textAlign: TextAlign.left,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: len >= max
                            ? Theme.of(context).colorScheme.error
                            : Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}
