import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show deletedChildrenProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/widgets/field_history_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/form_field_def.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

/// Returns the color for a given typeId (page=blue, collection=green, item=orange).
Color typeColorForId(String? typeId) {
  return switch (typeId) {
    'page' => Colors.blue.shade700,
    'collection' => Colors.green.shade700,
    'item' => Colors.orange.shade700,
    _ => Colors.orange.shade700, // Predefined items (travel_passport, profile_identity, etc.)
  };
}

class UnifiedObjectTrashCard extends ConsumerStatefulWidget {
  final UnifiedObject object;
  final VoidCallback onRestore;
  final VoidCallback onPurge;

  const UnifiedObjectTrashCard({
    super.key,
    required this.object,
    required this.onRestore,
    required this.onPurge,
  });

  @override
  ConsumerState<UnifiedObjectTrashCard> createState() =>
      _UnifiedObjectTrashCardState();
}

class _UnifiedObjectTrashCardState
    extends ConsumerState<UnifiedObjectTrashCard> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final object = widget.object;
    final isCollection = object.typeId == 'collection';
    final isPage = object.typeId == 'page';
    final isExpandable = isCollection || isPage;
    final deletedAt = object.deletedAt;
    final daysRemaining = deletedAt != null
        ? 30 - DateTime.now().difference(deletedAt).inDays
        : 30;
    final isExpiringSoon = daysRemaining <= 7;

    final fieldPrefix = fieldPrefixForTypeId(object.typeId ?? '');
    final history = fieldPrefix.isNotEmpty
        ? ref.watch(fieldHistoriesProvider.select(
            (h) => h.getHistory(object.id, fieldPrefix),
          ))
        : null;

    final typeDef = ObjectTypeRegistry.getType(object.typeId ?? '');
    final fieldDefs = typeDef?.properties.map((prop) {
          return FormFieldDef(fieldId: prop.id, label: prop.name);
        }).toList() ??
        [];

    final children = isExpandable
        ? ref.watch(deletedChildrenProvider(object.id))
        : <UnifiedObject>[];

    return Card(
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // Icon + name row
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
            child: Row(
              children: [
                Container(
                  width: 40,
                  height: 40,
                  decoration: BoxDecoration(
                    color: _typeColor(object.typeId).withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    UnifiedObjectService.getIconFromName(object.iconName),
                    color: _typeColor(object.typeId),
                    size: 20,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      object.name.trim().isNotEmpty
                          ? Text(
                              object.name,
                              style: theme.textTheme.titleSmall?.copyWith(
                                fontWeight: FontWeight.w600,
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            )
                          : Text.rich(
                              TextSpan(
                                children: [
                                  TextSpan(
                                    text: '${translateFieldLabel('Title', l10n)}: ',
                                    style: theme.textTheme.titleSmall?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant,
                                    ),
                                  ),
                                  TextSpan(
                                    text: l10n.commonEmpty,
                                    style: theme.textTheme.titleSmall?.copyWith(
                                      color: theme.colorScheme.onSurfaceVariant,
                                      fontStyle: FontStyle.italic,
                                    ),
                                  ),
                                ],
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                      const SizedBox(height: 2),
                      Text(
                        _localizedTypeId(object.typeId, l10n),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                if (isExpiringSoon)
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.orange.shade100,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Text(
                      '$daysRemaining days',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: Colors.orange.shade800,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          // Action bar with section-specific background
          Container(
            width: double.infinity,
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final narrow = constraints.maxWidth < 420;
                return Row(
                  children: [
                    Icon(
                      Icons.access_time,
                      size: 14,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 4),
                    Expanded(
                      child: Text(
                        deletedAt != null
                            ? l10n.trashDeletedAgo(
                                _formatTimeAgo(deletedAt, l10n))
                            : l10n.trashDeletedRecently,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        overflow: TextOverflow.ellipsis,
                        maxLines: 1,
                      ),
                    ),
                    _ActionButtonWidget(
                      narrow: narrow,
                      icon: Icons.info_outline,
                      label: l10n.trashDetailLabel,
                      onPressed: () =>
                          _showDetailDialog(widget.object),
                    ),
                    const SizedBox(width: 4),
                    _HistoryButtonWidget(
                      narrow: narrow,
                      count: history?.entries.length ?? 0,
                      onShowHistory: () => FieldHistoryDialog.show(
                        context: context,
                        title: object.name,
                        icon: UnifiedObjectService.getIconFromName(
                          object.iconName,
                        ),
                        fieldDefs: fieldDefs,
                        history: history,
                        fieldPrefix: fieldPrefix,
                      ),
                    ),
                    const SizedBox(width: 4),
                    if (isExpandable && children.isNotEmpty) ...[
                      _ActionButtonWidget(
                        narrow: narrow,
                        icon: _expanded
                            ? Icons.expand_less
                            : Icons.expand_more,
                        label: isPage
                            ? l10n.trashShowSections
                            : l10n.trashShowItems,
                        onPressed: () =>
                            setState(() => _expanded = !_expanded),
                      ),
                      const SizedBox(width: 4),
                    ],
                    _ActionButtonWidget(
                      narrow: narrow,
                      icon: Icons.restore_from_trash,
                      label: l10n.trashRestoreLabel,
                      onPressed: widget.onRestore,
                    ),
                    const SizedBox(width: 4),
                    _ActionButtonWidget(
                      narrow: narrow,
                      icon: Icons.delete_forever,
                      label: l10n.trashPurgeLabel,
                      onPressed: widget.onPurge,
                      color: AppTheme.errorColor,
                    ),
                  ],
                );
              },
            ),
          ),
          // Expandable children panel
          if (isExpandable && _expanded && children.isNotEmpty)
            _ChildrenPanel(
              children: children,
              theme: theme,
              l10n: l10n,
              onShowDetail: _showDetailDialog,
            ),
        ],
      ),
    );
  }

  String _localizedTypeId(String? typeId, AppLocalizations l10n) {
    return switch (typeId) {
      'collection' => l10n.typeCollection,
      'page' => l10n.typePage,
      'item' => l10n.typeItem,
      _ => l10n.typeUnknown,
    };
  }

  Color _typeColor(String? typeId) {
    return typeColorForId(typeId);
  }

  void _showDetailDialog(UnifiedObject object) {
    final context = this.context;
    final ref = this.ref;
    final l10n = AppLocalizations.of(context);
    final fieldPrefix = fieldPrefixForTypeId(object.typeId ?? '');
    final deletedAt = object.deletedAt;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(object.name),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              if (deletedAt != null) ...[
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Theme.of(ctx)
                        .colorScheme
                        .errorContainer
                        .withValues(alpha: 0.3),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.delete_outline,
                            size: 16,
                            color: Theme.of(ctx).colorScheme.error,
                          ),
                          const SizedBox(width: 8),
                          Text(
                            l10n.trashDeletedAgo(
                                _formatTimeAgo(deletedAt, l10n)),
                            style: Theme.of(ctx)
                                .textTheme
                                .bodyMedium
                                ?.copyWith(
                                  color: Theme.of(ctx).colorScheme.error,
                                  fontWeight: FontWeight.w600,
                                ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        _formatFullTimestamp(deletedAt),
                        style: Theme.of(ctx).textTheme.bodySmall?.copyWith(
                              color: Theme.of(ctx)
                                  .colorScheme
                                  .onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
              ],
              ...object.properties.entries.map((e) {
                final value = e.value;
                final text = propValueToString(value);
                final fieldId = fieldPrefix.isNotEmpty
                    ? '$fieldPrefix.${e.key}'
                    : e.key;
                final sensitivity =
                    ref.read(effectiveSensitivityProvider(fieldId));
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              translateFieldLabel(e.key, l10n),
                              style: Theme.of(ctx)
                                  .textTheme
                                  .labelSmall
                                  ?.copyWith(
                                    color: Theme.of(ctx)
                                        .colorScheme
                                        .onSurfaceVariant,
                                  ),
                            ),
                            const SizedBox(height: 2),
                            if (text.isNotEmpty)
                              Text(
                                text,
                                style: Theme.of(ctx).textTheme.bodyMedium,
                              )
                            else
                              Text(
                                l10n.commonEmpty,
                                style: Theme.of(ctx).textTheme.bodyMedium?.copyWith(
                                  color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                                  fontStyle: FontStyle.italic,
                                ),
                              ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 8),
                      SensitivityTag(level: sensitivity),
                    ],
                  ),
                );
              }),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(l10n.commonClose),
          ),
        ],
      ),
    );
  }

  String _formatFullTimestamp(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  String _formatTimeAgo(DateTime date, AppLocalizations l10n) {
    final diff = DateTime.now().difference(date);
    if (diff.inDays > 0) {
      return l10n.trashDaysAgo(diff.inDays);
    } else if (diff.inHours > 0) {
      return l10n.trashHoursAgo(diff.inHours);
    } else if (diff.inMinutes > 0) {
      return l10n.trashMinutesAgo(diff.inMinutes);
    } else {
      return l10n.trashJustNow;
    }
  }
}

