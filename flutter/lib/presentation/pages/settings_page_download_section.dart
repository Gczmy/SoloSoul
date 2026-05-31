part of 'settings_page.dart';

/// Download settings section for configuring the attachment download directory.
class _DownloadSettingsSection extends ConsumerStatefulWidget {
  const _DownloadSettingsSection();

  @override
  ConsumerState<_DownloadSettingsSection> createState() =>
      _DownloadSettingsSectionState();
}

class _DownloadSettingsSectionState
    extends ConsumerState<_DownloadSettingsSection> {
  String? _displayPath;

  @override
  void initState() {
    super.initState();
    _loadPath();
  }

  Future<void> _loadPath() async {
    final service = AttachmentDownloadService();
    final customPath = await service.getCustomDownloadPath();
    if (customPath != null) {
      if (mounted) setState(() => _displayPath = customPath);
      return;
    }
    final defaultDir = await service.getDefaultDownloadDirectory();
    if (mounted) setState(() => _displayPath = defaultDir.path);
  }

  Future<void> _chooseDirectory() async {
    final l10n = AppLocalizations.of(context);
    final selected = await FilePicker.getDirectoryPath(
      dialogTitle: l10n.chooseFolder,
    );
    if (selected == null || selected.isEmpty) return;

    // Verify the directory is writable
    final testFile = File('$selected/.solosoul_write_test');
    try {
      await testFile.writeAsString('test');
      await testFile.delete();
    } on Exception {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: l10n.folderNotWritable,
          type: SnackBarType.warning,
        );
      }
      return;
    }

    await AttachmentDownloadService().setDownloadDirectory(selected);
    if (mounted) {
      setState(() => _displayPath = selected);
      showOverlaySnackBar(
        context,
        content: l10n.downloadLocationUpdated,
        type: SnackBarType.success,
      );
    }
  }

  Future<void> _resetToDefault() async {
    final l10n = AppLocalizations.of(context);
    await AttachmentDownloadService().clearDownloadDirectory();
    final defaultDir = await AttachmentDownloadService().getDefaultDownloadDirectory();
    if (mounted) {
      setState(() => _displayPath = defaultDir.path);
      showOverlaySnackBar(
        context,
        content: l10n.downloadLocationReset,
        type: SnackBarType.success,
      );
    }
  }

  String _shortenPath(String path) {
    if (path.startsWith('/Users/')) {
      final parts = path.split('/');
      if (parts.length >= 3) {
        return '~/${parts.sublist(3).join('/')}';
      }
    }
    if (path.length > 50) {
      return '...${path.substring(path.length - 47)}';
    }
    return path;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    return SectionCard(
      title: l10n.downloadLocation,
      icon: Icons.download_outlined,
      children: [
        SettingsTile(
          icon: Icons.folder_outlined,
          title: l10n.downloadLocation,
          subtitle: _displayPath == null
              ? l10n.downloadLocationDesc
              : _shortenPath(_displayPath!),
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextButton(
                onPressed: _chooseDirectory,
                child: Text(l10n.chooseFolder),
              ),
              if (_displayPath != null)
                IconButton(
                  icon: const Icon(Icons.restore, size: 18),
                  tooltip: l10n.resetToDefault,
                  onPressed: _resetToDefault,
                  visualDensity: VisualDensity.compact,
                ),
            ],
          ),
        ),
      ],
    ).animate().fadeIn(delay: 250.ms, duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
