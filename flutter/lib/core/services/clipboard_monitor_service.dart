import 'dart:async';
import 'package:flutter/services.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';

/// Monitors clipboard for sensitive data and auto-clears after a delay.
/// Uses Timer to delay the clear operation.
class ClipboardMonitorService {
  static ClipboardMonitorService? _instance;
  Timer? _clearTimer;

  ClipboardMonitorService._();

  static ClipboardMonitorService get instance {
    _instance ??= ClipboardMonitorService._();
    return _instance!;
  }

  /// Notify that sensitive data was copied to clipboard.
  /// Starts a timer to clear the clipboard after the configured delay.
  Future<void> notifySensitiveCopied() async {
    _clearTimer?.cancel();

    // Get delay from SecurityService if available, otherwise use default
    final delaySeconds = SecurityService.instance.isInitialized
        ? SecurityService.instance.settings.clipboardClearDelaySeconds
        : 60;

    // -1 means "Never" - don't clear
    if (delaySeconds < 0) return;

    _clearTimer = Timer(Duration(seconds: delaySeconds), () {
      clearClipboard();
    });
  }

  /// Clears the system clipboard.
  Future<void> clearClipboard() async {
    await Clipboard.setData(const ClipboardData(text: ''));
  }

  /// Cancels any pending clear operation.
  void cancelPendingClear() {
    _clearTimer?.cancel();
    _clearTimer = null;
  }

  /// Disposes the service and cancels any pending operations.
  void dispose() {
    cancelPendingClear();
  }
}