/// Children items panel shown inside an expanded section trash card.
class _ChildrenPanel extends StatelessWidget {
  final List<UnifiedObject> children;
  final ThemeData theme;
  final AppLocalizations l10n;
  final ValueChanged<UnifiedObject>? onShowDetail;

  const _ChildrenPanel({
    required this.children,
    required this.theme,
    required this.l10n,
    this.onShowDetail,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        borderRadius: const BorderRadius.vertical(
          bottom: Radius.circular(12),
        ),
      ),
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Divider(height: 1, color: theme.dividerColor),
          const SizedBox(height: 8),
          for (final child in children) ...[
            _ChildItemRow(child: child, onShowDetail: onShowDetail),
            if (child != children.last) const SizedBox(height: 2),
          ],
        ],
      ),
    );
  }
}

/// Single child item row inside a section's expanded panel.
class _ChildItemRow extends ConsumerStatefulWidget {
  final UnifiedObject child;
  final ValueChanged<UnifiedObject>? onShowDetail;

  const _ChildItemRow({required this.child, this.onShowDetail});

  @override
  ConsumerState<_ChildItemRow> createState() => _ChildItemRowState();
}

class _ChildItemRowState extends ConsumerState<_ChildItemRow> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final isCollection = widget.child.typeId == 'collection';
    final grandchildren = isCollection
        ? ref.watch(deletedChildrenProvider(widget.child.id))
        : <UnifiedObject>[];

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Row(
            children: [
              Icon(
                Icons.circle,
                size: 6,
                color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
              ),
              const SizedBox(width: 10),
              Icon(
                UnifiedObjectService.getIconFromName(widget.child.iconName),
                size: 16,
                color: typeColorForId(widget.child.typeId),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  widget.child.name,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              if (widget.onShowDetail != null)
                TextButton.icon(
                  onPressed: () => widget.onShowDetail!(widget.child),
                  icon: const Icon(Icons.info_outline, size: 14),
                  label: Text(l10n.trashDetailLabel),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 6),
                    minimumSize: Size.zero,
                  ),
                ),
              if (isCollection && grandchildren.isNotEmpty) ...[
                const SizedBox(width: 4),
                TextButton.icon(
                  onPressed: () => setState(() => _expanded = !_expanded),
                  icon: Icon(
                    _expanded ? Icons.expand_less : Icons.expand_more,
                    size: 14,
                  ),
                  label: Text(l10n.trashShowItems),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 6),
                    minimumSize: Size.zero,
                  ),
                ),
              ],
            ],
          ),
        ),
        if (isCollection && _expanded && grandchildren.isNotEmpty)
          _GrandchildrenPanel(
            children: grandchildren,
            theme: theme,
            l10n: l10n,
            onShowDetail: widget.onShowDetail,
          ),
      ],
    );
  }
}

