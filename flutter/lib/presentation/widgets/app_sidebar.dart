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
                        if (_expanded && customPages.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(left: 12, bottom: 8),
                            child: Text(
                              'PAGES',
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                                fontWeight: FontWeight.w600,
                                letterSpacing: 0.8,
                              ),
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

                        // Add page input
                        if (_isAddingPage && _expanded)
                          TapRegion(
                            onTapOutside: (_) {
                              if (_isAddingPage && !_isPickingIcon) {
                                _addPageController.clear();
                                _newPageIconName = 'article';
                                setState(() => _isAddingPage = false);
                              }
                            },
                            child: _AddPageInput(
                              controller: _addPageController,
                              iconName: _newPageIconName,
                              onIconTap: _pickNewPageIcon,
                              onConfirm: _confirmAddPage,
                            ),
                          ),

                        const SizedBox(height: 8),

                        // Add page button
                        _AddPageButton(
                          expanded: _expanded,
                          onTap: () => setState(() => _isAddingPage = true),
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

class _PageTreeTile extends ConsumerWidget {
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
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final bgColor = isSelected
        ? theme.colorScheme.primary.withValues(alpha: 0.1)
        : Colors.transparent;
    final fgColor = isSelected
        ? theme.colorScheme.primary
        : theme.colorScheme.onSurface;
    final isExpanded = expandedPageIds.contains(page.id);

    final childPages = expanded
        ? ref.watch(childrenProvider(page.id))
            .where((c) => c.typeId == 'page')
            .toList()
        : <UnifiedObject>[];
    final hasChildren = childPages.isNotEmpty;

    final tile = Padding(
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
            borderRadius: BorderRadius.circular(8),
            child: Container(
              height: 40,
              alignment: expanded ? Alignment.centerLeft : Alignment.center,
              padding: expanded
                  ? const EdgeInsets.symmetric(horizontal: 12)
                  : const EdgeInsets.all(0),
              child: expanded
                  ? Row(
                      children: [
                        // Icon
                        if (onIconTap != null)
                          InkWell(
                            onTap: onIconTap,
                            borderRadius: BorderRadius.circular(6),
                            child: Padding(
                              padding: const EdgeInsets.all(4),
                              child: Icon(
                                UnifiedObjectService.getIconFromName(
                                    page.iconName),
                                size: 20,
                                color: fgColor,
                              ),
                            ),
                          )
                        else
                          Icon(
                            UnifiedObjectService.getIconFromName(page.iconName),
                            size: 20,
                            color: fgColor,
                          ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            page.name,
                            style: theme.textTheme.bodyMedium?.copyWith(
                              color: fgColor,
                              fontWeight:
                                  isSelected ? FontWeight.w600 : null,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        // Expand/collapse chevron (only if has children)
                        if (hasChildren)
                          InkWell(
                            onTap: () => onToggleExpand(page.id),
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
                        UnifiedObjectService.getIconFromName(page.iconName),
                        size: 22,
                        color: fgColor,
                      ),
                    ),
            ),
          ),
        ),
      ),
    );

    final children = (expanded && isExpanded && hasChildren)
        ? childPages.map((child) {
            final childLocation =
                '${AppRoutes.objects}/${child.id}';
            return _PageTreeTile(
              page: child,
              expanded: expanded,
              depth: depth + 1,
              isSelected: GoRouterState.of(context).matchedLocation ==
                  childLocation,
              onTap: () => context.go(childLocation),
              expandedPageIds: expandedPageIds,
              onToggleExpand: onToggleExpand,
            );
          }).toList()
        : <Widget>[];

    if (!expanded) {
      return Tooltip(message: page.name, child: tile);
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        tile,
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

// =============================================================================
// Add Page Button
// =============================================================================

class _AddPageButton extends StatelessWidget {
  final bool expanded;
  final VoidCallback onTap;

  const _AddPageButton({required this.expanded, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final button = Padding(
      padding: EdgeInsets.symmetric(
        horizontal: expanded ? 0 : 8,
        vertical: 2,
      ),
      child: Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: SizedBox(
            height: 40,
            child: DashedBorder(
              color: theme.colorScheme.outline.withValues(alpha: 0.4),
              borderRadius: 8,
              child: expanded
                  ? Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const SizedBox(width: 12),
                        Icon(
                          Icons.add,
                          size: 18,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(width: 12),
                        Flexible(
                          child: Text(
                            'Add a page',
                            style: theme.textTheme.bodyMedium?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    )
                  : Center(
                      child: Icon(
                        Icons.add,
                        size: 20,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
            ),
          ),
        ),
      ),
    );

    if (expanded) return button;
    return Tooltip(message: 'Add a page', child: button);
  }
}



// =============================================================================
// Dashed Border Helper
// =============================================================================

class DashedBorder extends StatelessWidget {
  final Widget child;
  final Color color;
  final double borderRadius;
  final double strokeWidth;
  final double dashWidth;
  final double dashGap;

  const DashedBorder({
    super.key,
    required this.child,
    required this.color,
    this.borderRadius = 8,
    this.strokeWidth = 1,
    this.dashWidth = 4,
    this.dashGap = 4,
  });

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _DashedBorderPainter(
        color: color,
        borderRadius: borderRadius,
        strokeWidth: strokeWidth,
        dashWidth: dashWidth,
        dashGap: dashGap,
      ),
      child: child,
    );
  }
}

class _DashedBorderPainter extends CustomPainter {
  final Color color;
  final double borderRadius;
  final double strokeWidth;
  final double dashWidth;
  final double dashGap;

  _DashedBorderPainter({
    required this.color,
    required this.borderRadius,
    required this.strokeWidth,
    required this.dashWidth,
    required this.dashGap,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke;

    final rrect = RRect.fromRectAndRadius(
      Rect.fromLTWH(0, 0, size.width, size.height),
      Radius.circular(borderRadius),
    );

    final path = Path()..addRRect(rrect);
    final dashedPath = _dashPath(path, dashWidth, dashGap);
    canvas.drawPath(dashedPath, paint);
  }

  Path _dashPath(Path source, double dashWidth, double dashGap) {
    final dashed = Path();
    for (final metric in source.computeMetrics()) {
      var distance = 0.0;
      while (distance < metric.length) {
        final length = dashWidth;
        dashed.addPath(
          metric.extractPath(distance, distance + length),
          Offset.zero,
        );
        distance += length + dashGap;
      }
    }
    return dashed;
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
