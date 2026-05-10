import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_actions_context.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_edit_mode_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/object_card/object_card_item_tile.dart';

/// Widget that decides between display mode and edit mode for an item tile.
class ObjectCardItemTileWidget extends StatelessWidget {
  final UnifiedObject item;
  final bool isEditing;
  final bool isHistoryExpanded;
  final Widget Function(BuildContext, UnifiedObject, {required bool isEditing})?
      displayItemBuilder;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onCopy;
  final VoidCallback onToggleHistory;
  final VoidCallback onStartEdit;
  final String historyFieldIdPrefix;
  final String Function(Map<String, String>)? nameExtractor;
  final String titlePropertyKey;

  // Edit mode delegation
  final Map<String, PropertyValue> template;
  final Map<String, TextEditingController> editControllers;
  final bool hasChanges;
  final VoidCallback onSaveEditItem;
  final VoidCallback onCancelEditItem;
  final Widget Function(
    BuildContext,
    ThemeData,
    Map<String, TextEditingController>,
    String,
    VoidCallback,
    VoidCallback,
    Map<String, SensitivityLevel>,
  )? customFormBuilder;
  final void Function(String key, bool? value) onCheckboxChanged;
  final bool showDeprecated;
  final List<String> deprecatedKeys;
  final VoidCallback onToggleDeprecated;

  const ObjectCardItemTileWidget({
    super.key,
    required this.item,
    required this.isEditing,
    required this.isHistoryExpanded,
    this.displayItemBuilder,
    required this.onEdit,
    required this.onDelete,
    required this.onCopy,
    required this.onToggleHistory,
    required this.onStartEdit,
    required this.historyFieldIdPrefix,
    this.nameExtractor,
    required this.titlePropertyKey,
    required this.template,
    required this.editControllers,
    required this.hasChanges,
    required this.onSaveEditItem,
    required this.onCancelEditItem,
    this.customFormBuilder,
    required this.onCheckboxChanged,
    this.showDeprecated = false,
    this.deprecatedKeys = const [],
    this.onToggleDeprecated = _noop,
  });

  static void _noop() {}

  @override
  Widget build(BuildContext context) {
    if (isEditing) {
      return ObjectCardEditModeWidget(
        item: item,
        template: template,
        editControllers: editControllers,
        hasChanges: hasChanges,
        onSave: onSaveEditItem,
        onCancel: onCancelEditItem,
        customFormBuilder: customFormBuilder,
        titlePropertyKey: titlePropertyKey,
        onCheckboxChanged: onCheckboxChanged,
        showDeprecated: showDeprecated,
        deprecatedKeys: deprecatedKeys,
        onToggleDeprecated: onToggleDeprecated,
      );
    }
    if (displayItemBuilder != null) {
      return EntryActionsContext(
        onEdit: onEdit,
        onDelete: onDelete,
        onCopy: (_) async => onCopy(),
        onToggleHistory: onToggleHistory,
        child: displayItemBuilder!(context, item, isEditing: false),
      );
    }
    return ObjectCardItemTile(
      item: item,
      isHistoryExpanded: isHistoryExpanded,
      onToggleHistory: onToggleHistory,
      onCopy: onCopy,
      onStartEdit: onStartEdit,
      onDelete: onDelete,
      historyFieldIdPrefix: historyFieldIdPrefix,
      nameExtractor: nameExtractor,
      titlePropertyKey: titlePropertyKey,
      template: template,
    );
  }
}
