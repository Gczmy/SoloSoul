import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// File Path Resolver
// =============================================================================

/// 将文件选择器返回的路径解析为 Rust / Dart 可读取的真实路径。
///
/// 跨平台处理：
/// - Desktop（macOS/Windows/Linux）：返回真实路径，直接使用
/// - iOS：FilePicker 通常已复制到临时目录，直接使用
/// - Android：可能返回 `content://` URI，需要复制到应用临时目录
///
/// 返回 `(String realPath, bool isTemporary)`：
/// - `realPath`：可读取的文件路径
/// - `isTemporary`：为 true 时，调用方需在完成后删除该临时文件
class FilePathResolver {
  FilePathResolver._();

  /// 解析文件路径。
  ///
  /// 对于 Android content URI，尝试复制到应用临时目录。
  /// 复制过程流式进行（边读边写），内存可控。
  static Future<(String, bool)> resolve(String uriOrPath) async {
    // Desktop / iOS 真实路径：直接使用
    if (!uriOrPath.startsWith('content://')) {
      return (uriOrPath, false);
    }

    // Android content URI：复制到临时目录
    return await _copyContentUriToTemp(uriOrPath);
  }

  /// 复制 Android content URI 到临时文件。
  ///
  /// 先尝试 Dart `File.copy`（某些系统可能已将 content URI 映射为可读路径）。
  /// 如果失败，使用流式读写手动复制。
  static Future<(String, bool)> _copyContentUriToTemp(String contentUri) async {
    final tempDir = await getTemporaryDirectory();
    final fileName = _extractFileName(contentUri);
    final tempPath =
        '${tempDir.path}/solosoul_temp_${const Uuid().v4()}_$fileName';

    // 尝试 File.copy（某些 Android 版本支持）
    try {
      await File(contentUri).copy(tempPath);
      SoloLog.d('FilePathResolver', 'Copied content URI via File.copy: $contentUri');
      return (tempPath, true);
    } on Exception catch (e) {
      SoloLog.d('FilePathResolver', 'File.copy failed for content URI: $e');
    }

    // 流式手动复制
    try {
      final srcFile = File(contentUri);
      final dstFile = File(tempPath);

      if (!await srcFile.exists()) {
        SoloLog.w('FilePathResolver', 'Content URI not accessible: $contentUri');
        return (contentUri, false);
      }

      // 使用 openRead / openWrite 流式复制
      final srcStream = srcFile.openRead();
      final sink = dstFile.openWrite();

      await for (final chunk in srcStream) {
        sink.add(chunk);
      }
      await sink.close();

      SoloLog.d('FilePathResolver', 'Stream-copied content URI to: $tempPath');
      return (tempPath, true);
    } on Exception catch (e) {
      SoloLog.e('FilePathResolver', 'Failed to copy content URI: $e');
      return (contentUri, false);
    }
  }

  /// 从 URI 或路径中提取文件名。
  static String _extractFileName(String uriOrPath) {
    // 尝试从 content URI 中提取文件名
    // content://com.android.providers.media.documents/document/image%3A12345
    final decoded = Uri.decodeComponent(uriOrPath);
    final segments = decoded.split('/');
    if (segments.isNotEmpty && segments.last.isNotEmpty) {
      return segments.last;
    }
    return 'unknown';
  }

  /// 删除临时文件（安全忽略不存在的情况）。
  static Future<void> cleanup(String? path) async {
    if (path == null || path.isEmpty) return;
    try {
      final file = File(path);
      if (await file.exists()) {
        await file.delete();
        SoloLog.d('FilePathResolver', 'Cleaned up temp file: $path');
      }
    } on Exception catch (e) {
      SoloLog.w('FilePathResolver', 'Failed to cleanup temp file: $e');
    }
  }

  /// 批量删除多个临时文件。
  static Future<void> cleanupAll(List<String> paths) async {
    for (final p in paths) {
      await cleanup(p);
    }
  }
}
