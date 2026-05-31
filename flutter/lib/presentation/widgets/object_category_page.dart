import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/add_section_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/dynamic_section_card.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

/// Generic page scaffold for object category pages (Profile, Travel, Financial, Professional).
///
/// Renders all child sections of the given [pageId] dynamically using
/// [DynamicSectionCard]. Preset sections keep their rich rendering;
/// custom sections fall back to the generic [ObjectCard].
///
/// When [pageId] is provided, a "+" button is shown in the AppBar for adding
/// custom sections.
class ObjectCategoryPage extends ConsumerWidget {
  final String title;

  /// The page ID (e.g. [DefaultPageIds.profile]). When non-null,
  /// enables dynamic section rendering and custom section creation.
  final String? pageId;

  /// Optional widgets shown at the top of the page content (before sections).
  /// Typically used for [ScanDocumentButton] or page-level actions.
  final List<Widget>? headerWidgets;

  const ObjectCategoryPage({
    super.key,
    required this.title,
    this.pageId,
    this.headerWidgets,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pid = pageId;
    final sections = pid != null
        ? ref
            .watch(childrenProvider(pid))
            .where((o) => o.typeId != 'page' && !o.isDeleted)
            .toList()
        : <UnifiedObject>[];

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(title),
        backRoute: AppRoutes.home,
        actions: [
          if (pageId != null)
            IconButton(
              icon: const Icon(Icons.add),
              tooltip: AppLocalizations.of(context).workspaceAddSection,
              onPressed: () => _addCustomSection(context, ref),
            ),
          const HeaderActionButtons(),
        ],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ...?headerWidgets,
            for (final section in sections)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: DynamicSectionCard(section: section),
              ),
            if (pid != null && sections.isEmpty)
              _RestoreDefaultsWidget(pageId: pid),
          ],
        ),
      ),
    );
  }

  Future<void> _addCustomSection(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final result = await showDialog<Map<String, String>>(
      context: context,
      builder: (_) => const AddSectionDialog(),
    );
    if (result == null || !context.mounted) return;

    await ref.read(unifiedObjectProvider.notifier).createObject(
          name: result['title']!,
          typeId: 'collection',
          parentId: pageId,
          iconName: result['icon']!,
        );
  }
}

/// Button shown when a page has no sections, allowing the user to
/// restore the original default sections.
class _RestoreDefaultsWidget extends ConsumerWidget {
  final String pageId;

  const _RestoreDefaultsWidget({required this.pageId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.folder_open_outlined,
              size: 48,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 12),
            Text(
              l10n.pageNoSections,
              style: theme.textTheme.bodyLarge?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: () async {
                await ref
                    .read(unifiedObjectProvider.notifier)
                    .createDefaultSectionsForPage(pageId);
              },
              icon: const Icon(Icons.restore),
              label: Text(l10n.pageRestoreDefaults),
            ),
          ],
        ),
      ),
    );
  }
}
