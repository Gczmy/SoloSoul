import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/user_preferences_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

import 'package:solosoul_flutter/presentation/widgets/home/add_button.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/home/add_quick_action_dialog.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/presentation/widgets/home/delete_badge.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/home/page_editor.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action.dart';
import 'package:solosoul_flutter/presentation/widgets/home/quick_action_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/home/security_item.dart';

// =============================================================================
// HomePage — Dashboard with quick actions + inline page editor (Liquid Glass)
// =============================================================================

String _localizedLabel(String route, AppLocalizations l10n) {
  return switch (route) {
    AppRoutes.profile => l10n.sidebarProfile,
    AppRoutes.travel => l10n.sidebarTravel,
    AppRoutes.financial => l10n.sidebarFinancial,
    AppRoutes.professional => l10n.sidebarProfessional,
    AppRoutes.trash => l10n.sidebarTrash,
    AppRoutes.settings => l10n.sidebarSettings,
    AppRoutes.securitySettings => l10n.sidebarSecurity,
    AppRoutes.operationLog => l10n.sidebarOperationLog,
    AppRoutes.sensitivitySettings => l10n.sidebarSensitivity,
    AppRoutes.search => l10n.sidebarSearch,
    _ => '',
  };
}

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  bool _isEditingPage = false;
  String? _editingPageId;

  void _closeEditor() => setState(() {
        _isEditingPage = false;
        _editingPageId = null;
      });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).mainAppTitle),
        actions: const [HeaderActionButtons()],
      ),
      body: _isEditingPage
          ? PageEditor(
              pageId: _editingPageId,
              onClose: _closeEditor,
            )
          : const _MainDashboard(),
      floatingActionButton: _isEditingPage
          ? null
          : FloatingActionButton.extended(
              onPressed: () => _showOcrScanner(context),
              icon: const Icon(Icons.document_scanner_outlined),
              label: Text(AppLocalizations.of(context).homeScan),
            ),
    );
  }

  Future<void> _showOcrScanner(BuildContext context) async {
    await showModalBottomSheet<SmartOcrResult?>(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => const OcrScannerSheet(),
    );
  }
}

// =============================================================================
// Main Dashboard (Default State)
// =============================================================================

class _MainDashboard extends ConsumerStatefulWidget {
  const _MainDashboard();

  @override
  ConsumerState<_MainDashboard> createState() => _MainDashboardState();
}

