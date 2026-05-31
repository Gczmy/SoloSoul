import 'dart:io';

import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Attachment Download Service
// =============================================================================

/// Downloads decrypted attachments to the local filesystem.
///
/// Default download directory is the system Downloads folder.
/// Handles filename conflicts by appending incremental numbers.
class AttachmentDownloadService {
  static final AttachmentDownloadService _instance =
      AttachmentDownloadService._internal();
  factory AttachmentDownloadService() => _instance;
  AttachmentDownloadService._internal();

  static const _kDownloadPathKey = 'solosoul_download_path';

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
  /// Returns the final saved file path on success, null on failure.
  Future<String?> downloadAttachment({
    required String accountId,
    required Attachment attachment,
    required Directory downloadDir,
  }) async {
    try {
      // Ensure download directory exists
      if (!await downloadDir.exists()) {
        await downloadDir.create(recursive: true);
      }

      // Decrypt attachment
      final bytes = await AttachmentStorageService().loadAttachment(
        accountId: accountId,
        fileId: attachment.fileId,
      );
      if (bytes == null) {
        SoloLog.e('AttachmentDownload', 'Failed to load attachment: ${attachment.fileId}');
        return null;
      }

      // Resolve unique filename
      final targetPath = resolveUniquePath(downloadDir.path, attachment.fileName);

      // Write to disk
      final file = File(targetPath);
      await file.writeAsBytes(bytes);

      SoloLog.d(
        'AttachmentDownload',
        'Saved attachment: ${attachment.fileName} → $targetPath (${bytes.length} bytes)',
      );
      return targetPath;
    } on Exception catch (e, stackTrace) {
      SoloLog.e('AttachmentDownload', 'Download failed', e, stackTrace);
      return null;
    }
  }
}
