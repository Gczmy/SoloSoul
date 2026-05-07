import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/presentation/widgets/folder_picker_dialog.dart';
// ignore_for_file: deprecated_member_use

import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/scan/scan_config_provider.dart';

// =============================================================================
// Local Search Config Page
// =============================================================================

class LocalSearchConfigPage extends ConsumerStatefulWidget {
  const LocalSearchConfigPage({super.key});

  @override
  ConsumerState<LocalSearchConfigPage> createState() => _LocalSearchConfigPageState();
}

class _LocalSearchConfigPageState extends ConsumerState<LocalSearchConfigPage> {
  ScanConfigNotifier? _scanConfigNotifier;

  static const List<_ExtensionOption> _kExtensionOptions = [
    // Documents
    _ExtensionOption('.pdf', 'PDF', Icons.picture_as_pdf, defaultLimitMb: 5),
    _ExtensionOption('.docx', 'Word', Icons.description, defaultLimitMb: 1),
    _ExtensionOption('.xlsx', 'Excel', Icons.table_chart, defaultLimitMb: 1),
    _ExtensionOption('.csv', 'CSV', Icons.grid_on, defaultLimitMb: 1),
    _ExtensionOption('.json', 'JSON', Icons.data_object, defaultLimitMb: 1),
    _ExtensionOption('.txt', 'Text', Icons.text_snippet, defaultLimitMb: 1),
    _ExtensionOption('.md', 'Markdown', Icons.edit_note, defaultLimitMb: 1),
    // Images (with OCR)
    _ExtensionOption('.png', 'PNG', Icons.image, defaultLimitMb: 5),
    _ExtensionOption('.jpg', 'JPG', Icons.image, defaultLimitMb: 5),
    _ExtensionOption('.jpeg', 'JPEG', Icons.image, defaultLimitMb: 5),
    _ExtensionOption('.webp', 'WebP', Icons.image, defaultLimitMb: 5),
    _ExtensionOption('.bmp', 'BMP', Icons.image, defaultLimitMb: 5),
    _ExtensionOption('.tiff', 'TIFF', Icons.image, defaultLimitMb: 10),
  ];

