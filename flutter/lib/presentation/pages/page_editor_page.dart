import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/pages/object_editor_page.dart'
    show ObjectParentDropdown;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;

/// Editor specifically for Page-type UnifiedObjects.
class PageEditorPage extends ConsumerStatefulWidget {
  final String? objectId;
  final String? parentId;

  const PageEditorPage({
    super.key,
    this.objectId,
    this.parentId,
  });

  @override
  ConsumerState<PageEditorPage> createState() => _PageEditorPageState();
}

class _PageEditorPageState extends ConsumerState<PageEditorPage> {
  late final TextEditingController _nameController;
  late final TextEditingController _iconController;
  String? _selectedParentId;

  bool get _isEditing => widget.objectId != null;
  UnifiedObject? _existingObject;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController();
    _iconController = TextEditingController(text: 'article');
    _selectedParentId = widget.parentId;

    if (_isEditing) {
      _loadExistingObject();
    }
  }

  void _loadExistingObject() {
    final object = ref.read(objectByIdProvider(widget.objectId!));
    if (object == null) return;

    _existingObject = object;
    _nameController.text = object.name;
    _iconController.text = object.iconName;
    _selectedParentId = object.parentId;
  }

  @override
  void dispose() {
    _nameController.dispose();
    _iconController.dispose();
    super.dispose();
  }

  void _savePage() async {
    if (_nameController.text.trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Name is required')),
      );
      return;
    }

    final notifier = ref.read(unifiedObjectProvider.notifier);

    if (_isEditing && _existingObject != null) {
      // Handle parent change
      final oldParentId = _existingObject!.parentId;
      final newParentId = _selectedParentId;

      if (oldParentId != newParentId) {
        await notifier.moveObject(_existingObject!.id, newParentId);
      }

      await notifier.updateObject(
        _existingObject!.id,
        name: _nameController.text.trim(),
        iconName: _iconController.text.trim(),
      );
    } else {
      await notifier.createObject(
        name: _nameController.text.trim(),
        typeId: 'page',
        parentId: _selectedParentId,
        iconName: _iconController.text.trim(),
      );
    }

    if (mounted) {
      context.pop();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(_isEditing ? 'Edit Page' : 'New Page'),
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Name
            Text('Name', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(
                hintText: 'Enter page name',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 24),

            // Icon
            Text('Icon', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            InkWell(
              onTap: () async {
                final result = await showModalBottomSheet<String>(
                  context: context,
                  builder: (ctx) => IconPickerSheet(
                    currentIcon: _iconController.text.isEmpty
                        ? 'article'
                        : _iconController.text,
                  ),
                );
                if (result != null) {
                  setState(() {
                    _iconController.text = result;
                  });
                }
              },
              borderRadius: BorderRadius.circular(12),
              child: Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  UnifiedObjectService.getIconFromName(_iconController.text),
                  color: theme.colorScheme.primary,
                  size: 28,
                ),
              ),
            ),
            const SizedBox(height: 24),

            // Parent
            Text('Parent', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            ObjectParentDropdown(
              selectedParentId: _selectedParentId,
              objectId: widget.objectId,
              onChanged: (value) {
                setState(() {
                  _selectedParentId = value;
                });
              },
            ),
            const SizedBox(height: 32),

            // Save
            Center(
              child: OutlinedButton(
                onPressed: _savePage,
                child: const Text('Save'),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}
