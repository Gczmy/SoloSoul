// ===========================================================================
// BackupService — 加密备份与恢复（含附件）
// ===========================================================================
// 将用户数据以 Vault 相同的 AES-256-GCM 加密后导出到独立目录，防止 App 更新
// 或异常导致数据丢失。保留最近 N 份备份，支持手动与自动备份。
//
// 备份结构：
//   {appSupportDir}/solosoul_backups/{accountId}/
//     ├── backup_YYYY-MM-DD_HH-mm-ss[_vX.Y.Z].backup          ← 结构化数据
//     ├── backup_YYYY-MM-DD_HH-mm-ss[_vX.Y.Z].backup.attachments/  ← 附件副本
//     │     ├── {fileId}.solo
//     │     └── ...
//     └── special/
//           ├── {name}.backup
//           └── {name}.backup.attachments/
//
// 注意：备份使用与 Vault 相同的加密密钥，恢复时需要 Vault 已解锁。
// ===========================================================================

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:path_provider/path_provider.dart';

import 'package:solosoul_flutter/core/services/attachment_pool_service.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'profile_storage_service.dart';
import 'rust_vault_service.dart';

/// 单条备份信息
class BackupEntry {
  final String fileName;
  final DateTime createdAt;
  final int sizeBytes;
  final bool isSpecial;

  String get displayName => isSpecial ? fileName.replaceAll('.backup', '') : fileName;

  String get displayTime =>
      '${createdAt.year}-${_two(createdAt.month)}-${_two(createdAt.day)} '
      '${_two(createdAt.hour)}:${_two(createdAt.minute)}:${_two(createdAt.second)}';

  String get displaySize {
    if (sizeBytes < 1024) return '${sizeBytes}B';
    if (sizeBytes < 1024 * 1024) return '${(sizeBytes / 1024).toStringAsFixed(1)}KB';
    return '${(sizeBytes / (1024 * 1024)).toStringAsFixed(1)}MB';
  }

  const BackupEntry({
    required this.fileName,
    required this.createdAt,
    required this.sizeBytes,
    this.isSpecial = false,
  });

  static String _two(int n) => n.toString().padLeft(2, '0');
}

class BackupService {
  BackupService._();
  static final BackupService instance = BackupService._();

  static const _backupDirName = 'solosoul_backups';
  static const _specialDirName = 'special';
  static const _filePrefix = 'backup_';
  static const _fileExt = '.backup';
  static const _attachmentsSuffix = '.attachments';

  /// 最多保留的常规备份数量（自动清理旧备份）
  static const int maxBackupCount = 5;

  /// 最多保留的特别备份数量
  static const int maxSpecialBackupCount = 5;

  // -------------------------------------------------------------------------
  // 路径构造
  // -------------------------------------------------------------------------

