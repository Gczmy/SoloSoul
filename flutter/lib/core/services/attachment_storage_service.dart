import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/attachment_task_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// MIME Type Constants
// =============================================================================

const String mimeTypePptx =
    'application/vnd.openxmlformats-officedocument.presentationml.presentation';
const String mimeTypePpt = 'application/vnd.ms-powerpoint';
const String mimeTypeDocx =
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
const String mimeTypeDoc = 'application/msword';
const String mimeTypeXlsx =
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
const String mimeTypeXls = 'application/vnd.ms-excel';

// =============================================================================
// Attachment Storage Service
// =============================================================================

/// 管理加密附件文件的本地存取。
///
/// 支持两种加密路径：
/// - **小文件**（≤ 50MB）：内存中一次性加密（SOLO blob v2），通过 [saveAttachment] / [loadAttachment]
/// - **大文件**（> 50MB 或任意大小）：Rust 端流式分块加密（SOLO blob v3），通过 [saveAttachmentFromPath] / [decryptAttachmentToPath]
///
/// 文件内容通过 [RustVaultService] 使用当前解锁 Vault 的 master key 加密。
/// 元数据（[Attachment]）保存在 [UnifiedObject.attachments] 中。
class AttachmentStorageService {
  static final AttachmentStorageService _instance =
      AttachmentStorageService._internal();
  factory AttachmentStorageService() => _instance;
  AttachmentStorageService._internal();

  static const _attachmentsDirName = 'solosoul_storage/attachments';

  /// 小文件阈值：≤ 此大小使用一次性内存加密（v2），> 此大小使用流式加密（v3）
  static const int smallFileThreshold = 50 * 1024 * 1024;

  /// 预览上限：> 此大小的 v3 文件拒绝内存加载预览
  static const int maxPreviewSize = 10 * 1024 * 1024;

