import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/export_import_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

// =============================================================================
// Import Preview Dialog
// =============================================================================

class ImportPreviewDialog extends StatefulWidget {
  final List<ImportCollection> collections;
  final List<ImportTargetPageOption> pageOptions;
  final List<UnifiedObject> currentObjects;

  const ImportPreviewDialog({
    super.key,
    required this.collections,
    required this.pageOptions,
    required this.currentObjects,
  });

  @override
  State<ImportPreviewDialog> createState() => _ImportPreviewDialogState();
}

class _ImportPreviewDialogState extends State<ImportPreviewDialog> {
  late List<ImportCollection> _collections;

  @override
  void initState() {
    super.initState();
    _collections = List.from(widget.collections);
    // Set default target page for each collection
    for (final col in _collections) {
      col.targetPageId ??= col.originalParentPageId ?? _fallbackPageId();
    }
  }

  String? _fallbackPageId() {
    if (widget.pageOptions.isEmpty) return null;
    return widget.pageOptions.first.pageId;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final selectedCount = _collections.where((c) => c.selected).length;
    final totalItems = _collections
        .where((c) => c.selected)
        .fold<int>(0, (sum, c) => sum + c.itemCount);

    return AlertDialog(
      title: Text(l10n.importPreviewTitle),
      content: SizedBox(
        width: double.maxFinite,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.importPreviewSelectedCount(selectedCount, totalItems),
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 12),
            Flexible(
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: _collections.length,
                itemBuilder: (context, index) {
                  final col = _collections[index];
                  return _CollectionTile(
                    collection: col,
                    pageOptions: widget.pageOptions,
                    currentObjects: widget.currentObjects,
                    onChanged: (value) {
                      setState(() {
                        col.selected = value ?? false;
                      });
                    },
                    onTargetPageChanged: (pageId) {
                      setState(() {
                        col.targetPageId = pageId;
                      });
                    },
                  );
                },
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.commonCancel),
        ),
        FilledButton(
          onPressed: selectedCount > 0
              ? () => Navigator.of(context).pop(_collections)
              : null,
          child: Text(l10n.importConfirm),
        ),
      ],
    );
  }
}

// =============================================================================
// Collection Tile (expandable)
// =============================================================================

class _CollectionTile extends StatefulWidget {
  final ImportCollection collection;
  final List<ImportTargetPageOption> pageOptions;
  final List<UnifiedObject> currentObjects;
  final ValueChanged<bool?> onChanged;
  final ValueChanged<String?> onTargetPageChanged;

  const _CollectionTile({
    required this.collection,
    required this.pageOptions,
    required this.currentObjects,
    required this.onChanged,
    required this.onTargetPageChanged,
  });

  @override
  State<_CollectionTile> createState() => _CollectionTileState();
}

class _CollectionTileState extends State<_CollectionTile> {
  bool _expanded = false;

  /// Get localized display name for the collection.
  String _getLocalizedName(AppLocalizations l10n, ImportCollection col) {
    // Try preset section config by section ID
    final config = SectionRendererRegistry.getConfigBySectionId(col.originalId);
    if (config != null) return config.l10nTitle(l10n);
    // Fallback to raw name
    return col.name;
  }

