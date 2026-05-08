import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';

// =============================================================================
// Scan Progress Banner
// =============================================================================

/// A compact top banner that appears when a local filesystem scan is running
/// in the background. Shows live progress stats and allows the user to stop
/// the scan or navigate back to the progress page.
class ScanProgressBanner extends ConsumerWidget {
  const ScanProgressBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Local import is a debug-only feature for now.
    if (!kDebugMode) return const SizedBox.shrink();

    final l10n = AppLocalizations.of(context);
    final state = ref.watch(localSearchProvider);
    final theme = Theme.of(context);
    final isScanning = state.isScanning;

    return AnimatedContainer(
      duration: const Duration(milliseconds: 300),
      curve: Curves.easeInOut,
      height: isScanning ? 56 : 0,
      clipBehavior: Clip.hardEdge,
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer,
      ),
      child: isScanning
          ? Material(
              color: Colors.transparent,
              child: InkWell(
                onTap: () => context.go(AppRoutes.localSearchProgress),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Row(
                    children: [
                      // Animated progress indicator
                      SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2.5,
                          valueColor: AlwaysStoppedAnimation<Color>(
                            theme.colorScheme.primary,
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),

                      // Scanning label
                      Text(
                        'Scanning',
                        style: theme.textTheme.labelLarge?.copyWith(
                          fontWeight: FontWeight.w600,
                          color: theme.colorScheme.onPrimaryContainer,
                        ),
                      ),
                      const SizedBox(width: 16),

                      // Stats
                      Expanded(
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            _Stat(label: l10n.localSearchScanned, value: state.scannedCount),
                            const SizedBox(width: 16),
                            _Stat(label: l10n.localSearchFound, value: state.foundCount),
                            const SizedBox(width: 16),
                            _Stat(label: l10n.localSearchSkipped, value: state.skippedFiles.length),
                          ],
                        ),
                      ),

                      // Stop button
                      IconButton(
                        icon: Icon(
                          Icons.stop_circle_outlined,
                          color: theme.colorScheme.error,
                        ),
                        tooltip: l10n.scanStopScan,
                        onPressed: () {
                          ref.read(localSearchProvider.notifier).cancelScan();
                        },
                      ),
                    ],
                  ),
                ),
              ),
            )
          : const SizedBox.shrink(),
    );
  }
}

class _Stat extends StatelessWidget {
  final String label;
  final int value;

  const _Stat({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          '$value',
          style: theme.textTheme.labelLarge?.copyWith(
            fontWeight: FontWeight.bold,
            color: theme.colorScheme.onPrimaryContainer,
          ),
        ),
        const SizedBox(width: 2),
        Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onPrimaryContainer.withValues(alpha: 0.7),
          ),
        ),
      ],
    );
  }
}
