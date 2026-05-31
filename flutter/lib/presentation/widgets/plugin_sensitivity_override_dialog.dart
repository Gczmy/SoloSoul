import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 插件运行时敏感度超出处理弹窗。
///
/// 当插件尝试读取的字段实际敏感度超过其声明上限时弹出，
/// 让用户选择如何处理。
class PluginSensitivityOverrideDialog extends StatefulWidget {
  final String pluginName;
  final String fieldLabel;
  final String fieldKey;
  final SensitivityLevel actualSensitivity;
  final SensitivityLevel requiredSensitivity;
  final ValueChanged<SensitivityOverrideStrategy?> onDecision;

  const PluginSensitivityOverrideDialog({
    super.key,
    required this.pluginName,
    required this.fieldLabel,
    required this.fieldKey,
    required this.actualSensitivity,
    required this.requiredSensitivity,
    required this.onDecision,
  });

  @override
  State<PluginSensitivityOverrideDialog> createState() =>
      _PluginSensitivityOverrideDialogState();
}

class _PluginSensitivityOverrideDialogState
    extends State<PluginSensitivityOverrideDialog> {
  SensitivityOverrideStrategy _selectedStrategy =
      SensitivityOverrideStrategy.deny;
  bool _rememberChoice = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return AlertDialog(
      icon: const Icon(
        Icons.warning_amber,
        color: Colors.orange,
        size: 32,
      ),
      title: Text(l10n.sensitivityOverrideTitle),
      content: SizedBox(
        width: double.maxFinite,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 说明文字
              Text(
                l10n.sensitivityOverrideDescription(
                  widget.pluginName,
                  widget.fieldLabel,
                  widget.actualSensitivity.localizedLabel(l10n),
                  widget.requiredSensitivity.localizedLabel(l10n),
                ),
                style: theme.textTheme.bodyMedium,
              ),

              const SizedBox(height: 20),

              // 选项列表
              RadioGroup<SensitivityOverrideStrategy>(
                groupValue: _selectedStrategy,
                onChanged: (v) {
                  if (v != null) setState(() => _selectedStrategy = v);
                },
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _buildStrategyOption(
                      strategy: SensitivityOverrideStrategy.deny,
                      title: l10n.sensitivityOverrideDenyTitle,
                      description: l10n.sensitivityOverrideDenyDesc,
                      icon: Icons.block,
                      iconColor: Colors.red,
                      theme: theme,
                    ),
                    const SizedBox(height: 8),
                    _buildStrategyOption(
                      strategy: SensitivityOverrideStrategy.mask,
                      title: l10n.sensitivityOverrideMaskTitle,
                      description: l10n.sensitivityOverrideMaskDesc,
                      icon: Icons.visibility_off,
                      iconColor: Colors.orange,
                      theme: theme,
                    ),
                    const SizedBox(height: 8),
                    _buildStrategyOption(
                      strategy: SensitivityOverrideStrategy.allow,
                      title: l10n.sensitivityOverrideAllowTitle,
                      description: l10n.sensitivityOverrideAllowDesc,
                      icon: Icons.check_circle,
                      iconColor: Colors.green,
                      theme: theme,
                    ),
                  ],
                ),
              ),

              const SizedBox(height: 16),

              // 记住选择
              CheckboxListTile(
                value: _rememberChoice,
                onChanged: (v) => setState(() => _rememberChoice = v ?? false),
                title: Text(
                  l10n.sensitivityOverrideRemember,
                  style: theme.textTheme.bodySmall,
                ),
                controlAffinity: ListTileControlAffinity.leading,
                contentPadding: EdgeInsets.zero,
                dense: true,
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => widget.onDecision(null),
          child: Text(l10n.commonCancel),
        ),
        FilledButton(
          onPressed: () => widget.onDecision(_selectedStrategy),
          child: Text(l10n.sensitivityOverrideConfirm),
        ),
      ],
    );
  }

  Widget _buildStrategyOption({
    required SensitivityOverrideStrategy strategy,
    required String title,
    required String description,
    required IconData icon,
    required Color iconColor,
    required ThemeData theme,
  }) {
    final isSelected = _selectedStrategy == strategy;

    return InkWell(
      onTap: () => setState(() => _selectedStrategy = strategy),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: isSelected
              ? theme.colorScheme.primaryContainer.withValues(alpha: 0.3)
              : theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected
                ? theme.colorScheme.primary
                : theme.colorScheme.outline.withValues(alpha: 0.2),
            width: isSelected ? 1.5 : 1,
          ),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Radio<SensitivityOverrideStrategy>(
              value: strategy,
              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
            const SizedBox(width: 4),
            Icon(icon, size: 20, color: iconColor),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    description,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 敏感度超出处理策略
enum SensitivityOverrideStrategy {
  deny,   // 拒绝访问
  mask,   // 返回脱敏数据
  allow,  // 允许访问并记录日志
}
