import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/utils/icon_resolver.dart';

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
  void didUpdateWidget(PageTreeTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Cancel editing and revert name when the page is no longer selected
    // or when editing an unselected page and parent rebuilds (e.g. navigation).
    if (_isEditing && !widget.isSelected) {
      setState(() {
        _isEditing = false;
        _editController.text = widget.page.name;
      });
    }
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

    final tile = _TreeTile(
      expanded: widget.expanded,
      depth: widget.depth,
      bgColor: bgColor,
      onTap: widget.onTap,
      onLongPress: widget.expanded
          ? () {
              _editController.text = widget.page.name;
              setState(() => _isEditing = true);
            }
          : null,
      iconName: widget.page.iconName,
      fgColor: fgColor,
      onIconTap: widget.onIconTap,
      isEditing: _isEditing,
      pageName: widget.page.name,
      isSelected: widget.isSelected,
      editController: _editController,
      onConfirmRename: _confirmRename,
      hasChildren: hasChildren,
      isExpanded: isExpanded,
      onToggleExpand: () => widget.onToggleExpand(widget.page.id),
    );

    final draggableTile = _TreeTileDraggable(
      pageId: widget.page.id,
      pageName: widget.page.name,
      iconName: widget.page.iconName,
      tile: tile,
    );

    if (!widget.expanded) {
      return Tooltip(message: widget.page.name, child: draggableTile);
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        draggableTile,
        _TreeTileChildren(
          expanded: widget.expanded,
          isExpanded: isExpanded,
          hasChildren: hasChildren,
          childPages: childPages,
          depth: widget.depth,
          expandedPageIds: widget.expandedPageIds,
          onToggleExpand: widget.onToggleExpand,
        ),
      ],
    );
  }
}

class _TreeTile extends StatelessWidget {
  final bool expanded;
  final int depth;
  final Color bgColor;
  final VoidCallback onTap;
  final VoidCallback? onLongPress;
  final String iconName;
  final Color fgColor;
  final VoidCallback? onIconTap;
  final bool isEditing;
  final String pageName;
  final bool isSelected;
  final TextEditingController editController;
  final VoidCallback onConfirmRename;
  final bool hasChildren;
  final bool isExpanded;
  final VoidCallback onToggleExpand;

  const _TreeTile({
    required this.expanded,
    required this.depth,
    required this.bgColor,
    required this.onTap,
    this.onLongPress,
    required this.iconName,
    required this.fgColor,
    this.onIconTap,
    required this.isEditing,
    required this.pageName,
    required this.isSelected,
    required this.editController,
    required this.onConfirmRename,
    required this.hasChildren,
    required this.isExpanded,
    required this.onToggleExpand,
  });

