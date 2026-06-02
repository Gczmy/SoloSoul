import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/utils/icon_resolver.dart';

/// Generic tile for displaying any UnifiedObject.
/// Used in reorderable lists and tree views.
class ObjectTile extends StatelessWidget {
  final UnifiedObject object;
  final VoidCallback? onTap;
  final VoidCallback? onEdit;
  final VoidCallback? onDelete;
  final bool showDragHandle;
  final int dragIndex;

  const ObjectTile({
    super.key,
    required this.object,
    this.onTap,
    this.onEdit,
    this.onDelete,
    this.showDragHandle = true,
    this.dragIndex = 0,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final icon = IconResolver.resolve(object.iconName);
    final displayName = getLocalizedObjectName(l10n, object);
    final type = ObjectTypeRegistry.getType(object.typeId ?? '');
    final typeLabel = type?.name ?? object.typeId ?? l10n.commonObject;

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              if (showDragHandle) ...[
                ReorderableDragStartListener(
                  index: dragIndex,
                  child: Icon(
                    Icons.drag_handle,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 12),
              ],
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  icon,
                  color: theme.colorScheme.primary,
                  size: 20,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      displayName,
                      style: theme.textTheme.titleMedium,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      typeLabel,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              if (object.childrenIds.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: Text(
                    '${object.childrenIds.length}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              if (onEdit != null)
                IconButton(
                  icon: const Icon(Icons.edit_outlined, size: 20),
                  onPressed: onEdit,
                  tooltip: l10n.commonEdit,
                ),
              if (onDelete != null)
                IconButton(
                  icon: Icon(
                    Icons.delete_outline,
                    size: 20,
                    color: theme.colorScheme.error,
                  ),
                  onPressed: onDelete,
                  tooltip: l10n.commonDelete,
                ),
            ],
          ),
        ),
      ),
    );
  }

}
