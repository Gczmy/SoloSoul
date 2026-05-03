import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

class PageTreeTile extends ConsumerStatefulWidget {
  final UnifiedObject page;
  final bool expanded;
  final int depth;
  final bool isSelected;
  final VoidCallback onTap;
  final VoidCallback? onIconTap;
  final Set<String> expandedPageIds;
  final ValueChanged<String> onToggleExpand;

  const PageTreeTile({
    super.key,
    required this.page,
    required this.expanded,
    this.depth = 0,
    required this.isSelected,
    required this.onTap,
    this.onIconTap,
    required this.expandedPageIds,
    required this.onToggleExpand,
  });

  @override
  ConsumerState<PageTreeTile> createState() => _PageTreeTileState();
}

class _PageTreeTileState extends ConsumerState<PageTreeTile> {
  bool _isEditing = false;
  late final TextEditingController _editController;

  @override
  void initState() {
    super.initState();
    _editController = TextEditingController(text: widget.page.name);
  }

  @override
  void dispose() {
    _editController.dispose();
    super.dispose();
  }

  void _confirmRename() {
    final name = _editController.text.trim();
    if (name.isNotEmpty && name != widget.page.name) {
      ref.read(unifiedObjectProvider.notifier).updateObject(
            widget.page.id,
            name: name,
          );
    }
    setState(() => _isEditing = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bgColor = widget.isSelected
        ? theme.colorScheme.primary.withValues(alpha: 0.1)
        : Colors.transparent;
    final fgColor = widget.isSelected
        ? theme.colorScheme.primary
        : theme.colorScheme.onSurface;
    final isExpanded = widget.expandedPageIds.contains(widget.page.id);

    final childPages = widget.expanded
        ? ref.watch(childrenProvider(widget.page.id))
            .where((c) => c.typeId == 'page')
            .toList()
        : <UnifiedObject>[];
    final hasChildren = childPages.isNotEmpty;

    final nameWidget = _isEditing && widget.expanded
        ? TextField(
            controller: _editController,
            autofocus: true,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: fgColor,
              fontWeight: widget.isSelected ? FontWeight.w600 : null,
            ),
            decoration: const InputDecoration(
              isDense: true,
              contentPadding: EdgeInsets.zero,
              border: InputBorder.none,
            ),
            onSubmitted: (_) => _confirmRename(),
            onTapOutside: (_) => _confirmRename(),
          )
        : Text(
            widget.page.name,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: fgColor,
              fontWeight: widget.isSelected ? FontWeight.w600 : null,
            ),
            overflow: TextOverflow.ellipsis,
          );

    final tile = Padding(
      padding: EdgeInsets.only(
        left: widget.expanded ? (widget.depth * 16.0) : 8,
        right: widget.expanded ? 0 : 8,
        top: 2,
        bottom: 2,
      ),
      child: SizedBox(
        width: double.infinity,
        child: Material(
          color: bgColor,
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            onTap: widget.onTap,
            onDoubleTap: widget.expanded
                ? () {
                    _editController.text = widget.page.name;
                    setState(() => _isEditing = true);
                  }
                : null,
            borderRadius: BorderRadius.circular(8),
            child: Container(
              height: 40,
              alignment: widget.expanded ? Alignment.centerLeft : Alignment.center,
              padding: widget.expanded
                  ? const EdgeInsets.symmetric(horizontal: 12)
                  : const EdgeInsets.all(0),
              child: widget.expanded
                  ? Row(
                      children: [
                        if (widget.onIconTap != null)
                          InkWell(
                            onTap: widget.onIconTap,
                            borderRadius: BorderRadius.circular(6),
                            child: Padding(
                              padding: const EdgeInsets.all(4),
                              child: Icon(
                                UnifiedObjectService.getIconFromName(
                                    widget.page.iconName),
                                size: 20,
                                color: fgColor,
                              ),
                            ),
                          )
                        else
                          Icon(
                            UnifiedObjectService.getIconFromName(
                                widget.page.iconName),
                            size: 20,
                            color: fgColor,
                          ),
                        const SizedBox(width: 8),
                        Expanded(child: nameWidget),
                        if (hasChildren && !_isEditing)
                          InkWell(
                            onTap: () => widget.onToggleExpand(widget.page.id),
                            borderRadius: BorderRadius.circular(6),
                            child: SizedBox(
                              width: 24,
                              height: 24,
                              child: Icon(
                                isExpanded
                                    ? Icons.expand_more
                                    : Icons.chevron_right,
                                size: 18,
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ),
                      ],
                    )
                  : Center(
                      child: Icon(
                        UnifiedObjectService.getIconFromName(
                            widget.page.iconName),
                        size: 22,
                        color: fgColor,
                      ),
                    ),
            ),
          ),
        ),
      ),
    );

    final draggableTile = Draggable<String>(
      data: widget.page.id,
      feedback: Material(
        elevation: 4,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          width: 220,
          height: 40,
          padding: const EdgeInsets.symmetric(horizontal: 12),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            children: [
              Icon(
                UnifiedObjectService.getIconFromName(widget.page.iconName),
                size: 20,
                color: theme.colorScheme.onSurface,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  widget.page.name,
                  style: theme.textTheme.bodyMedium,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ),
      childWhenDragging: Opacity(
        opacity: 0.35,
        child: tile,
      ),
      child: DragTarget<String>(
        onWillAcceptWithDetails: (details) {
          final draggedId = details.data;
          if (draggedId == widget.page.id) return false;
          final allObjects = ref.read(unifiedObjectProvider).objects;
          final descendants =
              UnifiedObjectService.instance.getDescendantIds(allObjects, draggedId);
          return !descendants.contains(widget.page.id);
        },
        onAcceptWithDetails: (details) {
          ref.read(unifiedObjectProvider.notifier).moveObject(
            details.data,
            widget.page.id,
          );
        },
        builder: (context, candidateData, rejectedData) {
          final isHovering = candidateData.isNotEmpty;
          return AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(8),
              color: isHovering
                  ? theme.colorScheme.primary.withValues(alpha: 0.12)
                  : null,
            ),
            child: tile,
          );
        },
      ),
    );

    final children = (widget.expanded && isExpanded && hasChildren)
        ? childPages.map((child) {
            final childLocation =
                '${AppRoutes.objects}/${child.id}';
            return PageTreeTile(
              key: ValueKey(child.id),
              page: child,
              expanded: widget.expanded,
              depth: widget.depth + 1,
              isSelected: GoRouterState.of(context).matchedLocation ==
                  childLocation,
              onTap: () => context.go(childLocation),
              expandedPageIds: widget.expandedPageIds,
              onToggleExpand: widget.onToggleExpand,
            );
          }).toList()
        : <Widget>[];

    if (!widget.expanded) {
      return Tooltip(message: widget.page.name, child: draggableTile);
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        draggableTile,
        ...children,
      ],
    );
  }
}
