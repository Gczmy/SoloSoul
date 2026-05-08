import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/home/icon_picker.dart';

/// Inline full-page editor for creating or editing a custom "page" [UnifiedObject]
/// and managing its child "section" objects.
class PageEditor extends ConsumerStatefulWidget {
  final String? pageId;
  final VoidCallback onClose;

  const PageEditor({super.key, this.pageId, required this.onClose});

  @override
  ConsumerState<PageEditor> createState() => _PageEditorState();
}

class _PageEditorState extends ConsumerState<PageEditor> {
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
      builder: (ctx) => const SectionDialog(),
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
      builder: (ctx) => SectionDialog(
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
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.dialogDeleteSection),
        content: Text(l10n.dialogDeleteSectionConfirm),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(l10n.commonCancel)),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: Text(l10n.commonDelete),
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
    final l10n = AppLocalizations.of(context);
    final page = _existingPage;

    return Scaffold(
      backgroundColor: theme.colorScheme.surface,
      appBar: AppBar(
        title: Text(page != null ? l10n.pageEditorEditPage : l10n.pageEditorNewPage),
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
              child: Text(l10n.commonSave),
            ),
        ],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Page title & icon
            Row(
              children: [
                IconPicker(
                  iconName: _iconName,
                  onChanged: (v) => setState(() => _iconName = v),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: TextField(
                    controller: _titleController,
                    style: theme.textTheme.headlineSmall,
                    decoration: InputDecoration(
                      hintText: l10n.pageEditorPageTitleHint,
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
                Text(l10n.homePageEditorSections, style: theme.textTheme.titleLarge),
                FilledButton.icon(
                  onPressed: page != null ? _addSection : null,
                  icon: const Icon(Icons.add, size: 18),
                  label: Text(l10n.workspaceAddSectionButton),
                ),
              ],
            ),
            if (page == null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  l10n.pageEditorSaveFirst,
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
                        l10n.pageEditorNoSections,
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

/// AlertDialog for adding or editing a section within a page.
class SectionDialog extends StatefulWidget {
  final String? initialTitle;
  final String? initialIcon;

  const SectionDialog({super.key, this.initialTitle, this.initialIcon});

  @override
  State<SectionDialog> createState() => _SectionDialogState();
}

class _SectionDialogState extends State<SectionDialog> {
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
    final l10n = AppLocalizations.of(context);

    return AlertDialog(
      title: Text(widget.initialTitle == null ? l10n.workspaceAddSectionButton : l10n.pageEditorEditSectionTitle),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            controller: _titleController,
            autofocus: true,
            decoration: InputDecoration(
              labelText: l10n.homePageEditorSectionTitle,
              hintText: l10n.pageEditorEnterSectionTitle,
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text(l10n.homePageEditorIcon, style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          IconPicker(
            iconName: _iconName,
            onChanged: (v) => setState(() => _iconName = v),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.commonCancel),
        ),
        FilledButton(
          onPressed: () {
            if (_titleController.text.trim().isEmpty) return;
            Navigator.pop(context, {
              'title': _titleController.text.trim(),
              'icon': _iconName,
            });
          },
          child: Text(l10n.commonSave),
        ),
      ],
    );
  }
}
