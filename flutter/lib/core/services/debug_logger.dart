import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

/// Simple file-based debug logger for troubleshooting.
/// Only active in debug mode to prevent privacy leaks in release builds.
class DebugLogger {
  static final DebugLogger _instance = DebugLogger._();
  static DebugLogger get instance => _instance;

  DebugLogger._();

  File? _logFile;

  Future<void> init() async {
    if (!kDebugMode) return;
    if (_logFile != null) return;
    final dir = await getApplicationSupportDirectory();
    _logFile = File('${dir.path}/solosoul_debug.log');
    await _logFile!.writeAsString('=== Debug log started at ${DateTime.now()} ===\n');
  }

  void log(String message) {
    if (!kDebugMode) return;
    if (_logFile == null) return;
    final entry = '[${DateTime.now()}] $message\n';
    _logFile!.writeAsString(entry, mode: FileMode.append);
  }

  void logError(String tag, String message) {
    log('[$tag] ERROR: $message');
  }

  void logInfo(String tag, String message) {
    log('[$tag] INFO: $message');
  }
}
