import 'package:flutter/foundation.dart';

/// Memory-based debug logger for troubleshooting.
/// Only active when debugModeProvider is true (user-activated in release builds).
/// Automatically clears on dispose for "burn after reading" behavior.
class DebugLogger {
  static final DebugLogger _instance = DebugLogger._();
  static DebugLogger get instance => _instance;

  DebugLogger._();

  final List<String> _logBuffer = [];
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
    _log('DEBUG_MODE_ENABLED', 'Debug logging activated');
  }

  /// Deactivate and clear all logs - "burn after reading"
  void deactivate() {
    _log('DEBUG_MODE_DISABLED', 'Debug logging deactivated, buffer cleared');
    _isActive = false;
    _logBuffer.clear();
  }

  /// Check if logging is active
  bool get isActive => _isActive;

  /// Get all buffered logs as a single string (for export)
  String getExportLog() {
    if (_logBuffer.isEmpty) {
      return 'No debug logs available.';
    }
    return _logBuffer.join('\n');
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

  void _log(String tag, String message) {
    if (!_isActive) return;

    final sanitized = _sanitize(message);
    final entry = '[${DateTime.now().toIso8601String()}] [$tag] $sanitized';

    _logBuffer.add(entry);

    // Prevent unbounded memory growth
    if (_logBuffer.length > _maxBufferSize) {
      _logBuffer.removeAt(0);
    }

    // Also print to console in debug mode
    if (kDebugMode) {
      // ignore: avoid_print
      print(entry);
    }
  }

  void log(String tag, String message) {
    _log(tag, message);
  }

  void logError(String tag, String message) {
    _log(tag, 'ERROR: $message');
  }

  void logInfo(String tag, String message) {
    _log(tag, 'INFO: $message');
  }

  void logDebug(String tag, String message) {
    _log(tag, 'DEBUG: $message');
  }

  void logWarning(String tag, String message) {
    _log(tag, 'WARNING: $message');
  }
}
