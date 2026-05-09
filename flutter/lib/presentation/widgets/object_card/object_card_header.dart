import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

class ObjectCardHeader extends StatelessWidget {
  final UnifiedObject object;
  final VoidCallback onChangeIcon;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onAddItem;
  final bool showEditActions;
  final bool showAddButton;
  final bool showEditSection;

  const ObjectCardHeader({
    super.key,
    required this.object,
    required this.onChangeIcon,
    required this.onEdit,
    required this.onDelete,
    required this.onAddItem,
    required this.showEditActions,
    required this.showAddButton,
    this.showEditSection = false,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final icon = UnifiedObjectService.getIconFromName(object.iconName);

    return Row(
      children: [
        InkWell(
          onTap: onChangeIcon,
          borderRadius: BorderRadius.circular(6),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Icon(icon, color: theme.colorScheme.primary, size: 20),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            object.name,
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w600,
            ),
            overflow: TextOverflow.ellipsis,
          ),
        ),
        if (showEditActions && !showEditSection)
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: IconButton(
              icon: const Icon(Icons.edit_outlined, size: 18),
              onPressed: onEdit,
              tooltip: l10n.commonEdit,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            ),
          ),
        if (showAddButton)
          if (showEditSection)
            IconButton(
              icon: const Icon(Icons.edit_note, size: 18),
              onPressed: onEdit,
              tooltip: l10n.pageEditorEditSectionTitle,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            )
          else
            IconButton(
              icon: const Icon(Icons.add, size: 18),
              onPressed: onAddItem,
              tooltip: l10n.objectCardAddItem,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            ),
        if (showEditActions)
          Padding(
            padding: const EdgeInsets.only(left: 8),
            child: IconButton(
              icon: const Icon(Icons.delete_outline, size: 18),
              onPressed: onDelete,
              tooltip: l10n.commonDelete,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              visualDensity: VisualDensity.compact,
            ),
          ),
      ],
    );
  }
}
