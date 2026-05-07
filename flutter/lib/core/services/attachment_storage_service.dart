import 'dart:io';
import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// Attachment Storage Service
// =============================================================================

/// 管理加密附件文件的本地存取。
///
/// 文件内容通过 [RustVaultService.encryptBytes] 使用当前解锁 Vault 的 master key
/// 加密后保存到应用文档目录。元数据（[Attachment]）保存在 [UnifiedObject.attachments] 中。
class AttachmentStorageService {
  static final AttachmentStorageService _instance =
      AttachmentStorageService._internal();
  factory AttachmentStorageService() => _instance;
  AttachmentStorageService._internal();

  static const _attachmentsDirName = 'solosoul_storage/attachments';

  Future<Directory> _getAttachmentsDir(String accountId) async {
    final appDir = await getApplicationDocumentsDirectory();
    final dir = Directory('${appDir.path}/$_attachmentsDirName/$accountId');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  /// 保存附件：加密 bytes 并写入磁盘，返回 [Attachment] 元数据。
  ///
  /// [fileName] 建议使用原始文件名（如 `IMG_12345.jpg`）。
  Future<Attachment> saveAttachment({
    required String accountId,
    required String fileName,
    required Uint8List bytes,
  }) async {
    final fileId = const Uuid().v4();
    final dir = await _getAttachmentsDir(accountId);
    final file = File('${dir.path}/$fileId.solo');

    final encrypted = await RustVaultService.instance.encryptBytes(bytes);
    if (encrypted == null) {
      throw Exception('Attachment encryption failed');
    }
    await file.writeAsBytes(encrypted);

    SoloLog.d(
      'AttachmentStorage',
      'Saved attachment: $fileName (${bytes.length} bytes) -> ${file.path}',
    );

    return Attachment(
      id: const Uuid().v4(),
      fileId: fileId,
      fileName: fileName,
      mimeType: _guessMimeType(fileName),
      size: bytes.length,
      createdAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  /// 读取并解密附件内容。
  Future<Uint8List?> loadAttachment({
    required String accountId,
    required String fileId,
  }) async {
    final dir = await _getAttachmentsDir(accountId);
    final file = File('${dir.path}/$fileId.solo');
    if (!await file.exists()) {
      SoloLog.w('AttachmentStorage', 'Attachment file not found: $fileId');
      return null;
    }

    final encrypted = await file.readAsBytes();
    final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
    if (decrypted == null) {
      SoloLog.e('AttachmentStorage', 'Failed to decrypt attachment: $fileId');
    }
    return decrypted;
  }

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
      _ => 'application/octet-stream',
    };
  }
}
