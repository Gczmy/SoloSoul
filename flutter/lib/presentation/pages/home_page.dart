import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/services/user_preferences_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

import 'package:solosoul_flutter/presentation/widgets/home/page_editor.dart';

// =============================================================================
// HomePage — Dashboard with quick actions + inline page editor
// =============================================================================

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
      appBar: AppBar(
        title: const Text('SoloSoul'),
        actions: const [HeaderActionButtons()],
      ),
      body: _isEditingPage
          ? PageEditor(
              pageId: _editingPageId,
              onClose: _closeEditor,
            )
          : const _MainDashboard(),
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
  late List<_QuickAction> _actions;
  bool _isEditing = false;
  late AnimationController _wobbleController;
  OverlayEntry? _topOverlayEntry;
  Timer? _topOverlayTimer;

  @override
  void initState() {
    super.initState();
    _actions = List.from(const [
      _QuickAction(icon: Icons.person_outline, label: 'Profile', route: AppRoutes.profile, color: Colors.blue),
      _QuickAction(icon: Icons.flight_outlined, label: 'Travel', route: AppRoutes.travel, color: Colors.teal),
      _QuickAction(icon: Icons.account_balance_outlined, label: 'Financial', route: AppRoutes.financial, color: Colors.green),
      _QuickAction(icon: Icons.work_outline, label: 'Professional', route: AppRoutes.professional, color: Colors.orange),
      _QuickAction(icon: Icons.delete_outline, label: 'Trash', route: AppRoutes.trash, color: Colors.red),
      _QuickAction(icon: Icons.settings_outlined, label: 'Settings', route: AppRoutes.settings, color: Colors.grey),
    ]);
    _wobbleController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 400),
    );
    _loadQuickActions();
  }

  Future<void> _loadQuickActions() async {
    final savedRoutes = await UserPreferencesService.instance.loadQuickActions();
    if (savedRoutes.isEmpty) return;
    if (!mounted) return;

    final rebuilt = _rebuildActionsFromRoutes(savedRoutes);
    if (rebuilt.isNotEmpty) {
      setState(() => _actions = rebuilt);
    }
  }

  List<_QuickAction> _rebuildActionsFromRoutes(List<String> routes) {
    final all = [..._allAvailableActions];
    // Append custom pages
    final customPages = ref.read(unifiedObjectProvider).objects
        .where((o) => o.typeId == 'page' && !o.isDeleted);
    for (final page in customPages) {
      all.add(_QuickAction(
        icon: UnifiedObjectService.getIconFromName(page.iconName),
        label: page.name,
        route: '${AppRoutes.objects}/${page.id}',
        color: _colorForPage(page.name),
      ));
    }

    final result = <_QuickAction>[];
    for (final route in routes) {
      final match = all.where((a) => a.route == route).firstOrNull;
      if (match != null) result.add(match);
    }
    return result;
  }

  Future<void> _persistQuickActions() async {
    final routes = _actions.map((a) => a.route).toList();
    await UserPreferencesService.instance.saveQuickActions(routes);
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

  static const List<_QuickAction> _allAvailableActions = [
    _QuickAction(icon: Icons.person_outline, label: 'Profile', route: AppRoutes.profile, color: Colors.blue),
    _QuickAction(icon: Icons.flight_outlined, label: 'Travel', route: AppRoutes.travel, color: Colors.teal),
    _QuickAction(icon: Icons.account_balance_outlined, label: 'Financial', route: AppRoutes.financial, color: Colors.green),
    _QuickAction(icon: Icons.work_outline, label: 'Professional', route: AppRoutes.professional, color: Colors.orange),
    _QuickAction(icon: Icons.delete_outline, label: 'Trash', route: AppRoutes.trash, color: Colors.red),
    _QuickAction(icon: Icons.settings_outlined, label: 'Settings', route: AppRoutes.settings, color: Colors.grey),
    _QuickAction(icon: Icons.security_outlined, label: 'Security', route: AppRoutes.securitySettings, color: Colors.indigo),
    _QuickAction(icon: Icons.history_outlined, label: 'Operation Log', route: AppRoutes.operationLog, color: Colors.purple),
    _QuickAction(icon: Icons.visibility_outlined, label: 'Sensitivity', route: AppRoutes.sensitivitySettings, color: Colors.cyan),
    _QuickAction(icon: Icons.search_outlined, label: 'Search', route: AppRoutes.search, color: Colors.deepOrange),
  ];

  void _showAddActionDialog() async {
    final customPages = ref.read(unifiedObjectProvider).objects
        .where((o) => o.typeId == 'page' && !o.isDeleted)
        .toList();

    final customActions = customPages.map((page) => _QuickAction(
          icon: UnifiedObjectService.getIconFromName(page.iconName),
          label: page.name,
          route: '${AppRoutes.objects}/${page.id}',
          color: _colorForPage(page.name),
        )).toList();

    final available = [
      ..._allAvailableActions.where((a) {
        return !_actions.any((existing) => existing.route == a.route);
      }),
      ...customActions.where((a) {
        return !_actions.any((existing) => existing.route == a.route);
      }),
    ];

    if (available.isEmpty) {
      if (!mounted) return;
      _showTopOverlay('No more pages to add');
      return;
    }

    final selected = await showDialog<_QuickAction>(
      context: context,
      builder: (ctx) => _AddQuickActionDialog(actions: available),
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

  Widget _buildActionSlot(int index) {
    if (_isEditing) {
      return DragTarget<int>(
        onWillAcceptWithDetails: (details) => details.data != index,
        onAcceptWithDetails: (details) {
          final oldIndex = details.data;
          setState(() {
            _actions = List.from(_actions);
            final item = _actions.removeAt(oldIndex);
            _actions.insert(oldIndex < index ? index - 1 : index, item);
          });
          _persistQuickActions();
        },
        builder: (context, candidateData, rejectedData) {
          if (candidateData.isNotEmpty) {
            return const DashedPlaceholder();
          }
          return _buildEditingCard(index);
        },
      );
    }

    return _QuickActionTile(
      icon: _actions[index].icon,
      label: _actions[index].label,
      color: _actions[index].color,
      onTap: () => context.push(_actions[index].route),
    );
  }

  Widget _buildEditingCard(int index) {
    final action = _actions[index];

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
                  child: _QuickActionTile(
                    icon: action.icon,
                    label: action.label,
                    color: action.color,
                  ),
                ),
              ),
              childWhenDragging: const DashedPlaceholder(),
              child: AnimatedBuilder(
                animation: _wobbleController,
                builder: (context, child) {
                  final angle = math.sin(
                    _wobbleController.value * math.pi * 4 + index * 0.6,
                  ) * 0.035;
                  return Transform.rotate(angle: angle, child: child);
                },
                child: _QuickActionTile(
                  icon: action.icon,
                  label: action.label,
                  color: action.color,
                ),
              ),
            ),
          ),
          Positioned(
            top: -4,
            right: -4,
            child: _DeleteBadge(onTap: () => _deleteAction(index)),
          ),
        ],
      ),
    );
  }

  Widget _buildAddButton() {
    return _AddButton(onTap: _showAddActionDialog);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final authState = ref.watch(authNotifierProvider.select((a) => a.value));

    return SingleChildScrollView(
      padding: AppTheme.kPagePadding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Status card
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
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
                          'Vault Unlocked',
                          style: theme.textTheme.titleMedium,
                        ),
                        const SizedBox(height: 4),
                        Text(
                          ref.watch(authNotifierProvider.notifier).selectedAccount?.name ?? 'Account',
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
                          authState == AuthState.unlocked ? 'Online' : 'Offline',
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
          ),

          const SizedBox(height: 32),

          // Quick Actions Grid
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Quick Actions', style: theme.textTheme.titleLarge),
              IconButton(
                icon: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 200),
                  child: Icon(
                    _isEditing ? Icons.check : Icons.edit,
                    key: ValueKey(_isEditing),
                  ),
                ),
                tooltip: _isEditing ? 'Done' : 'Edit quick actions',
                onPressed: _toggleEditMode,
              ),
            ],
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: _isEditing ? 20 : 12,
            runSpacing: _isEditing ? 20 : 12,
            children: [
              for (int i = 0; i < _actions.length; i++)
                _buildActionSlot(i),
              _buildAddButton(),
            ],
          ),

          const SizedBox(height: 32),

          // Security Status
          Text('Security Status', style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),
          const Card(
            child: Padding(
              padding: EdgeInsets.all(20),
              child: Column(
                children: [
                  _SecurityItem(
                    icon: Icons.check_circle,
                    color: AppTheme.successColor,
                    title: 'End-to-End Encrypted',
                    subtitle: 'AES-256-GCM + Argon2id',
                  ),
                  Divider(height: 24),
                  _SecurityItem(
                    icon: Icons.check_circle,
                    color: AppTheme.successColor,
                    title: 'Local Storage',
                    subtitle: 'Data encrypted and stored locally',
                  ),
                  Divider(height: 24),
                  _SecurityItem(
                    icon: Icons.check_circle,
                    color: AppTheme.successColor,
                    title: 'Zero Knowledge',
                    subtitle: 'Master password never stored',
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _QuickAction {
  final IconData icon;
  final String label;
  final String route;
  final Color color;
  const _QuickAction({required this.icon, required this.label, required this.route, required this.color});
}

class _QuickActionTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color color;
  final VoidCallback? onTap;

  const _QuickActionTile({
    required this.icon,
    required this.label,
    required this.color,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SizedBox(
      width: 90,
      height: 90,
      child: Card(
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(icon, color: color, size: 20),
                ),
                const SizedBox(height: 8),
                Text(
                  label,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _SecurityItem extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;

  const _SecurityItem({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      children: [
        Icon(icon, color: color, size: 24),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: theme.textTheme.titleSmall),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _AddQuickActionDialog extends StatelessWidget {
  final List<_QuickAction> actions;

  const _AddQuickActionDialog({required this.actions});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add Quick Action'),
      content: SizedBox(
        width: double.maxFinite,
        child: ListView.builder(
          shrinkWrap: true,
          itemCount: actions.length,
          itemBuilder: (context, index) {
            final action = actions[index];
            return ListTile(
              leading: Container(
                width: 36,
                height: 36,
                decoration: BoxDecoration(
                  color: action.color.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(action.icon, color: action.color, size: 20),
              ),
              title: Text(action.label),
              onTap: () => Navigator.pop(context, action),
            );
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}

// =============================================================================
// Delete Badge with hover scale animation
// =============================================================================

class _DeleteBadge extends StatefulWidget {
  final VoidCallback onTap;

  const _DeleteBadge({required this.onTap});

  @override
  State<_DeleteBadge> createState() => _DeleteBadgeState();
}

class _DeleteBadgeState extends State<_DeleteBadge> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _isHovered ? 1.2 : 1.0,
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeOut,
          child: Container(
            width: 20,
            height: 20,
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.error,
              shape: BoxShape.circle,
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.2),
                  blurRadius: 2,
                  offset: const Offset(0, 1),
                ),
              ],
            ),
            child: const Icon(
              Icons.close,
              size: 12,
              color: Colors.white,
            ),
          ),
        ),
      ),
    );
  }
}

// =============================================================================
// Add Button with hover gray effect
// =============================================================================

class _AddButton extends StatefulWidget {
  final VoidCallback onTap;

  const _AddButton({required this.onTap});

  @override
  State<_AddButton> createState() => _AddButtonState();
}

class _AddButtonState extends State<_AddButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final borderColor = _isHovered
        ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4)
        : theme.colorScheme.primary.withValues(alpha: 0.4);
    final iconColor = _isHovered
        ? theme.colorScheme.onSurfaceVariant
        : theme.colorScheme.primary;

    return SizedBox(
      width: 90,
      height: 90,
      child: MouseRegion(
        onEnter: (_) => setState(() => _isHovered = true),
        onExit: (_) => setState(() => _isHovered = false),
        child: GestureDetector(
          onTap: widget.onTap,
          behavior: HitTestBehavior.opaque,
          child: DashedPlaceholder(
            color: borderColor,
            child: Container(
              color: _isHovered
                  ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.12)
                  : null,
              child: Center(
                child: Icon(
                  Icons.add,
                  color: iconColor,
                  size: 28,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
