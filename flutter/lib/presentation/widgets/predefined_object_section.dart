import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart'
    show OperationLogService;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/utils/property_value_utils.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card.dart';

/// A section widget for default pages that uses predefined UnifiedObject schemas.
///
/// Unlike custom pages where users can modify schemas, this widget:
/// - Loads the schema from [ObjectTypeRegistry] (fixed properties)
/// - Only allows users to add/delete/edit item *values*
/// - Does not expose any UI for adding/removing/modifying property keys
class PredefinedObjectSection extends ConsumerStatefulWidget {
  final String sectionId;
  final String typeId;
  final String title;
  final IconData icon;
  final int maxVisibleItems;

  /// Optional custom form builder for fields that need special UI
  /// (e.g. dropdowns, date pickers).
  final Widget Function(
    BuildContext context,
    ThemeData theme,
    Map<String, TextEditingController> controllers,
    String mode,
    VoidCallback onSubmit,
    VoidCallback onCancel,
    Map<String, SensitivityLevel> fieldSensitivities,
  )? customFormBuilder;

  /// Builder for the display card of each item.
  /// Receives the [UnifiedObject] and a map of its property values.
  final Widget Function(UnifiedObject item, Map<String, String> propertyMap)?
      displayItemBuilder;

  /// Optional callback when an item is deleted (for notifications).
  final void Function(UnifiedObject item, int index)? onDidDelete;

  /// Optional callback when delete fails.
  final void Function(UnifiedObject item, int index)? onDeleteFailed;

  /// Optional callback for copying all fields.
  final Future<void> Function(UnifiedObject item, String formattedText)?
      onCopyAll;

  const PredefinedObjectSection({
    super.key,
    required this.sectionId,
    required this.typeId,
    required this.title,
    required this.icon,
    this.maxVisibleItems = 3,
    this.customFormBuilder,
    this.displayItemBuilder,
    this.onDidDelete,
    this.onDeleteFailed,
    this.onCopyAll,
  });

  @override
  ConsumerState<PredefinedObjectSection> createState() => _PredefinedObjectSectionState();
}

class _PredefinedObjectSectionState extends ConsumerState<PredefinedObjectSection> {
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    // Query from unified object state directly (avoid generated provider dependency)
    final allObjects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
    final objectMap = {for (final o in allObjects) o.id: o};
    final section = objectMap[widget.sectionId];
    final items = section?.childrenIds
            .where((id) => objectMap.containsKey(id))
            .map((id) => objectMap[id]!)
            .where((o) => !o.isDeleted)
            .toList() ??
        [];

    // If the section was soft-deleted by the user, don't render it at all.
    // The user can restore it from Trash or use "Restore defaults".
    if (section != null && section.isDeleted) {
      return const SizedBox.shrink();
    }

    // Use stored name if available, fallback to widget.title (l10n) for
    // first-time rendering before the section object is persisted.
    final effectiveName = section?.name.isNotEmpty == true ? section!.name : widget.title;
    final sectionObject = section ??
        UnifiedObject(
          id: widget.sectionId,
          typeId: widget.typeId,
          name: effectiveName,
          iconName: getSectionMeta(widget.sectionId)?.iconName ?? 'folder',
          properties: const {},
          createdAt: DateTime.now().millisecondsSinceEpoch,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        );

    // Fallback: if section has no schema yet (edge case), build from registry.
    final prefix = fieldPrefixForTypeId(widget.typeId);
    final Map<String, PropertyValue>? itemTemplate;
    if (sectionObject.properties.isEmpty) {
      final typeDef = ObjectTypeRegistry.getType(widget.typeId);
      if (typeDef != null) {
        itemTemplate = <String, PropertyValue>{
          for (final prop in typeDef.properties)
            prop.id: emptyPropertyValueForType(
              prop.type,
              lookupFieldSensitivity('$prefix.${prop.id}'),
            ),
        };
      } else {
        itemTemplate = null;
      }
    } else {
      itemTemplate = null; // Use section.properties as schema
    }

