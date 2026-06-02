import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/pptx_thumbnail_extractor.dart';
import 'package:solosoul_flutter/core/services/quicklook_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart'
    show AppLocalizations;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

// =============================================================================
// PPTX Preview Dialog
// =============================================================================

/// PPTX 预览对话框。
///
/// 显示提取的第一页缩略图（如果有），并提供"浏览所有幻灯片"按钮。
/// 点击后调用系统原生 QuickLook（macOS）或系统默认应用（其他平台）。
///
/// 临时文件在 [dispose] 时自动清理。
class PptxPreviewDialog extends StatefulWidget {
  final String fileName;
  final int fileSize;
  final String filePath;
  final Attachment attachment;
  final VoidCallback onDownload;

  const PptxPreviewDialog({
    super.key,
    required this.fileName,
    required this.fileSize,
    required this.filePath,
    required this.attachment,
    required this.onDownload,
  });

  @override
  State<PptxPreviewDialog> createState() => _PptxPreviewDialogState();
}

class _PptxPreviewDialogState extends State<PptxPreviewDialog> {
  Uint8List? _thumbnailBytes;
  bool _loadingThumbnail = true;
  bool _opening = false;

  @override
  void initState() {
    super.initState();
    _loadThumbnail();
  }

  Future<void> _loadThumbnail() async {
    final thumbnail = PptxThumbnailExtractor.extractThumbnailFromPath(
      widget.filePath,
    );
    if (mounted) {
      setState(() {
        _thumbnailBytes = thumbnail;
        _loadingThumbnail = false;
      });
    }
  }

  Future<void> _browseAllSlides() async {
    if (_opening) return;
    setState(() => _opening = true);

    try {
      await QuickLookService().showWithFallback(
        widget.filePath,
        onClosed: () {
          // QuickLook 面板关闭后删除临时文件
          _cleanupTempFile();
        },
      );
    } on Exception catch (e) {
      SoloLog.e('PptxPreview', 'Failed to browse slides', e);
      if (mounted) {
        final l10n = AppLocalizations.of(context);
        showOverlaySnackBar(
          context,
          content: '${l10n.commonError}: $e',
          type: SnackBarType.error,
        );
      }
    } finally {
      if (mounted) {
        setState(() => _opening = false);
      }
    }
  }

  void _cleanupTempFile() {
    try {
      final file = File(widget.filePath);
      if (file.existsSync()) {
        file.deleteSync();
      }
      final parent = file.parent;
      if (parent.path.contains('solosoul_pptx_')) {
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

  String _formatSize(int bytes) {
    if (bytes < 1024) return '${bytes}B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)}KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)}MB';
  }

  String _browseButtonLabel(AppLocalizations l10n) {
    if (Platform.isMacOS || Platform.isIOS) {
      if (_isDocument) return l10n.browseAllPages;
      if (_isSpreadsheet) return l10n.browseAllSheets;
      if (_isPresentation) return l10n.browseAllSlides;
      return l10n.openWithSystemApp;
    }
    return l10n.openWithSystemApp;
  }

  // ---------------------------------------------------------------------------
  // MIME type helpers
  // ---------------------------------------------------------------------------

  String get _mime => widget.attachment.mimeType;
  String get _fileNameLower => widget.fileName.toLowerCase();

  bool get _isDocument =>
      _mime == 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' ||
      _mime == 'application/msword' ||
      _fileNameLower.endsWith('.docx') ||
      _fileNameLower.endsWith('.doc');

  bool get _isSpreadsheet =>
      _mime == 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' ||
      _mime == 'application/vnd.ms-excel' ||
      _fileNameLower.endsWith('.xlsx') ||
      _fileNameLower.endsWith('.xls');

  bool get _isPresentation =>
      _mime == 'application/vnd.openxmlformats-officedocument.presentationml.presentation' ||
      _mime == 'application/vnd.ms-powerpoint' ||
      _fileNameLower.endsWith('.pptx') ||
      _fileNameLower.endsWith('.ppt');

  String _previewTitle(AppLocalizations l10n) {
    if (_isDocument) return l10n.docxPreviewTitle;
    if (_isSpreadsheet) return l10n.xlsxPreviewTitle;
    if (_isPresentation) return l10n.pptxPreviewTitle;
    return l10n.pptxPreviewTitle;
  }

  IconData get _fallbackIcon {
    if (_isDocument) return Icons.description;
    if (_isSpreadsheet) return Icons.table_chart;
    if (_isPresentation) return Icons.slideshow;
    return Icons.insert_drive_file;
  }

  String _noThumbnailText(AppLocalizations l10n) {
    if (_isDocument) return l10n.docxNoThumbnail;
    if (_isSpreadsheet) return l10n.xlsxNoThumbnail;
    if (_isPresentation) return l10n.pptxNoThumbnail;
    return l10n.pptxNoThumbnail;
  }

  @override
  void dispose() {
    _cleanupTempFile();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final hasThumbnail = _thumbnailBytes != null;

    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480, maxHeight: 640),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Title bar
            AppBar(
              title: Text(
                _previewTitle(l10n),
                style: theme.textTheme.titleMedium,
              ),
              actions: [
                IconButton(
                  icon: const Icon(Icons.download),
                  tooltip: l10n.downloadAttachment,
                  onPressed: widget.onDownload,
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  tooltip: l10n.commonClose,
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            // Content
            Flexible(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(20),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Thumbnail or placeholder
                    Container(
                      constraints: const BoxConstraints(maxHeight: 360),
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(12),
                        color: theme.colorScheme.surfaceContainerHighest,
                      ),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(12),
                        child: _loadingThumbnail
                            ? SizedBox(
                                height: 200,
                                child: Center(
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                    color: theme.colorScheme.primary,
                                  ),
                                ),
                              )
                            : hasThumbnail
                                ? Image.memory(
                                    _thumbnailBytes!,
                                    fit: BoxFit.contain,
                                    errorBuilder: (context, error, stackTrace) {
                                      return _buildFallbackUI(theme, l10n);
                                    },
                                  )
                                : _buildFallbackUI(theme, l10n),
                      ),
                    ),
                    const SizedBox(height: 20),
                    // File info
                    Text(
                      widget.fileName,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                      textAlign: TextAlign.center,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      _formatSize(widget.fileSize),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(height: 20),
                    // Open with system app / QuickLook button
                    SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: _opening ? null : _browseAllSlides,
                        icon: _opening
                            ? const SizedBox(
                                width: 18,
                                height: 18,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                  color: Colors.white,
                                ),
                              )
                            : const Icon(Icons.open_in_new),
                        label: Text(_browseButtonLabel(l10n)),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFallbackUI(ThemeData theme, AppLocalizations l10n) {
    return SizedBox(
      height: 200,
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              _fallbackIcon,
              size: 64,
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
            ),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Text(
                _noThumbnailText(l10n),
                textAlign: TextAlign.center,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
