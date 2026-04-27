import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

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
          ? _PageEditor(
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

class _MainDashboard extends ConsumerWidget {
  const _MainDashboard();

  static const List<_QuickAction> _actions = [
    _QuickAction(icon: Icons.person_outline, label: 'Profile', route: AppRoutes.profile, color: Colors.blue),
    _QuickAction(icon: Icons.flight_outlined, label: 'Travel', route: AppRoutes.travel, color: Colors.teal),
    _QuickAction(icon: Icons.account_balance_outlined, label: 'Financial', route: AppRoutes.financial, color: Colors.green),
    _QuickAction(icon: Icons.work_outline, label: 'Professional', route: AppRoutes.professional, color: Colors.orange),
    _QuickAction(icon: Icons.delete_outline, label: 'Trash', route: AppRoutes.trash, color: Colors.red),
    _QuickAction(icon: Icons.settings_outlined, label: 'Settings', route: AppRoutes.settings, color: Colors.grey),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final authState = ref.watch(authNotifierProvider).value;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
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
          Text('Quick Actions', style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: _actions.map((action) => _QuickActionTile(
              icon: action.icon,
              label: action.label,
              color: action.color,
              onTap: () => context.push(action.route),
            )).toList(),
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
  final VoidCallback onTap;

  const _QuickActionTile({
    required this.icon,
    required this.label,
    required this.color,
    required this.onTap,
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

// =============================================================================
// Page Editor (Inline in main content)
// =============================================================================

class _PageEditor extends ConsumerStatefulWidget {
  final String? pageId;
  final VoidCallback onClose;

  const _PageEditor({this.pageId, required this.onClose});

  @override
  ConsumerState<_PageEditor> createState() => _PageEditorState();
}

class _PageEditorState extends ConsumerState<_PageEditor> {
  late final TextEditingController _titleController;
  late String _iconName;
  bool _isSaving = false;

  UnifiedObject? get _existingPage =>
      widget.pageId != null
          ? ref.read(objectByIdProvider(widget.pageId!))
          : null;

  List<UnifiedObject> get _sections {
    if (widget.pageId == null) return [];
    return ref.read(childrenProvider(widget.pageId!));
  }

  @override
  void initState() {
    super.initState();
    _titleController = TextEditingController(
      text: _existingPage?.name ?? 'New Page',
    );
    _iconName = _existingPage?.iconName ?? 'article';
  }

  @override
  void dispose() {
    _titleController.dispose();
    super.dispose();
  }

  Future<void> _savePage({bool closeAfter = true}) async {
    if (_titleController.text.trim().isEmpty) return;
    setState(() => _isSaving = true);

    final notifier = ref.read(unifiedObjectProvider.notifier);
    if (_existingPage != null) {
      await notifier.updateObject(
        _existingPage!.id,
        name: _titleController.text.trim(),
        iconName: _iconName,
      );
    } else {
      await notifier.createObject(
        name: _titleController.text.trim(),
        typeId: 'page',
        iconName: _iconName,
      );
    }

    setState(() => _isSaving = false);
    if (closeAfter) widget.onClose();
  }

  Future<void> _addSection() async {
    final page = _existingPage;
    if (page == null) {
      // Page must be saved first
      await _savePage(closeAfter: false);
    }
    if (!mounted) return;

    final result = await showDialog<Map<String, String>>(
      context: context,
      builder: (ctx) => const _SectionDialog(),
    );

    if (result == null) return;
    if (!mounted) return;

    final notifier = ref.read(unifiedObjectProvider.notifier);
    final pageId = _existingPage?.id;
    if (pageId == null) return;

    await notifier.createObject(
      name: result['title']!,
      typeId: 'collection',
      parentId: pageId,
      iconName: result['icon']!,
    );
  }

  Future<void> _editSection(UnifiedObject section) async {
    final result = await showDialog<Map<String, String>>(
      context: context,
      builder: (ctx) => _SectionDialog(
        initialTitle: section.name,
        initialIcon: section.iconName,
      ),
    );

    if (result == null) return;

    await ref.read(unifiedObjectProvider.notifier).updateObject(
      section.id,
      name: result['title']!,
      iconName: result['icon']!,
    );
  }

  Future<void> _deleteSection(String sectionId) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete Section?'),
        content: const Text('This section and its items will be moved to trash.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(sectionId);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final page = _existingPage;

    return Scaffold(
      backgroundColor: theme.colorScheme.surface,
      appBar: AppBar(
        title: Text(page != null ? 'Edit Page' : 'New Page'),
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: widget.onClose,
        ),
        actions: [
          if (_isSaving)
            const Padding(
              padding: EdgeInsets.only(right: 16),
              child: Center(child: SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))),
            )
          else
            TextButton(
              onPressed: _savePage,
              child: const Text('Save'),
            ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Page title & icon
            Row(
              children: [
                _IconPicker(
                  iconName: _iconName,
                  onChanged: (v) => setState(() => _iconName = v),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: TextField(
                    controller: _titleController,
                    style: theme.textTheme.headlineSmall,
                    decoration: const InputDecoration(
                      hintText: 'Page title',
                      border: InputBorder.none,
                    ),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 32),

            // Sections header
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Sections', style: theme.textTheme.titleLarge),
                FilledButton.icon(
                  onPressed: page != null ? _addSection : null,
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add Section'),
                ),
              ],
            ),
            if (page == null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  'Save the page first to add sections',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                    fontStyle: FontStyle.italic,
                  ),
                ),
              ),

            const SizedBox(height: 16),

            // Sections list
            if (_sections.isEmpty && page != null)
              Center(
                child: Padding(
                  padding: const EdgeInsets.all(32),
                  child: Column(
                    children: [
                      Icon(Icons.folder_open, size: 48, color: theme.colorScheme.onSurfaceVariant),
                      const SizedBox(height: 12),
                      Text(
                        'No sections yet',
                        style: theme.textTheme.titleMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              )
            else
              ..._sections.map((section) => Card(
                    margin: const EdgeInsets.only(bottom: 8),
                    child: ListTile(
                      leading: Icon(
                        UnifiedObjectService.getIconFromName(section.iconName),
                        color: theme.colorScheme.primary,
                      ),
                      title: Text(section.name),
                      subtitle: Text('${section.childrenIds.length} items'),
                      trailing: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            icon: const Icon(Icons.edit_outlined, size: 20),
                            onPressed: () => _editSection(section),
                          ),
                          IconButton(
                            icon: Icon(Icons.delete_outline, size: 20, color: theme.colorScheme.error),
                            onPressed: () => _deleteSection(section.id),
                          ),
                        ],
                      ),
                    ),
                  )),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Section Dialog
// =============================================================================

class _SectionDialog extends StatefulWidget {
  final String? initialTitle;
  final String? initialIcon;

  const _SectionDialog({this.initialTitle, this.initialIcon});

  @override
  State<_SectionDialog> createState() => _SectionDialogState();
}

class _SectionDialogState extends State<_SectionDialog> {
  late final TextEditingController _titleController;
  late String _iconName;

  @override
  void initState() {
    super.initState();
    _titleController = TextEditingController(text: widget.initialTitle ?? '');
    _iconName = widget.initialIcon ?? 'folder';
  }

  @override
  void dispose() {
    _titleController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return AlertDialog(
      title: Text(widget.initialTitle == null ? 'Add Section' : 'Edit Section'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            controller: _titleController,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Section Title',
              hintText: 'Enter section title',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text('Icon', style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          _IconPicker(
            iconName: _iconName,
            onChanged: (v) => setState(() => _iconName = v),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () {
            if (_titleController.text.trim().isEmpty) return;
            Navigator.pop(context, {
              'title': _titleController.text.trim(),
              'icon': _iconName,
            });
          },
          child: const Text('Save'),
        ),
      ],
    );
  }
}

// =============================================================================
// Icon Picker
// =============================================================================

class _IconPicker extends StatelessWidget {
  final String iconName;
  final ValueChanged<String> onChanged;

  const _IconPicker({required this.iconName, required this.onChanged});

  static const List<String> _iconNames = [
    'article', 'folder', 'note', 'person', 'flight', 'work',
    'school', 'account_balance', 'credit_card', 'home', 'language',
    'star', 'book', 'favorite', 'security', 'medical_services',
    'phone', 'email', 'link', 'description', 'check_circle',
    'restaurant', 'sports', 'music_note', 'movie', 'camera',
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: () async {
        final result = await showModalBottomSheet<String>(
          context: context,
          builder: (ctx) => SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Wrap(
                spacing: 12,
                runSpacing: 12,
                children: _iconNames.map((name) {
                  final isSelected = name == iconName;
                  return Material(
                    color: isSelected
                        ? theme.colorScheme.primary.withValues(alpha: 0.15)
                        : theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(10),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(10),
                      onTap: () => Navigator.pop(ctx, name),
                      child: Container(
                        width: 48,
                        height: 48,
                        decoration: BoxDecoration(
                          border: Border.all(
                            color: isSelected ? theme.colorScheme.primary : Colors.transparent,
                            width: 2,
                          ),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: Icon(
                          UnifiedObjectService.getIconFromName(name),
                          color: isSelected ? theme.colorScheme.primary : theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ),
          ),
        );
        if (result != null) onChanged(result);
      },
      borderRadius: BorderRadius.circular(12),
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          color: theme.colorScheme.primary.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Icon(
          UnifiedObjectService.getIconFromName(iconName),
          color: theme.colorScheme.primary,
        ),
      ),
    );
  }
}
