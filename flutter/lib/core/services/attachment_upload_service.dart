import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';

import 'package:solosoul_flutter/core/models/attachment_task_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/utils/file_path_resolver.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

// =============================================================================
// Attachment Upload Service
// =============================================================================

/// 统一封装附件的文件选择、敏感验证和加密存储流程。
///
/// 支持两种上传模式：
/// - **小文件**（≤ 50MB）：内存中一次性加密（v2），通过 [uploadFile]
/// - **大文件**（> 50MB 或任意大小）：Rust 端流式分块加密（v3），通过 [uploadFileFromPath]
///
/// 使用示例:
/// ```dart
/// // 选择文件（获取路径，不加载到内存）
/// final file = await AttachmentUploadService.pickFile();
/// if (file == null) return;
///
/// // 上传（自动选择小文件/大文件路径）
/// final attachment = await AttachmentUploadService.uploadAny(
///   accountId: accountId,
///   platformFile: file,
///   onProgress: (p) => print('Progress: ${(p * 100).toInt()}%'),
///   cancelToken: cancelToken,
/// );
/// ```
class AttachmentUploadService {
  AttachmentUploadService._();

  /// 小文件阈值：≤ 此大小使用一次性内存加密（v2）
  static const int smallFileThreshold = 50 * 1024 * 1024;

  /// 弹出文件选择器，返回选中的文件元数据（不加载数据到内存）。
  ///
  /// 使用 `withData: false` 避免大文件 OOM。返回的 [PlatformFile.path]
  /// 可能是真实路径或 Android content URI。
  ///
  /// 返回 `null` 表示用户取消选择。
  static Future<PlatformFile?> pickFile() async {
    final result = await FilePicker.pickFiles(
      type: FileType.any,
      allowMultiple: false,
      withData: false,
    );
    if (result == null || result.files.isEmpty) return null;
    return result.files.first;
  }

  /// 弹出文件选择器并加载数据到内存（仅用于小文件场景）。
  ///
  /// 使用 `withData: true`，适用于需要直接访问 bytes 的场景。
  /// 大文件会导致 OOM，谨慎使用。
  static Future<PlatformFile?> pickFileWithData() async {
    final result = await FilePicker.pickFiles(
      type: FileType.any,
      allowMultiple: false,
      withData: true,
    );
    if (result == null || result.files.isEmpty) return null;
    return result.files.first;
  }

  /// 自动判断文件大小，选择小文件/大文件上传路径。
  ///
  /// [platformFile] 为 [pickFile] 返回的结果（withData: false）。
  /// [onProgress] 在流式大文件模式下通过进度文件轮询更新。
  static Future<Attachment?> uploadAny({
    required String accountId,
    required PlatformFile platformFile,
    required ValueChanged<double> onProgress,
    CancelToken? cancelToken,
    String? progressPath,
    String? cancelPath,
  }) async {
    final fileSize = platformFile.size;
    final fileName = platformFile.name;

    // 小文件且已加载到内存：走 v2 快速路径
    if (fileSize <= smallFileThreshold && platformFile.bytes != null) {
      return uploadFile(
        accountId: accountId,
        fileName: fileName,
        bytes: platformFile.bytes!,
        onProgress: onProgress,
        cancelToken: cancelToken,
      );
    }

    // 大文件或未加载到内存：走 v3 流式路径
    final rawPath = platformFile.path;
    if (rawPath == null || rawPath.isEmpty) {
      SoloLog.e('AttachmentUpload', 'No file path available: $fileName');
      return null;
    }

    return uploadFileFromPath(
      accountId: accountId,
      fileName: fileName,
      rawPath: rawPath,
      fileSize: fileSize,
      onProgress: onProgress,
      cancelToken: cancelToken,
      progressPath: progressPath,
      cancelPath: cancelPath,
    );
  }

  /// 加密保存附件（小文件内存路径，v2）。
  ///
  /// [onProgress] 在关键阶段被调用：0.1（准备）、0.7（加密完成）、
  /// 0.9（写入完成）、1.0（完成）。
  ///
  /// [cancelToken] 在加密前和写入后检查，若已取消则清理残留文件。
  /// 注意：加密期间无法中断，UI 上应禁用取消按钮。
  static Future<Attachment?> uploadFile({
    required String accountId,
    required String fileName,
    required Uint8List bytes,
    required ValueChanged<double> onProgress,
    CancelToken? cancelToken,
  }) async {
    try {
      final attachment = await AttachmentStorageService().saveAttachment(
        accountId: accountId,
        fileName: fileName,
        bytes: bytes,
        onProgress: onProgress,
        cancelToken: cancelToken,
      );
      return attachment;
    } on Exception catch (e, stackTrace) {
      SoloLog.e('AttachmentUpload', 'Upload failed: $fileName', e, stackTrace);
      return null;
    }
  }

