import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter/services.dart';

// =============================================================================
// LLM Chat Bubble
// =============================================================================

/// 用户或 AI 消息气泡。
///
/// 消息文本支持基础 Markdown 样式（粗体 `**text**`、行内代码 `` `code` ``）。
/// 通过 [_renderMessage] 方法抽象渲染逻辑，未来可无缝迁移到 flutter_markdown。
/// AI 消息气泡下方提供一键复制按钮。
class LlmChatBubble extends StatelessWidget {
  /// 消息内容。
  final String message;

  /// `true` 表示用户发送的消息，`false` 表示 AI 回复。
  final bool isUser;

  /// 是否正在流式接收中。为 `true` 时在末尾显示脉冲点。
  final bool isStreaming;

  /// 消息创建时间戳（毫秒级 Unix epoch）。为 0 时不显示时间。
  final int createdAt;

  const LlmChatBubble({
    super.key,
    required this.message,
    required this.isUser,
    this.isStreaming = false,
    this.createdAt = 0,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final bgColor = isUser
        ? theme.colorScheme.primaryContainer
        : theme.colorScheme.surfaceContainerHighest;

    final align = isUser ? CrossAxisAlignment.end : CrossAxisAlignment.start;

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.8,
        ),
        child: Column(
          crossAxisAlignment: align,
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
              decoration: BoxDecoration(
                color: bgColor,
                borderRadius: BorderRadius.circular(16),
              ),
              child: _renderMessage(message, theme, isUser),
            ),
            if (createdAt > 0) ...[
              const SizedBox(height: 2),
              _TimeLabel(timestamp: createdAt),
            ],
            if (!isUser && !isStreaming && message.isNotEmpty) ...[
              const SizedBox(height: 4),
              _CopyButton(text: message),
            ],
            if (isStreaming) ...[
              const SizedBox(height: 4),
              _TypingDots(color: theme.colorScheme.primary),
            ],
          ],
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // Message rendering — abstracted for future markdown migration
  // ---------------------------------------------------------------------------

  /// 渲染消息文本。当前手动解析粗体和行内代码；未来可替换为 MarkdownBody。
  Widget _renderMessage(String text, ThemeData theme, bool isUser) {
    final defaultStyle = theme.textTheme.bodyMedium?.copyWith(
      color: isUser
          ? theme.colorScheme.onPrimaryContainer
          : theme.colorScheme.onSurface,
    );

    final spans = _parseSpans(text, defaultStyle, theme);

    return SelectableText.rich(
      TextSpan(children: spans),
      style: defaultStyle,
    );
  }

  List<InlineSpan> _parseSpans(
    String text,
    TextStyle? defaultStyle,
    ThemeData theme,
  ) {
    final spans = <InlineSpan>[];
    final buffer = StringBuffer();

    // 合并正则：按顺序匹配 bold 或 code
    final combinedRe = RegExp(r'\*\*(.+?)\*\*|`([^`]+)`');

    var lastIndex = 0;
    for (final match in combinedRe.allMatches(text)) {
      // 添加匹配前的普通文本
      if (match.start > lastIndex) {
        buffer.write(text.substring(lastIndex, match.start));
      }
      if (buffer.isNotEmpty) {
        spans.add(TextSpan(text: buffer.toString(), style: defaultStyle));
        buffer.clear();
      }

      if (match.group(1) != null) {
        // Bold
        spans.add(TextSpan(
          text: match.group(1),
          style: defaultStyle?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ));
      } else if (match.group(2) != null) {
        // Inline code — 使用 TextSpan + 背景色模拟，兼容 SelectableText.rich
        spans.add(TextSpan(
          text: match.group(2),
          style: defaultStyle?.copyWith(
            backgroundColor: theme.colorScheme.surfaceContainerHighest,
            fontFamily: 'monospace',
            fontSize: (defaultStyle.fontSize ?? 14) * 0.9,
            color: theme.colorScheme.secondary,
          ),
        ));
      }

      lastIndex = match.end;
    }

    // 剩余文本
    if (lastIndex < text.length) {
      buffer.write(text.substring(lastIndex));
    }
    if (buffer.isNotEmpty) {
      spans.add(TextSpan(text: buffer.toString(), style: defaultStyle));
    }

    return spans;
  }
}

// =============================================================================
// Copy Button
// =============================================================================

class _CopyButton extends StatefulWidget {
  final String text;

  const _CopyButton({required this.text});

  @override
  State<_CopyButton> createState() => _CopyButtonState();
}

class _CopyButtonState extends State<_CopyButton> {
  bool _copied = false;

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: widget.text));
    if (!mounted) return;
    setState(() => _copied = true);
    Future.delayed(const Duration(seconds: 2), () {
      if (!mounted) return;
      setState(() => _copied = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: _copy,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              _copied ? Icons.check : Icons.copy,
              size: 12,
              color: _copied
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 4),
            Text(
              _copied
                  ? AppLocalizations.of(context).llmCopied
                  : AppLocalizations.of(context).llmCopy,
              style: theme.textTheme.labelSmall?.copyWith(
                color: _copied
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Typing Dots (pulse animation while streaming)
// =============================================================================

class _TypingDots extends StatefulWidget {
  final Color color;

  const _TypingDots({required this.color});

  @override
  State<_TypingDots> createState() => _TypingDotsState();
}

class _TypingDotsState extends State<_TypingDots>
    with TickerProviderStateMixin {
  late final List<AnimationController> _controllers;

  @override
  void initState() {
    super.initState();
    _controllers = List.generate(3, (i) {
      return AnimationController(
        vsync: this,
        duration: const Duration(milliseconds: 600),
      )..repeat(reverse: true, period: Duration(milliseconds: 900 + i * 150));
    });
  }

  @override
  void dispose() {
    for (final c in _controllers) {
      c.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: List.generate(3, (i) {
        return AnimatedBuilder(
          animation: _controllers[i],
          builder: (context, child) {
            return Container(
              margin: const EdgeInsets.symmetric(horizontal: 2),
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                color: widget.color.withValues(
                  alpha: 0.3 + 0.7 * _controllers[i].value,
                ),
                shape: BoxShape.circle,
              ),
            );
          },
        );
      }),
    );
  }
}

// =============================================================================
// Time Label
// =============================================================================

class _TimeLabel extends StatelessWidget {
  final int timestamp;

  const _TimeLabel({required this.timestamp});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dt = DateTime.fromMillisecondsSinceEpoch(timestamp);
    final l10n = AppLocalizations.of(context);
    final timeStr = _formatTime(dt, l10n);

    return Text(
      timeStr,
      style: theme.textTheme.labelSmall?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
        fontSize: 11,
      ),
    );
  }

  String _formatTime(DateTime dt, AppLocalizations? l10n) {
    final hour = dt.hour.toString().padLeft(2, '0');
    final minute = dt.minute.toString().padLeft(2, '0');
    final timePart = '$hour:$minute';

    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final msgDay = DateTime(dt.year, dt.month, dt.day);
    final diffDays = today.difference(msgDay).inDays;

    if (diffDays == 0) return timePart;
    if (diffDays == 1) {
      return '${l10n?.timeYesterday ?? '昨天'} $timePart';
    }
    if (dt.year == now.year) {
      return '${dt.month}/${dt.day} $timePart';
    }
    return '${dt.year}/${dt.month}/${dt.day} $timePart';
  }
}
