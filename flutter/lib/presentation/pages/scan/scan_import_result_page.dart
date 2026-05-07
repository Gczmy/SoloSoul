import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';

// =============================================================================
// Scan Import Result Page
// =============================================================================

class ScanImportResultPage extends ConsumerWidget {
  const ScanImportResultPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(localSearchProvider);
    final result = state.importResult;
    final theme = Theme.of(context);

    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.home,
        title: Text(l10n.scanImportComplete),
        centerTitle: true,
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.check_circle_outline,
                size: 80,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Import Successful',
                style: theme.textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Your scanned data has been imported into Vault.',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 32),

              if (result != null) ...[
                // Stats grid
                _ResultGrid(result: result),
                const SizedBox(height: 32),

                // Warnings
                if (result.warnings.isNotEmpty) ...[
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.errorContainer.withValues(alpha: 0.5),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(Icons.warning_amber, color: theme.colorScheme.error),
                            const SizedBox(width: 8),
                            Text(
                              'Warnings',
                              style: theme.textTheme.titleSmall?.copyWith(
                                color: theme.colorScheme.error,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        ...result.warnings.map((w) => Padding(
                              padding: const EdgeInsets.only(left: 32, top: 4),
                              child: Text(
                                '• $w',
                                style: theme.textTheme.bodySmall,
                              ),
                            )),
                      ],
                    ),
                  ),
                  const SizedBox(height: 32),
                ],
              ],

              // Actions
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  FilledButton.icon(
                    onPressed: () {
                      ref.read(localSearchProvider.notifier).reset();
                      context.go(AppRoutes.home);
                    },
                    icon: const Icon(Icons.home),
                    label: Text(l10n.scanImportGoHome),
                  ),
                  const SizedBox(width: 16),
                  OutlinedButton.icon(
                    onPressed: () {
                      ref.read(localSearchProvider.notifier).reset();
                      context.pop();
                    },
                    icon: const Icon(Icons.close),
                    label: Text(l10n.commonClose),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ResultGrid extends StatelessWidget {
  final dynamic result;

  const _ResultGrid({required this.result});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    final items = [
      _GridItem(
        label: l10n.scanImportCreated,
        value: '${result.itemsCreated}',
        icon: Icons.add_circle_outline,
        color: theme.colorScheme.primary,
      ),
      _GridItem(
        label: l10n.scanImportUpdated,
        value: '${result.itemsUpdated}',
        icon: Icons.update,
        color: theme.colorScheme.secondary,
      ),
      _GridItem(
        label: l10n.scanImportFields,
        value: '${result.fieldsWritten}',
        icon: Icons.check_circle_outline,
        color: theme.colorScheme.tertiary,
      ),
      _GridItem(
        label: l10n.scanImportSkipped,
        value: '${result.fieldsSkipped}',
        icon: Icons.skip_next,
        color: theme.colorScheme.outline,
      ),
    ];

    return Wrap(
      spacing: 12,
      runSpacing: 12,
      alignment: WrapAlignment.center,
      children: items.map((item) => _ScanImportCardWidget(item: item, theme: theme)).toList(),
    );
  }


}

class _GridItem {
  final String label;
  final String value;
  final IconData icon;
  final Color color;

  _GridItem({
    required this.label,
    required this.value,
    required this.icon,
    required this.color,
  });
}

class _ScanImportCardWidget extends StatelessWidget {
  final _GridItem item;
  final ThemeData theme;

  const _ScanImportCardWidget({required this.item, required this.theme});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 120,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: item.color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: item.color.withValues(alpha: 0.2)),
      ),
      child: Column(
        children: [
          Icon(item.icon, color: item.color, size: 28),
          const SizedBox(height: 8),
          Text(
            item.value,
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
              color: item.color,
            ),
          ),
          Text(
            item.label,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}