  @override
  Widget build(BuildContext context) {
    final child = expanded
        ? Row(
            children: [
              _TreeTileLeading(
                iconName: iconName,
                fgColor: fgColor,
                onIconTap: onIconTap,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _TreeTileTitle(
                  isEditing: isEditing,
                  pageName: pageName,
                  isSelected: isSelected,
                  fgColor: fgColor,
                  controller: editController,
                  onConfirmRename: onConfirmRename,
                ),
              ),
              _TreeTileTrailing(
                hasChildren: hasChildren,
                isEditing: isEditing,
                isExpanded: isExpanded,
                onToggleExpand: onToggleExpand,
              ),
            ],
          )
        : Center(
            child: _TreeTileLeading(
              iconName: iconName,
              fgColor: fgColor,
              onIconTap: onIconTap,
              iconSize: 22,
            ),
          );

    return Padding(
      padding: EdgeInsets.only(
        left: expanded ? (depth * 16.0) : 8,
        right: expanded ? 0 : 8,
        top: 2,
        bottom: 2,
      ),
      child: SizedBox(
        width: double.infinity,
        child: Material(
          color: bgColor,
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            onTap: onTap,
            onLongPress: onLongPress,
            borderRadius: BorderRadius.circular(8),
            child: Container(
              height: 40,
              alignment: expanded ? Alignment.centerLeft : Alignment.center,
              padding: expanded
                  ? const EdgeInsets.symmetric(horizontal: 12)
                  : const EdgeInsets.all(0),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

class _TreeTileDraggable extends ConsumerWidget {
  final String pageId;
  final String pageName;
  final String iconName;
  final Widget tile;

  const _TreeTileDraggable({
    required this.pageId,
    required this.pageName,
    required this.iconName,
    required this.tile,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Draggable<String>(
      data: pageId,
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
                IconResolver.resolve(iconName),
                size: 20,
                color: theme.colorScheme.onSurface,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  pageName,
                  style: theme.textTheme.bodyMedium,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ),
      // Lightweight placeholder instead of the full complex tile
      childWhenDragging: Opacity(
        opacity: 0.35,
        child: Container(
          height: 40,
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
        ),
      ),
      child: _PageTreeDragTarget(
        pageId: pageId,
        tile: tile,
      ),
    );
  }
}

/// Separated DragTarget to avoid rebuilding the entire Draggable on every hover frame.
class _PageTreeDragTarget extends ConsumerWidget {
  final String pageId;
  final Widget tile;

  const _PageTreeDragTarget({
    required this.pageId,
    required this.tile,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Cache descendant lookups within a single drag session
    final descendantCache = <String, Set<String>>{};

    return DragTarget<String>(
      onWillAcceptWithDetails: (details) {
        final draggedId = details.data;
        if (draggedId == pageId) return false;
        // Reuse cached result if the same draggedId was checked before
        var descendants = descendantCache[draggedId];
        if (descendants == null) {
          final allObjects = ref.read(unifiedObjectProvider).objects;
          descendants = UnifiedObjectService.instance.getDescendantIds(
            allObjects,
            draggedId,
          );
          descendantCache[draggedId] = descendants;
        }
        return !descendants.contains(pageId);
      },
      onAcceptWithDetails: (details) {
        ref.read(unifiedObjectProvider.notifier).moveObject(
          details.data,
          pageId,
        );
      },
      builder: (context, candidateData, rejectedData) {
        final isHovering = candidateData.isNotEmpty;
        if (!isHovering) return tile;
        return Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.12),
          ),
          child: tile,
        );
      },
    );
  }
}

class _TreeTileLeading extends StatelessWidget {
  final String iconName;
  final Color fgColor;
  final VoidCallback? onIconTap;
  final double iconSize;

  const _TreeTileLeading({
    required this.iconName,
    required this.fgColor,
    this.onIconTap,
    this.iconSize = 20,
  });

  @override
  Widget build(BuildContext context) {
    final icon = Icon(
      IconResolver.resolve(iconName),
      size: iconSize,
      color: fgColor,
    );

    if (onIconTap != null) {
      return InkWell(
        onTap: onIconTap,
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: icon,
        ),
      );
    }

    // Consistent padding so alignment matches whether or not onIconTap is present.
    return Padding(
      padding: const EdgeInsets.all(4),
      child: icon,
    );
  }
}

class _TreeTileTitle extends StatelessWidget {
  final bool isEditing;
  final String pageName;
  final bool isSelected;
  final Color fgColor;
  final TextEditingController controller;
  final VoidCallback onConfirmRename;

  const _TreeTileTitle({
    required this.isEditing,
    required this.pageName,
    required this.isSelected,
    required this.fgColor,
    required this.controller,
    required this.onConfirmRename,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (isEditing) {
      return TextField(
        controller: controller,
        autofocus: true,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: fgColor,
          fontWeight: isSelected ? FontWeight.w600 : null,
        ),
        decoration: const InputDecoration(
          isDense: true,
          contentPadding: EdgeInsets.zero,
          border: InputBorder.none,
        ),
        onSubmitted: (_) => onConfirmRename(),
        onTapOutside: (_) => onConfirmRename(),
      );
    }

    return Text(
      pageName,
      style: theme.textTheme.bodyMedium?.copyWith(
        color: fgColor,
        fontWeight: isSelected ? FontWeight.w600 : null,
      ),
      overflow: TextOverflow.ellipsis,
    );
  }
}

class _TreeTileTrailing extends StatelessWidget {
  final bool hasChildren;
  final bool isEditing;
  final bool isExpanded;
  final VoidCallback onToggleExpand;

  const _TreeTileTrailing({
    required this.hasChildren,
    required this.isEditing,
    required this.isExpanded,
    required this.onToggleExpand,
  });

  @override
  Widget build(BuildContext context) {
    if (!hasChildren || isEditing) {
      return const SizedBox.shrink();
    }

    return InkWell(
      onTap: onToggleExpand,
      borderRadius: BorderRadius.circular(6),
      child: SizedBox(
        width: 24,
        height: 24,
        child: Icon(
          isExpanded ? Icons.expand_more : Icons.chevron_right,
          size: 18,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _TreeTileChildren extends StatelessWidget {
  final bool expanded;
  final bool isExpanded;
  final bool hasChildren;
  final List<UnifiedObject> childPages;
  final int depth;
  final Set<String> expandedPageIds;
  final ValueChanged<String> onToggleExpand;

  const _TreeTileChildren({
    required this.expanded,
    required this.isExpanded,
    required this.hasChildren,
    required this.childPages,
    required this.depth,
    required this.expandedPageIds,
    required this.onToggleExpand,
  });

  @override
  Widget build(BuildContext context) {
    if (!expanded || !isExpanded || !hasChildren) {
      return const SizedBox.shrink();
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: childPages.map((child) {
        final childLocation = '${AppRoutes.objects}/${child.id}';
        return PageTreeTile(
          key: ValueKey(child.id),
          page: child,
          expanded: expanded,
          depth: depth + 1,
          isSelected: GoRouterState.of(context).matchedLocation == childLocation,
          onTap: () => context.go(childLocation),
          expandedPageIds: expandedPageIds,
          onToggleExpand: onToggleExpand,
        );
      }).toList(),
    );
  }
}
