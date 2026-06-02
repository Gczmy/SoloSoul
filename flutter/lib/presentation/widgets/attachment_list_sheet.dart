import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:pdfx/pdfx.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/attachment_task_model.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_download_service.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/services/attachment_upload_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider, objectByIdProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/pptx_preview_dialog.dart';
import 'package:uuid/uuid.dart';

// =============================================================================
// Attachment List Sheet
// =============================================================================

/// Bottom sheet that displays a list of attachments for a given object.
/// Supports preview, soft-delete, restore, permanent delete, upload, and download.
///
/// Upload tasks are managed internally — callers only need to open this sheet.
class AttachmentListSheet extends ConsumerStatefulWidget {
  final UnifiedObject object;
  final String? accountId;

  const AttachmentListSheet({
    super.key,
    required this.object,
    required this.accountId,
  });

  @override
  ConsumerState<AttachmentListSheet> createState() =>
      _AttachmentListSheetState();
}

class _AttachmentListSheetState extends ConsumerState<AttachmentListSheet> {
  final Map<String, bool> _loadingMap = {};
  final Map<String, bool> _completedMap = {};
  bool _deletedExpanded = false;

  // Upload / download task tracking
  final Map<String, UploadTask> _uploadTasks = {};
  final Map<String, DownloadTask> _downloadTasks = {};

  bool get _isSensitive => widget.object.properties.values.any(
        (p) =>
            p.sensitivity == SensitivityLevel.sensitive ||
            p.sensitivity == SensitivityLevel.critical,
      );

  @override
  void dispose() {
    // Cancel all active tasks to prevent memory leaks and clean up resources
    for (final task in _uploadTasks.values) {
      task.cancelToken.cancel();
    }
    for (final task in _downloadTasks.values) {
      task.cancelToken.cancel();
    }
    super.dispose();
  }

