import 'package:solosoul_flutter/core/services/debug_logger.dart';

/// Unified logging utility for SoloSoul.
///
/// Uses DebugLogger internally - only active when debug mode is enabled.
/// Provides structured logging with emoji severity indicators.
///
/// Usage:
///   SoloLog.d("TAG", "Info message");
///   SoloLog.w("TAG", "Warning message", error);
///   SoloLog.e("TAG", "Error message", error, stackTrace);
///   SoloLog.startTimer("TAG", "Operation name") -> returns a TimerHandle
///   SoloLog.endTimer(handle) -> logs duration
class SoloLog {
  SoloLog._();

  /// Internal stopwatch for timing operations
  static final Map<String, Stopwatch> _stopwatches = {};

  /// Debug/Info level log - ℹ️
  static void d(String tag, String message) {
    DebugLogger.instance.logInfo(tag, 'ℹ️ $message');
  }

  /// Warning level log - ⚠️
  static void w(String tag, String message, [Object? error]) {
    if (error != null) {
      DebugLogger.instance.logWarning(tag, '⚠️ $message | Error: $error');
    } else {
      DebugLogger.instance.logWarning(tag, '⚠️ $message');
    }
  }

  /// Error level log - ❌
  static void e(
    String tag,
    String message, [
    Object? error,
    StackTrace? stackTrace,
  ]) {
    String fullMessage = '❌ $message';
    if (error != null) {
      fullMessage += ' | Error: $error';
    }
    if (stackTrace != null) {
      fullMessage += '\nStack: $stackTrace';
    }
    DebugLogger.instance.logError(tag, fullMessage);
  }

  /// Debug level log (alias for d) - 🔍
  static void debug(String tag, String message) {
    DebugLogger.instance.logDebug(tag, '🔍 $message');
  }

  /// Start a named timer. Returns a handle to endTimer().
  /// Example:
  ///   final handle = SoloLog.startTimer("Auth", "Keychain read");
  ///   // ... operation ...
  ///   SoloLog.endTimer(handle);
  static String startTimer(String tag, String operationName) {
    final key = '$tag:$operationName';
    _stopwatches[key] = Stopwatch()..start();
    DebugLogger.instance.logDebug(tag, '🔍 [$operationName] started');
    return key;
  }

  /// End a timer and log the duration.
  /// Returns the elapsed milliseconds.
  static int endTimer(String handle) {
    final stopwatch = _stopwatches.remove(handle);
    if (stopwatch == null) {
      DebugLogger.instance.logWarning('SoloLog', '⚠️ Timer not found: $handle');
      return 0;
    }
    stopwatch.stop();
    final elapsed = stopwatch.elapsedMilliseconds;

    final parts = handle.split(':');
    final tag = parts[0];
    final operationName = parts.sublist(1).join(':');

    if (elapsed > 1000) {
      DebugLogger.instance.logWarning(tag, '⚠️ [$operationName] took ${elapsed}ms (SLOW)');
    } else if (elapsed > 100) {
      DebugLogger.instance.logDebug(tag, '🔍 [$operationName] took ${elapsed}ms');
    } else {
      DebugLogger.instance.logDebug(tag, '🔍 [$operationName] took ${elapsed}ms');
    }
    return elapsed;
  }

  /// Log with timing suffix based on a pre-existing elapsed duration.
  /// Use this when you already know how long something took.
  static void dWithTiming(String tag, String message, int elapsedMs) {
    final timingStr = elapsedMs > 1000
        ? '⚠️ (${elapsedMs}ms)'
        : '(${elapsedMs}ms)';
    DebugLogger.instance.logInfo(tag, 'ℹ️ $message $timingStr');
  }
}
