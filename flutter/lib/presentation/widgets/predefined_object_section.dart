import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/utils/log_section_utils.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart'
    show OperationLogService;
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

    // Load predefined schema from registry
    final typeDef = ObjectTypeRegistry.getType(widget.typeId);
    if (typeDef == null) {
      return _buildError('Unknown type: ${widget.typeId}');
    }

    // Build template from schema + FieldRegistry sensitivity
    final prefix = _fieldPrefix(widget.typeId);
    final template = <String, PropertyValue>{
      for (final prop in typeDef.properties)
        prop.id: TextProperty(
          text: '',
          sensitivity: _lookupSensitivity('$prefix.${prop.id}'),
        ),
    };

    // Fallback section object when not yet created (new account)
    final sectionObject = section ??
        UnifiedObject(
          id: widget.sectionId,
          typeId: widget.typeId,
          name: widget.title,
          iconName: getSectionMeta(widget.sectionId)?.iconName ?? 'folder',
          properties: const {},
          createdAt: DateTime.now().millisecondsSinceEpoch,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        );

    return ObjectCard(
      object: sectionObject,
      items: items,
      itemTypeId: widget.typeId,
      itemTemplate: template,
      historyFieldIdPrefix: prefix,
      nameExtractor: (props) {
        for (final key in ['title', 'name', 'fullName', 'destination', 'institution', 'company']) {
          if (props[key]?.isNotEmpty == true) return props[key]!;
        }
        return 'Untitled';
      },
      showEditActions: false,
      showAddButton: true,
      customFormBuilder: widget.customFormBuilder,
      displayItemBuilder: widget.displayItemBuilder != null
          ? (context, item, {required isEditing}) {
              final map = <String, String>{};
              for (final prop in typeDef.properties) {
                final value = item.properties[prop.id];
                map[prop.id] = switch (value) {
                  TextProperty() => value.text,
                  _ => '',
                };
              }
              return widget.displayItemBuilder!(item, map);
            }
          : null,
      onSaveItem: ({required itemId, required name, required properties}) async {
        final notifier = ref.read(unifiedObjectProvider.notifier);
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
            description: 'Deleted ${widget.title}: ${item?.name ?? ''}',
          );
          await OperationLogService.instance.addEntry(entry);
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

  Widget _buildError(String message) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Text(
        'Error: $message',
        style: const TextStyle(color: Colors.red),
      ),
    );
  }

  /// Map typeId to the field-prefix used by FieldRegistry.
  String _fieldPrefix(String typeId) {
    return switch (typeId) {
      'profile_identity' => 'identity',
      'profile_contact' => 'contact',
      'profile_id_card' => 'idCard',
      'profile_address' => 'address',
      'travel_passport' => 'passport',
      'travel_visa' => 'visa',
      'travel_history' => 'travel',
      'financial_bank_account' => 'bankAccount',
      'financial_card' => 'card',
      'financial_tax_id' => 'taxId',
      'professional_education' => 'education',
      'professional_employment' => 'employment',
      'professional_skill' => 'skill',
      'professional_language' => 'language',
      'professional_award' => 'award',
      _ => typeId,
    };
  }

  /// Map typeId to LogSection for operation logging.
  LogSection? _logSectionForTypeId(String typeId) => logSectionForTypeId(typeId);

  /// Look up sensitivity from FieldRegistry defaults.
  SensitivityLevel _lookupSensitivity(String fieldId) {
    try {
      return FieldRegistry.defaultFields
          .firstWhere((f) => f.fieldId == fieldId)
          .level;
    } on Object catch (_) {
      return SensitivityLevel.public;
    }
  }
}