  Future<Directory> _getAttachmentsDir(String accountId) async {
    final appDir = await getApplicationDocumentsDirectory();
    final dir = Directory('${appDir.path}/$_attachmentsDirName/$accountId');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  // ---------------------------------------------------------------------------
  // Save (small file — in-memory v2)
  // ---------------------------------------------------------------------------

  /// 保存附件：加密 bytes 并写入磁盘，返回 [Attachment] 元数据。
  ///
  /// 适用于小文件（≤ 50MB）。大文件请使用 [saveAttachmentFromPath]。
  ///
  /// [onProgress] 在关键阶段报告离散进度（0.1, 0.7, 0.9, 1.0）。
  /// [cancelToken] 用于在阶段间检查取消请求；加密期间无法中断，
  /// 加密完成后会检查并清理已生成文件。
  Future<Attachment> saveAttachment({
    required String accountId,
    required String fileName,
    required Uint8List bytes,
    ValueChanged<double>? onProgress,
    CancelToken? cancelToken,
  }) async {
    final fileId = const Uuid().v4();
    final dir = await _getAttachmentsDir(accountId);
    final file = File('${dir.path}/$fileId.solo');

    // 阶段 1：准备完成
    onProgress?.call(0.10);

    // 阶段 2：加密（阻塞调用，期间无法取消）
    final encrypted = await RustVaultService.instance.encryptBytes(bytes);
    if (encrypted == null) {
      throw Exception('Attachment encryption failed');
    }
    onProgress?.call(0.70);

    // 检查取消：加密完成后
    if (cancelToken?.isCancelled ?? false) {
      SoloLog.d('AttachmentStorage', 'Upload cancelled after encryption, discarding');
      throw Exception('Upload cancelled');
    }

    // 阶段 3：写入磁盘
    await file.writeAsBytes(encrypted);
    onProgress?.call(0.90);

    // 检查取消：写入完成后
    if (cancelToken?.isCancelled ?? false) {
      if (await file.exists()) {
        await file.delete();
        SoloLog.d('AttachmentStorage', 'Upload cancelled after write, deleted: ${file.path}');
      }
      throw Exception('Upload cancelled');
    }

    SoloLog.d(
      'AttachmentStorage',
      'Saved attachment: $fileName (${bytes.length} bytes) -> ${file.path}',
    );

    onProgress?.call(1.0);
    return Attachment(
      id: const Uuid().v4(),
      fileId: fileId,
      fileName: fileName,
      mimeType: _guessMimeType(fileName),
      size: bytes.length,
      createdAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  // ---------------------------------------------------------------------------
  // Save (large file — streaming v3 from path)
  // ---------------------------------------------------------------------------

  /// 从文件路径保存附件：使用 Rust 端流式分块加密（SOLO blob v3）。
  ///
  /// 适用于大文件或已有文件路径的场景。内存占用 O(1MB)。
  ///
  /// [srcPath] 为源文件路径（必须是 Rust 可读取的真实路径）。
  /// [fileSize] 为已知文件大小（可选，用于元数据）。
  /// [progressPath] 和 [cancelPath] 由调用方管理（临时文件）。
  /// [isSrcTemporary] 为 true 时，无论成功/失败/取消都会删除 [srcPath]。
  Future<Attachment> saveAttachmentFromPath({
    required String accountId,
    required String fileName,
    required String srcPath,
    int? fileSize,
    required String progressPath,
    required String cancelPath,
    bool isSrcTemporary = false,
  }) async {
    final fileId = const Uuid().v4();
    final dir = await _getAttachmentsDir(accountId);
    final dstPath = '${dir.path}/$fileId.solo';

    try {
      final success = await RustVaultService.instance.encryptFile(
        srcPath,
        dstPath,
        progressPath: progressPath,
        cancelPath: cancelPath,
      );

      if (!success) {
        throw Exception('Attachment file encryption failed');
      }

      // 获取实际文件大小（如果未提供）
      final actualSize = fileSize ?? await File(srcPath).length();

      SoloLog.d(
        'AttachmentStorage',
        'Saved attachment from path: $fileName ($actualSize bytes) -> $dstPath',
      );

      return Attachment(
        id: const Uuid().v4(),
        fileId: fileId,
        fileName: fileName,
        mimeType: _guessMimeType(fileName),
        size: actualSize,
        createdAt: DateTime.now().millisecondsSinceEpoch,
      );
    } finally {
      // 清理临时源文件
      if (isSrcTemporary) {
        try {
          await File(srcPath).delete();
        } on Exception catch (e) {
          SoloLog.w('AttachmentStorage', 'Failed to delete temp source: $e');
        }
      }
      // 清理进度和取消标志文件
      try { await File(progressPath).delete(); } on Exception catch (_) {}
      try { await File(cancelPath).delete(); } on Exception catch (_) {}
    }
  }

  // ---------------------------------------------------------------------------
  // Load (small file / preview — in-memory v2)
  // ---------------------------------------------------------------------------

  /// 读取并解密附件内容到内存。
  ///
  /// **限制**：v3 格式且文件大小 > [maxPreviewSize]（10MB）时，
  /// 抛出 [AttachmentTooLargeForPreviewException]，拒绝内存加载。
  /// 对于大文件预览或下载，请使用 [decryptAttachmentToPath]。
  Future<Uint8List?> loadAttachment({
    required String accountId,
    required String fileId,
    ValueChanged<double>? onProgress,
    CancelToken? cancelToken,
  }) async {
    final dir = await _getAttachmentsDir(accountId);
    final file = File('${dir.path}/$fileId.solo');
    if (!await file.exists()) {
      SoloLog.w('AttachmentStorage', 'Attachment file not found: $fileId');
      return null;
    }

    final fileSize = await file.length();

    // 检测 v3 格式：读取前 5 字节检查 Magic + Version
    final header = await file.openRead(0, 5).first;
    final isV3 = header.length >= 5 &&
        header[0] == 0x53 && // 'S'
        header[1] == 0x4F && // 'O'
        header[2] == 0x4C && // 'L'
        header[3] == 0x4F && // 'O'
        header[4] == 0x03;   // v3

    if (isV3 && fileSize > maxPreviewSize) {
      throw AttachmentTooLargeForPreviewException(fileSize);
    }

    // 阶段 1：读取文件
    final encrypted = await file.readAsBytes();
    onProgress?.call(0.20);

    // 检查取消
    if (cancelToken?.isCancelled ?? false) {
      throw Exception('Download cancelled');
    }

    // 阶段 2：解密（阻塞调用）
    final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
    onProgress?.call(0.80);

    if (decrypted == null) {
      SoloLog.e('AttachmentStorage', 'Failed to decrypt attachment: $fileId');
      return null;
    }

    // 检查取消
    if (cancelToken?.isCancelled ?? false) {
      SoloLog.d('AttachmentStorage', 'Download cancelled after decryption, discarding');
      throw Exception('Download cancelled');
    }

    onProgress?.call(1.0);
    return decrypted;
  }

  // ---------------------------------------------------------------------------
  // Decrypt to path (large file — streaming v3)
  // ---------------------------------------------------------------------------

  /// 将附件解密到指定路径（流式，适用于大文件下载和 PPTX 预览）。
  ///
  /// 内存占用 O(1MB)（v3）或文件大小（v2），支持任意大小的 v2/v3 文件。
  Future<bool> decryptAttachmentToPath({
    required String accountId,
    required String fileId,
    required String dstPath,
    required String progressPath,
    required String cancelPath,
  }) async {
    final dir = await _getAttachmentsDir(accountId);
    final srcPath = '${dir.path}/$fileId.solo';
    final srcFile = File(srcPath);

    if (!await srcFile.exists()) {
      SoloLog.w('AttachmentStorage', 'Attachment file not found: $fileId');
      return false;
    }

    return await RustVaultService.instance.decryptFile(
      srcPath,
      dstPath,
      progressPath: progressPath,
      cancelPath: cancelPath,
    );
  }

  // ---------------------------------------------------------------------------
  // Delete / Cleanup
  // ---------------------------------------------------------------------------

  /// 删除附件的加密文件。
  Future<bool> deleteAttachment({
    required String accountId,
    required String fileId,
  }) async {
    final dir = await _getAttachmentsDir(accountId);
    final file = File('${dir.path}/$fileId.solo');
    if (await file.exists()) {
      await file.delete();
      SoloLog.d('AttachmentStorage', 'Deleted attachment file: $fileId');
      return true;
    }
    return false;
  }

  /// 批量删除多个附件文件。
  Future<void> deleteAttachments({
    required String accountId,
    required List<Attachment> attachments,
  }) async {
    for (final a in attachments) {
      await deleteAttachment(accountId: accountId, fileId: a.fileId);
    }
  }

  /// 清理孤儿文件：扫描目录中未被任何对象引用的 .solo 文件并删除。
  /// 建议在应用启动时后台调用。
  Future<int> cleanupOrphanFiles({
    required String accountId,
    required Set<String> referencedFileIds,
  }) async {
    final dir = await _getAttachmentsDir(accountId);
    if (!await dir.exists()) return 0;

    int deletedCount = 0;
    await for (final entity in dir.list()) {
      if (entity is! File) continue;
      final name = entity.uri.pathSegments.last;
      if (!name.endsWith('.solo')) continue;

      final fileId = name.substring(0, name.length - 5); // remove .solo
      if (!referencedFileIds.contains(fileId)) {
        await entity.delete();
        deletedCount++;
        SoloLog.d('AttachmentStorage', 'Cleaned orphan file: $name');
      }
    }
    return deletedCount;
  }

  String _guessMimeType(String fileName) {
    final ext = fileName.contains('.')
        ? fileName.split('.').last.toLowerCase()
        : '';
    return switch (ext) {
      'jpg' || 'jpeg' => 'image/jpeg',
      'png' => 'image/png',
      'gif' => 'image/gif',
      'webp' => 'image/webp',
      'bmp' => 'image/bmp',
      'heic' => 'image/heic',
      'pdf' => 'application/pdf',
      'pptx' => mimeTypePptx,
      'ppt' => mimeTypePpt,
      'docx' => mimeTypeDocx,
      'doc' => mimeTypeDoc,
      'xlsx' => mimeTypeXlsx,
      'xls' => mimeTypeXls,
      _ => 'application/octet-stream',
    };
  }
}

// =============================================================================
// Exceptions
// =============================================================================

/// 附件过大，无法内存加载预览。
class AttachmentTooLargeForPreviewException implements Exception {
  final int fileSize;
  AttachmentTooLargeForPreviewException(this.fileSize);

  @override
  String toString() =>
      'AttachmentTooLargeForPreviewException: file size ${(fileSize / (1024 * 1024)).toStringAsFixed(1)}MB exceeds preview limit';
}
