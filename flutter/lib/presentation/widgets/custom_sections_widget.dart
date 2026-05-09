import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card.dart';

/// Renders custom sections (typeId: 'collection', non-default IDs) as
/// [ObjectCard] widgets at the bottom of a default page.
///
/// Section deletion is guarded by [ObjectCard]'s built-in confirmation
/// dialog, and the resulting SnackBar offers undo via [restoreObject].
class CustomSectionsWidget extends ConsumerWidget {
  final String pageId;
  final List<String> defaultSectionIds;

  /// Reserved for future repositioning (e.g. move-to-top, drag-to-reorder).
  /// When non-null, sections are sorted accordingly; otherwise appended.
  final int? sortOrder;

  const CustomSectionsWidget({
    super.key,
    required this.pageId,
    required this.defaultSectionIds,
    this.sortOrder,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final allChildren = ref.watch(childrenProvider(pageId));
    final customSections = allChildren
        .where(
          (o) =>
              o.typeId == 'collection' &&
              !defaultSectionIds.contains(o.id),
        )
        .toList();

    if (customSections.isEmpty) return const SizedBox.shrink();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final section in customSections)
          Padding(
            padding: const EdgeInsets.only(top: 16),
            child: _SectionCard(section: section),
          ),
      ],
    );
  }
}

/// Wraps a single custom section in an [ObjectCard], providing its child
/// items from the unified object tree.
class _SectionCard extends ConsumerWidget {
  final UnifiedObject section;

  const _SectionCard({required this.section});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(childrenProvider(section.id));

    return Container(
      margin: const EdgeInsets.only(bottom: 4),
      child: ObjectCard(
        object: section,
        items: items,
      ),
    );
  }
}
