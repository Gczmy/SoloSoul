import 'package:flutter/foundation.dart';

enum LogLevel { debug, info, warning, error }

/// A single log entry with structured data.
class LogEntry {
  final DateTime timestamp;
  final String tag;
  final LogLevel level;
  final String message;

  LogEntry({
    required this.timestamp,
    required this.tag,
    required this.level,
    required this.message,
  });

  String get _levelLabel {
    switch (level) {
      case LogLevel.debug:
        return 'DEBUG';
      case LogLevel.info:
        return 'INFO';
      case LogLevel.warning:
        return 'WARN';
      case LogLevel.error:
        return 'ERROR';
    }
  }

  /// Plain text line for export/copy.
  String toLine() {
    return '[${timestamp.toIso8601String()}] [$_levelLabel] [$tag] $message';
  }
}

/// Memory-based debug logger for troubleshooting.
/// Only active when debugModeProvider is true (user-activated in release builds).
/// Automatically clears on dispose for "burn after reading" behavior.
class DebugLogger {
  static final DebugLogger _instance = DebugLogger._();
  static DebugLogger get instance => _instance;

  DebugLogger._();

  final List<LogEntry> _logBuffer = [];
  bool _isActive = false;

  static const int _maxBufferSize = 1000;

  /// Patterns for sanitization - sensitive fields that should be redacted
  static final List<RegExp> _sensitivePatterns = [
    RegExp(r'(password|secret|key|token|auth)[=:]\s*[\w\-]+', caseSensitive: false),
    RegExp(r'"password"\s*:\s*"[^"]+"', caseSensitive: false),
    RegExp(r'"secret"\s*:\s*"[^"]+"', caseSensitive: false),
    RegExp(r'"vault_key"\s*:\s*"[^"]+"', caseSensitive: false),
    RegExp(r'"access_token"\s*:\s*"[^"]+"', caseSensitive: false),
    RegExp(r'"refresh_token"\s*:\s*"[^"]+"', caseSensitive: false),
  ];

  /// Activate logging - must be called when debugModeProvider becomes true
  void activate() {
    _isActive = true;
    _logBuffer.clear();
    log('DEBUG_MODE_ENABLED', 'Debug logging activated', LogLevel.info);
  }

  /// Deactivate and clear all logs - "burn after reading"
  void deactivate() {
    log('DEBUG_MODE_DISABLED', 'Debug logging deactivated, buffer cleared', LogLevel.info);
    _isActive = false;
    _logBuffer.clear();
  }

  /// Check if logging is active
  bool get isActive => _isActive;

  List<LogEntry> get entries => List.unmodifiable(_logBuffer);

  /// Get all buffered logs as a single string (for export)
  String getExportLog() {
    if (_logBuffer.isEmpty) {
      return 'No debug logs available.';
    }
    return _logBuffer.map((e) => e.toLine()).join('\n');
  }

  /// Sanitize a message by redacting sensitive patterns
  String _sanitize(String message) {
    String sanitized = message;
    for (final pattern in _sensitivePatterns) {
      sanitized = sanitized.replaceAllMapped(pattern, (match) {
        final matchStr = match.group(0) ?? '';
        final keyEnd = matchStr.indexOf(':');
        if (keyEnd > 0) {
          final key = matchStr.substring(0, keyEnd).trim();
          return '$key: ***';
        }
        return 'redacted: ***';
      });
    }
    return sanitized;
  }

  void _log(String tag, String message, LogLevel level) {
    if (!_isActive) return;

    final sanitized = _sanitize(message);
    final entry = LogEntry(
      timestamp: DateTime.now(),
      tag: tag,
      level: level,
      message: sanitized,
    );

    _logBuffer.add(entry);

    // Prevent unbounded memory growth
    if (_logBuffer.length > _maxBufferSize) {
      _logBuffer.removeAt(0);
    }

    // Also print to console in debug mode
    if (kDebugMode) {
      // ignore: avoid_print
      print(entry.toLine());
    }
  }

  void log(String tag, String message, [LogLevel level = LogLevel.debug]) {
    _log(tag, message, level);
  }

  void logError(String tag, String message) {
    _log(tag, message, LogLevel.error);
  }

  void logInfo(String tag, String message) {
    _log(tag, message, LogLevel.info);
  }

  void logDebug(String tag, String message) {
    _log(tag, message, LogLevel.debug);
  }

  void logWarning(String tag, String message) {
    _log(tag, message, LogLevel.warning);
  }
}