/// Grandchildren items panel (items inside a collection).
class _GrandchildrenPanel extends StatelessWidget {
  final List<UnifiedObject> children;
  final ThemeData theme;
  final AppLocalizations l10n;
  final ValueChanged<UnifiedObject>? onShowDetail;

  const _GrandchildrenPanel({
    required this.children,
    required this.theme,
    required this.l10n,
    this.onShowDetail,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(left: 24),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
      ),
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final child in children) ...[
            _GrandchildItemRow(child: child, onShowDetail: onShowDetail),
            if (child != children.last) const SizedBox(height: 2),
          ],
        ],
      ),
    );
  }
}

/// Single grandchild (item) row inside a collection's expanded panel.
class _GrandchildItemRow extends StatelessWidget {
  final UnifiedObject child;
  final ValueChanged<UnifiedObject>? onShowDetail;

  const _GrandchildItemRow({required this.child, this.onShowDetail});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          Icon(
            Icons.circle,
            size: 4,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
          ),
          const SizedBox(width: 8),
          Icon(
            UnifiedObjectService.getIconFromName(child.iconName),
            size: 14,
            color: typeColorForId(child.typeId),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              child.name,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontSize: 12,
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          TextButton.icon(
            onPressed: () => onShowDetail?.call(child),
            icon: const Icon(Icons.info_outline, size: 12),
            label: Text(l10n.trashDetailLabel),
            style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              minimumSize: Size.zero,
            ),
          ),
        ],
      ),
    );
  }
}

