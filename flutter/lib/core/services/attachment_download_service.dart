import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/attachment_task_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/utils/file_path_resolver.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Attachment Download Service
// =============================================================================

/// Downloads decrypted attachments to the local filesystem.
///
/// Default download directory is the system Downloads folder.
/// Handles filename conflicts by appending incremental numbers.
///
/// Supports two download modes:
/// - **小文件**（≤ 10MB）：内存中一次性解密（v2），通过 [loadAttachment]
/// - **大文件**（> 10MB）：Rust 端流式分块解密（v3），通过 [decryptAttachmentToPath]
class AttachmentDownloadService {
  static final AttachmentDownloadService _instance =
      AttachmentDownloadService._internal();
  factory AttachmentDownloadService() => _instance;
  AttachmentDownloadService._internal();

  static const _kDownloadPathKey = 'solosoul_download_path';

  /// 流式解密阈值：> 此大小使用 Rust 端流式解密（v3）
  static const int _streamThreshold = 10 * 1024 * 1024;

  /// Returns the configured download directory, or the system Downloads folder
  /// if no custom path has been set.
  Future<Directory> getDownloadDirectory() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final customPath = prefs.getString(_kDownloadPathKey);
      if (customPath != null && customPath.isNotEmpty) {
        final dir = Directory(customPath);
        if (await dir.exists()) return dir;
      }
    } on Exception catch (e) {
      SoloLog.w('AttachmentDownload', 'Failed to read custom download path: $e');
    }
    return getDefaultDownloadDirectory();
  }

  /// Returns the default system Downloads directory.
  /// Falls back to application documents directory if unavailable.
  Future<Directory> getDefaultDownloadDirectory() async {
    try {
      final dir = await getDownloadsDirectory();
      if (dir != null) return dir;
    } on Exception catch (e) {
      SoloLog.w('AttachmentDownload', 'getDownloadsDirectory failed: $e');
    }
    return getApplicationDocumentsDirectory();
  }

  /// Persists a custom download directory path.
  Future<void> setDownloadDirectory(String path) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kDownloadPathKey, path);
  }

  /// Clears the custom download directory, reverting to default.
  Future<void> clearDownloadDirectory() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_kDownloadPathKey);
  }

  /// Returns the currently configured download path for display.
  /// Returns null if using the default.
  Future<String?> getCustomDownloadPath() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_kDownloadPathKey);
  }

  /// Resolves a unique file path by handling naming conflicts.
  ///
  /// Examples:
  ///   report.pdf        → report.pdf (if available)
  ///   report.pdf        → report (1).pdf (if report.pdf exists)
  ///   report (2).pdf    → report (3).pdf (if report (2).pdf exists)
  String resolveUniquePath(String dir, String fileName) {
    final candidate = p.join(dir, fileName);
    final file = File(candidate);
    if (!file.existsSync()) return candidate;

    final base = p.basenameWithoutExtension(fileName);
    final ext = p.extension(fileName);

    // Check if filename already ends with " (n)"
    final match = RegExp(r' \((\d+)\)$').firstMatch(base);
    final prefix = match != null ? base.substring(0, match.start) : base;
    final start = match != null ? int.parse(match.group(1)!) + 1 : 1;

    for (int i = start;; i++) {
      final newName = ext.isNotEmpty ? '$prefix ($i)$ext' : '$prefix ($i)';
      final newPath = p.join(dir, newName);
      if (!File(newPath).existsSync()) return newPath;
    }
  }

  /// Downloads a decrypted attachment to the specified directory.
  ///
  /// If [downloadDir] is not writable (e.g. macOS sandbox revoked access
  /// after app restart), falls back to the system Downloads directory.
  ///
  /// [onProgress] 在关键阶段被调用：0.2（读取）、0.8（解密完成）、
  /// 0.9（写入下载目录）、1.0（完成）。
  /// [cancelToken] 在阶段间检查，若已取消则清理已写入的文件。
  ///
  /// Returns the final saved file path on success, null on failure.
  Future<String?> downloadAttachment({
    required String accountId,
    required Attachment attachment,
    required Directory downloadDir,
    ValueChanged<double>? onProgress,
    CancelToken? cancelToken,
    String? progressPath,
    String? cancelPath,
  }) async {
    Directory effectiveDir = downloadDir;

    // Verify directory is writable (handles macOS sandbox revocation)
    final isWritable = await _verifyWritable(downloadDir);
    if (!isWritable) {
      SoloLog.w(
        'AttachmentDownload',
        'Directory not writable: ${downloadDir.path}. Falling back to default.',
      );
      effectiveDir = await getDefaultDownloadDirectory();
    }

    // Ensure download directory exists
    if (!await effectiveDir.exists()) {
      await effectiveDir.create(recursive: true);
    }

    // Resolve unique filename
    final targetPath = resolveUniquePath(effectiveDir.path, attachment.fileName);

    // 判断走小文件内存路径还是大文件流式路径
    if (attachment.size > _streamThreshold) {
      return _downloadLargeFile(
        accountId: accountId,
        attachment: attachment,
        targetPath: targetPath,
        onProgress: onProgress,
        cancelToken: cancelToken,
        progressPath: progressPath,
        cancelPath: cancelPath,
      );
    } else {
      return _downloadSmallFile(
        accountId: accountId,
        attachment: attachment,
        targetPath: targetPath,
        onProgress: onProgress,
        cancelToken: cancelToken,
      );
    }
  }

  /// 小文件下载：内存中一次性解密（v2）
  Future<String?> _downloadSmallFile({
    required String accountId,
    required Attachment attachment,
    required String targetPath,
    ValueChanged<double>? onProgress,
    CancelToken? cancelToken,
  }) async {
    try {
      // Decrypt attachment (with progress)
      final bytes = await AttachmentStorageService().loadAttachment(
        accountId: accountId,
        fileId: attachment.fileId,
        onProgress: onProgress,
        cancelToken: cancelToken,
      );
      if (bytes == null) {
        SoloLog.e('AttachmentDownload', 'Failed to load attachment: ${attachment.fileId}');
        return null;
      }

      // Write to disk
      final file = File(targetPath);
      await file.writeAsBytes(bytes);
      onProgress?.call(0.90);

      // Check cancellation after write
      if (cancelToken?.isCancelled ?? false) {
        if (await file.exists()) {
          await file.delete();
          SoloLog.d('AttachmentDownload', 'Download cancelled, deleted: $targetPath');
        }
        throw Exception('Download cancelled');
      }

      onProgress?.call(1.0);
      SoloLog.d(
        'AttachmentDownload',
        'Saved attachment: ${attachment.fileName} → $targetPath (${bytes.length} bytes)',
      );
      return targetPath;
    } on Exception catch (e, stackTrace) {
      // Clean up partial file
      final partialFile = File(targetPath);
      if (await partialFile.exists()) {
        await partialFile.delete();
      }
      SoloLog.e('AttachmentDownload', 'Download failed', e, stackTrace);
      return null;
    }
  }

  /// 大文件下载：Rust 端流式分块解密（v3）
  Future<String?> _downloadLargeFile({
    required String accountId,
    required Attachment attachment,
    required String targetPath,
    ValueChanged<double>? onProgress,
    CancelToken? cancelToken,
    String? progressPath,
    String? cancelPath,
  }) async {
    // Create progress and cancel files
    final tempDir = await getTemporaryDirectory();
    final uuid = const Uuid().v4();
    final actualProgressPath = progressPath ?? '${tempDir.path}/dl_progress_$uuid.txt';
    final actualCancelPath = cancelPath ?? '${tempDir.path}/dl_cancel_$uuid.txt';

    // Start progress polling Timer
    Timer? progressTimer;
    progressTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
      final pf = File(actualProgressPath);
      if (!await pf.exists()) return;
      try {
        final content = await pf.readAsString();
        final progress = double.tryParse(content.trim()) ?? 0.0;
        onProgress?.call(progress);
      } on Exception catch (_) {
        // Ignore read errors
      }
    });

    try {
      final success = await AttachmentStorageService().decryptAttachmentToPath(
        accountId: accountId,
        fileId: attachment.fileId,
        dstPath: targetPath,
        progressPath: actualProgressPath,
        cancelPath: actualCancelPath,
      );

      if (!success) {
        throw Exception('File decryption failed');
      }

      onProgress?.call(1.0);
      SoloLog.d(
        'AttachmentDownload',
        'Saved large attachment: ${attachment.fileName} → $targetPath',
      );
      return targetPath;
    } on Exception catch (e, stackTrace) {
      if (cancelToken?.isCancelled ?? false) {
        SoloLog.d('AttachmentDownload', 'Download cancelled: ${attachment.fileName}');
      } else {
        SoloLog.e('AttachmentDownload', 'Large file download failed', e, stackTrace);
      }
      // Clean up partial file
      final partialFile = File(targetPath);
      if (await partialFile.exists()) {
        await partialFile.delete();
      }
      return null;
    } finally {
      progressTimer.cancel();
      await FilePathResolver.cleanup(actualProgressPath);
      await FilePathResolver.cleanup(actualCancelPath);
    }
  }

  /// Verifies that a directory is writable by creating and deleting a test file.
  Future<bool> _verifyWritable(Directory dir) async {
    if (!await dir.exists()) return false;
    try {
      final testFile = File(p.join(dir.path, '.solosoul_write_test'));
      await testFile.writeAsString('test', flush: true);
      await testFile.delete();
      return true;
    } on Exception {
      return false;
    }
  }
}
