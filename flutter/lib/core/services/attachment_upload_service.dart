import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
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
/// 使用示例:
/// ```dart
/// final attachment = await AttachmentUploadService.pickAndUpload(
///   context: context,
///   ref: ref,
///   requiresSensitiveCheck: true,
/// );
/// if (attachment != null) {
///   // 将 attachment 添加到对象的 attachments 列表
/// }
/// ```
class AttachmentUploadService {
  AttachmentUploadService._();

  /// 弹出文件选择器，完成敏感验证（如需要），加密保存附件。
  ///
  /// [requiresSensitiveCheck] 为 true 时，会先检查当前是否已通过
  /// 敏感数据验证；未通过则弹出密码验证对话框。
  ///
  /// 成功返回 [Attachment]，用户取消或出错返回 null。
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

    // 2. 选择文件
    final result = await FilePicker.pickFiles(
      type: FileType.any,
      allowMultiple: false,
      withData: true,
    );
    if (result == null || result.files.isEmpty) return null;

    final file = result.files.first;
    final bytes = file.bytes;
    if (bytes == null || bytes.isEmpty) {
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

    // 4. 加密保存
    try {
      final attachment = await AttachmentStorageService().saveAttachment(
        accountId: accountId,
        fileName: file.name,
        bytes: bytes,
      );

      if (context.mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentAdded,
          type: SnackBarType.success,
        );
      }
      return attachment;
    } on Exception catch (e, stackTrace) {
      SoloLog.e('AttachmentUpload', 'Upload failed', e, stackTrace);
      if (context.mounted) {
        showOverlaySnackBar(
          context,
          content: '${l10n.attachmentAddFailed}: $e',
          type: SnackBarType.error,
        );
      }
      return null;
    }
  }
}
