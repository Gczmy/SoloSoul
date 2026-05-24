import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

// ============================================================================
// 字段名本地化映射
// ============================================================================

final Map<String, String> _fieldNameMapEn = {
  'identity.full_name': 'Full Name',
  'identity.id_card.number': 'ID Card Number',
  'travel.primary_passport.number': 'Passport Number',
  'identity.contact.emails': 'Email Addresses',
  'identity.contact.phones': 'Phone Numbers',
  'financial.primary_bank_account.number': 'Bank Account Number',
  'address.street': 'Street',
  'address.city': 'City',
  'address.state': 'State / Province',
  'address.postalCode': 'Postal Code',
  'address.country': 'Country',
  'address.district': 'District / Label',
  'address.label': 'Label',
  'address.count': 'Address Count',
};

final Map<String, String> _fieldNameMapZh = {
  'identity.full_name': '真实姓名',
  'identity.id_card.number': '身份证号码',
  'travel.primary_passport.number': '护照号码',
  'identity.contact.emails': '电子邮箱',
  'identity.contact.phones': '手机号码',
  'financial.primary_bank_account.number': '银行卡号',
  'address.street': '街道地址',
  'address.city': '城市',
  'address.state': '省/州',
  'address.postalCode': '邮政编码',
  'address.country': '国家/地区',
  'address.district': '区/标签',
  'address.label': '标签',
  'address.count': '地址数量',
};

/// 将通配符字段展开为具体字段列表，用于批量授权对话框展示
List<String> _expandWildcardField(String fieldId) {
  if (fieldId == 'address.*') {
    return const [
      'address.street',
      'address.city',
      'address.state',
      'address.postalCode',
      'address.country',
      'address.district',
    ];
  }
  return [fieldId];
}

String _getFieldDisplayName(BuildContext context, String fieldId) {
  final l10n = AppLocalizations.of(context);
  final isZh = l10n.localeName.startsWith('zh');
  final map = isZh ? _fieldNameMapZh : _fieldNameMapEn;
  return map[fieldId] ?? fieldId;
}

Color _getSensitivityColor(SensitivityLevel sensitivity) {
  return switch (sensitivity) {
    SensitivityLevel.public => Colors.green,
    SensitivityLevel.internal => Colors.blue,
    SensitivityLevel.sensitive => Colors.orange,
    SensitivityLevel.critical => Colors.red,
  };
}

String _getSensitivityLabel(BuildContext context, SensitivityLevel sensitivity) {
  final l10n = AppLocalizations.of(context);
  return switch (sensitivity) {
    SensitivityLevel.public => l10n.pluginSensitivityPublic,
    SensitivityLevel.internal => l10n.pluginSensitivityInternal,
    SensitivityLevel.sensitive => l10n.pluginSensitivitySensitive,
    SensitivityLevel.critical => l10n.pluginSensitivityCritical,
  };
}

SensitivityLevel _parseSensitivityLevel(String raw) {
  return switch (raw.toLowerCase()) {
    'public' => SensitivityLevel.public,
    'internal' => SensitivityLevel.internal,
    'sensitive' => SensitivityLevel.sensitive,
    'critical' => SensitivityLevel.critical,
    _ => SensitivityLevel.sensitive,
  };
}

/// 插件数据授权弹窗（单字段）
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

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final fieldDisplayName = _getFieldDisplayName(context, fieldId);
    final sensitivityColor = _getSensitivityColor(sensitivity);

    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.2,
        vertical: 24,
      ),
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
                          '${l10n.sensitivityPublic}: ${_getSensitivityLabel(context, sensitivity)}',
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

/// 批量授权请求项
class BatchConsentRequest {
  final String requestId;
  final String field;
  final String sensitivity;

  const BatchConsentRequest({
    required this.requestId,
    required this.field,
    required this.sensitivity,
  });
}

/// 授权对话框展示项（通配符展开后的具体字段）
class _DisplayItem {
  final String field;
  final String sensitivity;

  const _DisplayItem({required this.field, required this.sensitivity});
}

/// 插件批量数据授权弹窗
///
/// 将插件请求的所有敏感字段汇总到一个对话框中，用户可一次性授权或拒绝。
class PluginBatchConsentDialog extends StatelessWidget {
  final String pluginId;
  final String pluginName;
  final List<BatchConsentRequest> requests;

  const PluginBatchConsentDialog({
    super.key,
    required this.pluginId,
    required this.pluginName,
    required this.requests,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    // 将通配符字段展开为具体字段，用于展示
    final displayItems = <_DisplayItem>[];
    for (final req in requests) {
      final expanded = _expandWildcardField(req.field);
      for (final field in expanded) {
        displayItems.add(_DisplayItem(
          field: field,
          sensitivity: req.sensitivity,
        ));
      }
    }

    return AlertDialog(
      insetPadding: EdgeInsets.symmetric(
        horizontal: MediaQuery.of(context).size.width * 0.2,
        vertical: 24,
      ),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      title: Row(
        children: [
          Icon(Icons.security, color: Theme.of(context).colorScheme.primary),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              l10n.pluginConsentDialogTitle,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
      content: SizedBox(
        width: double.maxFinite,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '「$pluginName」请求访问以下 ${displayItems.length} 个字段：',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 16),
            Container(
              constraints: const BoxConstraints(maxHeight: 280),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerHighest.withOpacity(0.3),
                borderRadius: BorderRadius.circular(12),
              ),
              child: ListView.separated(
                shrinkWrap: true,
                itemCount: displayItems.length,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final item = displayItems[index];
                  final sensitivity = _parseSensitivityLevel(item.sensitivity);
                  final displayName = _getFieldDisplayName(context, item.field);
                  final color = _getSensitivityColor(sensitivity);
                  final label = _getSensitivityLabel(context, sensitivity);

                  return Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                    child: Row(
                      children: [
                        Expanded(
                          child: Text(
                            displayName,
                            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: color.withOpacity(0.12),
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(color: color.withOpacity(0.3)),
                          ),
                          child: Text(
                            label,
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.w600,
                              color: color,
                            ),
                          ),
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
            const SizedBox(height: 12),
            Text(
              '授权后插件将一次性读取上述字段用于处理。',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
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
          child: Text('全部授权'),
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