  @override
  void initState() {
    super.initState();
    _scanConfigNotifier = ref.read(scanConfigProvider.notifier);
    // Initialize from persistent scan config on first visit
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await ref.read(localSearchProvider.notifier).initFromConfig();
    });
  }

  @override
  void dispose() {
    // Flush any pending auto-save before leaving the page so config is
    // guaranteed to be persisted to the Vault.
    _scanConfigNotifier?.flush();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final searchState = ref.watch(localSearchProvider);
    final notifier = ref.read(localSearchProvider.notifier);

    final useDefaultPaths = searchState.paths.isEmpty;
    final customPaths = searchState.paths;
    final selectedExtensions = searchState.extensions;
    final scanDepth = searchState.scanDepth;
    final sizeLimits = searchState.maxFileSizeByExtension;

    return Scaffold(
      appBar: const SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text('Local Search Import'),
        centerTitle: true,
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Scan Local Files',
              style: theme.textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Search your local files for personal information and import them into your Vault.',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 32),
            const _SectionTitle(icon: Icons.folder_outlined, title: 'Search Paths'),
            const SizedBox(height: 12),
            _SearchPathsSection(
              useDefaultPaths: useDefaultPaths,
              customPaths: customPaths,
              onUseDefaultPaths: () => notifier.setPaths([]),
              onUseCustomPaths: () {
                if (customPaths.isEmpty) {
                  _pickFolder();
                }
              },
              onRemovePath: (p) {
                final updated = [...customPaths]..remove(p);
                notifier.setPaths(updated);
              },
              onAddPath: _pickFolder,
            ),
            const SizedBox(height: 24),
            const _SectionTitle(icon: Icons.file_present_outlined, title: 'File Types'),
            const SizedBox(height: 8),
            Text(
              'Tap to select. Long press to adjust size limit.',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            _FileTypeFilterSection(
              options: _kExtensionOptions,
              selectedExtensions: selectedExtensions,
              sizeLimits: sizeLimits,
              onToggleExtension: (ext) {
                final updated = [...selectedExtensions];
                if (updated.contains(ext)) {
                  updated.remove(ext);
                } else {
                  updated.add(ext);
                }
                notifier.setExtensions(updated);
              },
              onShowSizeLimitDialog: _showSizeLimitDialog,
            ),
            const SizedBox(height: 24),
            const _SectionTitle(icon: Icons.tune_outlined, title: 'Scan Depth'),
            const SizedBox(height: 12),
            Card(
              child: Column(
                children: [
                  RadioListTile<String>(
                    title: const Text('Filename only'),
                    subtitle: const Text('Fastest — only check filenames'),
                    value: 'filename',
                    groupValue: scanDepth,
                    onChanged: (v) => notifier.setScanDepth(v!),
                  ),
                  RadioListTile<String>(
                    title: const Text('Filename + Content fingerprint'),
                    subtitle: const Text('Balanced — regex match on content'),
                    value: 'fingerprint',
                    groupValue: scanDepth,
                    onChanged: (v) => notifier.setScanDepth(v!),
                  ),
                  RadioListTile<String>(
                    title: const Text('Full text parsing'),
                    subtitle: const Text('Slowest — deep content analysis'),
                    value: 'full',
                    groupValue: scanDepth,
                    onChanged: (v) => notifier.setScanDepth(v!),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 24),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: theme.colorScheme.primaryContainer.withValues(alpha: 0.3),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.shield_outlined,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      'All scanning is done locally. No data leaves your device. '
                      'You will preview all results before importing.',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 32),
            _SearchActionButtons(
              isEnabled: selectedExtensions.isNotEmpty,
              onStartScan: _startScan,
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _pickFolder() async {
    String? result;

    // macOS/iOS 沙盒限制：纯 Dart 的 Directory.list() 无法访问用户目录，
    // 必须使用系统原生的 NSOpenPanel / UIDocumentPicker。
    if (Platform.isMacOS || Platform.isIOS) {
      result = await FilePicker.getDirectoryPath();
    } else {
      result = await showDialog<String>(
        context: context,
        builder: (ctx) => const FolderPickerDialog(),
      );
    }

    if (result == null) return; // User cancelled
    if (!mounted) return;
    final currentPaths = ref.read(localSearchProvider).paths;
    if (!currentPaths.contains(result)) {
      ref.read(localSearchProvider.notifier).setPaths([...currentPaths, result]);
    }
  }

  void _startScan() {
    // Config already synced to provider via setters; just navigate
    context.push(AppRoutes.localSearchProgress);
  }

  Future<void> _showSizeLimitDialog(String ext, String label, int currentMb) async {
    var value = currentMb.toDouble();
    final newValue = await showDialog<int>(
      context: context,
      builder: (ctx) {
        final theme = Theme.of(ctx);
        return StatefulBuilder(
          builder: (ctx, setState) {
            return AlertDialog(
              title: Text('$label size limit'),
              content: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Skip $label files larger than:',
                    style: theme.textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 16),
                  Text(
                    '${value.round()} MB',
                    style: theme.textTheme.headlineSmall?.copyWith(
                      fontWeight: FontWeight.bold,
                      color: theme.colorScheme.primary,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Slider(
                    value: value,
                    min: 1,
                    max: 50,
                    divisions: 49,
                    label: '${value.round()} MB',
                    onChanged: (v) => setState(() => value = v),
                  ),
                ],
              ),
              actions: [
                TextButton(
                  onPressed: () => Navigator.pop(ctx),
                  child: const Text('Cancel'),
                ),
                FilledButton(
                  onPressed: () => Navigator.pop(ctx, value.round()),
                  child: const Text('Save'),
                ),
              ],
            );
          },
        );
      },
    );
    if (newValue != null && mounted) {
      ref.read(localSearchProvider.notifier).setMaxFileSizeForExtension(ext, newValue);
    }
  }
}

class _SectionTitle extends StatelessWidget {
  final IconData icon;
  final String title;

  const _SectionTitle({required this.icon, required this.title});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        Icon(icon, size: 20, color: theme.colorScheme.primary),
        const SizedBox(width: 8),
        Text(
          title,
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
      ],
    );
  }
}

class _FileTypeButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final int sizeLimitMb;
  final bool selected;
  final VoidCallback onTap;
  final VoidCallback? onLongPress;

  const _FileTypeButton({
    required this.icon,
    required this.label,
    required this.sizeLimitMb,
    required this.selected,
    required this.onTap,
    this.onLongPress,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final fgColor = selected
        ? theme.colorScheme.onPrimary
        : theme.colorScheme.onSurface;
    final limitColor = selected
        ? theme.colorScheme.onPrimary.withValues(alpha: 0.7)
        : theme.colorScheme.onSurfaceVariant;

    return Material(
      color: selected ? theme.colorScheme.primary : theme.colorScheme.surface,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        onTap: onTap,
        onLongPress: onLongPress,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: selected
                  ? theme.colorScheme.primary
                  : theme.colorScheme.outlineVariant,
              width: 1,
            ),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(icon, size: 18, color: fgColor),
                  const SizedBox(width: 6),
                  Text(
                    label,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: fgColor,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 2),
              Text(
                '≤${sizeLimitMb}MB',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: limitColor,
                  fontSize: 10,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ExtensionOption {
  final String ext;
  final String label;
  final IconData icon;
  final int defaultLimitMb;

  const _ExtensionOption(this.ext, this.label, this.icon, {this.defaultLimitMb = 1});
}

// =============================================================================
// Extracted widgets
// =============================================================================

class _SearchPathsSection extends StatelessWidget {
  final bool useDefaultPaths;
  final List<String> customPaths;
  final VoidCallback onUseDefaultPaths;
  final VoidCallback onUseCustomPaths;
  final ValueChanged<String> onRemovePath;
  final VoidCallback onAddPath;

  const _SearchPathsSection({
    required this.useDefaultPaths,
    required this.customPaths,
    required this.onUseDefaultPaths,
    required this.onUseCustomPaths,
    required this.onRemovePath,
    required this.onAddPath,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Column(
        children: [
          RadioListTile<bool>(
            title: const Text('Use default paths'),
            subtitle: const Text('Documents, Desktop, Downloads'),
            value: true,
            groupValue: useDefaultPaths,
            onChanged: (v) {
              if (v == true) onUseDefaultPaths();
            },
          ),
          RadioListTile<bool>(
            title: const Text('Custom paths'),
            subtitle: const Text('Select specific folders'),
            value: false,
            groupValue: useDefaultPaths,
            onChanged: (v) {
              if (v == false) onUseCustomPaths();
            },
          ),
          if (!useDefaultPaths) ...[
            const Divider(height: 1),
            ...customPaths.map((p) => ListTile(
                  dense: true,
                  leading: const Icon(Icons.folder),
                  title: Text(
                    p,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  trailing: IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    onPressed: () => onRemovePath(p),
                  ),
                )),
            ListTile(
              leading: const Icon(Icons.add),
              title: const Text('Add folder'),
              onTap: onAddPath,
            ),
          ],
        ],
      ),
    );
  }
}

class _FileTypeFilterSection extends StatelessWidget {
  final List<_ExtensionOption> options;
  final List<String> selectedExtensions;
  final Map<String, int> sizeLimits;
  final ValueChanged<String> onToggleExtension;
  final void Function(String ext, String label, int currentMb) onShowSizeLimitDialog;

  const _FileTypeFilterSection({
    required this.options,
    required this.selectedExtensions,
    required this.sizeLimits,
    required this.onToggleExtension,
    required this.onShowSizeLimitDialog,
  });

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 10,
      runSpacing: 10,
      children: options.map((opt) {
        final selected = selectedExtensions.contains(opt.ext);
        final limit = sizeLimits[opt.ext] ?? opt.defaultLimitMb;
        return _FileTypeButton(
          icon: opt.icon,
          label: opt.label,
          sizeLimitMb: limit,
          selected: selected,
          onTap: () => onToggleExtension(opt.ext),
          onLongPress: () => onShowSizeLimitDialog(opt.ext, opt.label, limit),
        );
      }).toList(),
    );
  }
}

class _SearchActionButtons extends StatelessWidget {
  final bool isEnabled;
  final VoidCallback onStartScan;

  const _SearchActionButtons({
    required this.isEnabled,
    required this.onStartScan,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      height: 48,
      child: FilledButton.icon(
        onPressed: isEnabled ? onStartScan : null,
        icon: const Icon(Icons.search),
        label: const Text('Start Scan'),
      ),
    );
  }
}
