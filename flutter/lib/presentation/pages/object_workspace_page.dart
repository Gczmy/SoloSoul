import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/object_tile.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/property_editor_factory.dart';

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
    final currentObject = widget.objectId != null
        ? ref.watch(objectByIdProvider(widget.objectId!))
        : null;
    final allChildren = widget.objectId != null
        ? ref.watch(childrenProvider(widget.objectId!))
        : ref.watch(rootObjectsProvider);
    // Page-type children are shown in the sidebar tree, not in the workspace.
    final children = allChildren.where((c) => c.typeId != 'page').toList();

    final title = currentObject?.name ?? 'Objects';
    final isPage = currentObject?.typeId == 'page';

    return Scaffold(
      appBar: AppBar(
        title: Text(title),
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
                    return _ObjectCard(object: children[index]);
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
                          onTap: () => context.push('/objects/${child.id}'),
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
                ? _showAddObjectDialog()
                : _createObject(),
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

  void _createObject() {
    context.push('/object_editor?parentId=${widget.objectId}');
  }

  void _editObject(UnifiedObject object) {
    context.push('/object_editor?id=${object.id}');
  }

  void _deleteObject(UnifiedObject object) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Object'),
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
          const SnackBar(content: Text('Object deleted')),
        );
      }
    }
  }

  Future<void> _showAddObjectDialog() async {
    final result = await showDialog<Map<String, String>>(
      context: context,
      builder: (ctx) => const _AddObjectDialog(),
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
// Object Card — Card-style property management for Page children
// =============================================================================

class _ObjectCard extends ConsumerStatefulWidget {
  final UnifiedObject object;

  const _ObjectCard({required this.object});

  @override
  ConsumerState<_ObjectCard> createState() => _ObjectCardState();
}

class _ObjectCardState extends ConsumerState<_ObjectCard> {
  bool _isAddingItem = false;
  final _keyController = TextEditingController();
  final _valueController = TextEditingController();

  @override
  void dispose() {
    _keyController.dispose();
    _valueController.dispose();
    super.dispose();
  }

  Future<void> _saveItem() async {
    final key = _keyController.text.trim();
    final value = _valueController.text.trim();
    if (key.isEmpty || value.isEmpty) return;

    final updatedProps = Map<String, PropertyValue>.from(widget.object.properties);
    updatedProps[key] = TextProperty(text: value);

    await ref.read(unifiedObjectProvider.notifier).updateObject(
      widget.object.id,
      properties: updatedProps,
    );

    _keyController.clear();
    _valueController.clear();
    setState(() => _isAddingItem = false);
  }

  Future<void> _deleteItem(String key) async {
    final updatedProps = Map<String, PropertyValue>.from(widget.object.properties);
    updatedProps.remove(key);

    await ref.read(unifiedObjectProvider.notifier).updateObject(
      widget.object.id,
      properties: updatedProps,
    );
  }

  Future<void> _changeIcon() async {
    final result = await showModalBottomSheet<String>(
      context: context,
      builder: (ctx) => IconPickerSheet(currentIcon: widget.object.iconName),
    );
    if (result != null && result != widget.object.iconName) {
      await ref.read(unifiedObjectProvider.notifier).updateObject(
        widget.object.id,
        iconName: result,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = UnifiedObjectService.getIconFromName(widget.object.iconName);
    final properties = widget.object.properties.entries.toList();

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            // Header: icon + name + add button
            Row(
              children: [
                InkWell(
                  onTap: _changeIcon,
                  borderRadius: BorderRadius.circular(6),
                  child: Padding(
                    padding: const EdgeInsets.all(4),
                    child: Icon(icon, color: theme.colorScheme.primary, size: 20),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    widget.object.name,
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.add, size: 20),
                  onPressed: () => setState(() => _isAddingItem = true),
                  tooltip: 'Add item',
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
                ),
              ],
            ),

            const Divider(height: 24),

            // Properties / Items list
            if (properties.isEmpty && !_isAddingItem)
              Center(
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  child: TextButton.icon(
                    onPressed: () => setState(() => _isAddingItem = true),
                    icon: const Icon(Icons.add, size: 18),
                    label: const Text('Add item'),
                  ),
                ),
              )
            else
              ...properties.map((entry) => Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          flex: 2,
                          child: Text(
                            entry.key,
                            style: theme.textTheme.bodyMedium?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          flex: 3,
                          child: PropertyEditorFactory.buildDisplay(entry.value),
                        ),
                        IconButton(
                          icon: const Icon(Icons.close, size: 18),
                          onPressed: () => _deleteItem(entry.key),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(
                            minWidth: 28,
                            minHeight: 28,
                          ),
                        ),
                      ],
                    ),
                  )),

            // Inline input for adding new item
            if (_isAddingItem) ...[
              const SizedBox(height: 8),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    flex: 2,
                    child: TextField(
                      controller: _keyController,
                      autofocus: true,
                      decoration: const InputDecoration(
                        hintText: 'Name',
                        isDense: true,
                        contentPadding: EdgeInsets.symmetric(
                          horizontal: 10,
                          vertical: 10,
                        ),
                        border: OutlineInputBorder(),
                      ),
                      style: theme.textTheme.bodyMedium,
                      textInputAction: TextInputAction.next,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    flex: 3,
                    child: TextField(
                      controller: _valueController,
                      decoration: const InputDecoration(
                        hintText: 'Value',
                        isDense: true,
                        contentPadding: EdgeInsets.symmetric(
                          horizontal: 10,
                          vertical: 10,
                        ),
                        border: OutlineInputBorder(),
                      ),
                      style: theme.textTheme.bodyMedium,
                      onSubmitted: (_) => _saveItem(),
                    ),
                  ),
                  const SizedBox(width: 4),
                  IconButton(
                    icon: const Icon(Icons.check, size: 18),
                    onPressed: _saveItem,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(
                      minWidth: 28,
                      minHeight: 28,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    onPressed: () => setState(() => _isAddingItem = false),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(
                      minWidth: 28,
                      minHeight: 28,
                    ),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Add Object Dialog — Simple name + icon picker for Page children
// =============================================================================

class _AddObjectDialog extends StatefulWidget {
  const _AddObjectDialog();

  @override
  State<_AddObjectDialog> createState() => _AddObjectDialogState();
}

class _AddObjectDialogState extends State<_AddObjectDialog> {
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
      title: const Text('Add Object'),
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
              hintText: 'Enter object name',
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
          child: const Text('Add'),
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
