enum LogLevel { debug, info, warning, error }

/// Categories of sensitive data for structured tagging.
enum SensitiveType {
  /// Cryptographic keys, hashes, salts
  crypto,

  /// Passwords, PINs, passphrases
  credential,

  /// Session tokens, access tokens, refresh tokens
  token,

  /// Account IDs, user IDs
  identifier,

  /// File paths, directory paths
  path,

  /// Generic sensitive data (fallback)
  generic,
}

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

  /// Structured sensitive tag pattern: [[SENSITIVE:type:value]]
  /// This is the primary sanitization mechanism - callers explicitly tag data.
  static final RegExp _sensitiveTagPattern =
      RegExp(r'\[\[SENSITIVE:\w+:[^\]]*\]\]');

  /// Patterns for sanitization - safety net for untagged sensitive data.
  /// These catch cases where callers forget to use structured tags.
  static final List<RegExp> _sensitivePatterns = [
    // Key=value patterns (password, secret, key, token, auth, salt, hash)
    RegExp(
      r'(password|secret|key|token|auth|salt|hash|verify)[=:]\s*[\w\-]+',
      caseSensitive: false,
    ),
    // JSON field patterns
    RegExp(
      r'"(password|secret|vault_key|access_token|refresh_token|salt|verify_hash|session_key)"\s*:\s*"[^"]+"',
      caseSensitive: false,
    ),
    // Account ID patterns (acc_xxx)
    RegExp(r'acc_[a-f0-9\-]{20,}', caseSensitive: false),
    // File path patterns (vaultRoot, config.json, solosoul paths)
    RegExp(
      r'(\/[^\s]*?(?:solosoul|vault|config\.json|Application Support)[^\s]*)',
      caseSensitive: false,
    ),
    // Long hex strings (>16 chars, likely keys/hashes)
    RegExp(r'\b[a-f0-9]{16,}\b', caseSensitive: false),
    // Long base64 strings (>32 chars)
    RegExp(r'[A-Za-z0-9+/]{32,}={0,2}', caseSensitive: false),
    // Password length logging
    RegExp(r'pwdLen[=:]\s*\d+', caseSensitive: false),
    // Salt length logging
    RegExp(r'saltLen[=:]\s*\d+', caseSensitive: false),
  ];

  /// Activate logging - must be called when debugModeProvider becomes true
  void activate() {
    _isActive = true;
    _logBuffer.clear();
    log('DEBUG_MODE_ENABLED', 'Debug logging activated', LogLevel.info);
  }

  /// Deactivate and clear all logs - "burn after reading"
  void deactivate() {
    log(
      'DEBUG_MODE_DISABLED',
      'Debug logging deactivated, buffer cleared',
      LogLevel.info,
    );
    _isActive = false;
    _logBuffer.clear();
  }

  /// Check if logging is active
  bool get isActive => _isActive;

  List<LogEntry> get entries => List.unmodifiable(_logBuffer);

  /// Wrap a sensitive value with a structured tag.
  ///
  /// Use this to explicitly mark sensitive data before logging:
  /// ```dart
  /// logger.logInfo('AUTH', 'Unlocked ${redact(accountId, SensitiveType.identifier)}');
  /// ```
  static String redact(String value, SensitiveType type) {
    return '[[SENSITIVE:${type.name}:$value]]';
  }

  /// Get all buffered logs as a sanitized string (for export).
  /// Messages are double-sanitized at export time to catch any patterns
  /// that were added to _sensitivePatterns after the log was recorded.
  String getExportLog() {
    if (_logBuffer.isEmpty) {
      return 'No debug logs available.';
    }
    return _logBuffer.map((e) {
      final sanitizedMessage = _sanitize(e.message);
      return '[${e.timestamp.toIso8601String()}] '
          '[${e.level.name.toUpperCase()}] '
          '[${e.tag}] $sanitizedMessage';
    }).join('\n');
  }

  /// Sanitize a message by redacting sensitive patterns.
  ///
  /// First strips structured [[SENSITIVE:type:value]] tags,
  /// then applies regex-based safety net patterns.
  String _sanitize(String message) {
    String sanitized = message;

    // Step 1: Strip structured sensitive tags (primary mechanism)
    sanitized = sanitized.replaceAllMapped(_sensitiveTagPattern, (match) {
      final tag = match.group(0) ?? '';
      // Extract type from [[SENSITIVE:type:value]]
      final typeMatch = RegExp(r'\[\[SENSITIVE:(\w+):').firstMatch(tag);
      final type = typeMatch?.group(1) ?? 'generic';
      return '[REDACTED:$type]';
    });

    // Step 2: Apply regex safety net for untagged sensitive data
    for (final pattern in _sensitivePatterns) {
      sanitized = sanitized.replaceAllMapped(pattern, (match) {
        final matchStr = match.group(0) ?? '';
        final keyEnd = matchStr.indexOf(':');
        if (keyEnd > 0) {
          final key = matchStr.substring(0, keyEnd).trim();
          return '$key: [REDACTED]';
        }
        return '[REDACTED]';
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

    // Print to console only when user-activated debug mode is on.
    // Do NOT gate on kDebugMode — that would expose sensitive auth logs
    // in debug/profile builds without user consent.
    if (_isActive) {
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
