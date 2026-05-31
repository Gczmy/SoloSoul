import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 插件安装时的字段访问审查弹窗。
///
/// 展示插件请求的所有字段，以及每个字段的：
/// - 显示名称 + 机器 key（可展开）
/// - 所在分区
/// - 当前敏感度
/// - 插件要求的敏感度上限
/// - 访问模式
/// - 状态（符合/超出/缺失）
class PluginAccessReviewDialog extends StatelessWidget {
  final String pluginName;
  final List<FieldAccessStatus> fieldStatuses;
  final VoidCallback onModifySensitivity;
  final VoidCallback onCreateMissingFields;
  final VoidCallback onContinueInstall;
  final VoidCallback onCancel;

  const PluginAccessReviewDialog({
    super.key,
    required this.pluginName,
    required this.fieldStatuses,
    required this.onModifySensitivity,
    required this.onCreateMissingFields,
    required this.onContinueInstall,
    required this.onCancel,
  });

  bool get _hasExceeded => fieldStatuses.any((s) => s.status == AccessStatus.exceeded);
  bool get _hasMissing => fieldStatuses.any((s) => s.status == AccessStatus.missing);

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final languageCode = Localizations.localeOf(context).languageCode;

    return AlertDialog(
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.extension, color: theme.colorScheme.primary),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  l10n.pluginAccessReviewTitle(pluginName),
                  style: theme.textTheme.titleLarge,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            l10n.pluginAccessReviewSubtitle,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
      content: SizedBox(
        width: double.maxFinite,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              // 字段状态表格
              _buildFieldTable(l10n, theme, languageCode),

              const SizedBox(height: 16),

              // 超出警告
              if (_hasExceeded) _buildExceededWarning(l10n, theme),

              // 缺失警告
              if (_hasMissing) _buildMissingWarning(l10n, theme),
            ],
          ),
        ),
      ),
      actions: [
        // 修改敏感度按钮
        if (_hasExceeded)
          TextButton.icon(
            onPressed: onModifySensitivity,
            icon: const Icon(Icons.security, size: 18),
            label: Text(l10n.pluginAccessReviewModifySensitivity),
          ),

        // 创建缺失字段按钮
        if (_hasMissing)
          TextButton.icon(
            onPressed: onCreateMissingFields,
            icon: const Icon(Icons.add, size: 18),
            label: Text(l10n.pluginAccessReviewCreateMissing),
          ),

        const SizedBox(width: 8),

        // 取消
        TextButton(
          onPressed: onCancel,
          child: Text(l10n.commonCancel),
        ),

        // 继续安装
        FilledButton(
          onPressed: onContinueInstall,
          child: Text(l10n.pluginAccessReviewContinue),
        ),
      ],
    );
  }

  Widget _buildFieldTable(AppLocalizations l10n, ThemeData theme, String languageCode) {
    return Container(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outline.withValues(alpha: 0.2)),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        children: [
          // 表头
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
              borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
            ),
            child: Row(
              children: [
                Expanded(
                  flex: 3,
                  child: Text(
                    l10n.pluginAccessReviewHeaderField,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: Text(
                    l10n.pluginAccessReviewHeaderSection,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: Text(
                    l10n.pluginAccessReviewHeaderSensitivity,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                SizedBox(
                  width: 32,
                  child: Text(
                    l10n.pluginAccessReviewHeaderStatus,
                    style: theme.textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    textAlign: TextAlign.center,
                  ),
                ),
              ],
            ),
          ),

          // 数据行
          ...fieldStatuses.map((status) => _buildFieldRow(status, l10n, theme, languageCode)),
        ],
      ),
    );
  }

  Widget _buildFieldRow(FieldAccessStatus status, AppLocalizations l10n, ThemeData theme, String languageCode) {
    final statusColor = switch (status.status) {
      AccessStatus.ok => Colors.green,
      AccessStatus.exceeded => Colors.orange,
      AccessStatus.missing => Colors.grey,
    };

    final statusIcon = switch (status.status) {
      AccessStatus.ok => Icons.check_circle,
      AccessStatus.exceeded => Icons.warning,
      AccessStatus.missing => Icons.help_outline,
    };

    final semanticType = status.semanticType;
    final semanticTypeLabel = semanticType != null
        ? SemanticTypeRegistry.getType(semanticType)?.getLabel(languageCode) ?? semanticType
        : status.fieldLabel ?? status.fieldKey ?? '';

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        border: Border(
          top: BorderSide(color: theme.colorScheme.outline.withValues(alpha: 0.1)),
        ),
      ),
      child: Row(
        children: [
          // 字段名称（含 ⓘ 展开机器 key）
          Expanded(
            flex: 3,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    if (semanticType case final st?)
                      Padding(
                        padding: const EdgeInsets.only(right: 4),
                        child: Icon(
                          SemanticTypeRegistry.getType(st)?.icon ??
                              Icons.label,
                          size: 14,
                          color: theme.colorScheme.primary,
                        ),
                      ),
                    Expanded(
                      child: Text(
                        semanticTypeLabel,
                        style: theme.textTheme.bodySmall,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    if (status.fieldKey case final fieldKey?)
                      _MachineKeyIndicator(fieldKey: fieldKey),
                  ],
                ),
              ],
            ),
          ),

          // 所在分区
          Expanded(
            flex: 2,
            child: Text(
              status.sectionName ?? l10n.pluginAccessReviewNoSection,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),

          // 敏感度比较
          Expanded(
            flex: 2,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (status.status == AccessStatus.missing)
                  Text(
                    l10n.pluginAccessReviewMissing,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontStyle: FontStyle.italic,
                    ),
                  )
                else
                  Row(
                    children: [
                      _SensitivityDot(status.actualSensitivity),
                      const SizedBox(width: 4),
                      Text(
                        '≤ ${status.requiredSensitivity?.label ?? "?"}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: status.status == AccessStatus.exceeded
                              ? Colors.orange
                              : theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ),

          // 状态图标
          SizedBox(
            width: 32,
            child: Icon(statusIcon, size: 18, color: statusColor),
          ),
        ],
      ),
    );
  }

  Widget _buildExceededWarning(AppLocalizations l10n, ThemeData theme) {
    final exceededFields =
        fieldStatuses.where((s) => s.status == AccessStatus.exceeded).toList();

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.orange.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.warning_amber, size: 18, color: Colors.orange),
              const SizedBox(width: 8),
              Text(
                l10n.pluginAccessReviewExceededTitle,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: Colors.orange.shade800,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          ...exceededFields.map((f) {
            final label = f.fieldLabel ?? f.semanticType ?? f.fieldKey ?? '';
            return Text(
              '• ${l10n.pluginAccessReviewExceededItem(label, f.actualSensitivity?.label ?? '', f.requiredSensitivity?.label ?? '')}',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            );
          }),
        ],
      ),
    );
  }

  Widget _buildMissingWarning(AppLocalizations l10n, ThemeData theme) {
    final missingFields =
        fieldStatuses.where((s) => s.status == AccessStatus.missing).toList();

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.help_outline, size: 18, color: theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 8),
              Text(
                l10n.pluginAccessReviewMissingTitle,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          ...missingFields.map((f) {
            final label = f.fieldLabel ?? f.semanticType ?? f.fieldKey ?? '';
            return Text(
              '• ${l10n.pluginAccessReviewMissingItem(label)}',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            );
          }),
        ],
      ),
    );
  }
}

