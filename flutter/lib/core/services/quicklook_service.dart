import 'dart:io';

import 'package:flutter/services.dart';
import 'package:open_filex/open_filex.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// QuickLook Service
// =============================================================================

/// Cross-platform file preview service.
///
/// - **macOS**: Uses native QLPreviewPanel via Platform Channel (supports
///   PPTX multi-page browse, zoom, page flip — same as Finder Space preview).
/// - **iOS / Android / Windows / Linux**: Falls back to `open_filex` which
///   opens the system default app.
class QuickLookService {
  static final QuickLookService _instance = QuickLookService._internal();
  factory QuickLookService() => _instance;
  QuickLookService._internal();

  static const _channel = MethodChannel('solosoul/quicklook');

  /// Callback invoked when the native QuickLook panel is closed (macOS only).
  void Function(String filePath)? _onClosedCallback;

  /// Initialize the service. Must be called once at app startup (macOS only).
  void initialize() {
    if (!Platform.isMacOS) return;
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'onQuickLookClosed':
          final filePath = call.arguments as String?;
          if (filePath != null) {
            SoloLog.d('QuickLook', 'Panel closed for: $filePath');
            _onClosedCallback?.call(filePath);
          }
          break;
      }
    });
  }

  /// Register a callback to be called when the QuickLook panel is closed.
  void setOnClosedCallback(void Function(String filePath) callback) {
    _onClosedCallback = callback;
  }

  /// Show file preview.
  ///
  /// On macOS, opens QLPreviewPanel. On other platforms, falls back to
  /// `open_filex`.
  ///
  /// [filePath] must be an absolute path to an existing file.
  Future<bool> show(String filePath) async {
    if (Platform.isMacOS) {
      try {
        final result = await _channel.invokeMethod('showQuickLook', {
          'filePath': filePath,
        });
        return result == true;
      } on Exception catch (e) {
        SoloLog.e('QuickLook', 'Failed to show QuickLook panel', e);
        return false;
      }
    }
    return false;
  }

  /// Show file preview with fallback to `open_filex`.
  ///
  /// On macOS, tries QLPreviewPanel first; if it fails, falls back to
  /// `open_filex`. On all other platforms, directly uses `open_filex`.
  ///
  /// [onClosed] is called when the preview is dismissed (macOS only).
  Future<void> showWithFallback(
    String filePath, {
    VoidCallback? onClosed,
  }) async {
    if (Platform.isMacOS) {
      if (onClosed != null) {
        setOnClosedCallback((_) => onClosed());
      }
      final success = await show(filePath);
      if (success) return;
      SoloLog.w('QuickLook', 'QuickLook failed, falling back to open_filex');
    }

    // Fallback for all platforms
    final result = await OpenFilex.open(filePath);
    if (result.type != ResultType.done) {
      throw Exception('Failed to open file: ${result.message}');
    }
  }
}
