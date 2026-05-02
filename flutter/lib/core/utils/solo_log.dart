import 'package:flutter/foundation.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

/// Unified logging utility for SoloSoul.
///
/// Uses DebugLogger internally - only active when debug mode is enabled.
/// Provides structured logging with text severity indicators.
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

  static void _consolePrint(String level, String tag, String message) {
    if (kDebugMode) {
      // ignore: avoid_print
      print('[SoloLog] [$level] [$tag] $message');
    }
  }

  /// Debug/Info level log
  static void d(String tag, String message) {
    _consolePrint('INFO', tag, message);
    DebugLogger.instance.logInfo(tag, message);
  }

  /// Warning level log
  static void w(String tag, String message, [Object? error]) {
    final msg = error != null ? '$message | Error: $error' : message;
    _consolePrint('WARN', tag, msg);
    DebugLogger.instance.logWarning(tag, msg);
  }

  /// Error level log
  static void e(
    String tag,
    String message, [
    Object? error,
    StackTrace? stackTrace,
  ]) {
    String fullMessage = message;
    if (error != null) {
      fullMessage += ' | Error: $error';
    }
    if (stackTrace != null) {
      fullMessage += '\nStack: $stackTrace';
    }
    _consolePrint('ERROR', tag, fullMessage);
    DebugLogger.instance.logError(tag, fullMessage);
  }

  /// Debug level log (alias for d)
  static void debug(String tag, String message) {
    _consolePrint('DEBUG', tag, message);
    DebugLogger.instance.logDebug(tag, message);
  }

  /// Start a named timer. Returns a handle to endTimer().
  /// Example:
  ///   final handle = SoloLog.startTimer("Auth", "Keychain read");
  ///   // ... operation ...
  ///   SoloLog.endTimer(handle);
  static String startTimer(String tag, String operationName) {
    final key = '$tag:$operationName';
    _stopwatches[key] = Stopwatch()..start();
    _consolePrint('TIMER', tag, '[$operationName] started');
    DebugLogger.instance.logDebug(tag, '[$operationName] started');
    return key;
  }

  /// End a timer and log the duration.
  /// Returns the elapsed milliseconds.
  static int endTimer(String handle) {
    final stopwatch = _stopwatches.remove(handle);
    if (stopwatch == null) {
      DebugLogger.instance.logWarning('SoloLog', 'Timer not found: $handle');
      return 0;
    }
    stopwatch.stop();
    final elapsed = stopwatch.elapsedMilliseconds;

    final parts = handle.split(':');
    final tag = parts[0];
    final operationName = parts.sublist(1).join(':');

    final level = elapsed > 1000 ? 'WARN' : 'TIMER';
    final suffix = elapsed > 1000 ? ' (SLOW)' : '';
    _consolePrint(level, tag, '[$operationName] took ${elapsed}ms$suffix');

    if (elapsed > 1000) {
      DebugLogger.instance.logWarning(tag, '[$operationName] took ${elapsed}ms (SLOW)');
    } else {
      DebugLogger.instance.logDebug(tag, '[$operationName] took ${elapsed}ms');
    }
    return elapsed;
  }

  /// Log with timing suffix based on a pre-existing elapsed duration.
  /// Use this when you already know how long something took.
  static void dWithTiming(String tag, String message, int elapsedMs) {
    final timingStr = elapsedMs > 1000
        ? '(${elapsedMs}ms SLOW)'
        : '(${elapsedMs}ms)';
    DebugLogger.instance.logInfo(tag, '$message $timingStr');
  }
}
