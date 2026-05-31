import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/presentation/utils/format_field_label.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/attachment_upload_service.dart';
import 'package:solosoul_flutter/presentation/widgets/attachment_list_sheet.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_action_builder.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/presentation/widgets/entry_actions_context.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider, isSensitiveAccessGrantedProvider;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider, SensitivityDisplayMode;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show effectiveSensitivityProvider, SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

part 'entry_card_widget.g.dart';

/// Provider for per-item history expanded state, keyed by itemId or title.
@riverpod
class HistoryExpanded extends _$HistoryExpanded {
  @override
  bool build(String key) => false;

  void toggle() => state = !state;
  void expand() => state = true;
  void collapse() => state = false;
}

/// Generic entry card with actions, history, and sensitivity-aware access control.
class EntryCardWidget<T> extends ConsumerStatefulWidget {
  final T item;
  final String title;
  final String? subtitle;
  final IconData icon;
  final List<LabelValueField> fields;
  final String? itemId;
  final String? historyFieldId;
  final bool isSensitive;
  final bool isRestricted;

  /// Fallback callbacks when not used inside UnifiedFormSection.
  final void Function(T item)? onDelete;
  final void Function(T item)? onEdit;
  final String Function(T item)? formatAllFields;

  // === Auto-mode parameters ===
  /// Field prefix for auto-generating fieldIds (e.g., 'contact', 'idCard')
  final String? fieldPrefix;

  /// Item data map for auto-build mode. When provided alongside fieldPrefix,
  /// fields will be auto-generated from this map.
  final Map<String, dynamic>? itemData;

  /// Global sensitivity level override for all auto-built fields.
  final SensitivityLevel? sensitivityLevel;

  /// Field keys to exclude from auto-built fields (e.g., 'label' when already used as title).
  final Set<String>? excludeFields;

  /// Optional display labels for properties (key -> label).
  final Map<String, String>? propertyLabels;

  const EntryCardWidget({
    super.key,
    required this.item,
    required this.title,
    this.subtitle,
    required this.icon,
    this.fields = const [],
    this.itemId,
    this.historyFieldId,
    this.isSensitive = false,
    this.isRestricted = false,
    this.onDelete,
    this.onEdit,
    this.formatAllFields,
    this.fieldPrefix,
    this.itemData,
    this.sensitivityLevel,
    this.excludeFields,
    this.propertyLabels,
  });

  @override
  ConsumerState<EntryCardWidget<T>> createState() => _EntryCardWidgetState<T>();
}

class _EntryCardWidgetState<T> extends ConsumerState<EntryCardWidget<T>> {
  String get _historyKey => widget.itemId ?? widget.title;

  FieldHistory? get _history {
    final itemId = widget.itemId;
    final historyFieldId = widget.historyFieldId;
    if (itemId == null || historyFieldId == null) return null;
    return ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(itemId, historyFieldId);
  }

  Future<void> _handleCopy(String formattedText) async {
    await Clipboard.setData(ClipboardData(text: formattedText));
    unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
    if (mounted) {
      showOverlaySnackBar(
        context,
        content: AppLocalizations.of(context).commonCopiedToClipboard,
        type: SnackBarType.success,
      );
    }
  }

  Future<void> _handleHistoryPress(bool isSensitive) async {
    final currentExpanded = ref.read(historyExpandedProvider(_historyKey));
    final currentSettings = ref.read(accountStyleProvider).value;
    final isPrivacyMode = currentSettings?.displayMode != SensitivityDisplayMode.showAll;

    // Non-sensitive items: toggle freely
    if (!isSensitive) {
      ref.read(historyExpandedProvider(_historyKey).notifier).toggle();
      return;
    }

    // Restricted items in privacy mode: if expanded, collapse silently; if collapsed, require auth then expand
    if (widget.isRestricted && isPrivacyMode) {
      if (currentExpanded) {
        ref.read(historyExpandedProvider(_historyKey).notifier).collapse();
        return;
      }
      // Collapsed: require password to expand
      if (!ref.read(isSensitiveAccessGrantedProvider)) {
        final authNotifier = ref.read(authNotifierProvider.notifier);
        final selectedAccount = authNotifier.selectedAccount;
        final password = await showPasswordVerificationDialog(
          context: context,
          ref: ref,
          passwordHint: selectedAccount?.passwordHint,
          onVerify: authNotifier.verifyPasswordForSensitiveData,
        );
        if (password == null) return;
        ref.read(sensitivePageAccessProvider.notifier).markVerified();
      }
      if (mounted) {
        ref.read(historyExpandedProvider(_historyKey).notifier).expand();
      }
      return;
    }

    // Sensitive items: require password verification
    if (ref.read(isSensitiveAccessGrantedProvider)) {
      ref.read(historyExpandedProvider(_historyKey).notifier).toggle();
      return;
    }

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return;
    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    if (mounted) {
      ref.read(historyExpandedProvider(_historyKey).notifier).toggle();
    }
  }