class _MainDashboardState extends ConsumerState<_MainDashboard>
    with TickerProviderStateMixin {
  late List<QuickAction> _actions;
  bool _isEditing = false;
  late AnimationController _wobbleController;
  OverlayEntry? _topOverlayEntry;
  Timer? _topOverlayTimer;

  @override
  void initState() {
    super.initState();
    _actions = List.from(const [
      QuickAction(icon: Icons.person_outline, label: 'Profile', route: AppRoutes.profile, color: Colors.blue),
      QuickAction(icon: Icons.flight_outlined, label: 'Travel', route: AppRoutes.travel, color: Colors.teal),
      QuickAction(icon: Icons.account_balance_outlined, label: 'Financial', route: AppRoutes.financial, color: Colors.green),
      QuickAction(icon: Icons.work_outline, label: 'Professional', route: AppRoutes.professional, color: Colors.orange),
      QuickAction(icon: Icons.delete_outline, label: 'Trash', route: AppRoutes.trash, color: Colors.red),
      QuickAction(icon: Icons.settings_outlined, label: 'Settings', route: AppRoutes.settings, color: Colors.grey),
    ]);
    _wobbleController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 400),
    );
    _loadQuickActions();
  }

  Future<void> _loadQuickActions() async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final savedRoutes = await UserPreferencesService.instance.loadQuickActions(accountId);
    if (savedRoutes.isEmpty) return;
    if (!mounted) return;

    final rebuilt = _rebuildActionsFromRoutes(savedRoutes);
    if (rebuilt.isNotEmpty) {
      setState(() => _actions = rebuilt);
    }
  }

  /// Map default page IDs to their fixed routes.
  static String? _routeForPageId(String pageId) {
    return switch (pageId) {
      DefaultPageIds.profile => AppRoutes.profile,
      DefaultPageIds.travel => AppRoutes.travel,
      DefaultPageIds.financial => AppRoutes.financial,
      DefaultPageIds.professional => AppRoutes.professional,
      _ => null,
    };
  }

  /// Build quick actions for all pages (default + custom) from unified objects.
  List<QuickAction> _buildPageActions() {
    final pages = ref.read(unifiedObjectProvider).objects
        .where((o) => o.typeId == 'page' && !o.isDeleted);

    return pages.map((page) {
      final route = _routeForPageId(page.id) ?? '${AppRoutes.objects}/${page.id}';
      final l10n = AppLocalizations.of(context);
      return QuickAction(
        icon: UnifiedObjectService.getIconFromName(page.iconName),
        label: getLocalizedObjectName(l10n, page),
        route: route,
        color: _colorForPage(page.id),
        isCustom: true,
      );
    }).toList();
  }

  List<QuickAction> _rebuildActionsFromRoutes(List<String> routes) {
    final all = [..._allAvailableActions, ..._buildPageActions()];

    final result = <QuickAction>[];
    for (final route in routes) {
      final match = all.where((a) => a.route == route).firstOrNull;
      if (match != null) result.add(match);
    }
    return result;
  }

  Future<void> _persistQuickActions() async {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    if (accountId == null) return;

    final routes = _actions.map((a) => a.route).toList();
    await UserPreferencesService.instance.saveQuickActions(routes, accountId);
  }

  @override
  void dispose() {
    _topOverlayTimer?.cancel();
    _topOverlayEntry?.remove();
    _topOverlayEntry = null;
    _wobbleController.dispose();
    super.dispose();
  }

  void _toggleEditMode() {
    setState(() {
      _isEditing = !_isEditing;
      if (_isEditing) {
        _wobbleController.repeat();
      } else {
        _wobbleController.stop();
      }
    });
  }

  void _deleteAction(int index) {
    setState(() => _actions = List.from(_actions)..removeAt(index));
    _persistQuickActions();
  }

  static const List<QuickAction> _allAvailableActions = [
    QuickAction(icon: Icons.delete_outline, label: 'Trash', route: AppRoutes.trash, color: Colors.red),
    QuickAction(icon: Icons.settings_outlined, label: 'Settings', route: AppRoutes.settings, color: Colors.grey),
    QuickAction(icon: Icons.security_outlined, label: 'Security', route: AppRoutes.securitySettings, color: Colors.indigo),
    QuickAction(icon: Icons.history_outlined, label: 'Operation Log', route: AppRoutes.operationLog, color: Colors.purple),
    QuickAction(icon: Icons.visibility_outlined, label: 'Sensitivity', route: AppRoutes.sensitivitySettings, color: Colors.cyan),
    QuickAction(icon: Icons.search_outlined, label: 'Search', route: AppRoutes.search, color: Colors.deepOrange),
  ];

  void _showAddActionDialog() async {
    final pageActions = _buildPageActions();

    final available = [
      ..._allAvailableActions.where((a) {
        return !_actions.any((existing) => existing.route == a.route);
      }),
      ...pageActions.where((a) {
        return !_actions.any((existing) => existing.route == a.route);
      }),
    ];

    if (available.isEmpty) {
      if (!mounted) return;
      _showTopOverlay(AppLocalizations.of(context).homeNoMorePages);
      return;
    }

    final selected = await showDialog<QuickAction>(
      context: context,
      builder: (ctx) => AddQuickActionDialog(actions: available),
    );

    if (selected != null && mounted) {
      setState(() => _actions = List.from(_actions)..add(selected));
      await _persistQuickActions();
    }
  }

  Color _colorForPage(String name) {
    final colors = [
      Colors.blue, Colors.teal, Colors.green, Colors.orange,
      Colors.red, Colors.purple, Colors.indigo, Colors.cyan,
      Colors.deepOrange, Colors.pink, Colors.amber, Colors.lime,
    ];
    return colors[name.hashCode.abs() % colors.length];
  }

  void _showTopOverlay(String message) {
    _topOverlayEntry?.remove();
    _topOverlayEntry = null;

    final overlay = Overlay.of(context);
    final entry = OverlayEntry(
      builder: (ctx) => Positioned(
        top: MediaQuery.of(ctx).padding.top + kToolbarHeight + 8,
        left: 16,
        right: 16,
        child: Material(
          color: Colors.transparent,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: BoxDecoration(
              color: Theme.of(ctx).colorScheme.inverseSurface,
              borderRadius: BorderRadius.circular(12),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.15),
                  blurRadius: 10,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Text(
              message,
              style: Theme.of(ctx).textTheme.bodyMedium?.copyWith(
                color: Theme.of(ctx).colorScheme.onInverseSurface,
              ),
              textAlign: TextAlign.center,
            ),
          ),
        ),
      ),
    );
    _topOverlayEntry = entry;

    overlay.insert(entry);
    _topOverlayTimer?.cancel();
    _topOverlayTimer = Timer(AppTheme.kOverlayDuration, () {
      entry.remove();
      if (_topOverlayEntry == entry) {
        _topOverlayEntry = null;
      }
    });
  }


  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final authState = ref.watch(authNotifierProvider.select((a) => a.value));
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return SingleChildScrollView(
      padding: AppTheme.kPagePadding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Status card — Liquid Glass
          GlassCard(
            useOwnLayer: true,
            padding: const EdgeInsets.all(20),
            settings: isDark
                ? const LiquidGlassSettings(
                    thickness: 28,
                    blur: 10,
                    glassColor: Color(0x20FFFFFF),
                    refractiveIndex: 1.2,
                    lightIntensity: 1.1,
                  )
                : const LiquidGlassSettings(
                    thickness: 18,
                    blur: 8,
                    glassColor: Color(0x15D2DCF0),
                    refractiveIndex: 1.15,
                    lightIntensity: 1.0,
                  ),
            child: Row(
              children: [
                Container(
                  width: 48,
                  height: 48,
                  decoration: BoxDecoration(
                    color: AppTheme.successColor.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: const Icon(
                    Icons.shield,
                    color: AppTheme.successColor,
                  ),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        l10n.homeVaultUnlocked,
                        style: theme.textTheme.titleMedium,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        ref.watch(authNotifierProvider.notifier).selectedAccount?.name ?? l10n.homeDefaultAccountName,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: AppTheme.primaryColor,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  decoration: BoxDecoration(
                    color: authState == AuthState.unlocked
                        ? AppTheme.successColor.withValues(alpha: 0.1)
                        : Colors.blue.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(20),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        authState == AuthState.unlocked ? Icons.shield : Icons.lock,
                        size: 14,
                        color: authState == AuthState.unlocked
                            ? AppTheme.successColor
                            : Colors.blue,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        authState == AuthState.unlocked ? l10n.homeOnline : l10n.homeOffline,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: authState == AuthState.unlocked
                              ? AppTheme.successColor
                              : Colors.blue,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          const SizedBox(height: 32),

          // Quick Actions Grid
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(AppLocalizations.of(context).homeQuickActions, style: theme.textTheme.titleLarge),
              IconButton(
                icon: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 200),
                  child: Icon(
                    _isEditing ? Icons.check : Icons.edit,
                    key: ValueKey(_isEditing),
                  ),
                ),
                tooltip: _isEditing ? AppLocalizations.of(context).homeEditQuickActionsDone : AppLocalizations.of(context).homeEditQuickActions,
                onPressed: _toggleEditMode,
              ),
            ],
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: _isEditing ? 20 : 12,
            runSpacing: _isEditing ? 20 : 12,
            children: [
              for (int i = 0; i < _actions.length; i++) ...[
                () {
                  final index = i;
                  return _ActionSlotWidget(
                    index: index,
                    isEditing: _isEditing,
                    action: _actions[index],
                    wobbleController: _wobbleController,
                    onDragAccept: (details) {
                      final oldIndex = details.data;
                      setState(() {
                        _actions = List.from(_actions);
                        final item = _actions.removeAt(oldIndex);
                        _actions.insert(oldIndex < index ? index - 1 : index, item);
                      });
                      _persistQuickActions();
                    },
                    onDelete: () => _deleteAction(index),
                    onTap: () => context.push(_actions[index].route),
                  );
                }(),
              ],
              _AddButtonWidget(onTap: _showAddActionDialog),
            ],
          ),

          const SizedBox(height: 32),

          // Security Status — Liquid Glass
          Text(AppLocalizations.of(context).homeSecurityStatus, style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),
          GlassCard(
            useOwnLayer: true,
            padding: const EdgeInsets.all(20),
            settings: isDark
                ? const LiquidGlassSettings(
                    thickness: 28,
                    blur: 10,
                    glassColor: Color(0x20FFFFFF),
                    refractiveIndex: 1.2,
                    lightIntensity: 1.1,
                  )
                : const LiquidGlassSettings(
                    thickness: 18,
                    blur: 8,
                    glassColor: Color(0x15D2DCF0),
                    refractiveIndex: 1.15,
                    lightIntensity: 1.0,
                  ),
            child: Column(
              children: [
                SecurityItem(
                  icon: Icons.check_circle,
                  color: AppTheme.successColor,
                  title: l10n.homeEndToEndEncrypted,
                  subtitle: l10n.homeEncryptionDesc,
                ),
                const Divider(height: 24),
                SecurityItem(
                  icon: Icons.check_circle,
                  color: AppTheme.successColor,
                  title: l10n.homeLocalStorage,
                  subtitle: l10n.homeLocalStorageDesc,
                ),
                const Divider(height: 24),
                SecurityItem(
                  icon: Icons.check_circle,
                  color: AppTheme.successColor,
                  title: l10n.homeZeroKnowledge,
                  subtitle: l10n.homeZeroKnowledgeDesc,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ActionSlotWidget extends StatelessWidget {
  final int index;
  final bool isEditing;
  final QuickAction action;
  final AnimationController wobbleController;
  final ValueChanged<DragTargetDetails<int>> onDragAccept;
  final VoidCallback onDelete;
  final VoidCallback onTap;

  const _ActionSlotWidget({
    required this.index,
    required this.isEditing,
    required this.action,
    required this.wobbleController,
    required this.onDragAccept,
    required this.onDelete,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    if (isEditing) {
      return DragTarget<int>(
        onWillAcceptWithDetails: (details) => details.data != index,
        onAcceptWithDetails: onDragAccept,
        builder: (context, candidateData, rejectedData) {
          if (candidateData.isNotEmpty) {
            return const DashedPlaceholder();
          }
          return _EditingCardWidget(
            index: index,
            action: action,
            wobbleController: wobbleController,
            onDelete: onDelete,
          );
        },
      );
    }

    final l10n = AppLocalizations.of(context);
    return QuickActionTile(
      icon: action.icon,
      label: _localizedLabel(action.route, l10n),
      color: action.color,
      onTap: onTap,
    );
  }
}

class _EditingCardWidget extends StatelessWidget {
  final int index;
  final QuickAction action;
  final AnimationController wobbleController;
  final VoidCallback onDelete;

  const _EditingCardWidget({
    required this.index,
    required this.action,
    required this.wobbleController,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 90,
      height: 90,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          Positioned.fill(
            child: Draggable<int>(
              data: index,
              feedback: Material(
                elevation: 8,
                borderRadius: BorderRadius.circular(12),
                color: Colors.transparent,
                child: SizedBox(
                  width: 90,
                  height: 90,
                  child: QuickActionTile(
                    icon: action.icon,
                    label: _localizedLabel(action.route, AppLocalizations.of(context)),
                    color: action.color,
                  ),
                ),
              ),
              childWhenDragging: const DashedPlaceholder(),
              child: AnimatedBuilder(
                animation: wobbleController,
                builder: (context, child) {
                  final angle = math.sin(
                    wobbleController.value * math.pi * 4 + index * 0.6,
                  ) * 0.035;
                  return Transform.rotate(angle: angle, child: child);
                },
                child: QuickActionTile(
                  icon: action.icon,
                  label: _localizedLabel(action.route, AppLocalizations.of(context)),
                  color: action.color,
                ),
              ),
            ),
          ),
          Positioned(
            top: -4,
            right: -4,
            child: DeleteBadge(onTap: onDelete),
          ),
        ],
      ),
    );
  }
}

class _AddButtonWidget extends StatelessWidget {
  final VoidCallback onTap;

  const _AddButtonWidget({required this.onTap});

  @override
  Widget build(BuildContext context) {
    return AddButton(onTap: onTap);
  }
}
