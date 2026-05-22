import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/add_section_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/custom_sections_widget.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

/// Generic page scaffold for object category pages (Profile, Travel, Financial, Professional).
///
/// Eliminates the boilerplate Scaffold -> AppBar -> SingleChildScrollView -> Column
/// duplication across category pages that only differ in their section content.
///
/// When [pageId] is provided, a "+" button is shown in the AppBar for adding
/// custom sections, and [CustomSectionsWidget] is appended after [sections].
class ObjectCategoryPage extends ConsumerWidget {
  final String title;
  final List<Widget> sections;

  /// The default page ID (e.g. [DefaultPageIds.profile]). When non-null,
  /// enables custom section creation and rendering.
  final String? pageId;

  /// Fixed default section IDs to exclude from the custom sections list.
  final List<String> defaultSectionIds;

  const ObjectCategoryPage({
    super.key,
    required this.title,
    required this.sections,
    this.pageId,
    this.defaultSectionIds = const [],
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allChildren = pageId != null
        ? ref.watch(childrenProvider(pageId!))
        : <UnifiedObject>[];
    final hasCollections = allChildren
        .any((o) => o.typeId == 'collection' && !o.isDeleted);

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
            ...sections,
            if (pageId != null)
              CustomSectionsWidget(
                pageId: pageId!,
                defaultSectionIds: defaultSectionIds,
              ),
            if (pageId != null && !hasCollections)
              _RestoreDefaultsWidget(pageId: pageId!),
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

/// Button shown when a default page has no sections, allowing the user to
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
