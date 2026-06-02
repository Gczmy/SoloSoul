import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/icon_picker_sheet.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme;
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/utils/icon_resolver.dart';

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
    final objectId = widget.objectId;
    if (objectId == null) return;
    final object = ref.read(objectByIdProvider(objectId));
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
        SnackBar(content: Text(AppLocalizations.of(context).pageEditorNameRequired)),
      );
      return;
    }

    final notifier = ref.read(unifiedObjectProvider.notifier);

    final existing = _existingObject;
    if (_isEditing && existing != null) {
      // Handle parent change
      final oldParentId = existing.parentId;
      final newParentId = _selectedParentId;

      if (oldParentId != newParentId) {
        await notifier.moveObject(existing.id, newParentId);
      }

      await notifier.updateObject(
        existing.id,
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
      appBar: SoloGlassAppBar(
        title: Text(_isEditing ? AppLocalizations.of(context).pageEditorEditPage : AppLocalizations.of(context).pageEditorNewPage),
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Name
            Text(AppLocalizations.of(context).pageEditorName, style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            TextField(
              controller: _nameController,
              decoration: InputDecoration(
                hintText: AppLocalizations.of(context).pageEditorEnterPageName,
                border: const OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 24),

            // Icon
            Text(AppLocalizations.of(context).pageEditorIcon, style: theme.textTheme.titleMedium),
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
                  IconResolver.resolve(_iconController.text),
                  color: theme.colorScheme.primary,
                  size: 28,
                ),
              ),
            ),
            const SizedBox(height: 24),

            // Parent (hidden - parent change disabled to avoid bugs)
            // Text(AppLocalizations.of(context).pageEditorParent, style: theme.textTheme.titleMedium),
            // const SizedBox(height: 12),
            // ObjectParentDropdown(
            //   selectedParentId: _selectedParentId,
            //   objectId: widget.objectId,
            //   onChanged: (value) {
            //     setState(() {
            //       _selectedParentId = value;
            //     });
            //   },
            // ),
            const SizedBox(height: 32),

            // Save
            Center(
              child: OutlinedButton(
                onPressed: _savePage,
                child: Text(AppLocalizations.of(context).commonSave),
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}
