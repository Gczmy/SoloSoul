import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';

/// 通用 OCR 结果预览组件
///
/// 显示识别出的文本块列表，每个块包含文本、置信度和相对坐标。
class OcrResultPreview extends StatelessWidget {
  final OcrResult result;

  const OcrResultPreview({super.key, required this.result});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 整体置信度
        Row(
          children: [
            Icon(
              Icons.analytics_outlined,
              size: 18,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Text(
              'Confidence: ${(result.confidence * 100).toStringAsFixed(1)}%',
              style: theme.textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
            ),
            const Spacer(),
            Text(
              '${result.blocks.length} blocks',
              style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        const Divider(),
        const SizedBox(height: 8),

        // 文本块列表
        ...result.blocks.asMap().entries.map((entry) {
          final index = entry.key;
          final block = entry.value;
          return _OcrBlockTile(
            index: index + 1,
            block: block,
          );
        }),

        const SizedBox(height: 12),
        const Divider(),
        const SizedBox(height: 8),

        // 原始文本
        Text(
          'Full Text',
          style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
        ),
        const SizedBox(height: 4),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: SelectableText(
            result.rawText,
            style: theme.textTheme.bodyMedium,
          ),
        ),
      ],
    );
  }
}

class _OcrBlockTile extends StatelessWidget {
  final int index;
  final OcrBlock block;

  const _OcrBlockTile({required this.index, required this.block});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final confidenceColor = _confidenceColor(block.confidence, theme);

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primaryContainer,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    '#$index',
                    style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onPrimaryContainer,
                          fontWeight: FontWeight.w600,
                        ),
                  ),
                ),
                const Spacer(),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: confidenceColor.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    '${(block.confidence * 100).toStringAsFixed(0)}%',
                    style: theme.textTheme.labelSmall?.copyWith(
                          color: confidenceColor,
                          fontWeight: FontWeight.w600,
                        ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            SelectableText(
              block.text,
              style: theme.textTheme.bodyLarge,
            ),
            const SizedBox(height: 4),
            Text(
              'x:${block.bbox.x.toStringAsFixed(2)} '
              'y:${block.bbox.y.toStringAsFixed(2)} '
              'w:${block.bbox.width.toStringAsFixed(2)} '
              'h:${block.bbox.height.toStringAsFixed(2)}',
              style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                    fontFamily: 'monospace',
                  ),
            ),
          ],
        ),
      ),
    );
  }

  Color _confidenceColor(double confidence, ThemeData theme) {
    if (confidence >= 0.9) return Colors.green;
    if (confidence >= 0.7) return Colors.orange;
    return theme.colorScheme.error;
  }
}
