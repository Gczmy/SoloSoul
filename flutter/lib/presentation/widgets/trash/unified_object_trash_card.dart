import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/widgets/field_history_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/form_field_def.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';

class UnifiedObjectTrashCard extends ConsumerWidget {
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
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
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

    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: Colors.orange.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      UnifiedObjectService.getIconFromName(object.iconName),
                      color: Colors.orange,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          object.name,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          object.typeId ?? 'object',
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
              const SizedBox(height: 12),
              LayoutBuilder(
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
                              ? 'Deleted ${_formatTimeAgo(deletedAt)}'
                              : 'Deleted recently',
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
                        label: 'Details',
                        onPressed: () => _showDetailDialog(context, ref),
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
                      _ActionButtonWidget(
                        narrow: narrow,
                        icon: Icons.restore_from_trash,
                        label: 'Restore',
                        onPressed: onRestore,
                      ),
                      const SizedBox(width: 4),
                      _ActionButtonWidget(
                        narrow: narrow,
                        icon: Icons.delete_forever,
                        label: 'Purge',
                        onPressed: onPurge,
                        color: AppTheme.errorColor,
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }



  void _showDetailDialog(BuildContext context, WidgetRef ref) {
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
                            'Deleted ${_formatTimeAgo(deletedAt)}',
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
                        style:
                            Theme.of(ctx).textTheme.bodySmall?.copyWith(
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
                              formatFieldLabel(e.key),
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
                            Text(
                              text.isEmpty ? '(empty)' : text,
                              style: Theme.of(ctx).textTheme.bodyMedium,
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
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  String _formatFullTimestamp(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  String _formatTimeAgo(DateTime date) {
    final diff = DateTime.now().difference(date);
    if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'Just now';
    }
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
                  content: 'No history available',
                  type: SnackBarType.info,
                ),
        padding: const EdgeInsets.all(2),
        constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        tooltip: hasHist ? 'History ($count)' : 'No history yet',
      );
    }

    return TextButton.icon(
      onPressed: hasHist
          ? onShowHistory
          : () => showOverlaySnackBar(
                context,
                content: 'No history available',
                type: SnackBarType.info,
              ),
      icon: stackIcon,
      label: const Text('History'),
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
