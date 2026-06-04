import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show showOverlaySnackBar, SnackBarType;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider, unifiedObjectCacheProvider;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/object_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/categorized_icon_grid.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/add_section_placeholder.dart';

/// Unified workspace for browsing objects and their children.
///
/// When [objectId] is null, shows the root-level objects.
/// When [objectId] is provided, shows that object's details and its children.
class ObjectWorkspacePage extends ConsumerStatefulWidget {
  final String? objectId;

  const ObjectWorkspacePage({
    super.key,
    this.objectId,
  });

  @override
  ConsumerState<ObjectWorkspacePage> createState() =>
      _ObjectWorkspacePageState();
}

class _ObjectWorkspacePageState extends ConsumerState<ObjectWorkspacePage> {
  bool _isReordering = false;

  @override
  Widget build(BuildContext context) {
    // 使用预计算缓存：数据变化时一次性重建索引，页面切换时直接 O(1) 读取
    final cache = ref.watch(
      unifiedObjectCacheProvider.select(
        (c) => (
          objectById: c.objectById,
          itemChildren: c.itemChildren,
          workspaceChildren: c.workspaceChildren,
          rootObjects: c.rootObjects,
        ),
      ),
    );

    final currentObject = widget.objectId != null
        ? cache.objectById[widget.objectId]
        : null;

    // Auto-navigate back if the current object has been deleted.
    if (widget.objectId != null && currentObject == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        if (context.canPop()) {
          context.pop();
        } else {
          context.go('/objects');
        }
      });
      return const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      );
    }

    final children = widget.objectId != null
        ? cache.workspaceChildren[widget.objectId] ?? []
        : cache.rootObjects.where((o) => o.typeId != 'page').toList();

    final l10n = AppLocalizations.of(context);
    final title = currentObject?.name ?? l10n.workspaceObjects;
    final isPage = currentObject?.typeId == 'page';

    // Diagnostic logging — always print, do not gate on DebugLogger.isActive
    // ignore: avoid_print
    print('[DIAG-WORKSPACE] build: objectId=${widget.objectId}, '
        'currentObject=${currentObject?.name}, isPage=$isPage, children=${children.length}, '
        'wsKeys=${cache.workspaceChildren.keys.take(5).toList()}, root=${cache.rootObjects.length}');

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(title),
        actions: const [HeaderActionButtons()],
      ),
      body: isPage
          ? ListView.builder(
              padding: const EdgeInsets.all(16),
              itemCount: children.length + 1,
              itemBuilder: (context, index) {
                if (index == children.length) {
                  return AddSectionPlaceholder(onTap: _showAddSectionDialog);
                }
                final child = children[index];
                return RepaintBoundary(
                  child: ObjectCard(
                    object: child,
                    items: cache.itemChildren[child.id] ?? [],
                  ),
                );
              },
            )
          : _isReordering
              ? ReorderableListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: children.length,
                  onReorder: (oldIndex, newIndex) => _handleReorder(
                    oldIndex,
                    newIndex,
                    children,
                  ),
                  itemBuilder: (context, index) {
                    final child = children[index];
                    return ObjectTile(
                      key: ValueKey(child.id),
                      object: child,
                      showDragHandle: true,
                      dragIndex: index,
                      onTap: null,
                    );
                  },
                )
              : ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: children.length + 1,
                  itemBuilder: (context, index) {
                    if (index == children.length) {
                      return AddSectionPlaceholder(onTap: _showAddSectionDialog);
                    }
                    final child = children[index];
                    return ObjectTile(
                      object: child,
                      onTap: child.typeId == 'page'
                          ? () => context.push('/objects/${child.id}')
                          : null,
                      onEdit: () => _editObject(child),
                      onDelete: () => _deleteObject(child),
                    );
                  },
                ),
      floatingActionButton: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (currentObject != null) ...[
            FloatingActionButton.small(
              heroTag: 'delete_page',
              onPressed: () => _deleteCurrentObject(currentObject),
              tooltip: l10n.workspaceDeleteSection,
              backgroundColor: Theme.of(context).colorScheme.errorContainer,
              foregroundColor: Theme.of(context).colorScheme.onErrorContainer,
              child: const Icon(Icons.delete_outline),
            ),
            const SizedBox(width: 8),
            FloatingActionButton.small(
              heroTag: 'edit_page',
              onPressed: () => _editObject(currentObject),
              tooltip: l10n.workspaceEditPage,
              child: const Icon(Icons.edit_outlined),
            ),
            const SizedBox(width: 8),
          ],
          if (!isPage) ...[
            FloatingActionButton.small(
              heroTag: 'reorder',
              onPressed: () => setState(() => _isReordering = !_isReordering),
              tooltip: _isReordering ? l10n.workspaceDone : l10n.workspaceReorder,
              child: Icon(_isReordering ? Icons.check : Icons.reorder),
            ),
            const SizedBox(width: 8),
            FloatingActionButton.small(
              heroTag: 'add',
              onPressed: _showAddSectionDialog,
              tooltip: l10n.workspaceAdd,
              child: const Icon(Icons.add),
            ),
          ],
        ],
      ),
    );
  }

  void _handleReorder(
    int oldIndex,
    int newIndex,
    List<UnifiedObject> children,
  ) {
    if (newIndex > oldIndex) newIndex--;
    if (widget.objectId == null) return;
    ref.read(unifiedObjectProvider.notifier).reorderChildren(
          widget.objectId!,
          oldIndex,
          newIndex,
        );
  }

  void _editObject(UnifiedObject object) {
    if (object.typeId == 'page') {
      context.push('/page_editor?id=${object.id}');
    } else {
      context.push('/object_editor?id=${object.id}');
    }
  }

  void _deleteObject(UnifiedObject object) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.workspaceDeleteSection),
        content: Text(l10n.workspaceDeleteSectionConfirm(object.name)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.commonCancel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: Text(l10n.commonDelete),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(object.id);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.workspaceSectionDeleted,
          type: SnackBarType.success,
        );
      }
    }
  }

  void _deleteCurrentObject(UnifiedObject object) async {
    final l10n = AppLocalizations.of(context);
    final descendantCount = UnifiedObjectService.instance
        .getDescendantIds(ref.read(unifiedObjectProvider).objects, object.id)
        .length;

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(object.typeId == 'page' ? l10n.workspaceDeletePage : l10n.workspaceDeleteSection),
        content: Text(object.typeId == 'page'
            ? l10n.workspaceDeletePageConfirm(object.name, descendantCount)
            : l10n.workspaceDeleteSectionConfirm(object.name)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.commonCancel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: Text(l10n.commonDelete),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(object.id);
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.workspaceMovedToTrash(object.name),
          type: SnackBarType.success,
        );
      }
    }
  }

  Future<void> _showAddSectionDialog() async {
    final result = await showDialog<Map<String, String>>(
      context: context,
      builder: (ctx) => const _AddSectionDialog(),
    );
    if (result == null) return;

    await ref.read(unifiedObjectProvider.notifier).createObject(
      name: result['name']!,
      typeId: 'collection',
      parentId: widget.objectId,
      iconName: result['icon']!,
    );
  }

}


