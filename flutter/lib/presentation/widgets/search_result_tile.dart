import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/models/search_models.dart' show SearchResultItem;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Search result tile widget displaying a single search result with sensitivity handling.
class SearchResultTile extends ConsumerWidget {
  final SearchResultItem result;
  final VoidCallback onReveal;

  const SearchResultTile({
    super.key,
    required this.result,
    required this.onReveal,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Watch revealed fields and sensitive access so rebuild happens when fields are revealed
    final revealedFields = ref.watch(
      accountStyleProvider.select((s) => s.value?.revealedFields ?? const {}),
    );
    final isSensitiveGranted = ref.watch(isSensitiveAccessGrantedProvider);
    final isRevealed = revealedFields.contains(result.fieldPath) &&
        (result.sensitivityLevel != SensitivityLevel.critical || isSensitiveGranted);
    final showMasked =
        result.sensitivityLevel != SensitivityLevel.public && !isRevealed;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    result.fieldName,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                SensitivityTag(level: result.sensitivityLevel),
                if (result.isDeleted) ...[
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.grey.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      l10n.searchDeleted,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: Colors.grey,
                      ),
                    ),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: Text(
                    showMasked ? '••••••••' : result.value,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: showMasked
                          ? theme.colorScheme.outline
                          : theme.colorScheme.onSurface,
                      fontFamily: showMasked ? null : 'monospace',
                    ),
                  ),
                ),
                if (showMasked)
                  TextButton.icon(
                    icon: const Icon(Icons.visibility_off, size: 16),
                    label: Text(l10n.searchReveal),
                    onPressed: onReveal,
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                  ),
              ],
            ),
            if (showMasked)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Row(
                  children: [
                    Icon(
                      Icons.info_outline,
                      size: 14,
                      color: theme.colorScheme.outline,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      result.sensitivityLevel == SensitivityLevel.critical
                          ? l10n.searchRestrictedHint
                          : l10n.searchPrivateHint,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.outline,
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}