import 'dart:io';

import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

// =============================================================================
// Folder Picker Dialog
// =============================================================================

/// A native-feeling folder picker dialog implemented in pure Dart.
/// Uses dart:io Directory API — no platform plugins required.
class FolderPickerDialog extends StatefulWidget {
  final String? initialPath;

  const FolderPickerDialog({super.key, this.initialPath});

  @override
  State<FolderPickerDialog> createState() => _FolderPickerDialogState();
}

class _FolderPickerDialogState extends State<FolderPickerDialog> {
  late String _currentPath;
  List<FileSystemEntity> _entries = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _currentPath = widget.initialPath ?? _homeDirectory();
    _loadDirectory(_currentPath);
  }

  String _homeDirectory() {
    return Platform.environment['HOME'] ??
        Platform.environment['USERPROFILE'] ??
        '/';
  }

  Future<void> _loadDirectory(String path) async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final dir = Directory(path);
      if (!await dir.exists()) {
        setState(() {
          _error = 'Directory does not exist';
          _loading = false;
        });
        return;
      }

      final list = await dir
          .list()
          .where((e) => e is Directory)
          .toList();

      list.sort((a, b) {
        final nameA = a.path.split(Platform.pathSeparator).last;
        final nameB = b.path.split(Platform.pathSeparator).last;
        return nameA.toLowerCase().compareTo(nameB.toLowerCase());
      });

      setState(() {
        _currentPath = path;
        _entries = list;
        _loading = false;
      });
    } on Exception catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  void _goUp() {
    final separator = Platform.pathSeparator;
    final parent = _currentPath.endsWith(separator)
        ? _currentPath.substring(0, _currentPath.length - 1)
        : _currentPath;
    final idx = parent.lastIndexOf(separator);
    if (idx > 0) {
      _loadDirectory(parent.substring(0, idx));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: Container(
        width: 560,
        height: 480,
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(Icons.folder_open, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text(
                  'Select Folder',
                  style: theme.textTheme.titleLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Current path bar
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.arrow_upward, size: 18),
                    tooltip: l10n.folderPickerGoUp,
                    onPressed: _goUp,
                  ),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      _currentPath,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontFamily: 'monospace',
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 8),

            // Folder list
            Expanded(
              child: _FolderPickerContent(
                theme: theme,
                loading: _loading,
                error: _error,
                entries: _entries,
                onLoadDirectory: _loadDirectory,
              ),
            ),

            const SizedBox(height: 12),
            const Divider(height: 1),
            const SizedBox(height: 12),

            // Actions
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: Text(l10n.commonCancel),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: () => Navigator.of(context).pop(_currentPath),
                  child: Text(l10n.dialogSelectFolder),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }


}

class _FolderPickerContent extends StatelessWidget {
  final ThemeData theme;
  final bool loading;
  final String? error;
  final List<FileSystemEntity> entries;
  final ValueChanged<String> onLoadDirectory;

  const _FolderPickerContent({
    required this.theme,
    required this.loading,
    required this.error,
    required this.entries,
    required this.onLoadDirectory,
  });

  @override
  Widget build(BuildContext context) {
    if (loading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline,
                size: 48, color: theme.colorScheme.error),
            const SizedBox(height: 12),
            Text(error!, textAlign: TextAlign.center),
          ],
        ),
      );
    }

    if (entries.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.folder_open,
                size: 48, color: theme.colorScheme.onSurfaceVariant),
            const SizedBox(height: 12),
            Text(
              'No subfolders',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      itemCount: entries.length,
      itemBuilder: (context, index) {
        final dir = entries[index] as Directory;
        final name = dir.path.split(Platform.pathSeparator).last;

        return ListTile(
          dense: true,
          leading: Icon(
            Icons.folder,
            color: theme.colorScheme.primary.withValues(alpha: 0.8),
          ),
          title: Text(name),
          trailing: const Icon(Icons.chevron_right, size: 18),
          onTap: () => onLoadDirectory(dir.path),
        );
      },
    );
  }
}
