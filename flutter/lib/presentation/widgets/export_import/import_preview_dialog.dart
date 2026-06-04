import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/export_import_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

// =============================================================================
// Import Preview Dialog
// =============================================================================

class ImportPreviewDialog extends StatefulWidget {
  final List<ImportCollection> collections;
  final List<String> currentPages;

  const ImportPreviewDialog({
    super.key,
    required this.collections,
    required this.currentPages,
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
                    currentPages: widget.currentPages,
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
// Collection Tile
// =============================================================================

class _CollectionTile extends StatelessWidget {
  final ImportCollection collection;
  final List<String> currentPages;
  final ValueChanged<bool?> onChanged;
  final ValueChanged<String?> onTargetPageChanged;

  const _CollectionTile({
    required this.collection,
    required this.currentPages,
    required this.onChanged,
    required this.onTargetPageChanged,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Checkbox(
                  value: collection.selected,
                  onChanged: onChanged,
                ),
                Expanded(
                  child: Text(
                    collection.name,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                _SensitivityBadge(level: collection.highestSensitivity),
              ],
            ),
            Padding(
              padding: const EdgeInsets.only(left: 40),
              child: Text(
                l10n.importObjectsCount(collection.itemCount),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
            if (collection.relationPropertyCount > 0)
              Padding(
                padding: const EdgeInsets.only(left: 40, top: 4),
                child: Row(
                  children: [
                    Icon(
                      Icons.link,
                      size: 14,
                      color: collection.crossPartitionRelationCount > 0
                          ? Colors.orange
                          : Colors.grey,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      l10n.importRelationCount(
                        collection.relationPropertyCount,
                        collection.crossPartitionRelationCount,
                      ),
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: collection.crossPartitionRelationCount > 0
                                ? Colors.orange
                                : Colors.grey,
                          ),
                    ),
                  ],
                ),
              ),
            if (collection.selected && currentPages.isNotEmpty) ...[
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.only(left: 40),
                child: DropdownButtonFormField<String>(
                  initialValue: collection.targetPageId ?? currentPages.first,
                  decoration: InputDecoration(
                    labelText: l10n.importTargetPageLabel,
                    isDense: true,
                  ),
                  items: currentPages.map((page) {
                    return DropdownMenuItem(
                      value: page,
                      child: Text(page),
                    );
                  }).toList(),
                  onChanged: onTargetPageChanged,
                ),
              ),
            ],
          ],
        ),
      ),
    );
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
    final (color, label) = switch (level) {
      SensitivityLevel.public => (Colors.green, 'Public'),
      SensitivityLevel.internal => (Colors.yellow, 'Internal'),
      SensitivityLevel.sensitive => (Colors.orange, 'Sensitive'),
      SensitivityLevel.critical => (Colors.red, 'Critical'),
    };

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
