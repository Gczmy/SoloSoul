import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_vault_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/add_page_input.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/nav_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/page_tree_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/sidebar/sidebar_header.dart';

// =============================================================================
// AppSidebar — Persistent sidebar for all protected pages (Liquid Glass)
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

  /// Map default page IDs to their fixed routes.
  /// Custom pages use the generic `/objects/:id` route.
  static String? _routeForPageId(String pageId) {
    return switch (pageId) {
      DefaultPageIds.profile => AppRoutes.profile,
      DefaultPageIds.travel => AppRoutes.travel,
      DefaultPageIds.financial => AppRoutes.financial,
      DefaultPageIds.professional => AppRoutes.professional,
      _ => null,
    };
  }

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
    List<UnifiedObject> allPages,
    ThemeData theme,
  ) {
    // Separate default pages (with fixed routes) from custom pages
    final defaultPageOrder = {
      DefaultPageIds.profile: 0,
      DefaultPageIds.travel: 1,
      DefaultPageIds.financial: 2,
      DefaultPageIds.professional: 3,
    };
    final defaultPages = allPages
        .where((p) => defaultPageOrder.containsKey(p.id))
        .toList();
    defaultPages.sort((a, b) {
      final orderA = defaultPageOrder[a.id] ?? 0;
      final orderB = defaultPageOrder[b.id] ?? 0;
      return orderA.compareTo(orderB);
    });

    final customPages = allPages
        .where((p) => !defaultPageOrder.containsKey(p.id))
        .toList();

    final items = <Widget>[
      // Home
      NavTile(
        icon: Icons.home_outlined,
        label: AppLocalizations.of(context).sidebarHome,
        expanded: _expanded,
        selected: location == AppRoutes.home,
        onTap: () => context.go(AppRoutes.home),
      ),
      // Search
      NavTile(
        icon: Icons.search,
        label: AppLocalizations.of(context).sidebarSearch,
        expanded: _expanded,
        selected: location == AppRoutes.search,
        onTap: () => context.go(AppRoutes.search),
      ),
      // Local Search Import
      NavTile(
        icon: Icons.document_scanner_outlined,
        label: AppLocalizations.of(context).sidebarLocalImport,
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
        label: AppLocalizations.of(context).sidebarAiChat,
        expanded: _expanded,
        selected: location == AppRoutes.llmChat,
        onTap: () => context.go(AppRoutes.llmChat),
      ),
      const SizedBox(height: 4),
      const Divider(height: 1),
      const SizedBox(height: 8),
      // All pages (default + custom), default pages first
      for (final page in defaultPages)
        NavTile(
          icon: UnifiedObjectService.getIconFromName(page.iconName),
          label: getLocalizedObjectName(AppLocalizations.of(context), page),
          expanded: _expanded,
          selected: location == _routeForPageId(page.id),
          onTap: () {
            final route = _routeForPageId(page.id);
            if (route != null) context.go(route);
          },
        ),
      for (final page in customPages.where((p) => p.parentId == null))
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
      const SizedBox(height: 16),
    ];

    return items;
  }

  LiquidGlassSettings _glassSettings(bool isDark) => isDark
      ? const LiquidGlassSettings(
          thickness: 30,
          blur: 20,
          glassColor: Color(0x1AFFFFFF),
          refractiveIndex: 1.3,
          lightIntensity: 0.8,
          ambientStrength: 0.15,
        )
      : const LiquidGlassSettings(
          thickness: 20,
          blur: 15,
          glassColor: Color(0x15D2DCF0),
          refractiveIndex: 1.15,
          lightIntensity: 0.9,
          ambientStrength: 0.1,
        );

  Color _glassBackground(bool isDark) =>
      isDark ? const Color(0x0DFFFFFF) : const Color(0x08D2DCF0);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sidebarWidth = _expanded ? _expandedWidth : _collapsedWidth;
    final location = GoRouterState.of(context).matchedLocation;
    final allPages = ref.watch(objectsByTypeProvider('page'))
        .where((p) => !p.isDeleted)
        .toList();

    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return SizedBox(
      width: sidebarWidth,
      child: Stack(
        children: [
          // Sidebar content with liquid glass background
          SizedBox(
            width: sidebarWidth,
            child: AdaptiveGlass(
              shape: const LiquidRoundedRectangle(borderRadius: 0),
              settings: _glassSettings(isDark),
              quality: GlassQuality.standard,
              child: Container(
                color: _glassBackground(isDark),
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
                            context,
                            location,
                            allPages,
                            theme,
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

                    // Fixed "Pages +" section above bottom actions
                    if (_expanded)
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            const Divider(height: 1),
                            const SizedBox(height: 8),
                            Row(
                              children: [
                                Text(
                                  AppLocalizations.of(context).sidebarPages,
                                  style: theme.textTheme.labelSmall?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                    fontWeight: FontWeight.w600,
                                    letterSpacing: 0.8,
                                  ),
                                ),
                                const Spacer(),
                                Tooltip(
                                  message: AppLocalizations.of(context).sidebarAddPage,
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
                            // Root-level drop zone
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
                                  margin: const EdgeInsets.symmetric(vertical: 4),
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
                                            AppLocalizations.of(context).sidebarDropToMakeRootPage,
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
                            // Add page input
                            if (_isAddingPage)
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
                          NavTile(
                            icon: Icons.extension_outlined,
                            label: AppLocalizations.of(context).sidebarPlugin,
                            expanded: _expanded,
                            selected: location == AppRoutes.pluginDashboard,
                            onTap: () => context.go(AppRoutes.pluginDashboard),
                          ),
                          NavTile(
                            icon: Icons.lock_outline,
                            label: AppLocalizations.of(context).sidebarLockVault,
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
                            label: AppLocalizations.of(context).sidebarTrash,
                            expanded: _expanded,
                            selected: location == AppRoutes.trash,
                            onTap: () => context.go(AppRoutes.trash),
                          ),
                          NavTile(
                            icon: Icons.sync,
                            label: AppLocalizations.of(context).sidebarSync,
                            expanded: _expanded,
                            selected: location == AppRoutes.sync,
                            onTap: () => context.go(AppRoutes.sync),
                          ),
                          NavTile(
                            icon: Icons.settings_outlined,
                            label: AppLocalizations.of(context).sidebarSettings,
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