// =============================================================================
// Add Section Dialog — Simple name + icon picker for Page children
// =============================================================================

class _AddSectionDialog extends StatefulWidget {
  const _AddSectionDialog();

  @override
  State<_AddSectionDialog> createState() => _AddSectionDialogState();
}

class _AddSectionDialogState extends State<_AddSectionDialog> {
  final _nameController = TextEditingController();
  String _iconName = 'folder';

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return AlertDialog(
      title: Text(AppLocalizations.of(context).workspaceAddSectionDialog),
      constraints: const BoxConstraints(maxWidth: 320),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            controller: _nameController,
            autofocus: true,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).workspaceSectionName,
              hintText: AppLocalizations.of(context).workspaceEnterSectionName,
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text(AppLocalizations.of(context).workspaceIcon, style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          SizedBox(
            height: 180,
            child: CategorizedIconGrid(
              currentIcon: _iconName,
              iconSize: 40,
              spacing: 8,
              onSelected: (name) => setState(() => _iconName = name),
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(AppLocalizations.of(context).commonCancel),
        ),
        FilledButton(
          onPressed: () {
            final name = _nameController.text.trim();
            if (name.isEmpty) return;
            Navigator.pop(context, {
              'name': name,
              'icon': _iconName,
            });
          },
          child: Text(AppLocalizations.of(context).workspaceAddSectionButton),
        ),
      ],
    );
  }
}




