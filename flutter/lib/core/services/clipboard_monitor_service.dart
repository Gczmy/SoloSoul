import 'dart:async';
import 'package:flutter/services.dart';

/// Monitors clipboard for sensitive data and auto-clears after a delay.
/// Uses Timer to delay the clear operation.
class ClipboardMonitorService {
  static ClipboardMonitorService? _instance;
  Timer? _clearTimer;

  /// Default delay before clearing clipboard (30 seconds).
  /// Will be replaced by SecurityService.instance.clipboardClearDelay when available.
  static const Duration _defaultClearDelay = Duration(seconds: 30);

  ClipboardMonitorService._();

  static ClipboardMonitorService get instance {
    _instance ??= ClipboardMonitorService._();
    return _instance!;
  }

  /// Notify that sensitive data was copied to clipboard.
  /// Starts a timer to clear the clipboard after the configured delay.
  Future<void> notifySensitiveCopied() async {
    // Cancel any existing timer
    _clearTimer?.cancel();

    // Start new timer to clear clipboard
    _clearTimer = Timer(_defaultClearDelay, () {
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