  Future<void> _verifyIfNeeded() async {
    if (!_isSensitive) return;
    if (ref.read(isSensitiveAccessGrantedProvider)) return;

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password != null) {
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }
  }

  Future<bool> _ensureVerified() async {
    if (!_isSensitive) return true;
    if (ref.read(isSensitiveAccessGrantedProvider)) return true;
    await _verifyIfNeeded();
    return ref.read(isSensitiveAccessGrantedProvider);
  }

  // ---------------------------------------------------------------------------
  // Upload (managed internally)
  // ---------------------------------------------------------------------------

  Future<void> _handleAddAttachment() async {
    final l10n = AppLocalizations.of(context);
    final accountId = widget.accountId;

    if (accountId == null) {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.loginNoAccountsYet,
          type: SnackBarType.error,
        );
      }
      return;
    }

    // 1. Pick file (withData: false, get path)
    final file = await AttachmentUploadService.pickFile();
    if (file == null) return;

    if (file.path == null || file.path!.isEmpty) {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentReadFailed,
          type: SnackBarType.error,
        );
      }
      return;
    }

    // 2. Create upload task
    final tempId = const Uuid().v4();
    final task = UploadTask(
      tempId: tempId,
      fileName: file.name,
      size: file.size,
      cancelToken: CancelToken(),
    );

    if (mounted) {
      setState(() => _uploadTasks[tempId] = task);
    }

    // 3. Upload (auto-selects v2/v3 based on size)
    final attachment = await AttachmentUploadService.uploadAny(
      accountId: accountId,
      platformFile: file,
      onProgress: (progress) {
        if (!mounted) return;
        setState(() {
          task.progress = progress;
          if (progress < 0.70) {
            task.status = TaskStatus.encrypting;
          } else if (progress < 1.0) {
            task.status = TaskStatus.writing;
          } else {
            task.status = TaskStatus.completed;
          }
        });
      },
      cancelToken: task.cancelToken,
    );

    if (!mounted) return;

    // 5. Handle result
    if (attachment != null && !task.cancelToken.isCancelled) {
      // Add to object
      final liveObject = ref.read(objectByIdProvider(widget.object.id));
      final currentObject = liveObject ?? widget.object;
      final updatedAttachments = [...currentObject.attachments, attachment];

      await ref.read(unifiedObjectProvider.notifier).updateObject(
        widget.object.id,
        attachments: updatedAttachments,
      );

      if (!mounted) return;
      setState(() => _uploadTasks.remove(tempId));

      showOverlaySnackBar(
        context,
        content: l10n.attachmentAdded,
        type: SnackBarType.success,
      );
    } else {
      if (!mounted) return;
      setState(() => _uploadTasks.remove(tempId));

      if (!task.cancelToken.isCancelled) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentAddFailed,
          type: SnackBarType.error,
        );
      } else {
        showOverlaySnackBar(
          context,
          content: l10n.uploadCancelled,
          type: SnackBarType.info,
        );
      }
    }
  }

  void _cancelUpload(String tempId) {
    final task = _uploadTasks[tempId];
    if (task == null) return;

    if (!task.isCancellable) {
      // 加密期间无法取消，提示用户
      showOverlaySnackBar(
        context,
        content: AppLocalizations.of(context).encryptionInProgress,
        type: SnackBarType.info,
      );
      return;
    }

    task.cancelToken.cancel();
    setState(() => _uploadTasks.remove(tempId));

    showOverlaySnackBar(
      context,
      content: AppLocalizations.of(context).uploadCancelled,
      type: SnackBarType.info,
    );
  }

  // ---------------------------------------------------------------------------
  // Download
  // ---------------------------------------------------------------------------

  Future<void> _handleDownload(Attachment attachment) async {
    // Dedup: ignore if already downloading this attachment
    if (_downloadTasks.containsKey(attachment.id)) return;

    final accountId = widget.accountId;
    if (accountId == null) {
      final l10n = AppLocalizations.of(context);
      showOverlaySnackBar(
        context,
        content: l10n.loginNoAccountsYet,
        type: SnackBarType.error,
      );
      return;
    }

    final task = DownloadTask(
      attachmentId: attachment.id,
      fileName: attachment.fileName,
      size: attachment.size,
      cancelToken: CancelToken(),
    );
    setState(() => _downloadTasks[attachment.id] = task);

    final downloadDir = await AttachmentDownloadService().getDownloadDirectory();
    final savedPath = await AttachmentDownloadService().downloadAttachment(
      accountId: accountId,
      attachment: attachment,
      downloadDir: downloadDir,
      onProgress: (progress) {
        if (!mounted) return;
        setState(() {
          task.progress = progress;
          if (progress < 0.80) {
            task.status = TaskStatus.encrypting; // decrypting
          } else if (progress < 1.0) {
            task.status = TaskStatus.writing;
          } else {
            task.status = TaskStatus.completed;
          }
        });
      },
      cancelToken: task.cancelToken,
    );

    if (!mounted) return;

    final l10n = AppLocalizations.of(context);
    if (savedPath != null && !task.cancelToken.isCancelled) {
      setState(() => _completedMap[attachment.id] = true);
      Future.delayed(const Duration(seconds: 1), () {
        if (mounted) {
          setState(() => _completedMap.remove(attachment.id));
        }
      });
      if (mounted) {
        final fileName = savedPath.split('/').last;
        showOverlaySnackBar(
          context,
          content: l10n.attachmentDownloaded('Downloads/$fileName'),
          type: SnackBarType.success,
        );
      }
    } else if (task.cancelToken.isCancelled) {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.downloadCancelled,
          type: SnackBarType.info,
        );
      }
    } else {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentDownloadFailed,
          type: SnackBarType.error,
        );
      }
    }

    setState(() => _downloadTasks.remove(attachment.id));
  }

  void _cancelDownload(String attachmentId) {
    final task = _downloadTasks[attachmentId];
    if (task == null) return;

    if (!task.isCancellable) {
      showOverlaySnackBar(
        context,
        content: AppLocalizations.of(context).encryptionInProgress,
        type: SnackBarType.info,
      );
      return;
    }

    task.cancelToken.cancel();
    setState(() => _downloadTasks.remove(attachmentId));

    showOverlaySnackBar(
      context,
      content: AppLocalizations.of(context).downloadCancelled,
      type: SnackBarType.info,
    );
  }

  // ---------------------------------------------------------------------------
  // Delete / Restore / Permanent Delete
  // ---------------------------------------------------------------------------

  Future<void> _handleDelete(Attachment attachment) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.deleteAttachment),
        content: Text(l10n.deleteAttachmentConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.commonCancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            style: TextButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
            child: Text(l10n.commonDelete),
          ),
        ],
      ),
    );
    if (confirmed != true) return;

    if (!await _ensureVerified()) return;

    final success = await ref
        .read(unifiedObjectProvider.notifier)
        .softDeleteAttachment(widget.object.id, attachment.id);

    if (mounted && success) {
      showOverlaySnackBar(
        context,
        content: l10n.attachmentDeleted,
        type: SnackBarType.success,
      );
    }
  }

  Future<void> _handleRestore(Attachment attachment) async {
    if (!await _ensureVerified()) return;

    final success = await ref
        .read(unifiedObjectProvider.notifier)
        .restoreAttachment(widget.object.id, attachment.id);

    if (mounted && success) {
      showOverlaySnackBar(
        context,
        content: AppLocalizations.of(context).attachmentRestored,
        type: SnackBarType.success,
      );
    }
  }

  Future<void> _handlePermanentDelete(Attachment attachment) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.permanentlyDeleteAttachment),
        content: Text(l10n.attachmentPermanentlyDeleteConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.commonCancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            style: TextButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
            child: Text(l10n.permanentlyDeleteAttachment),
          ),
        ],
      ),
    );
    if (confirmed != true) return;

    if (!await _ensureVerified()) return;

    final success = await ref
        .read(unifiedObjectProvider.notifier)
        .permanentlyDeleteAttachment(
          widget.object.id,
          attachment.id,
          accountId: widget.accountId,
        );

    if (mounted && !success) {
      showOverlaySnackBar(
        context,
        content: l10n.attachmentDeleteFailed,
        type: SnackBarType.error,
      );
    }
  }

  // ---------------------------------------------------------------------------
  // Preview
  // ---------------------------------------------------------------------------

  Future<void> _openAttachment(Attachment attachment) async {
    final l10n = AppLocalizations.of(context);
    final accountId = widget.accountId;
    if (accountId == null) {
      showOverlaySnackBar(
        context,
        content: 'No account selected',
        type: SnackBarType.error,
      );
      return;
    }

    SoloLog.d('AttachmentPreview',
        'Opening attachment: name=${attachment.fileName}, mime=${attachment.mimeType}, size=${attachment.size}');

    // Office 文档（PPTX/PPT/DOCX/DOC/XLSX/XLS）：流式解密到临时文件，支持任意大小
    if (_isOfficeDocument(attachment)) {
      await _openPptxOrPpt(attachment);
      return;
    }

    // PDF：流式解密到临时文件，支持任意大小，绕过 10MB 内存加载限制
    if (attachment.mimeType == 'application/pdf' ||
        attachment.fileName.toLowerCase().endsWith('.pdf')) {
      await _openPdfFromFile(attachment);
      return;
    }

    // 大文件预览限制：>10MB 禁止内存加载（仅针对 image）
    if (attachment.size > AttachmentStorageService.maxPreviewSize) {
      showOverlaySnackBar(
        context,
        content: l10n.attachmentTooLargeForPreview,
        type: SnackBarType.info,
      );
      return;
    }

    setState(() => _loadingMap[attachment.id] = true);

    try {
      final bytes = await AttachmentStorageService().loadAttachment(
        accountId: accountId,
        fileId: attachment.fileId,
      );

      if (!mounted) return;
      setState(() => _loadingMap[attachment.id] = false);

      if (bytes == null) {
        showOverlaySnackBar(
          context,
          content: 'Failed to load attachment',
          type: SnackBarType.error,
        );
        return;
      }

      if (attachment.mimeType.startsWith('image/')) {
        _showImagePreview(bytes, attachment.fileName, attachment);
      } else {
        showOverlaySnackBar(
          context,
          content: 'Preview not supported for this file type',
          type: SnackBarType.info,
        );
      }
    } on AttachmentTooLargeForPreviewException catch (_) {
      if (mounted) {
        setState(() => _loadingMap[attachment.id] = false);
        showOverlaySnackBar(
          context,
          content: l10n.attachmentTooLargeForPreview,
          type: SnackBarType.info,
        );
      }
    }
  }

  /// 打开 PPTX / PPT 文件：流式解密到临时文件，然后弹出预览对话框。
  /// 支持任意大小，不受 10MB 预览限制。
  Future<void> _openPptxOrPpt(Attachment attachment) async {
    setState(() => _loadingMap[attachment.id] = true);

    try {
      final accountId = widget.accountId;
      if (accountId == null) return;

      final tempDir = await Directory.systemTemp.createTemp('solosoul_pptx_');
      final tempPath = '${tempDir.path}/${attachment.fileName}';
      final progressPath = '${tempDir.path}/progress.txt';
      final cancelPath = '${tempDir.path}/cancel.txt';

      final success = await AttachmentStorageService().decryptAttachmentToPath(
        accountId: accountId,
        fileId: attachment.fileId,
        dstPath: tempPath,
        progressPath: progressPath,
        cancelPath: cancelPath,
      );

      if (!mounted) return;
      setState(() => _loadingMap[attachment.id] = false);

      if (!success) {
        showOverlaySnackBar(
          context,
          content: 'Failed to decrypt attachment',
          type: SnackBarType.error,
        );
        return;
      }

      await showDialog(
        context: context,
        builder: (context) => PptxPreviewDialog(
          fileName: attachment.fileName,
          fileSize: attachment.size,
          filePath: tempPath,
          attachment: attachment,
          onDownload: () => _handleDownload(attachment),
        ),
      );
    } on Exception catch (e) {
      SoloLog.e('AttachmentPreview', 'Failed to open PPTX/PPT', e);
      if (mounted) {
        setState(() => _loadingMap[attachment.id] = false);
        showOverlaySnackBar(
          context,
          content: 'Failed to open file: $e',
          type: SnackBarType.error,
        );
      }
    }
  }

  /// 打开 PDF 文件：流式解密到临时文件，然后用 PdfDocument.openFile 按需渲染。
  /// 支持任意大小，不受 10MB 预览限制。
  Future<void> _openPdfFromFile(Attachment attachment) async {
    setState(() => _loadingMap[attachment.id] = true);

    try {
      final accountId = widget.accountId;
      if (accountId == null) return;

      final tempDir = await Directory.systemTemp.createTemp('solosoul_pdf_');
      final tempPath = '${tempDir.path}/${attachment.fileName}';
      final progressPath = '${tempDir.path}/progress.txt';
      final cancelPath = '${tempDir.path}/cancel.txt';

      final success = await AttachmentStorageService().decryptAttachmentToPath(
        accountId: accountId,
        fileId: attachment.fileId,
        dstPath: tempPath,
        progressPath: progressPath,
        cancelPath: cancelPath,
      );

      if (!mounted) return;
      setState(() => _loadingMap[attachment.id] = false);

      if (!success) {
        showOverlaySnackBar(
          context,
          content: 'Failed to decrypt attachment',
          type: SnackBarType.error,
        );
        return;
      }

      await _showPdfPreviewFromFile(tempPath, attachment.fileName, attachment);
      await _cleanupPdfTempFile(tempPath);
    } on Exception catch (e) {
      SoloLog.e('AttachmentPreview', 'Failed to open PDF', e);
      if (mounted) {
        setState(() => _loadingMap[attachment.id] = false);
        showOverlaySnackBar(
          context,
          content: 'Failed to open file: $e',
          type: SnackBarType.error,
        );
      }
    }
  }

  void _showImagePreview(Uint8List bytes, String fileName, Attachment attachment) {
    showDialog(
      context: context,
      builder: (context) => Dialog(
        insetPadding: EdgeInsets.zero,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AppBar(
              title: Text(fileName),
              actions: [
                IconButton(
                  icon: const Icon(Icons.download),
                  tooltip: AppLocalizations.of(context).downloadAttachment,
                  onPressed: () => _handleDownload(attachment),
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            Flexible(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  return InteractiveViewer(
                    constrained: false,
                    minScale: 0.5,
                    maxScale: 4.0,
                    child: SizedBox(
                      width: constraints.maxWidth,
                      height: constraints.maxHeight,
                      child: Image.memory(
                        bytes,
                        fit: BoxFit.contain,
                        errorBuilder: (context, error, stackTrace) {
                          SoloLog.e('AttachmentPreview', 'Image decode error', error);
                          return const Center(
                            child: Padding(
                              padding: EdgeInsets.all(32),
                              child: Icon(Icons.broken_image, size: 64),
                            ),
                          );
                        },
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _showPdfPreviewFromFile(
    String filePath,
    String fileName,
    Attachment attachment,
  ) async {
    final controller = PdfController(
      document: PdfDocument.openFile(filePath),
    );
    PdfDocument? loadedDoc;

    await showDialog(
      context: context,
      builder: (context) => Dialog(
        insetPadding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AppBar(
              title: Text(fileName),
              actions: [
                IconButton(
                  icon: const Icon(Icons.download),
                  tooltip: AppLocalizations.of(context).downloadAttachment,
                  onPressed: () => _handleDownload(attachment),
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            Flexible(
              child: PdfView(
                controller: controller,
                scrollDirection: Axis.vertical,
                onDocumentLoaded: (doc) => loadedDoc = doc,
                builders: PdfViewBuilders<DefaultBuilderOptions>(
                  options: const DefaultBuilderOptions(),
                  errorBuilder: (context, error) {
                    SoloLog.e('AttachmentPreview', 'PDF decode error', error);
                    return const Center(
                      child: Padding(
                        padding: EdgeInsets.all(32),
                        child: Icon(Icons.picture_as_pdf, size: 64),
                      ),
                    );
                  },
                ),
              ),
            ),
            // Page navigation bar
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              decoration: BoxDecoration(
                border: Border(
                  top: BorderSide(
                    color: Theme.of(context).dividerColor,
                  ),
                ),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  IconButton(
                    icon: const Icon(Icons.navigate_before),
                    tooltip: 'Previous page',
                    onPressed: () => controller.previousPage(
                      curve: Curves.ease,
                      duration: const Duration(milliseconds: 100),
                    ),
                  ),
                  PdfPageNumber(
                    controller: controller,
                    builder: (_, loadingState, page, pagesCount) {
                      final total = pagesCount ?? 0;
                      return TextButton(
                        onPressed: total <= 1
                            ? null
                            : () => _showPdfPagePicker(
                                  context,
                                  controller: controller,
                                  currentPage: page,
                                  totalPages: total,
                                ),
                        child: Text(
                          '$page / $total',
                          style: Theme.of(context).textTheme.bodyMedium,
                        ),
                      );
                    },
                  ),
                  IconButton(
                    icon: const Icon(Icons.navigate_next),
                    tooltip: 'Next page',
                    onPressed: () => controller.nextPage(
                      curve: Curves.ease,
                      duration: const Duration(milliseconds: 100),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );

    controller.dispose();
    await loadedDoc?.close();
  }

  /// 弹出页码输入对话框，快速跳转到指定页面。
  Future<void> _showPdfPagePicker(
    BuildContext context, {
    required PdfController controller,
    required int currentPage,
    required int totalPages,
  }) async {
    final textController = TextEditingController(text: '$currentPage');
    final confirmed = await showDialog<int>(
      context: context,
      builder: (context) {
        final l10n = AppLocalizations.of(context);
        return AlertDialog(
          title: const Text('跳转到页面'),
          content: TextField(
            controller: textController,
            keyboardType: TextInputType.number,
            autofocus: true,
            decoration: InputDecoration(
              labelText: '页码 (1 - $totalPages)',
              hintText: '输入页码',
            ),
            onSubmitted: (value) {
              final page = int.tryParse(value.trim());
              if (page != null && page >= 1 && page <= totalPages) {
                Navigator.of(context).pop(page);
              }
            },
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(l10n.commonCancel),
            ),
            TextButton(
              onPressed: () {
                final page = int.tryParse(textController.text.trim());
                if (page != null && page >= 1 && page <= totalPages) {
                  Navigator.of(context).pop(page);
                }
              },
              child: const Text('跳转'),
            ),
          ],
        );
      },
    );
    // 延迟 dispose，等待对话框关闭动画完成后再释放 controller
    WidgetsBinding.instance.addPostFrameCallback((_) {
      textController.dispose();
    });

    if (confirmed != null) {
      controller.jumpToPage(confirmed);
    }
  }

  /// 清理 PDF 临时文件，延迟 300ms 确保平台引擎释放文件句柄。
  Future<void> _cleanupPdfTempFile(String filePath) async {
    await Future.delayed(const Duration(milliseconds: 300));
    try {
      final file = File(filePath);
      if (file.existsSync()) {
        file.deleteSync();
      }
      final parent = file.parent;
      if (parent.path.contains('solosoul_pdf_')) {
        try {
          parent.deleteSync(recursive: true);
        } on Exception catch (_) {
          // Ignore
        }
      }
    } on Exception catch (_) {
      // Ignore cleanup errors
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  String _formatSize(int bytes) {
    if (bytes < 1024) return '${bytes}B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)}KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)}MB';
  }

  IconData _iconForMime(String mime, String fileName) {
    if (mime.startsWith('image/')) return Icons.image;
    if (mime == 'application/pdf') return Icons.picture_as_pdf;
    if (mime == mimeTypePptx ||
        mime == mimeTypePpt ||
        _isOfficeDocumentByExtension(fileName, ext: 'pptx') ||
        _isOfficeDocumentByExtension(fileName, ext: 'ppt')) {
      return Icons.slideshow;
    }
    if (mime == mimeTypeDocx ||
        mime == mimeTypeDoc ||
        _isOfficeDocumentByExtension(fileName, ext: 'docx') ||
        _isOfficeDocumentByExtension(fileName, ext: 'doc')) {
      return Icons.description;
    }
    if (mime == mimeTypeXlsx ||
        mime == mimeTypeXls ||
        _isOfficeDocumentByExtension(fileName, ext: 'xlsx') ||
        _isOfficeDocumentByExtension(fileName, ext: 'xls')) {
      return Icons.table_chart;
    }
    return Icons.insert_drive_file;
  }

  /// 判断附件是否为 Office 文档（PPTX/PPT/DOCX/DOC/XLSX/XLS）。
  bool _isOfficeDocument(Attachment attachment) {
    final mime = attachment.mimeType;
    final name = attachment.fileName;
    return mime == mimeTypePptx ||
        mime == mimeTypePpt ||
        mime == mimeTypeDocx ||
        mime == mimeTypeDoc ||
        mime == mimeTypeXlsx ||
        mime == mimeTypeXls ||
        _isOfficeDocumentByExtension(name, ext: 'pptx') ||
        _isOfficeDocumentByExtension(name, ext: 'ppt') ||
        _isOfficeDocumentByExtension(name, ext: 'docx') ||
        _isOfficeDocumentByExtension(name, ext: 'doc') ||
        _isOfficeDocumentByExtension(name, ext: 'xlsx') ||
        _isOfficeDocumentByExtension(name, ext: 'xls');
  }

  /// 根据文件名扩展名判断是否为指定 Office 文档类型。
  /// 用于 MIME 类型识别失败时的 fallback（如旧数据为 application/octet-stream）。
  bool _isOfficeDocumentByExtension(String fileName, {required String ext}) {
    final nameLower = fileName.toLowerCase();
    return nameLower.endsWith('.$ext');
  }

  String _formatDeletedAt(int millis) {
    final dt = DateTime.fromMillisecondsSinceEpoch(millis);
    final days = DateTime.now().difference(dt).inDays;
    final l10n = AppLocalizations.of(context);
    if (days <= 0) return l10n.loginToday;
    return l10n.loginDaysAgo(days);
  }

  String _formatDate(int millis) {
    final dt = DateTime.fromMillisecondsSinceEpoch(millis);
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }

  String _statusText(TaskStatus status, AppLocalizations l10n) {
    return switch (status) {
      TaskStatus.pending => l10n.uploading,
      TaskStatus.encrypting => l10n.encryptionInProgress,
      TaskStatus.writing => l10n.uploading,
      TaskStatus.completed => l10n.uploading,
      TaskStatus.cancelled => l10n.uploadCancelled,
      TaskStatus.error => l10n.attachmentAddFailed,
    };
  }

  // ---------------------------------------------------------------------------
  // UI Builders
  // ---------------------------------------------------------------------------

  Widget _buildUploadTaskItem(UploadTask task, AppLocalizations l10n, ThemeData theme) {
    final isEncrypting = task.status == TaskStatus.encrypting;

    return ListTile(
      leading: Icon(
        Icons.insert_drive_file,
        color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
      ),
      title: Text(
        task.fileName,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
        ),
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '${_formatSize(task.size)} • ${_statusText(task.status, l10n)}',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
            ),
          ),
          const SizedBox(height: 4),
          ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: LinearProgressIndicator(
              value: task.progress,
              minHeight: 4,
              backgroundColor: theme.colorScheme.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation<Color>(
                theme.colorScheme.primary.withValues(alpha: 0.5),
              ),
            ),
          ),
        ],
      ),
      trailing: IconButton(
        icon: Icon(
          isEncrypting ? Icons.hourglass_empty : Icons.close,
          size: 20,
          color: isEncrypting
              ? theme.colorScheme.onSurface.withValues(alpha: 0.3)
              : theme.colorScheme.onSurface.withValues(alpha: 0.5),
        ),
        tooltip: isEncrypting ? l10n.encryptionInProgress : l10n.cancelUpload,
        onPressed: isEncrypting ? null : () => _cancelUpload(task.tempId),
        visualDensity: VisualDensity.compact,
      ),
    );
  }

  Widget _buildDownloadTrailing(Attachment attachment, AppLocalizations l10n, ThemeData theme) {
    final task = _downloadTasks[attachment.id];

    // Normal download button
    if (task == null) {
      final isCompleted = _completedMap[attachment.id] == true;
      if (isCompleted) {
        return const Icon(Icons.check, size: 18, color: Colors.green);
      }
      return IconButton(
        icon: const Icon(Icons.download, size: 20),
        tooltip: l10n.downloadAttachment,
        onPressed: () => _handleDownload(attachment),
        visualDensity: VisualDensity.compact,
      );
    }

    // Download in progress
    final isDecrypting = task.status == TaskStatus.encrypting;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          width: 80,
          child: ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: LinearProgressIndicator(
              value: task.progress,
              minHeight: 4,
              backgroundColor: theme.colorScheme.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation<Color>(
                theme.colorScheme.primary.withValues(alpha: 0.7),
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        IconButton(
          icon: Icon(
            isDecrypting ? Icons.hourglass_empty : Icons.close,
            size: 18,
            color: isDecrypting
                ? theme.colorScheme.onSurface.withValues(alpha: 0.3)
                : theme.colorScheme.onSurface.withValues(alpha: 0.5),
          ),
          tooltip: isDecrypting ? l10n.encryptionInProgress : l10n.cancelDownload,
          onPressed: isDecrypting ? null : () => _cancelDownload(attachment.id),
          visualDensity: VisualDensity.compact,
        ),
      ],
    );
  }

  // ---------------------------------------------------------------------------
  // Build
  // ---------------------------------------------------------------------------

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    // Watch the object for real-time updates
    final liveObject = ref.watch(objectByIdProvider(widget.object.id));
    final object = liveObject ?? widget.object;
    final activeAttachments = object.attachments.where((a) => !a.isDeleted).toList();
    final deletedAttachments = object.attachments.where((a) => a.isDeleted).toList();

    // Combine upload tasks + active attachments for display
    final hasUploadTasks = _uploadTasks.isNotEmpty;

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Drag handle
              Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: theme.colorScheme.outlineVariant,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(height: 12),
              // Header
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: Row(
                  children: [
                    Text(
                      l10n.entryAttachments(activeAttachments.length),
                      style: theme.textTheme.titleLarge,
                    ),
                    const Spacer(),
                    IconButton(
                      icon: const Icon(Icons.attach_file, size: 22),
                      tooltip: l10n.addAttachment,
                      onPressed: _handleAddAttachment,
                    ),
                    Text(
                      '${activeAttachments.length}',
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              const Divider(height: 1),
              // Content
              if (!hasUploadTasks && activeAttachments.isEmpty)
                Padding(
                  padding: const EdgeInsets.all(32),
                  child: Text(
                    l10n.noAttachments,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                )
              else
                Flexible(
                  child: ListView.builder(
                    shrinkWrap: true,
                    itemCount: _uploadTasks.length + activeAttachments.length,
                    itemBuilder: (context, index) {
                      // Upload tasks first
                      if (index < _uploadTasks.length) {
                        final task = _uploadTasks.values.elementAt(index);
                        return _buildUploadTaskItem(task, l10n, theme);
                      }

                      // Then active attachments
                      final aIndex = index - _uploadTasks.length;
                      final a = activeAttachments[aIndex];
                      final isLoading = _loadingMap[a.id] == true;

                      return ListTile(
                        leading: Icon(
                          _iconForMime(a.mimeType, a.fileName),
                          color: theme.colorScheme.primary,
                        ),
                        title: Text(a.fileName),
                        subtitle: Text('${_formatSize(a.size)} • ${_formatDate(a.createdAt)}'),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            _buildDownloadTrailing(a, l10n, theme),
                            IconButton(
                              icon: const Icon(Icons.delete_outline, size: 20),
                              tooltip: l10n.deleteAttachment,
                              onPressed: () => _handleDelete(a),
                              visualDensity: VisualDensity.compact,
                            ),
                            isLoading
                                ? const SizedBox(
                                    width: 20,
                                    height: 20,
                                    child: CircularProgressIndicator(strokeWidth: 2),
                                  )
                                : const Icon(Icons.chevron_right),
                          ],
                        ),
                        onTap: isLoading ? null : () => _openAttachment(a),
                      );
                    },
                  ),
                ),
              // Deleted attachments panel
              if (deletedAttachments.isNotEmpty) ...[
                const Divider(height: 1),
                InkWell(
                  onTap: () => setState(() => _deletedExpanded = !_deletedExpanded),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                    child: Row(
                      children: [
                        Icon(
                          _deletedExpanded ? Icons.expand_less : Icons.expand_more,
                          size: 20,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 8),
                        Text(
                          '${l10n.deletedAttachments} (${deletedAttachments.length})',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                AnimatedCrossFade(
                  firstChild: const SizedBox.shrink(),
                  secondChild: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: deletedAttachments.map((a) {
                      return ListTile(
                        leading: Icon(
                          _iconForMime(a.mimeType, a.fileName),
                          color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
                          size: 20,
                        ),
                        title: Text(
                          a.fileName,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
                          ),
                        ),
                        subtitle: Text(
                          '${_formatSize(a.size)} • ${_formatDeletedAt(a.deletedAt!)}',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
                          ),
                        ),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            IconButton(
                              icon: const Icon(Icons.restore, size: 18),
                              tooltip: l10n.restoreAttachment,
                              onPressed: () => _handleRestore(a),
                              visualDensity: VisualDensity.compact,
                              color: theme.colorScheme.primary,
                            ),
                            IconButton(
                              icon: const Icon(Icons.delete_forever, size: 18),
                              tooltip: l10n.permanentlyDeleteAttachment,
                              onPressed: () => _handlePermanentDelete(a),
                              visualDensity: VisualDensity.compact,
                              color: theme.colorScheme.error,
                            ),
                          ],
                        ),
                      );
                    }).toList(),
                  ),
                  crossFadeState: _deletedExpanded
                      ? CrossFadeState.showSecond
                      : CrossFadeState.showFirst,
                  duration: const Duration(milliseconds: 200),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
