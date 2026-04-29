import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/models/sensitivity_models.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart';

/// A section widget for default pages that uses predefined UnifiedObject schemas.
///
/// Unlike custom pages where users can modify schemas, this widget:
/// - Loads the schema from [ObjectTypeRegistry] (fixed properties)
/// - Only allows users to add/delete/edit item *values*
/// - Does not expose any UI for adding/removing/modifying property keys
class PredefinedObjectSection extends ConsumerWidget {
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
  Widget build(BuildContext context, WidgetRef ref) {
    // Query from unified object state directly (avoid generated provider dependency)
    final allObjects = ref.watch(unifiedObjectProvider.select((d) => d.objects));
    final objectMap = {for (final o in allObjects) o.id: o};
    final section = objectMap[sectionId];
    final items = section?.childrenIds
            .where((id) => objectMap.containsKey(id))
            .map((id) => objectMap[id]!)
            .where((o) => !o.isDeleted)
            .toList() ??
        [];

    // Load predefined schema from registry
    final typeDef = ObjectTypeRegistry.getType(typeId);
    if (typeDef == null) {
      return _buildError('Unknown type: $typeId');
    }

    // Build field definitions from schema + FieldRegistry sensitivity
    final fieldDefs = typeDef.properties.map((prop) {
      // FieldRegistry uses lowercase section prefix (e.g. 'passport.title')
      final fieldId = '${_fieldPrefix(typeId)}.${prop.id}';
      final sensitivity = _lookupSensitivity(fieldId);
      return FormFieldDef(
        fieldId: prop.id, // Use short key for controllers
        label: prop.name,
        sensitivity: sensitivity,
      );
    }).toList();

    return UnifiedFormSection<UnifiedObject>(
      title: title,
      icon: icon,
      items: items,
      maxVisibleItems: maxVisibleItems,
      fieldDefs: fieldDefs,
      itemFactory: (values, {String? id}) {
        // Compute a display name from the first property that looks like a title
        String name = 'Untitled';
        for (final key in ['title', 'name', 'destination', 'institution', 'company']) {
          if (values[key]?.isNotEmpty == true) {
            name = values[key]!;
            break;
          }
        }
        return UnifiedObject(
          id: id ?? '', // ID will be assigned by createObject in onSave
          typeId: typeId,
          name: name,
          parentId: sectionId,
          properties: {
            for (final entry in values.entries)
              entry.key: TextProperty(text: entry.value),
          },
          createdAt: DateTime.now().millisecondsSinceEpoch,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        );
      },
      itemToMap: (item) {
        final map = <String, String>{};
        for (final prop in typeDef.properties) {
          final value = item.properties[prop.id];
          map[prop.id] = switch (value) {
            TextProperty() => value.text,
            _ => '',
          };
        }
        return map;
      },
      displayItemBuilder: displayItemBuilder ?? _defaultDisplayBuilder,
      onDelete: (item) async {
        await ref
            .read(unifiedObjectProvider.notifier)
            .deleteDefaultItem(item.id);
      },
      onSave: (newItem, values, editingItem) async {
        final notifier = ref.read(unifiedObjectProvider.notifier);
        if (editingItem == null) {
          // Adding
          String name = 'Untitled';
          for (final key in ['title', 'name', 'destination', 'institution', 'company']) {
            if (values[key]?.isNotEmpty == true) {
              name = values[key]!;
              break;
            }
          }
          await notifier.createDefaultItem(
            sectionId: sectionId,
            typeId: typeId,
            name: name,
            values: values,
          );
        } else {
          // Editing
          String name = editingItem.name;
          for (final key in ['title', 'name', 'destination', 'institution', 'company']) {
            if (values[key]?.isNotEmpty == true) {
              name = values[key]!;
              break;
            }
          }
          await notifier.updateDefaultItem(
            editingItem.id,
            name: name,
            values: values,
          );
        }
      },
      customFormBuilder: customFormBuilder != null
          ? (context, theme, controllers, mode, onSubmit, onCancel, sensitivities) {
              return customFormBuilder!(
                context,
                theme,
                controllers,
                mode,
                onSubmit,
                onCancel,
                sensitivities,
              );
            }
          : null,
      onDidDelete: onDidDelete != null
          ? (item, index) => onDidDelete!(item, index)
          : null,
      onDeleteFailed: onDeleteFailed != null
          ? (item, index) => onDeleteFailed!(item, index)
          : null,
      onCopyAll: onCopyAll != null
          ? (item, text) => onCopyAll!(item, text)
          : null,
      itemIdExtractor: (item) => item.id,
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

  Widget _defaultDisplayBuilder(UnifiedObject item, Map<String, String> map) {
    // Default fallback: show a simple card with the item name
    final subtitle = map.values.where((v) => v.isNotEmpty).take(2).join(', ');
    return Card(
      child: ListTile(
        title: Text(item.name),
        subtitle: subtitle.isNotEmpty ? Text(subtitle) : null,
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
