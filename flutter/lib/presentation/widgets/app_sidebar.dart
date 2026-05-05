import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_vault_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/add_page_input.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/nav_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/page_tree_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/sidebar_header.dart';

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
  /// Hysteresis buffer to prevent flickering when dragging near [_minWidth].
  /// Once collapsed, the sidebar will not re-expand until dragged past
  /// [_minWidth] + this value in the same drag gesture.
  static const double _collapseHysteresis = 20;

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

  /// Build the list of sidebar items for ListView.builder.
  List<Widget> _buildSidebarItems(
    BuildContext context,
    String location,
    List<UnifiedObject> customPages,
    ThemeData theme,
  ) {
    final items = <Widget>[
      // Home
      NavTile(
        icon: Icons.home_outlined,
        label: 'Home',
        expanded: _expanded,
        selected: location == AppRoutes.home,
        onTap: () => context.go(AppRoutes.home),
      ),
      // Search
      NavTile(
        icon: Icons.search,
        label: 'Search',
        expanded: _expanded,
        selected: location == AppRoutes.search,
        onTap: () => context.go(AppRoutes.search),
      ),
      // Local Search Import (debug only)
      if (kDebugMode)
        NavTile(
          icon: Icons.document_scanner_outlined,
          label: 'Local Import',
          expanded: _expanded,
          selected: location == AppRoutes.localSearch ||
              location == AppRoutes.localSearchProgress ||
              location == AppRoutes.scanPreview ||
              location == AppRoutes.scanImportResult,
          onTap: () => context.go(AppRoutes.localSearch),
        ),
      // AI Chat
      NavTile(
        icon: Icons.chat_bubble_outline,
        label: 'AI 对话',
        expanded: _expanded,
        selected: location == AppRoutes.llmChat,
        onTap: () => context.go(AppRoutes.llmChat),
      ),
      if (kDebugMode) const SizedBox(height: 4),
      const Divider(height: 1),
      const SizedBox(height: 8),
      // Default pages
      NavTile(
        icon: Icons.person_outline,
        label: 'Profile',
        expanded: _expanded,
        selected: location == AppRoutes.profile,
        onTap: () => context.go(AppRoutes.profile),
      ),
      NavTile(
        icon: Icons.flight_outlined,
        label: 'Travel',
        expanded: _expanded,
        selected: location == AppRoutes.travel,
        onTap: () => context.go(AppRoutes.travel),
      ),
      NavTile(
        icon: Icons.account_balance_outlined,
        label: 'Financial',
        expanded: _expanded,
        selected: location == AppRoutes.financial,
        onTap: () => context.go(AppRoutes.financial),
      ),
      NavTile(
        icon: Icons.work_outline,
        label: 'Professional',
        expanded: _expanded,
        selected: location == AppRoutes.professional,
        onTap: () => context.go(AppRoutes.professional),
      ),
      const SizedBox(height: 16),
    ];

    if (_expanded) {
      items.addAll([
        const Divider(height: 1),
        const SizedBox(height: 8),
        // Custom pages label
        Padding(
          padding: const EdgeInsets.only(left: 12, right: 4, bottom: 8),
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
      ]);

      // Custom pages tree (root-level only)
      for (final page in customPages.where((p) => p.parentId == null)) {
        items.add(
          PageTreeTile(
            key: ValueKey(page.id),
            page: page,
            expanded: _expanded,
            depth: 0,
            isSelected: location == '${AppRoutes.objects}/${page.id}',
            onTap: () => context.go('${AppRoutes.objects}/${page.id}'),
            onIconTap: () => _changePageIcon(page.id, page.iconName),
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
          ),
        );
      }

      // Root-level drop zone
      items.add(
        DragTarget<String>(
          onWillAcceptWithDetails: (_) => true,
          onAcceptWithDetails: (details) {
            ref.read(unifiedObjectProvider.notifier).moveObject(details.data, null);
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
                    ? theme.colorScheme.primary.withValues(alpha: 0.08)
                    : null,
                border: isHovering
                    ? Border.all(
                        color: theme.colorScheme.primary.withValues(alpha: 0.3),
                        width: 1.5,
                      )
                    : null,
              ),
              child: isHovering
                  ? Center(
                      child: Text(
                        'Drop to make root page',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.primary,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    )
                  : null,
            );
          },
        ),
      );

      // Add page input
      if (_isAddingPage) {
        items.add(
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
            child: AddPageInput(
              controller: _addPageController,
              iconName: _newPageIconName,
              onIconTap: _pickNewPageIcon,
              onConfirm: _confirmAddPage,
            ),
          ),
        );
      }
    }

    return items;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sidebarWidth = _expanded ? _expandedWidth : _collapsedWidth;
    final location = GoRouterState.of(context).matchedLocation;
    final allPages = ref.watch(objectsByTypeProvider('page'));
    // Filter out default pages (Profile, Travel, Financial, Professional)
    // so they don't appear in the custom pages section.
    final customPages = allPages.where((p) =>
        p.id != DefaultPageIds.profile &&
        p.id != DefaultPageIds.travel &&
        p.id != DefaultPageIds.financial &&
        p.id != DefaultPageIds.professional).toList();

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
                  SidebarHeader(expanded: _expanded, onToggle: _toggle),
                  const Divider(height: 1),

                  // Nav items + custom pages
                  Expanded(
                    child: Builder(
                      builder: (context) {
                        final sidebarItems = _buildSidebarItems(
                          context, location, customPages, theme,
                        );
                        return ListView.builder(
                          padding: EdgeInsets.symmetric(
                            horizontal: _expanded ? 12 : 0,
                            vertical: 8,
                          ),
                          itemCount: sidebarItems.length,
                          itemBuilder: (context, index) => sidebarItems[index],
                        );
                      },
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
                        NavTile(
                          icon: Icons.lock_outline,
                          label: 'Lock Vault',
                          expanded: _expanded,
                          onTap: () async {
                            final confirmed = await showLockVaultDialog(context);
                            if (confirmed == true && context.mounted) {
                              await ref.read(authNotifierProvider.notifier).lockVault();
                            }
                          },
                        ),
                        NavTile(
                          icon: Icons.delete_outline,
                          label: 'Trash',
                          expanded: _expanded,
                          selected: location == AppRoutes.trash,
                          onTap: () => context.go(AppRoutes.trash),
                        ),
                        NavTile(
                          icon: Icons.sync,
                          label: 'Sync',
                          expanded: _expanded,
                          selected: location == AppRoutes.sync,
                          onTap: () => context.go(AppRoutes.sync),
                        ),
                        NavTile(
                          icon: Icons.settings_outlined,
                          label: 'Settings',
                          expanded: _expanded,
                          selected: location == AppRoutes.settings ||
                              location == AppRoutes.securitySettings ||
                              location == AppRoutes.sensitivitySettings ||
                              location == AppRoutes.operationLog ||
                              location == AppRoutes.llmConfig,
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
                    final newWidth = _expandedWidth + details.delta.dx;
                    if (_expanded) {
                      // Collapse when dragged below minimum width
                      if (newWidth < _minWidth) {
                        _expanded = false;
                      } else {
                        _expandedWidth = newWidth.clamp(_minWidth, _maxWidth);
                      }
                    } else {
                      // Re-expand only when dragged past minWidth + hysteresis
                      // to prevent flickering from hand jitter near the threshold.
                      if (newWidth > _minWidth + _collapseHysteresis) {
                        _expanded = true;
                        _expandedWidth = newWidth.clamp(_minWidth, _maxWidth);
                      }
                    }
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