class _HistoryButtonWidget extends StatelessWidget {
  final bool narrow;
  final int count;
  final VoidCallback onShowHistory;

  const _HistoryButtonWidget({
    required this.narrow,
    required this.count,
    required this.onShowHistory,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final hasHist = count > 0;
    final iconColor = hasHist
        ? null
        : Theme.of(context)
            .colorScheme
            .onSurfaceVariant
            .withValues(alpha: 0.4);
    final icon =
        Icon(Icons.history, size: narrow ? 18 : 16, color: iconColor);

    final stackIcon = Stack(
      clipBehavior: Clip.none,
      children: [
        icon,
        Positioned(
          right: -6,
          top: -6,
          child: Text(
            '$count',
            style: TextStyle(
              fontSize: 10,
              color: iconColor,
              fontWeight: FontWeight.w500,
              height: 1,
            ),
          ),
        ),
      ],
    );

    if (narrow) {
      return IconButton(
        icon: stackIcon,
        onPressed: hasHist
            ? onShowHistory
            : () => showOverlaySnackBar(
                  context,
                  content: l10n.entryNoHistory,
                  type: SnackBarType.info,
                ),
        padding: const EdgeInsets.all(2),
        constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        tooltip:
            hasHist ? l10n.entryHistoryCount(count) : l10n.entryNoHistory,
      );
    }

    return TextButton.icon(
      onPressed: hasHist
          ? onShowHistory
          : () => showOverlaySnackBar(
                context,
                content: l10n.entryNoHistory,
                type: SnackBarType.info,
              ),
      icon: stackIcon,
      label: Text(l10n.trashHistory),
      style: TextButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        minimumSize: Size.zero,
        foregroundColor: iconColor,
      ),
    );
  }
}

class _ActionButtonWidget extends StatelessWidget {
  final bool narrow;
  final IconData icon;
  final String label;
  final VoidCallback onPressed;
  final Color? color;

  const _ActionButtonWidget({
    required this.narrow,
    required this.icon,
    required this.label,
    required this.onPressed,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    if (narrow) {
      return IconButton(
        icon: Icon(icon, size: 18, color: color),
        onPressed: onPressed,
        padding: const EdgeInsets.all(2),
        constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        tooltip: label,
      );
    }
    return TextButton.icon(
      onPressed: onPressed,
      icon: Icon(icon, size: 16),
      label: Text(label),
      style: TextButton.styleFrom(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        minimumSize: Size.zero,
        foregroundColor: color,
      ),
    );
  }
}
