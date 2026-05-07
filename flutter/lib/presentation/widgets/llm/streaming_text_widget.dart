import 'dart:async';

import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

// =============================================================================
// Streaming Text Widget
// =============================================================================

/// 接收 [Stream<String>] 并逐字渲染的文本组件，带打字机光标动画。
///
/// 当底层 Stream 结束时光标自动消失；若发生错误则显示错误状态。
/// Widget dispose 时自动取消订阅，防止内存泄漏。
class StreamingTextWidget extends StatefulWidget {
  /// 文本流。每个事件为增量字符或片段。
  final Stream<String> stream;

  /// 基础文本样式。
  final TextStyle? style;

  /// 光标颜色。默认为主题 primary 色。
  final Color? cursorColor;

  /// 光标闪烁周期。
  final Duration cursorBlinkDuration;

  /// 最大行数限制。
  final int? maxLines;

  const StreamingTextWidget({
    super.key,
    required this.stream,
    this.style,
    this.cursorColor,
    this.cursorBlinkDuration = const Duration(milliseconds: 530),
    this.maxLines,
  });

  @override
  State<StreamingTextWidget> createState() => _StreamingTextWidgetState();
}

class _StreamingTextWidgetState extends State<StreamingTextWidget> {
  final StringBuffer _buffer = StringBuffer();
  StreamSubscription<String>? _sub;
  bool _isDone = false;
  bool _hasError = false;
  String? _errorMessage;
  bool _showCursor = true;

  @override
  void initState() {
    super.initState();
    _subscribe();
    _startCursorBlink();
  }

  @override
  void didUpdateWidget(covariant StreamingTextWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.stream != widget.stream) {
      _sub?.cancel();
      _buffer.clear();
      _isDone = false;
      _hasError = false;
      _errorMessage = null;
      _subscribe();
    }
  }

  void _subscribe() {
    _sub = widget.stream.listen(
      (chunk) {
        if (!mounted) return;
        setState(() {
          _buffer.write(chunk);
        });
      },
      onError: (Object err) {
        if (!mounted) return;
        setState(() {
          _hasError = true;
          _errorMessage = err.toString();
          _isDone = true;
        });
      },
      onDone: () {
        if (!mounted) return;
        setState(() => _isDone = true);
      },
    );
  }

  void _startCursorBlink() {
    Future.doWhile(() async {
      if (!mounted) return false;
      await Future.delayed(widget.cursorBlinkDuration);
      if (!mounted) return false;
      if (_isDone) {
        setState(() => _showCursor = false);
        return false;
      }
      setState(() => _showCursor = !_showCursor);
      return true;
    });
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final textStyle = widget.style ?? theme.textTheme.bodyMedium;
    final color = widget.cursorColor ?? theme.colorScheme.primary;

    if (_hasError) {
      return Text(
        '⚠️ ${AppLocalizations.of(context).llmInferenceError}: $_errorMessage',
        style: textStyle?.copyWith(color: theme.colorScheme.error),
      );
    }

    return RichText(
      maxLines: widget.maxLines,
      overflow: TextOverflow.ellipsis,
      text: TextSpan(
        style: textStyle,
        children: [
          TextSpan(text: _buffer.toString()),
          if (!_isDone)
            WidgetSpan(
              alignment: PlaceholderAlignment.middle,
              child: AnimatedOpacity(
                opacity: _showCursor ? 1.0 : 0.0,
                duration: const Duration(milliseconds: 100),
                child: Container(
                  width: 2,
                  height: (textStyle?.fontSize ?? 14) * 1.2,
                  color: color,
                ),
              ),
            ),
        ],
      ),
    );
  }
}
