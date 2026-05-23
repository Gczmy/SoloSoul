import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// 插件数据授权弹窗
///
/// 显示插件请求的字段、敏感度级别，并让用户选择授权或拒绝。
/// 支持 i18n 和 Liquid Glass 风格（通过 AppTheme 适配）。
class PluginConsentDialog extends ConsumerWidget {
  final String pluginId;
  final String pluginName;
  final String fieldId;
  final String requestId;
  final SensitivityLevel sensitivity;

  const PluginConsentDialog({
    super.key,
    required this.pluginId,
    required this.pluginName,
    required this.fieldId,
    required this.requestId,
    required this.sensitivity,
  });

  static final Map<String, String> _fieldNameMapEn = {
    'identity.full_name': 'Full Name',
    'identity.id_card.number': 'ID Card Number',
    'travel.primary_passport.number': 'Passport Number',
    'identity.contact.emails': 'Email Addresses',
    'identity.contact.phones': 'Phone Numbers',
    'financial.primary_bank_account.number': 'Bank Account Number',
  };

  static final Map<String, String> _fieldNameMapZh = {
    'identity.full_name': '真实姓名',
    'identity.id_card.number': '身份证号码',
    'travel.primary_passport.number': '护照号码',
    'identity.contact.emails': '电子邮箱',
    'identity.contact.phones': '手机号码',
    'financial.primary_bank_account.number': '银行卡号',
  };

  String _getFieldDisplayName(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isZh = l10n.localeName.startsWith('zh');
    final map = isZh ? _fieldNameMapZh : _fieldNameMapEn;
    return map[fieldId] ?? fieldId;
  }

  Color _getSensitivityColor() {
    return switch (sensitivity) {
      SensitivityLevel.public => Colors.green,
      SensitivityLevel.internal => Colors.blue,
      SensitivityLevel.sensitive => Colors.orange,
      SensitivityLevel.critical => Colors.red,
    };
  }

  String _getSensitivityLabel(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return switch (sensitivity) {
      SensitivityLevel.public => l10n.pluginSensitivityPublic,
      SensitivityLevel.internal => l10n.pluginSensitivityInternal,
      SensitivityLevel.sensitive => l10n.pluginSensitivitySensitive,
      SensitivityLevel.critical => l10n.pluginSensitivityCritical,
    };
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final fieldDisplayName = _getFieldDisplayName(context);
    final sensitivityColor = _getSensitivityColor();

    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      title: Row(
        children: [
          Icon(Icons.security, color: Theme.of(context).colorScheme.primary),
          const SizedBox(width: 8),
          Text(l10n.pluginConsentDialogTitle),
        ],
      ).animate().fadeIn(duration: 200.ms),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.pluginConsentDialogSubtitle(pluginName),
            style: Theme.of(context).textTheme.bodyMedium,
          ),
          const SizedBox(height: 16),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: sensitivityColor.withValues(alpha: 0.08),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: sensitivityColor.withValues(alpha: 0.4)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: sensitivityColor, size: 24),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        fieldDisplayName,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 15,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                        decoration: BoxDecoration(
                          color: sensitivityColor.withValues(alpha: 0.15),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          '${l10n.sensitivityPublic}: ${_getSensitivityLabel(context)}',
                          style: TextStyle(
                            fontSize: 12,
                            color: sensitivityColor,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(delay: 100.ms).slideY(begin: 0.1, end: 0),
          const SizedBox(height: 16),
          Text(
            l10n.pluginConsentDialogDataLifetime,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.pluginConsentButtonDeny),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          style: FilledButton.styleFrom(
            backgroundColor: Theme.of(context).colorScheme.primary,
          ),
          child: Text(l10n.pluginConsentButtonAuthorize),
        ),
      ],
    );
  }
}

/// 显示插件授权弹窗的便捷函数
///
/// 返回 `true` 表示用户授权，`false` 表示拒绝，`null` 表示弹窗被关闭（按返回键）
Future<bool?> showPluginConsentDialog({
  required BuildContext context,
  required String pluginId,
  required String pluginName,
  required String fieldId,
  required String requestId,
  required SensitivityLevel sensitivity,
}) async {
  return showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (context) => PluginConsentDialog(
      pluginId: pluginId,
      pluginName: pluginName,
      fieldId: fieldId,
      requestId: requestId,
      sensitivity: sensitivity,
    ),
  );
}
