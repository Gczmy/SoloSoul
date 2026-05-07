import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_storage_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

// =============================================================================
// Attachment List Sheet
// =============================================================================

/// Bottom sheet that displays a list of attachments for a given object.
/// Tapping an item decrypts and previews the file.
class AttachmentListSheet extends ConsumerStatefulWidget {
  final List<Attachment> attachments;
  final String? accountId;

  const AttachmentListSheet({
    super.key,
    required this.attachments,
    required this.accountId,
  });

  @override
  ConsumerState<AttachmentListSheet> createState() =>
      _AttachmentListSheetState();
}

class _AttachmentListSheetState extends ConsumerState<AttachmentListSheet> {
  final Map<String, bool> _loadingMap = {};

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
      _showImagePreview(bytes, attachment.fileName);
    } else {
      showOverlaySnackBar(
        context,
        content: 'Preview not supported for this file type',
        type: SnackBarType.info,
      );
    }
  }

  void _showImagePreview(Uint8List bytes, String fileName) {
    showDialog(
      context: context,
      builder: (context) => Dialog(
        insetPadding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AppBar(
              title: Text(fileName),
              leading: IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.of(context).pop(),
              ),
              automaticallyImplyLeading: false,
            ),
            Flexible(
              child: InteractiveViewer(
                minScale: 0.5,
                maxScale: 4.0,
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
            ),
          ],
        ),
      ),
    );
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Row(
                children: [
                  Text(
                    'Attachments',
                    style: theme.textTheme.titleLarge,
                  ),
                  const Spacer(),
                  Text(
                    '${widget.attachments.length}',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 8),
            const Divider(height: 1),
            Flexible(
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: widget.attachments.length,
                itemBuilder: (context, index) {
                  final a = widget.attachments[index];
                  final isLoading = _loadingMap[a.id] == true;

                  return ListTile(
                    leading: Icon(
                      _iconForMime(a.mimeType),
                      color: theme.colorScheme.primary,
                    ),
                    title: Text(a.fileName),
                    subtitle: Text('${_formatSize(a.size)} • ${_formatDate(a.createdAt)}'),
                    trailing: isLoading
                        ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.chevron_right),
                    onTap: isLoading ? null : () => _openAttachment(a),
                  );
                },
              ),
            ),
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
