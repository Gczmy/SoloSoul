import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';

// =============================================================================
// Local Search Progress Page
// =============================================================================

class LocalSearchProgressPage extends ConsumerStatefulWidget {
  const LocalSearchProgressPage({super.key});

  @override
  ConsumerState<LocalSearchProgressPage> createState() => _LocalSearchProgressPageState();
}

class _LocalSearchProgressPageState extends ConsumerState<LocalSearchProgressPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(localSearchProvider.notifier).startScan();
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final state = ref.watch(localSearchProvider);
    final theme = Theme.of(context);

    // Auto-navigate to preview when scan completes (not canceled)
    ref.listen(localSearchProvider, (previous, next) {
      if (previous?.isScanning == true &&
          next.isScanning == false &&
          !next.wasCanceled &&
          next.scanError == null) {
        if (next.scanResults.isNotEmpty) {
          context.pushReplacement(AppRoutes.scanPreview);
        } else {
          _showNoResultsDialog();
        }
      }
    });

    // Determine status text
    final String statusText;
    final IconData statusIcon;
    final Color statusColor;
    if (state.isScanning) {
      statusText = 'Scanning files...';
      statusIcon = Icons.sync;
      statusColor = theme.colorScheme.primary;
    } else if (state.wasCanceled) {
      statusText = 'Scan canceled';
      statusIcon = Icons.cancel_outlined;
      statusColor = theme.colorScheme.primary;
    } else {
      statusText = 'Scan complete';
      statusIcon = Icons.check_circle_outline;
      statusColor = theme.colorScheme.primary;
    }

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(l10n.localSearchScanning),
        centerTitle: true,
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () {
            ref.read(localSearchProvider.notifier).cancelScan();
            context.pop();
          },
        ),
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // Status icon
              SizedBox(
                width: 120,
                height: 120,
                child: state.isScanning
                    ? const CircularProgressIndicator(strokeWidth: 8)
                    : Icon(statusIcon, size: 80, color: statusColor),
              ),
              const SizedBox(height: 32),

              Text(
                statusText,
                style: theme.textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: statusColor,
                ),
              ),
              const SizedBox(height: 16),

              // Current path (only while scanning)
              if (state.isScanning && state.currentPath.isNotEmpty)
                Text(
                  state.currentPath.split('/').last,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              const SizedBox(height: 24),

              // Stats row
              Wrap(
                spacing: 12,
                runSpacing: 12,
                alignment: WrapAlignment.center,
                children: [
                  _StatButton(
                    label: l10n.localSearchScanned,
                    value: '${state.scannedCount}',
                    icon: Icons.folder_open,
                    files: state.scannedFiles,
                    color: theme.colorScheme.primary,
                  ),
                  _StatButton(
                    label: l10n.localSearchFound,
                    value: '${state.foundCount}',
                    icon: Icons.find_in_page,
                    files: state.foundFiles,
                    color: theme.colorScheme.tertiary,
                  ),
                  _StatButton(
                    label: l10n.localSearchSkipped,
                    value: '${state.skippedFiles.length}',
                    icon: Icons.skip_next,
                    files: state.skippedFiles,
                    color: theme.colorScheme.outline,
                  ),
                ],
              ),
              const SizedBox(height: 32),

              // Error display
              if (state.scanError != null)
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.errorContainer,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.error_outline, color: theme.colorScheme.error),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          state.scanError!,
                          style: TextStyle(color: theme.colorScheme.onErrorContainer),
                        ),
                      ),
                    ],
                  ),
                ),

              const SizedBox(height: 32),

              // Action buttons
              if (state.isScanning)
                OutlinedButton.icon(
                  onPressed: () {
                    ref.read(localSearchProvider.notifier).cancelScan();
                  },
                  icon: const Icon(Icons.stop),
                  label: Text(l10n.localSearchCancelScan),
                )
              else if (state.wasCanceled)
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    OutlinedButton.icon(
                      onPressed: () => context.pop(),
                      icon: const Icon(Icons.arrow_back),
                      label: Text(l10n.localSearchGoBack),
                    ),
                    const SizedBox(width: 16),
                    FilledButton.icon(
                      onPressed: () {
                        ref.read(localSearchProvider.notifier).startScan();
                      },
                      icon: const Icon(Icons.refresh),
                      label: Text(l10n.localSearchScanAgain),
                    ),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }

  void _showNoResultsDialog() {
    final l10n = AppLocalizations.of(context);
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.localSearchNoResults),
        content: const Text(
          'No personal information was found in the scanned files. '
          'Try using "Full text parsing" mode or adding more folders.',
        ),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              context.pop();
            },
            child: Text(l10n.settingsOk),
          ),
        ],
      ),
    );
  }
}

// =============================================================================
// Stat Button (clickable card)
// =============================================================================

class _StatButton extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;
  final List<String> files;
  final Color color;

  const _StatButton({
    required this.label,
    required this.value,
    required this.icon,
    required this.files,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      color: color.withValues(alpha: 0.08),
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        onTap: files.isEmpty ? null : () => _showFileList(context),
        borderRadius: BorderRadius.circular(12),
        child: Container(
          width: 100,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 16),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: color.withValues(alpha: 0.3)),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, color: color, size: 24),
              const SizedBox(height: 8),
              Text(
                value,
                style: theme.textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: color,
                ),
              ),
              Text(
                label,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showFileList(BuildContext context) {
    final theme = Theme.of(context);

    showDialog(
      context: context,
      builder: (ctx) => Dialog(
        insetPadding: const EdgeInsets.all(24),
        child: Container(
          width: 520,
          height: 480,
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(icon, color: color),
                  const SizedBox(width: 8),
                  Text(
                    '$label (${files.length})',
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.of(ctx).pop(),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              const Divider(height: 1),
              Expanded(
                child: files.isEmpty
                    ? Center(
                        child: Text(
                          'No files',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      )
                    : ListView.builder(
                        itemCount: files.length,
                        itemBuilder: (context, index) {
                          final path = files[index];
                          final name = path.split('/').last;
                          return ListTile(
                            dense: true,
                            leading: Icon(
                              Icons.insert_drive_file_outlined,
                              size: 18,
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                            title: Text(
                              name,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            subtitle: Text(
                              path,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                                fontFamily: 'monospace',
                              ),
                            ),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
