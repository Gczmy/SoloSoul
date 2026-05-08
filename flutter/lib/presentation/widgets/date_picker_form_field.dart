import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/utils/mrz_date_utils.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

// =============================================================================
// Date Picker Form Field
// =============================================================================

/// 日期选择器表单字段。
///
/// 显示当前日期值（ISO 8601 格式 YYYY-MM-DD），点击后弹出 Material DatePicker。
/// 支持长按清空日期。
class DatePickerFormField extends StatelessWidget {
  final String label;
  final String? initialDate;
  final SensitivityLevel sensitivity;
  final ValueChanged<String?> onDateChanged;

  const DatePickerFormField({
    super.key,
    required this.label,
    this.initialDate,
    this.sensitivity = SensitivityLevel.public,
    required this.onDateChanged,
  });

  Future<void> _pickDate(BuildContext context) async {
    final initial = parseIsoDate(initialDate ?? '');
    final now = DateTime.now();

    final picked = await showDatePicker(
      context: context,
      initialDate: initial ?? now,
      firstDate: DateTime(1900),
      lastDate: DateTime(2100),
      helpText: label,
      cancelText: 'Cancel',
      confirmText: 'OK',
    );

    if (picked != null) {
      final iso = '${picked.year.toString().padLeft(4, '0')}-'
          '${picked.month.toString().padLeft(2, '0')}-'
          '${picked.day.toString().padLeft(2, '0')}';
      onDateChanged(iso);
    }
  }

  void _clearDate() => onDateChanged(null);

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final hasDate = initialDate != null && initialDate!.isNotEmpty;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: InkWell(
            onTap: () => _pickDate(context),
            onLongPress: hasDate ? _clearDate : null,
            borderRadius: BorderRadius.circular(8),
            child: InputDecorator(
              decoration: InputDecoration(
                labelText: label,
                border: const OutlineInputBorder(),
                suffixIcon: hasDate
                    ? IconButton(
                        icon: const Icon(Icons.clear, size: 18),
                        onPressed: _clearDate,
                        tooltip: l10n.datePickerClear,
                      )
                    : const Icon(Icons.calendar_today, size: 18),
              ),
              child: Text(
                hasDate ? initialDate! : 'Select date',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: hasDate
                      ? theme.colorScheme.onSurface
                      : theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 64,
          child: Center(child: SensitivityTag(level: sensitivity)),
        ),
      ],
    );
  }
}
