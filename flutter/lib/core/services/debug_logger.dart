import 'dart:io';
import 'package:path_provider/path_provider.dart';

/// Simple file-based debug logger for troubleshooting
class DebugLogger {
  static final DebugLogger _instance = DebugLogger._();
  static DebugLogger get instance => _instance;

  DebugLogger._();

  File? _logFile;

  Future<void> init() async {
    if (_logFile != null) return;
    final dir = await getApplicationSupportDirectory();
    _logFile = File('${dir.path}/solosoul_debug.log');
    await _logFile!.writeAsString('=== Debug log started at ${DateTime.now()} ===\n');
  }

  void log(String message) {
    print('[DEBUG_LOGGER] $message');
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
