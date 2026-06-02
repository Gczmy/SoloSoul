import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';

/// Shows a legal document (Privacy Policy or Terms of Service) in a scrollable sheet
Future<void> showLegalDocumentSheet({
  required BuildContext context,
  required String title,
  required String assetPath,
}) async {
  String content = '';
  try {
    content = await rootBundle.loadString(assetPath);
  } on Exception catch (e) {
    content = 'Error loading document: $e';
  }

  if (!context.mounted) return;

  unawaited(showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: Colors.transparent,
    builder: (context) => _LegalDocumentSheet(
      title: title,
      content: content,
    ),
  ));
}

class _LegalDocumentSheet extends StatelessWidget {
  final String title;
  final String content;

  const _LegalDocumentSheet({
    required this.title,
    required this.content,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final mediaQuery = MediaQuery.of(context);

    return Container(
      height: mediaQuery.size.height * 0.9,
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        children: [
          // Handle bar
          Container(
            margin: const EdgeInsets.only(top: 12),
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),

          // Header
          Padding(
            padding: const EdgeInsets.all(20),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    title,
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                IconButton(
                  onPressed: () => Navigator.pop(context),
                  icon: const Icon(Icons.close),
                  style: IconButton.styleFrom(
                    backgroundColor: theme.colorScheme.surfaceContainerHighest,
                  ),
                ),
              ],
            ),
          ),

          const Divider(height: 1),

          // Content
          Expanded(
            child: Markdown(
              data: content,
              padding: const EdgeInsets.all(20),
              styleSheet: MarkdownStyleSheet(
                // 一级标题：大号 + 底部细线
                h1: theme.textTheme.headlineMedium?.copyWith(
                  fontWeight: FontWeight.w700,
                  color: theme.colorScheme.onSurface,
                ),
                h1Padding: const EdgeInsets.only(bottom: 8),

                // 二级标题：蓝色（Notion 风格）
                h2: theme.textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: const Color(0xFF487CA5),
                ),
                h2Padding: const EdgeInsets.only(top: 16, bottom: 8),

                // 引用块 / Callout：浅蓝背景 + 圆角 + 左侧蓝色竖线
                blockquote: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                  height: 1.5,
                ),
                blockquotePadding: const EdgeInsets.all(12),
                blockquoteDecoration: const BoxDecoration(
                  color: Color(0xFFE9F3F7),
                  borderRadius: BorderRadius.all(Radius.circular(8)),
                  border: Border(
                    left: BorderSide(
                      color: Color(0xFF487CA5),
                      width: 4,
                    ),
                  ),
                ),

                // 表格：表头加粗 + 细边框
                tableHead: theme.textTheme.bodyMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: theme.colorScheme.onSurface,
                ),
                tableBody: theme.textTheme.bodyMedium,
                tableBorder: TableBorder.all(
                  color: theme.colorScheme.outlineVariant,
                  width: 0.5,
                ),
                tableHeadAlign: TextAlign.center,
                tableCellsPadding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                tableColumnWidth: const FlexColumnWidth(),

                // 列表：统一缩进
                listIndent: 28,
                listBulletPadding: const EdgeInsets.only(right: 12),

                // 分隔线：1px 细线 + 垂直间距
                horizontalRuleDecoration: BoxDecoration(
                  border: Border(
                    top: BorderSide(
                      color: theme.colorScheme.outlineVariant,
                      width: 1,
                    ),
                  ),
                ),

                // 代码块：背景 + 圆角
                code: theme.textTheme.bodyMedium?.copyWith(
                  backgroundColor: theme.colorScheme.surfaceContainerHighest,
                  fontFamily: 'monospace',
                  color: theme.colorScheme.secondary,
                ),
                codeblockPadding: const EdgeInsets.all(12),
                codeblockDecoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),

                // 段落：舒适行高
                p: theme.textTheme.bodyMedium?.copyWith(
                  height: 1.7,
                  color: theme.colorScheme.onSurface,
                ),
                pPadding: const EdgeInsets.only(bottom: 8),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
