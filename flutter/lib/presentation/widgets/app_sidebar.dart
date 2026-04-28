import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_vault_dialog.dart';

// =============================================================================
// AppSidebar — Persistent sidebar for all protected pages
// =============================================================================

class AppSidebar extends ConsumerStatefulWidget {
  const AppSidebar({super.key});

  @override
  ConsumerState<AppSidebar> createState() => _AppSidebarState();
}

class _AppSidebarState extends ConsumerState<AppSidebar> {
  bool _expanded = true;
  double _expandedWidth = 260;
  bool _isAddingPage = false;
  bool _isPickingIcon = false;
  String _newPageIconName = 'article';
  final _addPageController = TextEditingController();
  final Set<String> _expandedPageIds = {};

  static const double _collapsedWidth = 72;
  static const double _minWidth = 180;
  static const double _maxWidth = 400;

  void _toggle() => setState(() => _expanded = !_expanded);

  void _confirmAddPage() {
    final name = _addPageController.text.trim();
    if (name.isEmpty) return;
    ref.read(unifiedObjectProvider.notifier).createObject(
          name: name,
          typeId: 'page',
          iconName: _newPageIconName,
        );
    _addPageController.clear();
    _newPageIconName = 'article';
    setState(() => _isAddingPage = false);
  }

  Future<void> _pickNewPageIcon() async {
    setState(() => _isPickingIcon = true);
    final result = await showModalBottomSheet<String>(
      context: context,
      builder: (ctx) => IconPickerSheet(currentIcon: _newPageIconName),
    );
    if (mounted) {
      setState(() => _isPickingIcon = false);
      if (result != null) {
        _newPageIconName = result;
      }
    }
  }

  Future<void> _changePageIcon(String pageId, String currentIcon) async {
    final result = await showModalBottomSheet<String>(
      context: context,
      builder: (ctx) => IconPickerSheet(currentIcon: currentIcon),
    );
    if (result != null && result != currentIcon) {
      await ref.read(unifiedObjectProvider.notifier).updateObject(
            pageId,
            iconName: result,
          );
    }
  }