  /// 从文件路径加密保存附件（大文件流式路径，v3）。
  ///
  /// [rawPath] 为原始路径（可能是 content URI）。
  /// 内部会调用 [FilePathResolver.resolve] 解析为真实路径。
  /// 对于 content URI，会自动复制到临时目录并在完成后清理。
  static Future<Attachment?> uploadFileFromPath({
    required String accountId,
    required String fileName,
    required String rawPath,
    int? fileSize,
    required ValueChanged<double> onProgress,
    CancelToken? cancelToken,
    String? progressPath,
    String? cancelPath,
  }) async {
    // 1. 解析文件路径（处理 Android content URI）
    final (resolvedPath, isTemporary) = await FilePathResolver.resolve(rawPath);

    // 如果解析失败（content URI 无法读取），返回 null
    if (resolvedPath.startsWith('content://')) {
      SoloLog.e('AttachmentUpload', 'Cannot read content URI: $rawPath');
      return null;
    }

    // 2. 创建进度文件和取消标志文件
    final tempDir = await getTemporaryDirectory();
    final uuid = const Uuid().v4();
    final actualProgressPath = progressPath ?? '${tempDir.path}/progress_$uuid.txt';
    final actualCancelPath = cancelPath ?? '${tempDir.path}/cancel_$uuid.txt';

    // 3. 启动进度轮询 Timer
    Timer? progressTimer;
    progressTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
      final pf = File(actualProgressPath);
      if (!await pf.exists()) return;
      try {
        final content = await pf.readAsString();
        final progress = double.tryParse(content.trim()) ?? 0.0;
        onProgress(progress);
      } on Exception catch (_) {
        // Ignore read errors
      }
    });

    try {
      final attachment = await AttachmentStorageService().saveAttachmentFromPath(
        accountId: accountId,
        fileName: fileName,
        srcPath: resolvedPath,
        fileSize: fileSize,
        progressPath: actualProgressPath,
        cancelPath: actualCancelPath,
        isSrcTemporary: isTemporary,
      );

      onProgress(1.0);
      return attachment;
    } on Exception catch (e, stackTrace) {
      // 检查是否是取消导致的
      if (cancelToken?.isCancelled ?? false) {
        SoloLog.d('AttachmentUpload', 'Upload cancelled: $fileName');
      } else {
        SoloLog.e('AttachmentUpload', 'Upload from path failed: $fileName', e, stackTrace);
      }
      return null;
    } finally {
      progressTimer.cancel();
      await FilePathResolver.cleanup(actualProgressPath);
      await FilePathResolver.cleanup(actualCancelPath);
    }
  }

  /// 一次性完成文件选择、敏感验证和上传（旧版 API，向后兼容）。
  ///
  /// 简单场景下仍可使用；自动根据文件大小选择 v2/v3 路径。
  static Future<Attachment?> pickAndUpload({
    required BuildContext context,
    required WidgetRef ref,
    bool requiresSensitiveCheck = false,
  }) async {
    final l10n = AppLocalizations.of(context);

    // 1. 敏感数据验证（可选）
    if (requiresSensitiveCheck) {
      final isGranted = ref.read(isSensitiveAccessGrantedProvider);
      if (!isGranted) {
        final authNotifier = ref.read(authNotifierProvider.notifier);
        final selectedAccount = authNotifier.selectedAccount;
        final password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          passwordHint: selectedAccount?.passwordHint,
          onVerify: authNotifier.verifyPasswordForSensitiveData,
        );
        if (password == null) return null;
        ref.read(sensitivePageAccessProvider.notifier).markVerified();
      }
    }

    // 2. 选择文件（withData: false，避免大文件 OOM）
    final file = await pickFile();
    if (file == null) return null;

    if (file.path == null || file.path!.isEmpty) {
      if (context.mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentReadFailed,
          type: SnackBarType.error,
        );
      }
      return null;
    }

    // 3. 获取当前账户
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) {
      if (context.mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.loginNoAccountsYet,
          type: SnackBarType.error,
        );
      }
      return null;
    }

    // 4. 上传（自动选择 v2/v3 路径）
    final attachment = await uploadAny(
      accountId: accountId,
      platformFile: file,
      onProgress: (_) {}, // 旧版无进度回调
    );

    if (attachment != null && context.mounted) {
      showOverlaySnackBar(
        context,
        content: l10n.attachmentAdded,
        type: SnackBarType.success,
      );
    } else if (attachment == null && context.mounted) {
      showOverlaySnackBar(
        context,
        content: l10n.attachmentAddFailed,
        type: SnackBarType.error,
      );
    }
    return attachment;
  }
}
