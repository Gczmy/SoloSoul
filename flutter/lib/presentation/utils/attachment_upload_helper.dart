import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_upload_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

/// Presentation-layer helper that orchestrates file picking, sensitive-data
/// verification and upload in one shot.
///
/// This was extracted from [AttachmentUploadService] to keep the service layer
/// free of UI dependencies ([BuildContext] / [WidgetRef]).
class AttachmentUploadHelper {
  AttachmentUploadHelper._();

  /// 一次性完成文件选择、敏感验证和上传。
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
    final file = await AttachmentUploadService.pickFile();
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
    final attachment = await AttachmentUploadService.uploadAny(
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