/// 机器 key 指示器（ⓘ 图标，悬停/点击显示）
class _MachineKeyIndicator extends StatefulWidget {
  final String fieldKey;

  const _MachineKeyIndicator({required this.fieldKey});

  @override
  State<_MachineKeyIndicator> createState() => _MachineKeyIndicatorState();
}

class _MachineKeyIndicatorState extends State<_MachineKeyIndicator> {
  bool _showTooltip = false;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () => setState(() => _showTooltip = !_showTooltip),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.info_outline,
            size: 14,
            color: Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
          ),
          if (_showTooltip)
            Container(
              margin: const EdgeInsets.only(left: 4),
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                widget.fieldKey,
                style: TextStyle(
                  fontSize: 10,
                  fontFamily: 'monospace',
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// 敏感度圆点
class _SensitivityDot extends StatelessWidget {
  final SensitivityLevel? level;

  const _SensitivityDot(this.level);

  @override
  Widget build(BuildContext context) {
    final color = switch (level) {
      SensitivityLevel.public => Colors.green,
      SensitivityLevel.internal => Colors.blue,
      SensitivityLevel.sensitive => Colors.orange,
      SensitivityLevel.critical => Colors.red,
      null => Colors.grey,
    };

    return Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
      ),
    );
  }
}

/// 字段访问状态
enum AccessStatus {
  ok,       // 实际敏感度 ≤ 要求
  exceeded, // 实际敏感度 > 要求
  missing,  // 字段不存在
}

/// 单个字段的审查状态
class FieldAccessStatus {
  final String? fieldKey;       // 机器 key
  final String? fieldLabel;     // 显示标签
  final String? semanticType;   // 语义类型 ID
  final String? sectionName;    // 所在分区名称
  final SensitivityLevel? actualSensitivity;    // 实际敏感度
  final SensitivityLevel? requiredSensitivity;  // 插件要求
  final AccessStatus status;

  FieldAccessStatus({
    this.fieldKey,
    this.fieldLabel,
    this.semanticType,
    this.sectionName,
    this.actualSensitivity,
    this.requiredSensitivity,
    required this.status,
  });
}