  @override
  void dispose() {
    _addPageController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sidebarWidth = _expanded ? _expandedWidth : _collapsedWidth;
    final location = GoRouterState.of(context).matchedLocation;
    final customPages = ref.watch(objectsByTypeProvider('page'));

    return SizedBox(
      width: sidebarWidth,
      child: Stack(
        children: [
          // Sidebar content
          Container(
            width: sidebarWidth,
            clipBehavior: Clip.hardEdge,
            decoration: const BoxDecoration(),
            child: Container(
              color: theme.colorScheme.surfaceContainerLowest,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Header
                  _SidebarHeader(expanded: _expanded, onToggle: _toggle),
                  const Divider(height: 1),

                  // Nav items + custom pages
                  Expanded(
                    child: ListView(
                      padding: EdgeInsets.symmetric(
                        horizontal: _expanded ? 12 : 0,
                        vertical: 8,
                      ),
                      children: [
                        // Home
                        _NavTile(
                          icon: Icons.home_outlined,
                          label: 'Home',
                          expanded: _expanded,
                          selected: location == AppRoutes.home,
                          onTap: () => context.go(AppRoutes.home),
                        ),

                        // Search
                        _NavTile(
                          icon: Icons.search,
                          label: 'Search',
                          expanded: _expanded,
                          selected: location == AppRoutes.search,
                          onTap: () => context.go(AppRoutes.search),
                        ),

                        // Scan (placeholder)
                        _NavTile(
                          icon: Icons.document_scanner_outlined,
                          label: 'Scan',
                          expanded: _expanded,
                          onTap: () {
                            showDialog(
                              context: context,
                              builder: (ctx) => const AlertDialog(
                                title: Text('OCR Scan'),
                                content: Text(
                                  'Document scanning will be available in a future update.',
                                ),
                              ),
                            );
                          },
                        ),

                        const SizedBox(height: 4),
                        const Divider(height: 1),
                        const SizedBox(height: 8),

                        // Default pages
                        _NavTile(
                          icon: Icons.person_outline,
                          label: 'Profile',
                          expanded: _expanded,
                          selected: location == AppRoutes.profile,
                          onTap: () => context.go(AppRoutes.profile),
                        ),
                        _NavTile(
                          icon: Icons.flight_outlined,
                          label: 'Travel',
                          expanded: _expanded,
                          selected: location == AppRoutes.travel,
                          onTap: () => context.go(AppRoutes.travel),
                        ),
                        _NavTile(
                          icon: Icons.account_balance_outlined,
                          label: 'Financial',
                          expanded: _expanded,
                          selected: location == AppRoutes.financial,
                          onTap: () => context.go(AppRoutes.financial),
                        ),
                        _NavTile(
                          icon: Icons.work_outline,
                          label: 'Professional',
                          expanded: _expanded,
                          selected: location == AppRoutes.professional,
                          onTap: () => context.go(AppRoutes.professional),
                        ),
                        const SizedBox(height: 16),
                        if (_expanded) const Divider(height: 1),
                        if (_expanded) const SizedBox(height: 8),

                        // Custom pages label
                        if (_expanded)
                          Padding(
                            padding: const EdgeInsets.only(
                                left: 12, right: 4, bottom: 8),
                            child: Row(
                              children: [
                                Text(
                                  'PAGES',
                                  style: theme.textTheme.labelSmall?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                    fontWeight: FontWeight.w600,
                                    letterSpacing: 0.8,
                                  ),
                                ),
                                const Spacer(),
                                Tooltip(
                                  message: 'Add Page',
                                  child: InkWell(
                                    onTap: () => setState(() => _isAddingPage = true),
                                    borderRadius: BorderRadius.circular(6),
                                    child: Padding(
                                      padding: const EdgeInsets.all(4),
                                      child: Icon(
                                        Icons.add,
                                        size: 16,
                                        color: theme.colorScheme.onSurfaceVariant,
                                      ),
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),

                        // Custom pages tree (root-level only)
                        ...customPages
                            .where((p) => p.parentId == null)
                            .map((page) => _PageTreeTile(
                                  page: page,
                                  expanded: _expanded,
                                  depth: 0,
                                  isSelected: location ==
                                      '${AppRoutes.objects}/${page.id}',
                                  onTap: () => context.go(
                                      '${AppRoutes.objects}/${page.id}'),
                                  onIconTap: () => _changePageIcon(
                                      page.id, page.iconName),
                                  expandedPageIds: _expandedPageIds,
                                  onToggleExpand: (id) {
                                    setState(() {
                                      if (_expandedPageIds.contains(id)) {
                                        _expandedPageIds.remove(id);
                                      } else {
                                        _expandedPageIds.add(id);
                                      }
                                    });
                                  },
                                )),

                        // Root-level drop zone (drag pages here to unparent)
                        if (_expanded)
                          DragTarget<String>(
                            onWillAcceptWithDetails: (_) => true,
                            onAcceptWithDetails: (details) {
                              ref
                                  .read(unifiedObjectProvider.notifier)
                                  .moveObject(details.data, null);
                            },
                            builder: (context, candidateData, rejectedData) {
                              final isHovering = candidateData.isNotEmpty;
                              return AnimatedContainer(
                                duration: const Duration(milliseconds: 150),
                                height: isHovering ? 36 : 4,
                                margin: const EdgeInsets.symmetric(horizontal: 12),
                                decoration: BoxDecoration(
                                  borderRadius: BorderRadius.circular(8),
                                  color: isHovering
                                      ? theme.colorScheme.primary
                                          .withValues(alpha: 0.08)
                                      : null,
                                  border: isHovering
                                      ? Border.all(
                                          color: theme.colorScheme.primary
                                              .withValues(alpha: 0.3),
                                          width: 1.5,
                                        )
                                      : null,
                                ),
                                child: isHovering
                                    ? Center(
                                        child: Text(
                                          'Drop to make root page',
                                          style: theme.textTheme.bodySmall
                                              ?.copyWith(
                                            color: theme.colorScheme.primary,
                                            fontWeight: FontWeight.w500,
                                          ),
                                        ),
                                      )
                                    : null,
                              );
                            },
                          ),

                        // Add page input
                        if (_isAddingPage && _expanded)
                          TapRegion(
                            onTapOutside: (_) {
                              if (_isAddingPage && !_isPickingIcon) {
                                if (_addPageController.text.trim().isNotEmpty) {
                                  _confirmAddPage();
                                } else {
                                  _addPageController.clear();
                                  _newPageIconName = 'article';
                                  setState(() => _isAddingPage = false);
                                }
                              }
                            },
                            child: _AddPageInput(
                              controller: _addPageController,
                              iconName: _newPageIconName,
                              onIconTap: _pickNewPageIcon,
                              onConfirm: _confirmAddPage,
                            ),
                          ),


                      ],
                    ),
                  ),

                  // Bottom actions: Lock + Trash + Settings
                  Padding(
                    padding: EdgeInsets.symmetric(
                      horizontal: _expanded ? 12 : 0,
                      vertical: 8,
                    ),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        _NavTile(
                          icon: Icons.lock_outline,
                          label: 'Lock Vault',
                          expanded: _expanded,
                          onTap: () async {
                            final confirmed = await showLockVaultDialog(context);
                            if (confirmed == true && context.mounted) {
                              ref.read(authNotifierProvider.notifier).lockVault();
                            }
                          },
                        ),
                        _NavTile(
                          icon: Icons.delete_outline,
                          label: 'Trash',
                          expanded: _expanded,
                          selected: location == AppRoutes.trash,
                          onTap: () => context.go(AppRoutes.trash),
                        ),
                        _NavTile(
                          icon: Icons.settings_outlined,
                          label: 'Settings',
                          expanded: _expanded,
                          selected: location == AppRoutes.settings ||
                              location == AppRoutes.securitySettings ||
                              location == AppRoutes.sensitivitySettings ||
                              location == AppRoutes.operationLog,
                          onTap: () => context.go(AppRoutes.settings),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),

          // Resize handle
          Positioned(
            right: 0,
            top: 0,
            bottom: 0,
            child: MouseRegion(
              cursor: SystemMouseCursors.resizeLeftRight,
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onHorizontalDragUpdate: (details) {
                  setState(() {
                    _expandedWidth = (_expandedWidth + details.delta.dx)
                        .clamp(_minWidth, _maxWidth);
                    _expanded = true;
                  });
                },
                child: Container(
                  width: 8,
                  color: Colors.transparent,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// =============================================================================
// Sidebar Header
// =============================================================================

class _SidebarHeader extends StatelessWidget {
  final bool expanded;
  final VoidCallback onToggle;

  const _SidebarHeader({required this.expanded, required this.onToggle});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SizedBox(
      height: 64,
      child: expanded
          ? LayoutBuilder(
              builder: (context, constraints) {
                final showText = constraints.maxWidth >= 140;
                return Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          color: AppTheme.primaryColor.withValues(alpha: 0.15),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: const Icon(
                          Icons.auto_awesome,
                          color: AppTheme.primaryColor,
                          size: 20,
                        ),
                      ),
                      if (showText) ...[
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'SoloSoul',
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w700,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                      if (!showText) const Spacer(),
                      IconButton(
                        icon: const Icon(Icons.chevron_left),
                        onPressed: onToggle,
                        tooltip: 'Collapse',
                      ),
                    ],
                  ),
                );
              },
            )
          : Center(
              child: IconButton(
                icon: const Icon(Icons.auto_awesome),
                onPressed: onToggle,
                tooltip: 'Expand',
              ),
            ),
    );
  }
}

// =============================================================================
// Nav Tile
// =============================================================================

class _NavTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool expanded;
  final bool selected;
  final VoidCallback onTap;
  final VoidCallback? onIconTap;

  const _NavTile({
    required this.icon,
    required this.label,
    required this.expanded,
    this.selected = false,
    required this.onTap,
    // ignore: unused_element_parameter
    this.onIconTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bgColor = selected
        ? theme.colorScheme.primary.withValues(alpha: 0.1)
        : Colors.transparent;
    final fgColor = selected
        ? theme.colorScheme.primary
        : theme.colorScheme.onSurface;

    final tile = Padding(
      padding: EdgeInsets.symmetric(
        horizontal: expanded ? 0 : 8,
        vertical: 2,
      ),
      child: SizedBox(
        width: double.infinity,
        child: Material(
          color: bgColor,
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            onTap: onTap,
            borderRadius: BorderRadius.circular(8),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final showLabel = expanded && constraints.maxWidth >= 50;
                return Container(
                  height: 40,
                  alignment: showLabel ? Alignment.centerLeft : Alignment.center,
                  padding: showLabel
                      ? const EdgeInsets.symmetric(horizontal: 12)
                      : const EdgeInsets.all(0),
                  child: showLabel
                      ? Row(
                          children: [
                            if (onIconTap != null)
                              InkWell(
                                onTap: onIconTap,
                                borderRadius: BorderRadius.circular(6),
                                child: Padding(
                                  padding: const EdgeInsets.all(4),
                                  child: Icon(icon, size: 20, color: fgColor),
                                ),
                              )
                            else
                              Icon(icon, size: 20, color: fgColor),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                label,
                                style: theme.textTheme.bodyMedium?.copyWith(
                                  color: fgColor,
                                  fontWeight: selected ? FontWeight.w600 : null,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          ],
                        )
                      : Center(child: Icon(icon, size: 22, color: fgColor)),
                );
              },
            ),
          ),
        ),
      ),
    );

    if (expanded) return tile;
    return Tooltip(message: label, child: tile);
  }
}

// =============================================================================
// Page Tree Tile — Expandable tree node for custom pages
// =============================================================================

class _PageTreeTile extends ConsumerStatefulWidget {
  final UnifiedObject page;
  final bool expanded;
  final int depth;
  final bool isSelected;
  final VoidCallback onTap;
  final VoidCallback? onIconTap;
  final Set<String> expandedPageIds;
  final ValueChanged<String> onToggleExpand;

  const _PageTreeTile({
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
  ConsumerState<_PageTreeTile> createState() => _PageTreeTileState();
}

class _PageTreeTileState extends ConsumerState<_PageTreeTile> {
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
                        // Icon
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
                        // Expand/collapse chevron (only if has children)
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

    // Drag source + drop target wrapper
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
            return _PageTreeTile(
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

// =============================================================================
// Add Page Input
// =============================================================================

class _AddPageInput extends StatelessWidget {
  final TextEditingController controller;
  final String iconName;
  final VoidCallback onIconTap;
  final VoidCallback onConfirm;

  const _AddPageInput({
    required this.controller,
    required this.iconName,
    required this.onIconTap,
    required this.onConfirm,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 0, vertical: 2),
      child: Container(
        height: 40,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        decoration: BoxDecoration(
          color: theme.colorScheme.primary.withValues(alpha: 0.05),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.primary.withValues(alpha: 0.3),
          ),
        ),
        child: Row(
          children: [
            InkWell(
              onTap: onIconTap,
              borderRadius: BorderRadius.circular(6),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(
                  UnifiedObjectService.getIconFromName(iconName),
                  size: 20,
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: controller,
                autofocus: true,
                decoration: const InputDecoration(
                  border: InputBorder.none,
                  isDense: true,
                  contentPadding: EdgeInsets.zero,
                ),
                style: theme.textTheme.bodyMedium,
                onSubmitted: (_) => onConfirm(),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.check, size: 18),
              onPressed: onConfirm,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
            ),
          ],
        ),
      ),
    );
  }
}