  bool _canAutoBuild() {
    return widget.fields.isEmpty && widget.itemData != null && widget.fieldPrefix != null;
  }

  SensitivityLevel _getSensitivityForField(String fieldKey) {
    if (widget.sensitivityLevel != null) {
      return widget.sensitivityLevel!;
    }
    // Always add prefix since itemToMap returns unprefixed keys.
    final fieldId = '${widget.fieldPrefix}.$fieldKey';
    return ref.watch(effectiveSensitivityProvider(fieldId));
  }

  String _formatLabel(String key) {
    final l10n = AppLocalizations.of(context);
    return widget.propertyLabels?[key] ?? translateFieldLabel(key, l10n);
  }

  List<LabelValueField> _autoBuildFields() {
    final fields = <LabelValueField>[];
    final exclude = widget.excludeFields ?? {};
    final itemData = widget.itemData;
    if (itemData == null) return fields;
    itemData.forEach((key, value) {
      if (value == null || (value is String && value.isEmpty)) return;
      if (exclude.contains(key)) return;
      // Always add prefix since itemToMap returns unprefixed keys.
      final fieldId = '${widget.fieldPrefix}.$key';
      final sensitivity = _getSensitivityForField(key);
      final isSensitive = sensitivity == SensitivityLevel.critical;
      fields.add(LabelValueField(
        label: _formatLabel(key),
        value: value.toString(),
        fieldId: fieldId,
        isSensitive: isSensitive,
        sensitivityLevel: sensitivity,
      ));
    });
    return fields;
  }

  List<LabelValueField> _buildFields() {
    if (_canAutoBuild()) {
      return _autoBuildFields();
    }
    return widget.fields;
  }

