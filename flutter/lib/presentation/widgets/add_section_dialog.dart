import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/home/icon_picker.dart';

/// Dialog for creating a custom section — name + icon picker.
///
/// Structurally identical to [SectionDialog] in `page_editor.dart` so a
/// future merge is a trivial import swap.
///
/// Returns `{'title': ..., 'icon': ...}` via [Navigator.pop] on save,
/// or `null` on cancel.
class AddSectionDialog extends StatefulWidget {
  const AddSectionDialog({super.key});

  @override
  State<AddSectionDialog> createState() => _AddSectionDialogState();
}

class _AddSectionDialogState extends State<AddSectionDialog> {
  late final TextEditingController _titleController;
  String _iconName = 'folder';

  @override
  void initState() {
    super.initState();
    _titleController = TextEditingController();
  }

  @override
  void dispose() {
    _titleController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return AlertDialog(
      title: Text(l10n.workspaceAddSectionDialog),
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
            final title = _titleController.text.trim();
            if (title.isEmpty) return;
            Navigator.pop(context, {
              'title': title,
              'icon': _iconName,
            });
          },
          child: Text(l10n.workspaceAddSectionButton),
        ),
      ],
    );
  }
}
