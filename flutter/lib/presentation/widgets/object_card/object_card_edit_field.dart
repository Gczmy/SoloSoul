import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/date_picker_form_field.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

const kMaxPropertyLength = 100;

class ObjectCardEditField extends StatelessWidget {
  final String propertyKey;
  final PropertyValue? value;
  final TextEditingController? controller;
  final bool isTitle;
  final ValueChanged<bool?>? onCheckboxChanged;
  final String? displayLabel;

  const ObjectCardEditField({
    super.key,
    required this.propertyKey,
    this.value,
    required this.controller,
    this.isTitle = false,
    this.onCheckboxChanged,
    this.displayLabel,
  });

  static final _dummyValueNotifier = ValueNotifier<TextEditingValue>(const TextEditingValue());

  @override
  Widget build(BuildContext context) {
    // DateProperty → 日期选择器
    if (!isTitle && value is DateProperty) {
      final dateProp = value as DateProperty;
      return ValueListenableBuilder<TextEditingValue>(
        valueListenable: controller ?? _dummyValueNotifier,
        builder: (context, val, child) {
          return DatePickerFormField(
            label: isTitle ? AppLocalizations.of(context).commonTitle : translateFieldLabel(propertyKey, AppLocalizations.of(context)),
            initialDate: val.text.isNotEmpty ? val.text : dateProp.isoDate,
            sensitivity: dateProp.sensitivity,
            onDateChanged: (newDate) {
              controller?.text = newDate ?? '';
            },
          );
        },
      );
    }

    // CheckboxProperty → 复选框
    if (!isTitle && value is CheckboxProperty) {
      // 从 controller.text 读取当前勾选状态，因为外部通过更新 controller 来同步
      final controllerText = controller?.text.toLowerCase() ?? '';
      final checked = controllerText == 'true' ||
          controllerText == '1' ||
          controllerText == 'yes';
      return Row(
        children: [
          Checkbox(
            value: checked,
            onChanged: onCheckboxChanged,
          ),
          Text(translateFieldLabel(propertyKey, AppLocalizations.of(context))),
          const SizedBox(width: 8),
          SensitivityTag(level: value!.sensitivity),
        ],
      );
    }

    // 其他类型 → 文本输入框
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: TextField(
            controller: controller,
            maxLength: kMaxPropertyLength,
            buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
            decoration: InputDecoration(
              labelText: isTitle ? AppLocalizations.of(context).commonTitle : (displayLabel ?? translateFieldLabel(propertyKey, AppLocalizations.of(context))),
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
            valueListenable: controller ?? _dummyValueNotifier,
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
