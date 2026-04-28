import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider, unifiedObjectCacheProvider;
import 'package:solosoul_flutter/presentation/widgets/object_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

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
    final cache = ref.watch(unifiedObjectCacheProvider);

    final currentObject = widget.objectId != null
        ? cache.objectById[widget.objectId]
        : null;

    // Auto-navigate back if the current object has been deleted.
    if (widget.objectId != null && currentObject == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) context.pop();
      });
      return const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      );
    }

    final children = widget.objectId != null
        ? cache.workspaceChildren[widget.objectId] ?? []
        : cache.rootObjects.where((o) => o.typeId != 'page').toList();

    final title = currentObject?.name ?? 'Objects';
    final isPage = currentObject?.typeId == 'page';

    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        actions: const [HeaderActionButtons()],
      ),
      body: children.isEmpty
          ? _EmptyState(
              message: currentObject != null
                  ? 'No items yet'
                  : 'No objects yet',
              hint: currentObject != null
                  ? 'Add your first item'
                  : 'Create your first object to get started',
            )
          : isPage
              ? ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: children.length,
                  itemBuilder: (context, index) {
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
                          onTap: () {},
                        );
                      },
                    )
                  : ListView.builder(
                      padding: const EdgeInsets.all(16),
                      itemCount: children.length,
                      itemBuilder: (context, index) {
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
              tooltip: 'Delete',
              backgroundColor: Theme.of(context).colorScheme.errorContainer,
              foregroundColor: Theme.of(context).colorScheme.onErrorContainer,
              child: const Icon(Icons.delete_outline),
            ),
            const SizedBox(width: 8),
            FloatingActionButton.small(
              heroTag: 'edit_page',
              onPressed: () => _editObject(currentObject),
              tooltip: 'Edit Page',
              child: const Icon(Icons.edit_outlined),
            ),
            const SizedBox(width: 8),
          ],
          if (!isPage) ...[
            FloatingActionButton.small(
              heroTag: 'reorder',
              onPressed: () => setState(() => _isReordering = !_isReordering),
              tooltip: _isReordering ? 'Done' : 'Reorder',
              child: Icon(_isReordering ? Icons.check : Icons.reorder),
            ),
            const SizedBox(width: 8),
          ],
          FloatingActionButton.small(
            heroTag: 'add',
            onPressed: () => isPage
                ? _showPageAddMenu()
                : _showAddSectionDialog(),
            tooltip: 'Add',
            child: const Icon(Icons.add),
          ),
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
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Section'),
        content: Text(
          'Are you sure you want to delete "${object.name}"?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(object.id);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Section deleted')),
        );
      }
    }
  }

  void _deleteCurrentObject(UnifiedObject object) async {
    final descendantCount = UnifiedObjectService.instance
        .getDescendantIds(ref.read(unifiedObjectProvider).objects, object.id)
        .length;

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(object.typeId == 'page' ? 'Delete Page' : 'Delete Section'),
        content: Text(
          'Are you sure you want to delete "${object.name}"?'
          '${descendantCount > 0 ? '\n\nAll $descendantCount item(s) inside this ${object.typeId == 'page' ? 'page' : 'section'} will also be moved to trash.' : ''}',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      await ref.read(unifiedObjectProvider.notifier).deleteObject(object.id);
      if (mounted) {
        context.pop();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('"${object.name}" moved to trash')),
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

  void _showPageAddMenu() async {
    final choice = await showModalBottomSheet<String>(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.add),
              title: const Text('Add Sub-Page'),
              onTap: () => Navigator.pop(ctx, 'page'),
            ),
            ListTile(
              leading: const Icon(Icons.folder_outlined),
              title: const Text('Add Section'),
              onTap: () => Navigator.pop(ctx, 'section'),
            ),
          ],
        ),
      ),
    );
    if (choice == 'page') {
      if (!mounted) return;
      await context.push('/page_editor?parentId=${widget.objectId}');
    } else if (choice == 'section') {
      await _showAddSectionDialog();
    }
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
      title: const Text('Add Section'),
      constraints: const BoxConstraints(maxWidth: 320),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            controller: _nameController,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Name',
              hintText: 'Enter section name',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text('Icon', style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: kIconNames.map((name) {
              final isSelected = name == _iconName;
              return Material(
                color: isSelected
                    ? theme.colorScheme.primary.withValues(alpha: 0.15)
                    : theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
                child: InkWell(
                  borderRadius: BorderRadius.circular(8),
                  onTap: () => setState(() => _iconName = name),
                  child: Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: isSelected
                            ? theme.colorScheme.primary
                            : Colors.transparent,
                        width: 2,
                      ),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      UnifiedObjectService.getIconFromName(name),
                      size: 20,
                      color: isSelected
                          ? theme.colorScheme.primary
                          : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              );
            }).toList(),
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
            final name = _nameController.text.trim();
            if (name.isEmpty) return;
            Navigator.pop(context, {
              'name': name,
              'icon': _iconName,
            });
          },
          child: const Text('Add Section'),
        ),
      ],
    );
  }
}

// =============================================================================
// Empty State
// =============================================================================

class _EmptyState extends StatelessWidget {
  final String message;
  final String hint;

  const _EmptyState({
    required this.message,
    required this.hint,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_open_outlined,
            size: 64,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 16),
          Text(
            message,
            style: theme.textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          Text(
            hint,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}
