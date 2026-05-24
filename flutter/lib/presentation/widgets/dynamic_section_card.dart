import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card.dart';
import 'package:solosoul_flutter/presentation/widgets/section_renderer_registry.dart';

/// Renders a single section (preset or generic) using the appropriate
/// configuration from [SectionRendererRegistry].
///
/// For preset types (e.g. 'profile_identity'), preserves the rich
/// [EntryCardWidget] rendering. For generic types (e.g. 'collection'),
/// falls back to the standard [ObjectCard].
class DynamicSectionCard extends ConsumerWidget {
  final UnifiedObject section;

  const DynamicSectionCard({
    super.key,
    required this.section,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(childrenProvider(section.id));
    // Preset sections are stored with typeId 'collection', so look up by
    // section ID first, then fall back to typeId for non-preset types.
    final config = SectionRendererRegistry.getConfigBySectionId(section.id) ??
        SectionRendererRegistry.getConfig(section.typeId ?? '');

    if (config != null) {
      return _buildPresetSection(context, items, config);
    }

    // Generic section (custom collection, note, task, etc.)
    return ObjectCard(
      object: section,
      items: items,
    );
  }

  Widget _buildPresetSection(
    BuildContext context,
    List<UnifiedObject> items,
    PresetSectionConfig config,
  ) {
    return ObjectCard(
      object: section,
      items: items,
      itemTypeId: config.typeId,
      historyFieldIdPrefix: config.fieldPrefix,
      titlePropertyKey: config.titlePropertyKey,
      displayItemBuilder: (context, item, {required isEditing}) {
        return _buildEntryCard(context, item, config);
      },
    );
  }

  Widget _buildEntryCard(
    BuildContext context,
    UnifiedObject item,
    PresetSectionConfig config,
  ) {
    final l10n = AppLocalizations.of(context);
    final itemMap = itemToMap(item);

    // Compute display title using the same priority as ObjectCard
    final titleKey = config.titlePropertyKey;
    String title;
    if (itemMap[titleKey]?.isNotEmpty == true) {
      title = itemMap[titleKey]!;
    } else if (item.name.isNotEmpty) {
      title = item.name;
    } else {
      title = l10n.commonUntitled;
    }

    return EntryCardWidget<UnifiedObject>(
      item: item,
      title: title,
      icon: config.itemIcon,
      itemId: item.id,
      historyFieldId: config.historyFieldId,
      isRestricted: config.isRestricted,
      formatAllFields: config.formatAllFields != null
          ? (_) => config.formatAllFields!(l10n, item)
          : null,
      itemData: itemMap,
      fieldPrefix: config.fieldPrefix,
      excludeFields: config.excludeFields,
      propertyLabels: section.propertyLabels,
    );
  }
}
