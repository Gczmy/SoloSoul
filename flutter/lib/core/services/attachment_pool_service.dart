// ===========================================================================
// AttachmentPoolService — 备份附件引用池
// ===========================================================================
// 所有备份共享一个全局附件池，以 fileId 为键存储加密后的 .solo 文件。
// 备份 sidecar 目录中只保存 manifest（引用的 fileId 列表），不再复制附件。
// 删除备份时通过扫描所有 manifest 的「懒引用计数」来清理未引用的池文件。
//
// 目录结构：
//   {appSupportDir}/solosoul_backups/{accountId}/attachments_pool/
//     ├── {fileId}.solo
//     └── ...
// ===========================================================================

import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'package:solosoul_flutter/core/utils/solo_log.dart';

class AttachmentPoolService {
  AttachmentPoolService._();
  static final AttachmentPoolService instance = AttachmentPoolService._();

  static const _backupDirName = 'solosoul_backups';
  static const _poolDirName = 'attachments_pool';

  // -------------------------------------------------------------------------
  // 目录管理
  // -------------------------------------------------------------------------

  Future<Directory> _poolDir(String accountId) async {
    final appSupportDir = await getApplicationSupportDirectory();
    final dir = Directory(
      '${appSupportDir.path}/$_backupDirName/$accountId/$_poolDirName',
    );
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  Future<String> _poolFilePath(String accountId, String fileId) async {
    final dir = await _poolDir(accountId);
    return '${dir.path}/$fileId.solo';
  }

  // -------------------------------------------------------------------------
  // 核心 API
  // -------------------------------------------------------------------------

  /// 检查池中是否已存在指定 fileId 的附件。
  Future<bool> poolFileExists(String accountId, String fileId) async {
    final path = await _poolFilePath(accountId, fileId);
    return File(path).exists();
  }

  /// 将源附件文件复制到池中（若已存在则跳过）。
  /// 返回 true 表示成功放入池中（或已存在）。
  Future<bool> ensureInPool(
    String accountId,
    String fileId,
    String srcPath,
  ) async {
    final poolPath = await _poolFilePath(accountId, fileId);
    final poolFile = File(poolPath);
    if (await poolFile.exists()) {
      return true;
    }

    final srcFile = File(srcPath);
    if (!await srcFile.exists()) {
      SoloLog.w('AttachmentPool', 'Source file not found: $srcPath');
      return false;
    }

    try {
      await srcFile.copy(poolPath);
      SoloLog.d('AttachmentPool', 'Copied to pool: $fileId');
      return true;
    } on Exception catch (e) {
      SoloLog.e('AttachmentPool', 'Failed to copy $fileId to pool: $e');
      return false;
    }
  }

  /// 从池中复制附件到目标路径。
  /// 返回 true 表示成功。若池中不存在该 fileId，返回 false。
  Future<bool> getFromPool(
    String accountId,
    String fileId,
    String dstPath,
  ) async {
    final poolPath = await _poolFilePath(accountId, fileId);
    final poolFile = File(poolPath);
    if (!await poolFile.exists()) {
      SoloLog.w('AttachmentPool', 'Pool file missing: $fileId');
      return false;
    }

    try {
      await poolFile.copy(dstPath);
      SoloLog.d('AttachmentPool', 'Restored from pool: $fileId -> $dstPath');
      return true;
    } on Exception catch (e) {
      SoloLog.e('AttachmentPool', 'Failed to restore $fileId: $e');
      return false;
    }
  }

  /// 从池中删除指定 fileId 的附件（无引用时调用）。
  Future<void> removeFromPool(String accountId, String fileId) async {
    final poolPath = await _poolFilePath(accountId, fileId);
    final poolFile = File(poolPath);
    if (await poolFile.exists()) {
      try {
        await poolFile.delete();
        SoloLog.d('AttachmentPool', 'Removed from pool: $fileId');
      } on Exception catch (e) {
        SoloLog.w('AttachmentPool', 'Failed to remove $fileId: $e');
      }
    }
  }

  // -------------------------------------------------------------------------
  // 统计与查询
  // -------------------------------------------------------------------------

  /// 获取池中所有 fileId。
  Future<Set<String>> getPoolFileIds(String accountId) async {
    final dir = await _poolDir(accountId);
    if (!await dir.exists()) return const {};

    final fileIds = <String>{};
    await for (final entity in dir.list()) {
      if (entity is! File) continue;
      final name = entity.uri.pathSegments.last;
      if (!name.endsWith('.solo')) continue;
      fileIds.add(name.substring(0, name.length - 5));
    }
    return fileIds;
  }

  /// 获取池目录总大小（字节）。
  Future<int> getPoolSize(String accountId) async {
    final dir = await _poolDir(accountId);
    if (!await dir.exists()) return 0;

    int total = 0;
    await for (final entity in dir.list()) {
      if (entity is File) {
        total += await entity.length();
      }
    }
    return total;
  }

  /// 获取池中指定 fileId 的文件大小（字节），不存在返回 0。
  Future<int> getPoolFileSize(String accountId, String fileId) async {
    final poolPath = await _poolFilePath(accountId, fileId);
    final poolFile = File(poolPath);
    if (await poolFile.exists()) {
      return await poolFile.length();
    }
    return 0;
  }
}
