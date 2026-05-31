import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:pdfx/pdfx.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_download_service.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider, objectByIdProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

// =============================================================================
// Attachment List Sheet
// =============================================================================

/// Bottom sheet that displays a list of attachments for a given object.
/// Supports preview, soft-delete, restore, and permanent delete.
class AttachmentListSheet extends ConsumerStatefulWidget {
  final UnifiedObject object;
  final String? accountId;

  /// Optional callback to add a new attachment from within this sheet.
  final VoidCallback? onAddAttachment;

  const AttachmentListSheet({
    super.key,
    required this.object,
    required this.accountId,
    this.onAddAttachment,
  });

  @override
  ConsumerState<AttachmentListSheet> createState() =>
      _AttachmentListSheetState();
}

class _AttachmentListSheetState extends ConsumerState<AttachmentListSheet> {
  final Map<String, bool> _loadingMap = {};
  final Map<String, bool> _downloadingMap = {};
  final Map<String, bool> _completedMap = {};
  bool _deletedExpanded = false;

  bool get _isSensitive => widget.object.properties.values.any(
        (p) =>
            p.sensitivity == SensitivityLevel.sensitive ||
            p.sensitivity == SensitivityLevel.critical,
      );

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

  Future<void> _handleDownload(Attachment attachment) async {
    // Dedup: ignore if already downloading this attachment
    if (_downloadingMap[attachment.id] == true) return;

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

    setState(() {
      _downloadingMap[attachment.id] = true;
      _completedMap.remove(attachment.id);
    });

    final downloadDir = await AttachmentDownloadService().getDownloadDirectory();
    final savedPath = await AttachmentDownloadService().downloadAttachment(
      accountId: accountId,
      attachment: attachment,
      downloadDir: downloadDir,
    );

    if (!mounted) return;
    setState(() {
      _downloadingMap[attachment.id] = false;
    });

    final l10n = AppLocalizations.of(context);
    if (savedPath != null) {
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
    } else {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.attachmentDownloadFailed,
          type: SnackBarType.error,
        );
      }
    }
  }

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

  Future<void> _openAttachment(Attachment attachment) async {
    final accountId = widget.accountId;
    if (accountId == null) {
      showOverlaySnackBar(
        context,
        content: 'No account selected',
        type: SnackBarType.error,
      );
      return;
    }

    setState(() => _loadingMap[attachment.id] = true);

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
    } else if (attachment.mimeType == 'application/pdf') {
      _showPdfPreview(bytes, attachment.fileName, attachment);
    } else {
      showOverlaySnackBar(
        context,
        content: 'Preview not supported for this file type',
        type: SnackBarType.info,
      );
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

  void _showPdfPreview(Uint8List bytes, String fileName, Attachment attachment) {
    final controller = PdfController(
      document: PdfDocument.openData(bytes),
    );

    showDialog(
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
          ],
        ),
      ),
    ).then((_) => controller.dispose());
  }

  String _formatSize(int bytes) {
    if (bytes < 1024) return '${bytes}B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)}KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)}MB';
  }

  IconData _iconForMime(String mime) {
    if (mime.startsWith('image/')) return Icons.image;
    if (mime == 'application/pdf') return Icons.picture_as_pdf;
    return Icons.insert_drive_file;
  }

  String _formatDeletedAt(int millis) {
    final dt = DateTime.fromMillisecondsSinceEpoch(millis);
    final days = DateTime.now().difference(dt).inDays;
    final l10n = AppLocalizations.of(context);
    if (days <= 0) return l10n.loginToday;
    return l10n.loginDaysAgo(days);
  }

  Widget _buildDownloadButton(Attachment attachment) {
    final isDownloading = _downloadingMap[attachment.id] == true;
    final isCompleted = _completedMap[attachment.id] == true;

    if (isDownloading) {
      return const SizedBox(
        width: 20,
        height: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      );
    }

    if (isCompleted) {
      return const Icon(Icons.check, size: 18, color: Colors.green);
    }

    return IconButton(
      icon: const Icon(Icons.download, size: 20),
      tooltip: AppLocalizations.of(context).downloadAttachment,
      onPressed: () => _handleDownload(attachment),
      visualDensity: VisualDensity.compact,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    // Watch the object for real-time updates
    final liveObject = ref.watch(objectByIdProvider(widget.object.id));
    final object = liveObject ?? widget.object;
    final activeAttachments = object.attachments.where((a) => !a.isDeleted).toList();
    final deletedAttachments = object.attachments.where((a) => a.isDeleted).toList();

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
                    if (widget.onAddAttachment != null)
                      IconButton(
                        icon: const Icon(Icons.attach_file, size: 22),
                        tooltip: l10n.addAttachment,
                        onPressed: () {
                          Navigator.of(context).pop();
                          widget.onAddAttachment!();
                        },
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
              // Active attachments list
              if (activeAttachments.isEmpty)
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
                    itemCount: activeAttachments.length,
                    itemBuilder: (context, index) {
                      final a = activeAttachments[index];
                      final isLoading = _loadingMap[a.id] == true;

                      return ListTile(
                        leading: Icon(
                          _iconForMime(a.mimeType),
                          color: theme.colorScheme.primary,
                        ),
                        title: Text(a.fileName),
                        subtitle: Text('${_formatSize(a.size)} • ${_formatDate(a.createdAt)}'),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            _buildDownloadButton(a),
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
                          _iconForMime(a.mimeType),
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

  String _formatDate(int millis) {
    final dt = DateTime.fromMillisecondsSinceEpoch(millis);
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }
}