    final typeDef = ObjectTypeRegistry.getType(widget.typeId);
    final titleKey = typeDef?.titlePropertyKey ?? 'Title';

    return ObjectCard(
      object: sectionObject,
      items: items,
      itemTypeId: widget.typeId,
      itemTemplate: itemTemplate,
      historyFieldIdPrefix: prefix,
      titlePropertyKey: titleKey,
      nameExtractor: (props) {
        // Prefer the type's designated title property key
        final primary = props[titleKey];
        if (primary?.isNotEmpty == true) return primary!;
        // Fallback to common title keys
        for (final key in ['title', 'Title', 'name', 'fullName', 'destination', 'institution', 'company', 'full_name']) {
          if (props[key]?.isNotEmpty == true) return props[key]!;
        }
        return l10n.commonUntitled;
      },
      showEditActions: true,
      showAddButton: section != null && !section.isDeleted,
      customFormBuilder: widget.customFormBuilder,
      displayItemBuilder: widget.displayItemBuilder != null
          ? (context, item, {required isEditing}) {
              final map = <String, String>{};
              for (final entry in item.properties.entries) {
                map[entry.key] = propValueToString(entry.value);
              }
              return widget.displayItemBuilder!(item, map);
            }
          : null,
      onSaveItem: ({required itemId, required name, required properties}) async {
        final notifier = ref.read(unifiedObjectProvider.notifier);

        // If the section was soft-deleted, restore it first so the new item
        // has a valid parent.
        final currentSection = ref.read(unifiedObjectProvider).objects
            .firstWhere((o) => o.id == widget.sectionId, orElse: () => sectionObject);
        if (currentSection.isDeleted) {
          await notifier.restoreObject(widget.sectionId);
        }

        if (itemId == null) {
          await notifier.createDefaultItem(
            sectionId: widget.sectionId,
            typeId: widget.typeId,
            name: name,
            properties: properties,
          );
          final newItem = ref.read(unifiedObjectProvider).objects
              .lastWhere((o) => o.parentId == widget.sectionId && o.typeId == widget.typeId);
          return newItem.id;
        } else {
          await notifier.updateDefaultItem(itemId, name: name, properties: properties);
          return itemId;
        }
      },
      onDeleteItem: (itemId) async {
        final item = objectMap[itemId];
        final index = items.indexWhere((i) => i.id == itemId);
        try {
          await ref.read(unifiedObjectProvider.notifier).deleteDefaultItem(itemId);
        } on Exception catch (_) {
          widget.onDeleteFailed?.call(
            item ?? UnifiedObject(
              id: itemId,
              typeId: widget.typeId,
              name: '',
              properties: const {},
              createdAt: DateTime.now().millisecondsSinceEpoch,
              updatedAt: DateTime.now().millisecondsSinceEpoch,
            ),
            index,
          );
          return false;
        }
        // Record operation log
        final logSection = _logSectionForTypeId(widget.typeId);
        if (logSection != null) {
          final entry = OperationLogger.logCustomSection(
            section: logSection.value,
            action: LogAction.delete,
            description: l10n.predefinedDeletedItem(widget.title, item?.name ?? ''),
            descriptionKey: 'deletedPredefinedItem',
            descriptionArgs: {'title': widget.title, 'name': item?.name ?? ''},
          );
          await OperationLogService.instance.addEntry(entry);
          unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Deleted item'));
        }
        widget.onDidDelete?.call(
          item ?? UnifiedObject(
            id: itemId,
            typeId: widget.typeId,
            name: '',
            properties: const {},
            createdAt: DateTime.now().millisecondsSinceEpoch,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
          index,
        );
        return true;
      },
      onCopyAll: widget.onCopyAll != null
          ? (item, text) => widget.onCopyAll!(item, text)
          : null,
    );
  }

  /// Map typeId to LogSection for operation logging.
  LogSection? _logSectionForTypeId(String typeId) => logSectionForTypeId(typeId);

  /// Infer the property key that acts as the title/name field for a given type.
}