  /// Find matching section name in the target page, or null if no match.
  String? _findMatchingSectionName(ImportCollection col) {
    final targetPageId = col.targetPageId;
    if (targetPageId == null) return null;

    // Find target page in current objects
    final page = widget.currentObjects.firstWhereOrNull(
      (o) => o.id == targetPageId && o.typeId == 'page',
    );
    if (page == null) return null;

    // Get all non-page children (sections)
    final sections = page.childrenIds
        .map((id) => widget.currentObjects.firstWhereOrNull((o) => o.id == id))
        .whereType<UnifiedObject>()
        .where((o) => o.typeId != 'page');

    // Match by name (case-insensitive, trimmed)
    final importName = col.name.trim().toLowerCase();
    final match = sections.firstWhereOrNull(
      (s) => s.name.trim().toLowerCase() == importName,
    );

    return match?.name;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final col = widget.collection;
    final displayName = _getLocalizedName(l10n, col);
    final matchedSectionName = _findMatchingSectionName(col);

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header row
          Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Checkbox(
                      value: col.selected,
                      onChanged: widget.onChanged,
                    ),
                    Expanded(
                      child: Text(
                        displayName,
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                    ),
                    _SensitivityBadge(level: col.highestSensitivity),
                    const SizedBox(width: 4),
                    // Expand/collapse button
                    IconButton(
                      icon: Icon(
                        _expanded ? Icons.expand_less : Icons.expand_more,
                        size: 20,
                      ),
                      onPressed: () => setState(() => _expanded = !_expanded),
                      tooltip: _expanded ? l10n.commonCollapse : l10n.commonExpand,
                    ),
                  ],
                ),
                Padding(
                  padding: const EdgeInsets.only(left: 40),
                  child: Text(
                    l10n.importObjectsCount(col.itemCount),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
                if (col.relationPropertyCount > 0)
                  Padding(
                    padding: const EdgeInsets.only(left: 40, top: 4),
                    child: Row(
                      children: [
                        Icon(
                          Icons.link,
                          size: 14,
                          color: col.crossPartitionRelationCount > 0
                              ? Colors.orange
                              : Colors.grey,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          l10n.importRelationCount(
                            col.relationPropertyCount,
                            col.crossPartitionRelationCount,
                          ),
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: col.crossPartitionRelationCount > 0
                                    ? Colors.orange
                                    : Colors.grey,
                              ),
                        ),
                      ],
                    ),
                  ),
                // Section merge/create hint
                if (col.selected) ...[
                  const SizedBox(height: 4),
                  Padding(
                    padding: const EdgeInsets.only(left: 40),
                    child: Text(
                      matchedSectionName != null
                          ? l10n.importSectionWillMerge(matchedSectionName)
                          : l10n.importSectionWillCreate(col.name),
                      style: TextStyle(
                        color: matchedSectionName != null ? Colors.blue : Colors.grey,
                        fontSize: 12,
                      ),
                    ),
                  ),
                ],
                if (col.selected && widget.pageOptions.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Padding(
                    padding: const EdgeInsets.only(left: 40),
                    child: DropdownButtonFormField<String>(
                      // ignore: deprecated_member_use
                      value: col.targetPageId ?? widget.pageOptions.first.pageId,
                      decoration: InputDecoration(
                        labelText: l10n.importTargetPageLabel,
                        isDense: true,
                      ),
                      items: widget.pageOptions.map((opt) {
                        return DropdownMenuItem(
                          value: opt.pageId,
                          child: Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(text: opt.displayName),
                                if (!opt.exists)
                                  TextSpan(
                                    text: ' ${l10n.importPageNotExistsAutoCreate}',
                                    style: const TextStyle(
                                      color: Colors.orange,
                                      fontSize: 11,
                                    ),
                                  ),
                              ],
                            ),
                            overflow: TextOverflow.ellipsis,
                            maxLines: 1,
                          ),
                        );
                      }).toList(),
                      onChanged: widget.onTargetPageChanged,
                    ),
                  ),
                ],
              ],
            ),
          ),

          // Expanded content
          if (_expanded) ...[
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Items list
                  if (col.items.isNotEmpty) ...[
                    Text(
                      l10n.importItemsLabel,
                      style: Theme.of(context).textTheme.labelLarge?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 8),
                    ...col.items.map((item) => _ObjectItemTile(object: item)),
                  ],

                  if (col.items.isEmpty)
                    Text(
                      l10n.importNoDetails,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Colors.grey,
                          ),
                    ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

// =============================================================================
// Object Item Tile (inside expanded collection)
// =============================================================================

class _ObjectItemTile extends StatelessWidget {
  final UnifiedObject object;

  const _ObjectItemTile({required this.object});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      elevation: 0,
      color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Object name + icon
            Row(
              children: [
                Icon(
                  _iconDataFor(object.iconName),
                  size: 16,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    object.name.isNotEmpty ? object.name : l10n.importUnnamedItem,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w500,
                        ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),

            // Properties
            if (object.properties.isNotEmpty) ...[
              const SizedBox(height: 6),
              ...object.properties.entries.map((entry) {
                final displayValue = propertyValueToDisplay(entry.value, l10n);
                final isSensitive = entry.value.sensitivity.index >=
                    SensitivityLevel.sensitive.index;
                return Padding(
                  padding: const EdgeInsets.only(left: 22, top: 2),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${entry.key}: ',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Colors.grey.shade600,
                            ),
                      ),
                      Expanded(
                        child: Text(
                          isSensitive && displayValue.isNotEmpty
                              ? '••••••'
                              : (displayValue.isEmpty ? '-' : displayValue),
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                fontWeight: isSensitive ? FontWeight.w500 : null,
                              ),
                          overflow: TextOverflow.ellipsis,
                          maxLines: 2,
                        ),
                      ),
                    ],
                  ),
                );
              }),
            ],

            // Object attachments
            if (object.attachments.isNotEmpty) ...[
              const SizedBox(height: 6),
              ...object.attachments.map((att) => _AttachmentTile(attachment: att)),
            ],
          ],
        ),
      ),
    );
  }

  IconData _iconDataFor(String iconName) {
    return switch (iconName) {
      'note' => Icons.note_outlined,
      'task' => Icons.check_circle_outlined,
      'event' => Icons.event_outlined,
      'contact' => Icons.person_outlined,
      'link' => Icons.link_outlined,
      'image' => Icons.image_outlined,
      'file' => Icons.insert_drive_file_outlined,
      'password' => Icons.password_outlined,
      'bookmark' => Icons.bookmark_outlined,
      'location' => Icons.location_on_outlined,
      'wallet' => Icons.account_balance_wallet_outlined,
      'flight' => Icons.flight_outlined,
      'hotel' => Icons.hotel_outlined,
      'restaurant' => Icons.restaurant_outlined,
      _ => Icons.folder_outlined,
    };
  }
}

// =============================================================================
// Attachment Tile
// =============================================================================

class _AttachmentTile extends StatelessWidget {
  final Attachment attachment;

  const _AttachmentTile({required this.attachment});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 4),
      child: Row(
        children: [
          Icon(
            _mimeIcon(attachment.mimeType),
            size: 16,
            color: Theme.of(context).colorScheme.primary,
          ),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              attachment.fileName,
              style: Theme.of(context).textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          Text(
            _formatSize(attachment.size),
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Colors.grey,
                ),
          ),
        ],
      ),
    );
  }

  IconData _mimeIcon(String mime) {
    if (mime.startsWith('image/')) return Icons.image_outlined;
    if (mime.startsWith('video/')) return Icons.videocam_outlined;
    if (mime.startsWith('audio/')) return Icons.audiotrack_outlined;
    if (mime.contains('pdf')) return Icons.picture_as_pdf_outlined;
    return Icons.insert_drive_file_outlined;
  }

  String _formatSize(int bytes) {
    if (bytes < 1024) return '${bytes}B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)}KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)}MB';
  }
}

// =============================================================================
// Sensitivity Badge
// =============================================================================

class _SensitivityBadge extends StatelessWidget {
  final SensitivityLevel level;

  const _SensitivityBadge({required this.level});

  @override
  Widget build(BuildContext context) {
    final color = getSensitivityColor(level);
    final label = getSensitivityLabel(level);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.5)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
