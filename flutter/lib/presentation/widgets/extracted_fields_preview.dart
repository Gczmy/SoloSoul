import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';
import 'package:solosoul_flutter/core/utils/field_label_resolver.dart';

/// 展示规则引擎提取出的结构化字段卡片，支持 checkbox 选择导入。
class ExtractedFieldsPreview extends StatelessWidget {
  final ExtractionResult result;

  /// 当前被选中的字段 key 集合。
  final Set<String> selectedKeys;

  /// 用户点击某个字段的 checkbox 时回调。
  final ValueChanged<String> onToggle;

  const ExtractedFieldsPreview({
    super.key,
    required this.result,
    required this.selectedKeys,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 文档类型徽章
        Container(
          margin: const EdgeInsets.only(bottom: 12),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          decoration: BoxDecoration(
            color: theme.colorScheme.primaryContainer,
            borderRadius: BorderRadius.circular(20),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                _typeIcon(result.documentType),
                size: 16,
                color: theme.colorScheme.onPrimaryContainer,
              ),
              const SizedBox(width: 6),
              Text(
                _typeLabel(result.documentType),
                style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onPrimaryContainer,
                      fontWeight: FontWeight.w600,
                    ),
              ),
            ],
          ),
        ),

        // 字段卡片（带 checkbox）
        if (result.hasFields) ...[
          ...result.fields.entries.map((entry) {
            final isSelected = selectedKeys.contains(entry.key);
            return _FieldCard(
              label: _fieldLabel(entry.key),
              value: entry.value.value,
              isSelected: isSelected,
              onToggle: () => onToggle(entry.key),
            );
          }),
        ] else ...[
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Text(
              'No structured fields detected. The text will be saved as a note.',
              style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
              textAlign: TextAlign.center,
            ),
          ),
        ],
      ],
    );
  }

  IconData _typeIcon(String type) {
    return switch (type) {
      'business_card' => Icons.contact_page_outlined,
      'invoice' => Icons.receipt_outlined,
      'resume' => Icons.badge_outlined,
      _ => Icons.document_scanner_outlined,
    };
  }

  String _typeLabel(String type) {
    return switch (type) {
      'business_card' => 'Business Card Detected',
      'invoice' => 'Invoice Detected',
      'resume' => 'Resume Detected',
      _ => 'Document Detected',
    };
  }

  String _fieldLabel(String key) {
    // 统一通过 FieldLabelResolver 解析，消除硬编码映射
    return FieldLabelResolver.resolve(key);
  }
}

class _FieldCard extends StatelessWidget {
  final String label;
  final String value;
  final bool isSelected;
  final VoidCallback onToggle;

  const _FieldCard({
    required this.label,
    required this.value,
    required this.isSelected,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: InkWell(
        onTap: onToggle,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 12),
          child: Row(
            children: [
              Checkbox(
                value: isSelected,
                onChanged: (_) => onToggle(),
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
              const SizedBox(width: 4),
              Expanded(
                flex: 2,
                child: Text(
                  label,
                  style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                ),
              ),
              Expanded(
                flex: 3,
                child: Text(
                  value,
                  style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                  textAlign: TextAlign.end,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
