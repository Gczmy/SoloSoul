// ===========================================================================
// BackupService — 加密备份与恢复
// ===========================================================================
// 将用户数据以 Vault 相同的 AES-256-GCM 加密后导出到独立目录，防止 App 更新
// 或异常导致数据丢失。保留最近 N 份备份，支持手动与自动备份。
//
// 备份路径：{appSupportDir}/solosoul_backups/{accountId}/{timestamp}.backup
// 文件名：backup_YYYY-MM-DD_HH-mm-ss[_vX.Y.Z].backup
//
// 特别备份路径：{appSupportDir}/solosoul_backups/{accountId}/special/{name}.backup
//
// 注意：备份使用与 Vault 相同的加密密钥，恢复时需要 Vault 已解锁。
// ===========================================================================

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';

import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'profile_storage_service.dart';
import 'rust_vault_service.dart';

/// 单条备份信息
class BackupEntry {
  final String fileName;
  final DateTime createdAt;
  final int sizeBytes;

  const BackupEntry({
    required this.fileName,
    required this.createdAt,
    required this.sizeBytes,
  });

  String get displayTime =>
      '${createdAt.year}-${_two(createdAt.month)}-${_two(createdAt.day)} '
      '${_two(createdAt.hour)}:${_two(createdAt.minute)}:${_two(createdAt.second)}';

  static String _two(int n) => n.toString().padLeft(2, '0');
}

class BackupService {
  BackupService._();
  static final BackupService instance = BackupService._();

  static const _backupDirName = 'solosoul_backups';
  static const _specialDirName = 'special';
  static const _filePrefix = 'backup_';
  static const _fileExt = '.backup';

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
  // Isolate helpers — 必须与 UI 回调隔离，避免 Dart closure context 共享
  // 导致 "object is unsendable" 错误。
  //
  // Dart 在同一作用域内定义的多个闭包会共享 context 对象。若 Isolate.run
  // 的闭包与捕获 Widget state 的 onProgress 回调定义在同一方法中，
  // 即使闭包本身未直接引用 onProgress，整个 context 也会被序列化到 isolate，
  // 从而触发 Illegal argument in isolate message。
  //
  // 安全做法：将 Isolate.run 调用封装到独立的 static 方法中，使其 closure
  // context 与 UI 回调完全隔离。
  // -------------------------------------------------------------------------
  static Uint8List encodeProfileToBytes(Map<String, dynamic> json) {
    final jsonString = jsonEncode(json);
    return Uint8List.fromList(utf8.encode(jsonString));
  }

  static Future<Uint8List> _encodeProfileInIsolate(Map<String, dynamic> json) {
    return Isolate.run(() => encodeProfileToBytes(json));
  }

  static ProfileData decodeProfileFromString(String jsonString) {
    final json = jsonDecode(jsonString) as Map<String, dynamic>;
    return ProfileData.fromJson(json);
  }

  static Future<ProfileData> _decodeProfileInIsolate(String jsonString) {
    return Isolate.run(() => decodeProfileFromString(jsonString));
  }

  // -------------------------------------------------------------------------
  // 核心：常规备份 / 恢复 / 列表 / 删除
  // -------------------------------------------------------------------------

  /// 立即为当前账户创建一份加密备份。
  /// [appVersion] 可选，会附加到文件名中便于追溯。
  /// [onProgress] 可选进度回调，范围 0.0 ~ 1.0。
  /// 返回备份文件名，失败返回 null。
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