  @override
  Widget build(BuildContext context) {
    final history = _history;
    final hasHistory = history != null;
    final fields = _buildFields();
    final isSensitive = widget.isSensitive || fields.any((f) => f.isSensitive);
    final isExpanded = ref.watch(historyExpandedProvider(_historyKey));

    // Suppress our own history button when rendered inside UnifiedFormSection's
    // _ItemWithHistory, which already provides one when showHistoryExpansion is true.
    final inFormSection = EntryActionsContext.of(context) != null;

    // Auto-collapse restricted history when entering privacy mode
    ref.listen(
      accountStyleProvider,
      (previous, next) {
        if (!widget.isRestricted) return;
        final prevStyle = previous?.value;
        final wasShowAll = prevStyle?.displayMode == SensitivityDisplayMode.showAll;
        final isNowPrivacy = next.value?.displayMode != SensitivityDisplayMode.showAll;
        if (wasShowAll && isNowPrivacy) {
          Future.microtask(() {
            if (context.mounted) {
              ref.read(historyExpandedProvider(_historyKey).notifier).collapse();
            }
          });
        }
      },
    );

    // Auto-collapse history when sensitive access is locked
    ref.listen(
      isSensitiveAccessGrantedProvider,
      (previous, next) {
        if (previous == true && next == false) {
          Future.microtask(() {
            if (context.mounted) {
              ref.read(historyExpandedProvider(_historyKey).notifier).collapse();
            }
          });
        }
      },
    );

    // Get EntryActionsContext for use inside UnifiedFormSection
    final actionsContext = EntryActionsContext.of(context);

    // History button in action row when inside UnifiedFormSection
    Widget? historyAction;
    if (inFormSection && actionsContext?.onToggleHistory != null) {
      final count = history?.entries.length ?? 0;
      final hasHist = count > 0;
      final iconData = isExpanded ? Icons.history_toggle_off : Icons.history;
      final iconColor = hasHist
          ? Theme.of(context).colorScheme.onSurfaceVariant
          : Theme.of(context).colorScheme.onSurfaceVariant.withValues(alpha: 0.4);
      final icon = Icon(iconData, size: 20, color: iconColor);

      historyAction = IconButton(
        icon: hasHist
            ? Stack(
                clipBehavior: Clip.none,
                children: [
                  icon,
                  Positioned(
                    right: -6,
                    top: -6,
                    child: Text(
                      '$count',
                      style: TextStyle(
                        fontSize: 10,
                        color: iconColor,
                        fontWeight: FontWeight.w500,
                        height: 1,
                      ),
                    ),
                  ),
                ],
              )
            : Stack(
                clipBehavior: Clip.none,
                children: [
                  icon,
                  Positioned(
                    right: -6,
                    top: -6,
                    child: Text(
                      '0',
                      style: TextStyle(
                        fontSize: 10,
                        color: iconColor,
                        fontWeight: FontWeight.w500,
                        height: 1,
                      ),
                    ),
                  ),
                ],
              ),
        tooltip: hasHist ? AppLocalizations.of(context).entryHistoryCount(count) : AppLocalizations.of(context).entryNoHistory,
        onPressed: hasHist
            ? () => _handleHistoryPress(isSensitive)
            : () {
                showOverlaySnackBar(
                  context,
                  content: AppLocalizations.of(context).entryNoHistory,
                  type: SnackBarType.info,
                );
              },
        visualDensity: VisualDensity.compact,
      );
    }

    final actions = _buildActions(
      context,
      actionsContext,
      isSensitive,
      historyAction: historyAction,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UniversalEntryCard(
          title: SelectableText(
            widget.title,
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
              fontWeight: FontWeight.w500,
            ),
          ),
          subtitle: widget.subtitle != null
              ? SelectableText(
                  widget.subtitle!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                )
              : null,
          leading: Icon(
            widget.icon,
            size: 20,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
          actions: actions,
          bottomActions: inFormSection
              ? []
              : [
                  TextButton.icon(
                    icon: Icon(
                      isExpanded ? Icons.expand_less : Icons.history,
                      size: 16,
                    ),
                    label: Text(AppLocalizations.of(context).entryHistoryCount(history?.entries.length ?? 0)),
                    onPressed: () => _handleHistoryPress(isSensitive),
                  ),
                ],
          children: fields.isNotEmpty
              ? [
                  const SizedBox(height: 4),
                  ResponsiveLabelField(
                    fields: fields,
                    labelValueSpacing: 4,
                    layoutAxis: Axis.vertical,
                  ),
                ]
              : [],
        ),
        if (hasHistory && isExpanded)
          Padding(
            padding: const EdgeInsets.only(left: 32, bottom: 8),
            child: FieldHistoryView(
              fieldName: widget.historyFieldId ?? widget.title,
              history: history,
            ),
          ),
      ],
    );
  }

  List<Widget> _buildActions(
    BuildContext context,
    EntryActionsContext? ctx,
    bool isSensitive, {
    Widget? historyAction,
  }) {
    final editAction = ctx != null && ctx.onEdit != null
        ? ctx.onEdit!
        : widget.onEdit != null
            ? () => widget.onEdit!(widget.item)
            : () {};
    final deleteAction = ctx != null && ctx.onDelete != null
        ? ctx.onDelete!
        : widget.onDelete != null
            ? () => widget.onDelete!(widget.item)
            : () {};
    void handleCopyAction() {
      final text = widget.formatAllFields?.call(widget.item) ??
          widget.item.toString();
      _handleCopy(text);
    }

    Widget? attachmentAction;
    if (widget.item is UnifiedObject) {
      final obj = widget.item as UnifiedObject;
      final count = obj.attachments.where((a) => !a.isDeleted).length;
      final hasAttachments = count > 0;
      attachmentAction = IconButton(
        icon: hasAttachments
            ? Stack(
                clipBehavior: Clip.none,
                children: [
                  const Icon(Icons.attach_file, size: 20),
                  Positioned(
                    right: -6,
                    top: -6,
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 4),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.primary,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        '$count',
                        style: TextStyle(
                          fontSize: 10,
                          color: Theme.of(context).colorScheme.onPrimary,
                          fontWeight: FontWeight.w500,
                          height: 1,
                        ),
                      ),
                    ),
                  ),
                ],
              )
            : const Icon(Icons.attach_file, size: 20),
        tooltip: hasAttachments
            ? AppLocalizations.of(context).entryAttachments(count)
            : AppLocalizations.of(context).addAttachment,
        onPressed: () => _handleAttachmentAction(context, obj),
        visualDensity: VisualDensity.compact,
      );
    }

    return EntryActionBuilder.buildActions(
      context: context,
      ref: ref,
      onCopy: handleCopyAction,
      onEdit: editAction,
      onDelete: deleteAction,
      config: const EntryActionsConfig(
        showCopy: true,
        showEdit: true,
        showDelete: true,
      ),
      isSensitive: isSensitive,
      historyAction: historyAction,
      attachmentAction: attachmentAction,
    );
  }

  Future<void> _handleAttachmentAction(BuildContext context, UnifiedObject obj) async {
    if (obj.attachments.isNotEmpty) {
      _showAttachments(context, obj);
    } else {
      await _addAttachment(context, obj);
    }
  }

  Future<void> _addAttachment(BuildContext context, UnifiedObject obj) async {
    final bool isSensitive = obj.properties.values.any(
          (p) => p.sensitivity == SensitivityLevel.sensitive ||
                 p.sensitivity == SensitivityLevel.critical,
        ) ||
        widget.isSensitive;

    final attachment = await AttachmentUploadService.pickAndUpload(
      context: context,
      ref: ref,
      requiresSensitiveCheck: isSensitive,
    );
    if (attachment == null) return;

    final updatedAttachments = [...obj.attachments, attachment];
    await ref.read(unifiedObjectProvider.notifier).updateObject(
      obj.id,
      attachments: updatedAttachments,
    );
  }

  void _showAttachments(BuildContext context, UnifiedObject obj) {
    final accountId = ref.read(authNotifierProvider.notifier).selectedAccountId;
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => AttachmentListSheet(
        object: obj,
        accountId: accountId,
        onAddAttachment: () => _addAttachment(context, obj),
      ),
    );
  }
}