  Future<Directory> _backupRootDir() async {
    final appSupportDir = await getApplicationSupportDirectory();
    final dir = Directory('${appSupportDir.path}/$_backupDirName');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  /// Set restrictive file permissions (owner-only read/write) on backup files.
  static Future<void> _setRestrictivePermissions(String path) async {
    try {
      final result = await Process.run('chmod', ['600', path]);
      if (result.exitCode != 0) {
        DebugLogger.instance.logWarning(
          'BACKUP',
          'chmod 600 failed for backup file (exit ${result.exitCode}).',
        );
      }
    } on Exception {
      // chmod not available on all platforms — best effort
    }
  }

  Future<Directory> _accountBackupDir(String accountId) async {
    final root = await _backupRootDir();
    final dir = Directory('${root.path}/$accountId');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  Future<Directory> _specialBackupDir(String accountId) async {
    final accountDir = await _accountBackupDir(accountId);
    final dir = Directory('${accountDir.path}/$_specialDirName');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  /// 附件备份目录路径：{backupFile}.backup.attachments/
  String _attachmentsDirPath(String backupFilePath) {
    return '$backupFilePath$_attachmentsSuffix';
  }

  static String backupFileName(DateTime dt, {String? appVersion}) {
    final ts =
        '${dt.year}-${_two(dt.month)}-${_two(dt.day)}_${_two(dt.hour)}-${_two(dt.minute)}-${_two(dt.second)}';
    final versionSuffix = appVersion != null ? '_v$appVersion' : '';
    return '$_filePrefix$ts$versionSuffix$_fileExt';
  }

  /// 清理用户输入的特别备份名称，防止路径遍历
  static String sanitizeSpecialName(String name) {
    return name
        .replaceAll('/', '-')
        .replaceAll('\\', '-')
        .replaceAll('..', '-')
        .trim();
  }

  static String _two(int n) => n.toString().padLeft(2, '0');

  // -------------------------------------------------------------------------
  // Isolate helpers
  // -------------------------------------------------------------------------

  static Uint8List encodeProfileToBytes(Map<String, dynamic> json) {
    final jsonString = jsonEncode(json);
    return Uint8List.fromList(utf8.encode(jsonString));
  }

  static Future<Uint8List> _encodeProfileInIsolate(Map<String, dynamic> json) async {
    // Run on main thread (Isolate.run can hang on macOS Release builds)
    return encodeProfileToBytes(json);
  }

  static ProfileData decodeProfileFromString(String jsonString) {
    final json = jsonDecode(jsonString) as Map<String, dynamic>;
    return ProfileData.fromJson(json);
  }

  static Future<ProfileData> _decodeProfileInIsolate(String jsonString) async {
    // Run on main thread (Isolate.run can hang on macOS Release builds)
    return decodeProfileFromString(jsonString);
  }

  // -------------------------------------------------------------------------
  // Manifest（附件引用清单）
  // -------------------------------------------------------------------------

  static const _manifestFileName = 'manifest.json';

  @visibleForTesting
  static void writeManifest(String sidecarDir, List<String> fileIds) {
    final dir = Directory(sidecarDir);
    if (!dir.existsSync()) {
      dir.createSync(recursive: true);
    }
    final manifest = <String, dynamic>{
      'version': 1,
      'fileIds': fileIds,
    };
    final file = File('$sidecarDir/$_manifestFileName');
    file.writeAsStringSync(jsonEncode(manifest));
  }

  @visibleForTesting
  static List<String> readManifest(String sidecarDir) {
    final file = File('$sidecarDir/$_manifestFileName');
    if (!file.existsSync()) return const [];
    try {
      final content = file.readAsStringSync();
      final json = jsonDecode(content) as Map<String, dynamic>;
      final ids = json['fileIds'];
      if (ids is List) {
        return ids.whereType<String>().toList();
      }
    } on Exception catch (e) {
      SoloLog.w('BACKUP', 'Failed to read manifest at $sidecarDir: $e');
    }
    return const [];
  }

  /// 扫描所有备份（常规 + 特别）的 manifest，汇总仍被引用的 fileId 集合。
  Future<Set<String>> _collectAllReferencedFileIds(String accountId) async {
    final referenced = <String>{};

    // 常规备份
    final regularDir = await _accountBackupDir(accountId);
    if (await regularDir.exists()) {
      await for (final entity in regularDir.list()) {
        if (entity is! File) continue;
        final name = entity.path.split(Platform.pathSeparator).last;
        if (!name.startsWith(_filePrefix) || !name.endsWith(_fileExt)) continue;
        final sidecar = _attachmentsDirPath(entity.path);
        referenced.addAll(readManifest(sidecar));
      }
    }

    // 特别备份
    final specialDir = await _specialBackupDir(accountId);
    if (await specialDir.exists()) {
      await for (final entity in specialDir.list()) {
        if (entity is! File) continue;
        final name = entity.path.split(Platform.pathSeparator).last;
        if (!name.endsWith(_fileExt)) continue;
        final sidecar = _attachmentsDirPath(entity.path);
        referenced.addAll(readManifest(sidecar));
      }
    }

    return referenced;
  }

  /// 旧格式迁移：sidecar 目录包含 .solo 文件但没有 manifest.json。
  /// 将 .solo 文件移至附件池，并生成 manifest.json。
  Future<void> _migrateOldSidecarFormat(
    String accountId,
    String sidecarDir,
  ) async {
    final dir = Directory(sidecarDir);
    if (!await dir.exists()) return;

    // 检查是否已有 manifest
    final manifestFile = File('$sidecarDir/$_manifestFileName');
    if (await manifestFile.exists()) return;

    final fileIds = <String>[];
    await for (final entity in dir.list()) {
      if (entity is! File) continue;
      final name = entity.uri.pathSegments.last;
      if (!name.endsWith('.solo')) continue;
      final fileId = name.substring(0, name.length - 5);
      // 移至附件池
      await AttachmentPoolService.instance.ensureInPool(
        accountId,
        fileId,
        entity.path,
      );
      fileIds.add(fileId);
    }

    // 写入 manifest
    writeManifest(sidecarDir, fileIds);
    DebugLogger.instance.logInfo(
      'BACKUP',
      'Migrated old sidecar format at $sidecarDir (${fileIds.length} files)',
    );
  }

  // -------------------------------------------------------------------------
  // 附件备份 / 恢复 / 删除（引用池模式）
  // -------------------------------------------------------------------------

  /// 将账户附件放入引用池，并在 sidecar 目录写入 manifest。
  Future<void> _backupAttachments(
    String accountId,
    String backupAttachmentsDir,
  ) async {
    final srcFileIds = await AttachmentStorageService().getAttachmentFileIds(accountId);
    if (srcFileIds.isEmpty) {
      // 写入空 manifest
      writeManifest(backupAttachmentsDir, const []);
      return;
    }

    final Directory srcDir;
    try {
      srcDir = await AttachmentStorageService().getAttachmentsDir(accountId);
    } on Exception catch (_) {
      writeManifest(backupAttachmentsDir, const []);
      return;
    }

    final dstDir = Directory(backupAttachmentsDir);
    if (!await dstDir.exists()) {
      await dstDir.create(recursive: true);
    }

    final pooledFileIds = <String>[];
    for (final fileId in srcFileIds) {
      final srcFile = File('${srcDir.path}/$fileId.solo');
      if (await srcFile.exists()) {
        final ok = await AttachmentPoolService.instance.ensureInPool(
          accountId,
          fileId,
          srcFile.path,
        );
        if (ok) pooledFileIds.add(fileId);
      }
    }

    writeManifest(backupAttachmentsDir, pooledFileIds);
    DebugLogger.instance.logInfo(
      'BACKUP',
      'Pooled ${pooledFileIds.length} attachments for backup',
    );
  }

  /// 从 manifest 读取引用，从附件池恢复到账户附件目录。
  Future<void> _restoreAttachments(
    String accountId,
    String backupAttachmentsDir,
  ) async {
    final fileIds = readManifest(backupAttachmentsDir);
    if (fileIds.isEmpty) return;

    final Directory dstDir;
    try {
      dstDir = await AttachmentStorageService().getAttachmentsDir(accountId);
    } on Exception catch (_) {
      return;
    }
    if (!await dstDir.exists()) {
      await dstDir.create(recursive: true);
    }

    int restoredCount = 0;
    int missingCount = 0;
    for (final fileId in fileIds) {
      final ok = await AttachmentPoolService.instance.getFromPool(
        accountId,
        fileId,
        '${dstDir.path}/$fileId.solo',
      );
      if (ok) {
        restoredCount++;
      } else {
        missingCount++;
        SoloLog.w('BACKUP', 'Pool file missing during restore: $fileId');
      }
    }

    if (restoredCount > 0) {
      DebugLogger.instance.logInfo(
        'BACKUP',
        'Restored $restoredCount attachments from pool'
        '${missingCount > 0 ? ' ($missingCount missing)' : ''}',
      );
    }
    if (missingCount > 0) {
      SoloLog.w(
        'BACKUP',
        '$missingCount attachment(s) missing from pool — '
        'may have been manually deleted.',
      );
    }
  }

  /// 删除备份的 sidecar 目录，并清理未被其他备份引用的池文件。
  Future<void> _deleteBackupWithCleanup(
    String accountId,
    String backupFilePath,
  ) async {
    final sidecarDir = _attachmentsDirPath(backupFilePath);

    // 1. 读取被删备份的 manifest
    final deletedFileIds = readManifest(sidecarDir);

    // 2. 删除备份文件
    final file = File(backupFilePath);
    if (await file.exists()) {
      await file.delete();
    }

    // 3. 删除 sidecar 目录
    final dir = Directory(sidecarDir);
    if (await dir.exists()) {
      await dir.delete(recursive: true);
    }

    // 4. 若该备份没有引用任何附件，无需清理池
    if (deletedFileIds.isEmpty) return;

    // 5. 扫描所有剩余备份的 manifest，计算仍被引用的 fileId
    final stillReferenced = await _collectAllReferencedFileIds(accountId);

    // 6. 清理不再被引用的池文件
    for (final fileId in deletedFileIds) {
      if (!stillReferenced.contains(fileId)) {
        await AttachmentPoolService.instance.removeFromPool(accountId, fileId);
      }
    }
  }

  @visibleForTesting
  static String? extractAccountIdFromBackupPath(String backupFilePath) {
    // 路径格式: .../solosoul_backups/{accountId}/backup_...backup
    final parts = backupFilePath.split(Platform.pathSeparator);
    final idx = parts.indexOf(_backupDirName);
    if (idx >= 0 && idx + 1 < parts.length) {
      return parts[idx + 1];
    }
    return null;
  }

  // -------------------------------------------------------------------------
  // 核心：常规备份 / 恢复 / 列表 / 删除
  // -------------------------------------------------------------------------

  /// 立即为当前账户创建一份加密备份（含附件）。
  Future<String?> createBackup(
    String accountId, {
    String? appVersion,
    void Function(double progress)? onProgress,
  }) async {
    void report(double p) => onProgress?.call(p);
    try {
      report(0.1);
      DebugLogger.instance.logInfo('BACKUP', 'Step 1/5: loadProfile start');
      final profileData = await ProfileStorageService.instance.loadProfile(accountId)
          .timeout(const Duration(seconds: 10), onTimeout: () {
        DebugLogger.instance.logError('BACKUP', 'loadProfile timed out');
        throw TimeoutException('loadProfile timed out after 10s');
      });
      if (profileData == null) {
        DebugLogger.instance.logWarning('BACKUP', 'loadProfile returned null');
        return null;
      }
      DebugLogger.instance.logInfo('BACKUP', 'Step 1/5: loadProfile done');

      report(0.3);
      DebugLogger.instance.logInfo('BACKUP', 'Step 2/5: jsonEncode start');
      final plainBytes = await _encodeProfileInIsolate(profileData.toJson());
      DebugLogger.instance.logInfo('BACKUP', 'Step 2/5: jsonEncode done, size=${plainBytes.length} bytes');

      report(0.5);
      DebugLogger.instance.logInfo('BACKUP', 'Step 3/5: encryptBytes start');
      final encrypted = await RustVaultService.instance.encryptBytes(plainBytes);
      if (encrypted == null) {
        DebugLogger.instance.logWarning('BACKUP', 'encryptBytes returned null');
        report(0);
        return null;
      }
      DebugLogger.instance.logInfo('BACKUP', 'Step 3/5: encryptBytes done');

      report(0.7);
      DebugLogger.instance.logInfo('BACKUP', 'Step 4/5: write file + attachments start');
      final dir = await _accountBackupDir(accountId);
      final now = DateTime.now();
      final fileName = BackupService.backupFileName(now, appVersion: appVersion);
      final file = File('${dir.path}/$fileName');
      await file.writeAsBytes(encrypted, flush: true);
      unawaited(_setRestrictivePermissions(file.path));

      // 备份附件
      final attachmentsDir = _attachmentsDirPath(file.path);
      await _backupAttachments(accountId, attachmentsDir);
      DebugLogger.instance.logInfo('BACKUP', 'Step 4/5: write file + attachments done -> $fileName');

      report(1.0);
      DebugLogger.instance.logInfo('BACKUP', 'Step 5/5: cleanup start');
      await _cleanupOldBackups(accountId);
      DebugLogger.instance.logInfo('BACKUP', 'Step 5/5: cleanup done');
      return fileName;
    } on TimeoutException {
      report(0);
      return null;
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('BACKUP', 'createBackup failed: $e\n$st');
      report(0);
      return null;
    }
  }

  /// 列出某账户的所有常规备份，按时间从新到旧排序。
  /// sizeBytes 只包含 .backup 文件本身大小（附件池共享，不重复计算）。
  Future<List<BackupEntry>> listBackups(String accountId) async {
    try {
      final dir = await _accountBackupDir(accountId);
      if (!await dir.exists()) return const [];

      final entries = <BackupEntry>[];
      await for (final entity in dir.list()) {
        if (entity is! File) continue;
        final name = entity.path.split(Platform.pathSeparator).last;
        if (!name.startsWith(_filePrefix) || !name.endsWith(_fileExt)) continue;

        final stat = await entity.stat();
        entries.add(BackupEntry(
          fileName: name,
          createdAt: stat.modified,
          sizeBytes: stat.size,
          isSpecial: false,
        ));
      }

      entries.sort((a, b) => b.createdAt.compareTo(a.createdAt));
      return entries;
    } on FileSystemException catch (_) {
      return const [];
    }
  }

  /// 列出所有备份（常规 + 特殊），按时间从新到旧排序。
  Future<List<BackupEntry>> listAllBackups(String accountId) async {
    final regular = await listBackups(accountId);
    final special = await listSpecialBackups(accountId);
    final all = [...regular, ...special];
    all.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return all;
  }

  /// 从指定备份文件恢复数据到 Vault（含附件）。
  Future<bool> restoreBackup(String accountId, String fileName) async {
    try {
      // 1. 先读取目标备份文件
      final dir = await _accountBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;

      // 1.5 旧格式迁移（若 sidecar 含 .solo 文件但无 manifest）
      await _migrateOldSidecarFormat(accountId, _attachmentsDirPath(file.path));

      final encrypted = await file.readAsBytes();
      final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
      if (decrypted == null) return false;

      final jsonString = utf8.decode(decrypted);
      final hasUnified = jsonString.contains('"unified_objects"');
      final preview = jsonString.length > 500 ? '${jsonString.substring(0, 500)}...' : jsonString;
      DebugLogger.instance.logInfo('BACKUP', 'restoreBackup: file=$fileName, has_unified_objects=$hasUnified, length=${jsonString.length}, preview=$preview');
      final profile = await _decodeProfileInIsolate(jsonString);

      // 2. 创建保护性备份（覆盖数据前）
      await createBackup(accountId);

      // 3. 保存恢复的数据到 Vault
      final saved = await ProfileStorageService.instance.saveProfile(accountId, profile);
      if (!saved) return false;

      // 4. 恢复附件
      final attachmentsDir = _attachmentsDirPath(file.path);
      await _restoreAttachments(accountId, attachmentsDir);

      return true;
    } on Exception catch (_) {
      return false;
    }
  }

  /// 删除某条常规备份（含附件目录及池清理）。
  Future<bool> deleteBackup(String accountId, String fileName) async {
    try {
      final dir = await _accountBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;
      await _deleteBackupWithCleanup(accountId, file.path);
      return true;
    } on FileSystemException catch (_) {
      return false;
    }
  }

  /// 获取最近一次备份的信息。
  Future<BackupEntry?> getLatestBackup(String accountId) async {
    final list = await listBackups(accountId);
    return list.isNotEmpty ? list.first : null;
  }

  /// 获取备份文件的总大小（.backup 文件 + 共享附件池，去重后）。
  Future<int> getTotalBackupSize(String accountId) async {
    final list = await listBackups(accountId);
    final backupFilesSize = list.fold<int>(0, (sum, e) => sum + e.sizeBytes);
    final poolSize = await getAttachmentPoolSize(accountId);
    return backupFilesSize + poolSize;
  }

  /// 获取附件池总大小（字节）。
  Future<int> getAttachmentPoolSize(String accountId) async {
    return AttachmentPoolService.instance.getPoolSize(accountId);
  }

  // -------------------------------------------------------------------------
  // 特别备份
  // -------------------------------------------------------------------------

  /// 创建一份特别备份（含附件）。
  Future<String?> createSpecialBackup(
    String accountId,
    String name, {
    void Function(double progress)? onProgress,
  }) async {
    void report(double p) => onProgress?.call(p);
    final sanitized = BackupService.sanitizeSpecialName(name);
    if (sanitized.isEmpty) return null;

    final specialList = await listSpecialBackups(accountId);
    if (specialList.length >= maxSpecialBackupCount) {
      DebugLogger.instance.logWarning(
          'BACKUP', 'Special backup limit reached ($maxSpecialBackupCount)');
      return null;
    }

    try {
      report(0.1);
      final profileData = await ProfileStorageService.instance.loadProfile(accountId)
          .timeout(const Duration(seconds: 10), onTimeout: () {
        throw TimeoutException('loadProfile timed out after 10s');
      });
      if (profileData == null) return null;

      report(0.3);
      final plainBytes = await _encodeProfileInIsolate(profileData.toJson());

      report(0.5);
      final encrypted = await RustVaultService.instance.encryptBytes(plainBytes);
      if (encrypted == null) {
        report(0);
        return null;
      }

      report(0.7);
      final dir = await _specialBackupDir(accountId);
      final fileName = '$sanitized$_fileExt';
      final file = File('${dir.path}/$fileName');
      await file.writeAsBytes(encrypted, flush: true);
      unawaited(_setRestrictivePermissions(file.path));

      // 备份附件
      final attachmentsDir = _attachmentsDirPath(file.path);
      await _backupAttachments(accountId, attachmentsDir);

      report(1.0);
      return fileName;
    } on TimeoutException {
      report(0);
      return null;
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('BACKUP', 'createSpecialBackup failed: $e\n$st');
      report(0);
      return null;
    }
  }

  /// 列出某账户的所有特别备份。
  /// sizeBytes 只包含 .backup 文件本身大小。
  Future<List<BackupEntry>> listSpecialBackups(String accountId) async {
    try {
      final dir = await _specialBackupDir(accountId);
      if (!await dir.exists()) return const [];

      final entries = <BackupEntry>[];
      await for (final entity in dir.list()) {
        if (entity is! File) continue;
        final name = entity.path.split(Platform.pathSeparator).last;
        if (!name.endsWith(_fileExt)) continue;

        final stat = await entity.stat();
        entries.add(BackupEntry(
          fileName: name,
          createdAt: stat.modified,
          sizeBytes: stat.size,
          isSpecial: true,
        ));
      }

      entries.sort((a, b) => b.createdAt.compareTo(a.createdAt));
      return entries;
    } on FileSystemException catch (_) {
      return const [];
    }
  }

  /// 重命名特别备份（含附件目录）。
  Future<String?> renameSpecialBackup(
    String accountId,
    String oldFileName,
    String newName,
  ) async {
    final sanitized = BackupService.sanitizeSpecialName(newName);
    if (sanitized.isEmpty) return null;

    final newFileName = '$sanitized$_fileExt';
    if (newFileName == oldFileName) return newFileName;

    try {
      final dir = await _specialBackupDir(accountId);
      final oldFile = File('${dir.path}/$oldFileName');
      if (!await oldFile.exists()) return null;

      final newFile = File('${dir.path}/$newFileName');
      if (await newFile.exists()) return null;

      await oldFile.rename(newFile.path);
      // 同时重命名附件目录
      final oldAttachDir = Directory(_attachmentsDirPath(oldFile.path));
      if (await oldAttachDir.exists()) {
        await oldAttachDir.rename(_attachmentsDirPath(newFile.path));
      }
      return newFileName;
    } on FileSystemException catch (_) {
      return null;
    }
  }

  /// 删除某条特别备份（含附件目录及池清理）。
  Future<bool> deleteSpecialBackup(String accountId, String fileName) async {
    try {
      final dir = await _specialBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;
      await _deleteBackupWithCleanup(accountId, file.path);
      return true;
    } on FileSystemException catch (_) {
      return false;
    }
  }

  /// 从特别备份恢复数据到 Vault（含附件）。
  Future<bool> restoreSpecialBackup(String accountId, String fileName) async {
    try {
      final dir = await _specialBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;

      // 旧格式迁移
      await _migrateOldSidecarFormat(accountId, _attachmentsDirPath(file.path));

      final encrypted = await file.readAsBytes();
      final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
      if (decrypted == null) return false;

      final jsonString = utf8.decode(decrypted);
      final hasUnifiedObjects = jsonString.contains('"unified_objects"');
      final hasUnifiedObjectsCamel = jsonString.contains('"unifiedObjects"');
      final hasIdentity = jsonString.contains('"identity"');
      final hasTravel = jsonString.contains('"travel"');
      final hasFinancial = jsonString.contains('"financial"');
      final hasProfessional = jsonString.contains('"professional"');
      DebugLogger.instance.logInfo(
        'BACKUP',
        'restoreSpecialBackup raw JSON: length=${jsonString.length}, '
        'has_unified_objects=$hasUnifiedObjects, has_unifiedObjects=$hasUnifiedObjectsCamel, '
        'has_identity=$hasIdentity, has_travel=$hasTravel, has_financial=$hasFinancial, has_professional=$hasProfessional',
      );
      final json = jsonDecode(jsonString) as Map<String, dynamic>;
      final profile = ProfileData.fromJson(json);
      DebugLogger.instance.logInfo(
        'BACKUP',
        'restoreSpecialBackup parsed: unifiedObjects=${profile.unifiedObjects != null}, '
        'schemaVersion=${profile.schemaVersion}, objectCount=${profile.unifiedObjects?.objects.length ?? 0}',
      );

      // 创建保护性备份（覆盖数据前）
      await createBackup(accountId);

      final saved = await ProfileStorageService.instance.saveProfile(accountId, profile, preserveCache: false);
      DebugLogger.instance.logInfo('BACKUP', 'restoreSpecialBackup saveProfile result: $saved');
      if (!saved) return false;

      // 恢复附件
      final attachmentsDir = _attachmentsDirPath(file.path);
      await _restoreAttachments(accountId, attachmentsDir);

      return true;
    } on Exception catch (_) {
      return false;
    }
  }

  /// 将普通备份提升为特别备份（复制文件 + 附件目录）。
  Future<String?> promoteBackupToSpecial(
    String accountId,
    String regularFileName,
    String name,
  ) async {
    final sanitized = BackupService.sanitizeSpecialName(name);
    if (sanitized.isEmpty) return null;

    final specialList = await listSpecialBackups(accountId);
    if (specialList.length >= maxSpecialBackupCount) {
      DebugLogger.instance.logWarning(
          'BACKUP', 'Special backup limit reached ($maxSpecialBackupCount)');
      return null;
    }

    try {
      final regularDir = await _accountBackupDir(accountId);
      final regularFile = File('${regularDir.path}/$regularFileName');
      if (!await regularFile.exists()) return null;

      final specialDir = await _specialBackupDir(accountId);
      final newFileName = '$sanitized$_fileExt';
      final specialFile = File('${specialDir.path}/$newFileName');
      if (await specialFile.exists()) return null;

      await regularFile.copy(specialFile.path);
      // 同时复制 manifest（引用相同的池文件，无需复制附件内容）
      final regularSidecar = _attachmentsDirPath(regularFile.path);
      final specialSidecar = _attachmentsDirPath(specialFile.path);
      final regularManifest = File('$regularSidecar/$_manifestFileName');
      if (await regularManifest.exists()) {
        final specialAttachDir = Directory(specialSidecar);
        if (!await specialAttachDir.exists()) {
          await specialAttachDir.create(recursive: true);
        }
        await regularManifest.copy('$specialSidecar/$_manifestFileName');
      }
      return newFileName;
    } on FileSystemException catch (e, st) {
      DebugLogger.instance.logError(
          'BACKUP', 'promoteBackupToSpecial failed: $e\n$st');
      return null;
    }
  }

  // -------------------------------------------------------------------------
  // 内部辅助
  // -------------------------------------------------------------------------

  Future<void> _cleanupOldBackups(String accountId) async {
    final list = await listBackups(accountId);
    if (list.length <= maxBackupCount) return;

    final toDelete = list.sublist(maxBackupCount);
    for (final entry in toDelete) {
      try {
        final dir = await _accountBackupDir(accountId);
        final file = File('${dir.path}/${entry.fileName}');
        if (await file.exists()) {
          await _deleteBackupWithCleanup(accountId, file.path);
        }
      } on FileSystemException catch (_) {
        // 忽略清理错误
      }
    }
  }

}
