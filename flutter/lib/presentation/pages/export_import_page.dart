import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';

import 'package:solosoul_flutter/core/services/export_import_models.dart';
import 'package:solosoul_flutter/core/services/export_import_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/export_import/import_preview_dialog.dart';

// =============================================================================
// Export / Import Page
// =============================================================================

class ExportImportPage extends ConsumerStatefulWidget {
  const ExportImportPage({super.key});

  @override
  ConsumerState<ExportImportPage> createState() => _ExportImportPageState();
}

class _ExportImportPageState extends ConsumerState<ExportImportPage>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(l10n.exportImportTitle),
        backRoute: '/settings/data-management',
        bottom: TabBar(
          controller: _tabController,
          tabs: [
            Tab(text: l10n.exportButton),
            Tab(text: l10n.importButton),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: const [
          _ExportSection(),
          _ImportSection(),
        ],
      ),
    );
  }
}

// =============================================================================
// Export Section
// =============================================================================

class _ExportSection extends ConsumerStatefulWidget {
  const _ExportSection();

  @override
  ConsumerState<_ExportSection> createState() => _ExportSectionState();
}

class _ExportSectionState extends ConsumerState<_ExportSection> {
  bool _isExporting = false;
  double _progress = 0.0;

  Future<void> _onExport() async {
    final l10n = AppLocalizations.of(context);
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final accountId = authNotifier.selectedAccountId;
    final account = authNotifier.selectedAccount;

    if (accountId == null || account == null) return;

    // Verify password
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      message: l10n.exportPasswordPrompt,
      passwordHint: account.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null || !mounted) return;

    // Pick save location
    final fileName = '${account.name}_$accountId.solosoul';
    final result = await FilePicker.saveFile(
      dialogTitle: l10n.exportFilePickerTitle,
      fileName: fileName,
      allowedExtensions: ['solosoul'],
      type: FileType.custom,
    );
    if (result == null || !mounted) return;

    setState(() {
      _isExporting = true;
      _progress = 0.0;
    });

    final savePath = result;
    final exported = await ExportImportService.instance.exportPackage(
      accountId: accountId,
      password: password,
      passwordHint: account.passwordHint,
      savePath: savePath,
    );

    if (!mounted) return;
    setState(() {
      _isExporting = false;
      _progress = 1.0;
    });

    if (exported != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.exportSuccess)),
      );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.exportFailed)),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final account = authNotifier.selectedAccount;

    return SoloGlassLayer(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.exportDescription,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 16),
            if (account != null) ...[
              Text('${l10n.accountName}: ${account.name}'),
              Text('${l10n.accountId}: ${account.id}'),
            ],
            const SizedBox(height: 24),
            if (_isExporting) ...[
              LinearProgressIndicator(value: _progress > 0 ? _progress : null),
              const SizedBox(height: 8),
              Text(l10n.exportInProgress),
            ] else
              ElevatedButton.icon(
                onPressed: _onExport,
                icon: const Icon(Icons.download),
                label: Text(l10n.exportButton),
              ),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Import Section
// =============================================================================

class _ImportSection extends ConsumerStatefulWidget {
  const _ImportSection();

  @override
  ConsumerState<_ImportSection> createState() => _ImportSectionState();
}

class _ImportSectionState extends ConsumerState<_ImportSection> {
  bool _isParsing = false;
  bool _isImporting = false;
  String? _passwordHint;
  ImportPreview? _preview;

  Future<void> _onSelectFile() async {
    final l10n = AppLocalizations.of(context);

    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['solosoul'],
      dialogTitle: l10n.importFilePickerTitle,
    );
    if (result == null || result.files.single.path == null) return;

    final filePath = result.files.single.path!;

    setState(() => _isParsing = true);

    final preview = await ExportImportService.instance.parseImportPackage(filePath);

    if (!mounted) return;
    setState(() => _isParsing = false);

    if (preview == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.importParseFailed)),
      );
      return;
    }

    setState(() {
      _preview = preview;
    });
  }

  Future<void> _onVerifyPassword(String password) async {
    final l10n = AppLocalizations.of(context);
    if (_preview == null) return;

    setState(() => _isParsing = true);

    try {
      final profile = await ExportImportService.instance.decryptAndVerify(
        _preview!,
        password,
      );

      if (!mounted) return;
      setState(() => _isParsing = false);

      if (profile != null) {
        await _showPreviewDialog(profile);
      } else {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.importDecryptFailed)),
        );
      }
    } on WrongPasswordException {
      if (!mounted) return;
      setState(() => _isParsing = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.importWrongPassword)),
      );
    }
  }

  Future<void> _showPreviewDialog(ProfileData profile) async {
    final l10n = AppLocalizations.of(context);
    final collections = ExportImportService.instance.buildImportCollections(profile);

    final result = await showDialog<List<ImportCollection>>(
      context: context,
      builder: (context) => ImportPreviewDialog(
        collections: collections,
        currentPages: _getCurrentPages(),
      ),
    );

    if (result == null || !mounted) return;

    // Execute import
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final accountId = authNotifier.selectedAccountId;
    if (accountId == null) return;

    final currentProfile = await ProfileStorageService.instance.loadProfile(accountId);
    if (currentProfile == null || !mounted) return;

    setState(() => _isImporting = true);

    // TODO: pass exportKey and tempAttachmentsDir from preview
    // For now, this is a placeholder that will be wired in later.

    setState(() => _isImporting = false);

    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.importSuccess)),
    );
  }

  List<String> _getCurrentPages() {
    // TODO: fetch actual current pages from current profile
    return [];
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    return SoloGlassLayer(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.importDescription,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 16),
            ElevatedButton.icon(
              onPressed: _isParsing ? null : _onSelectFile,
              icon: const Icon(Icons.upload_file),
              label: Text(_isParsing ? l10n.commonLoading : l10n.importSelectFile),
            ),
            if (_preview != null) ...[
              const SizedBox(height: 24),
              Text('${l10n.importObjectCount}: ${_preview!.manifest.objectCount}'),
              Text('${l10n.importAttachmentCount}: ${_preview!.manifest.attachmentCount}'),
              const SizedBox(height: 16),
              if (_passwordHint != null)
                Text('${l10n.importPasswordHint}: $_passwordHint'),
              const SizedBox(height: 8),
              SoloGlassTextField(
                placeholder: l10n.importPasswordHint,
                obscureText: true,
                onSubmitted: _onVerifyPassword,
              ),
              const SizedBox(height: 8),
              ElevatedButton(
                onPressed: _isParsing ? null : () {
                  // Trigger password verification
                },
                child: Text(_isParsing ? l10n.commonLoading : l10n.importPreviewButton),
              ),
            ],
            if (_isImporting) ...[
              const SizedBox(height: 16),
              const LinearProgressIndicator(),
              Text(l10n.importInProgress),
            ],
          ],
        ),
      ),
    );
  }
}