      report(0.9);
      DebugLogger.instance.logInfo('BACKUP', 'Step 4/5: write file start');
      final dir = await _accountBackupDir(accountId);
      final now = DateTime.now();
      final fileName = BackupService.backupFileName(now, appVersion: appVersion);
      final file = File('${dir.path}/$fileName');
      await file.writeAsBytes(encrypted, flush: true);
      unawaited(_setRestrictivePermissions(file.path));
      DebugLogger.instance.logInfo('BACKUP', 'Step 4/5: write file done -> $fileName');

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
        ));
      }

      entries.sort((a, b) => b.createdAt.compareTo(a.createdAt));
      return entries;
    } on FileSystemException catch (_) {
      return const [];
    }
  }

  /// 从指定备份文件恢复数据到 Vault。
  /// 恢复前会自动先创建一份当前状态的备份（如果当前有数据）。
  /// 返回是否成功。
  Future<bool> restoreBackup(String accountId, String fileName) async {
    try {
      // 1. 先读取目标备份文件（必须在创建保护性备份之前，
      //    否则 createBackup 的 cleanup 可能删掉这份旧备份）
      final dir = await _accountBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;

      final encrypted = await file.readAsBytes();
      final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
      if (decrypted == null) return false;

      final jsonString = utf8.decode(decrypted);
      final profile = await _decodeProfileInIsolate(jsonString);

      // 2. 创建保护性备份（覆盖数据前）
      await createBackup(accountId);

      // 3. 保存恢复的数据到 Vault
      final saved = await ProfileStorageService.instance.saveProfile(accountId, profile);
      return saved;
    } on Exception catch (_) {
      return false;
    }
  }

  /// 删除某条常规备份。
  Future<bool> deleteBackup(String accountId, String fileName) async {
    try {
      final dir = await _accountBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (await file.exists()) {
        await file.delete();
        return true;
      }
      return false;
    } on FileSystemException catch (_) {
      return false;
    }
  }

  /// 获取最近一次备份的信息（用于 UI 展示）。
  Future<BackupEntry?> getLatestBackup(String accountId) async {
    final list = await listBackups(accountId);
    return list.isNotEmpty ? list.first : null;
  }

  /// 获取备份文件的总大小。
  Future<int> getTotalBackupSize(String accountId) async {
    final list = await listBackups(accountId);
    return list.fold<int>(0, (sum, e) => sum + e.sizeBytes);
  }

  // -------------------------------------------------------------------------
  // 特别备份
  // -------------------------------------------------------------------------

  /// 创建一份特别备份（不参与常规 5 份循环）。
  /// [name] 为用户自定义名称，不需要加扩展名。
  /// 返回最终文件名（含 .backup 后缀），失败返回 null。
  /// 当特别备份已满时返回 null，调用方应先检查 listSpecialBackups 数量。
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

      report(0.9);
      final dir = await _specialBackupDir(accountId);
      final fileName = '$sanitized$_fileExt';
      final file = File('${dir.path}/$fileName');
      await file.writeAsBytes(encrypted, flush: true);
      unawaited(_setRestrictivePermissions(file.path));

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
        ));
      }

      entries.sort((a, b) => b.createdAt.compareTo(a.createdAt));
      return entries;
    } on FileSystemException catch (_) {
      return const [];
    }
  }

  /// 重命名特别备份。
  /// 返回新文件名（含 .backup 后缀），失败返回 null。
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
      return newFileName;
    } on FileSystemException catch (_) {
      return null;
    }
  }

  /// 删除某条特别备份。
  Future<bool> deleteSpecialBackup(String accountId, String fileName) async {
    try {
      final dir = await _specialBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (await file.exists()) {
        await file.delete();
        return true;
      }
      return false;
    } on FileSystemException catch (_) {
      return false;
    }
  }

  /// 从特别备份恢复数据到 Vault。
  Future<bool> restoreSpecialBackup(String accountId, String fileName) async {
    try {
      final dir = await _specialBackupDir(accountId);
      final file = File('${dir.path}/$fileName');
      if (!await file.exists()) return false;

      final encrypted = await file.readAsBytes();
      final decrypted = await RustVaultService.instance.decryptBytes(encrypted);
      if (decrypted == null) return false;

      final jsonString = utf8.decode(decrypted);
      final json = jsonDecode(jsonString) as Map<String, dynamic>;
      final profile = ProfileData.fromJson(json);

      // 创建保护性备份（覆盖数据前）
      await createBackup(accountId);

      final saved = await ProfileStorageService.instance.saveProfile(accountId, profile);
      return saved;
    } on Exception catch (_) {
      return false;
    }
  }

  /// 将普通备份提升为特别备份（复制文件，不移动）。
  /// [name] 为用户自定义名称，不需要加扩展名。
  /// 返回最终文件名，失败返回 null。
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
      return newFileName;
    } on FileSystemException catch (e, st) {
      DebugLogger.instance.logError(
          'BACKUP', 'promoteBackupToSpecial failed: $e\n$st');
      return null;
    }
  }

  // -------------------------------------------------------------------------
  // 内部：自动清理旧常规备份
  // -------------------------------------------------------------------------

  Future<void> _cleanupOldBackups(String accountId) async {
    final list = await listBackups(accountId);
    if (list.length <= maxBackupCount) return;

    final toDelete = list.sublist(maxBackupCount);
    final dir = await _accountBackupDir(accountId);
    for (final entry in toDelete) {
      try {
        final file = File('${dir.path}/${entry.fileName}');
        if (await file.exists()) await file.delete();
      } on FileSystemException catch (_) {
        // 忽略清理错误
      }
    }
  }
}
