import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart'
    show DebugLogger, LogLevel, LogEntry;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Debug log bottom sheet for settings page.
class DebugLogSheet extends StatefulWidget {
  final ScrollController scrollController;
  final Future<void> Function() onDisableDebugMode;

  const DebugLogSheet({
    super.key,
    required this.scrollController,
    required this.onDisableDebugMode,
  });

  @override
  State<DebugLogSheet> createState() => DebugLogSheetState();
}

class DebugLogSheetState extends State<DebugLogSheet> {
  List<LogEntry> _entries = [];

  @override
  void initState() {
    super.initState();
    _loadLog();
  }

  void _loadLog() {
    setState(() {
      _entries = DebugLogger.instance.entries;
    });
  }

  Future<void> _copyToClipboard() async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Copy Logs to Clipboard'),
        content: const Text(
          'Logs will be sanitized before copying, but clipboard content '
          'is accessible to all apps on this device.\n\n'
          'The clipboard should be cleared after use.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Copy'),
          ),
        ],
      ),
    );
    if (confirm != true) return;

    final text = DebugLogger.instance.getExportLog();
    await Clipboard.setData(ClipboardData(text: text));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: const Row(
          children: [
            Icon(Icons.check_circle, color: Colors.white, size: 20),
            SizedBox(width: 12),
            Text('Sanitized logs copied to clipboard'),
          ],
        ),
        backgroundColor: AppTheme.successColor,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        margin: const EdgeInsets.all(16),
        duration: const Duration(seconds: 4),
      ),
    );
  }



  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 16),

          // Warning banner
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 24),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.red.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.red.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: Colors.red.shade700, size: 20),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Debug mode is active. Logs are being recorded.',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: Colors.red.shade700,
                    ),
                  ),
                ),
              ],
            ),
          ),

          const SizedBox(height: 16),

          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Debug Log', style: theme.textTheme.titleLarge),
                Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.refresh),
                      onPressed: _loadLog,
                      tooltip: 'Refresh',
                    ),
                    IconButton(
                      icon: const Icon(Icons.copy),
                      onPressed: () async {
                        await _copyToClipboard();
                        if (context.mounted) Navigator.pop(context);
                      },
                      tooltip: 'Copy to clipboard',
                    ),
                    IconButton(
                      icon: const Icon(Icons.power_settings_new),
                      onPressed: () async {
                        await widget.onDisableDebugMode();
                        if (context.mounted) Navigator.pop(context);
                      },
                      tooltip: 'Disable debug mode',
                      color: Colors.red,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const Divider(),
          Expanded(
            child: SingleChildScrollView(
              controller: widget.scrollController,
              padding: const EdgeInsets.all(16),
              child: _LogTextWidget(entries: _entries),
            ),
          ),
        ],
      ),
    );
  }
}

class _LogTextWidget extends StatelessWidget {
  final List<LogEntry> entries;

  const _LogTextWidget({required this.entries});

  Color _levelColor(LogLevel level) {
    switch (level) {
      case LogLevel.error:
        return Colors.red.shade700;
      case LogLevel.warning:
        return Colors.orange.shade700;
      case LogLevel.info:
        return Colors.blue.shade700;
      case LogLevel.debug:
        return Colors.grey.shade600;
    }
  }

  TextStyle _levelStyle(LogLevel level, Color baseColor) {
    return TextStyle(
      fontFamily: 'monospace',
      fontSize: 11,
      color: baseColor,
      fontWeight: FontWeight.w600,
    );
  }

  TextStyle _normalStyle(Color baseColor) {
    return TextStyle(
      fontFamily: 'monospace',
      fontSize: 11,
      color: baseColor,
    );
  }

  @override
  Widget build(BuildContext context) {
    if (entries.isEmpty) {
      return Text(
        'No debug logs available.',
        style: TextStyle(
          fontFamily: 'monospace',
          fontSize: 11,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      );
    }

    final baseColor = Theme.of(context).colorScheme.onSurface;
    final spans = <TextSpan>[];

    for (final entry in entries) {
      final color = _levelColor(entry.level);
      spans.add(TextSpan(
        text: '[${entry.timestamp.toIso8601String()}] ',
        style: _normalStyle(baseColor),
      ));
      spans.add(TextSpan(
        text: '[${entry.level.name.toUpperCase()}] ',
        style: _levelStyle(entry.level, color),
      ));
      spans.add(TextSpan(
        text: '[${entry.tag}] ',
        style: _normalStyle(baseColor),
      ));
      spans.add(TextSpan(
        text: '${entry.message}\n',
        style: _normalStyle(baseColor),
      ));
    }

    return SelectableText.rich(
      TextSpan(children: spans),
    );
  }
}
