import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/entry_configs.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show EntryActionsContext;

/// A reusable template widget for displaying profile/professional entry items.
/// Standardizes:
/// - Icon + title/subtitle + custom fields layout
/// - Action buttons (copy_all, edit, delete)
/// - History button + FieldHistoryView (fetches internally when itemId is provided)
/// - Private data reveal/hide via SensitiveValueWidget
/// - Restricted password verification
/// - Copied toast notification
class EntryItemWidget<T> extends ConsumerStatefulWidget {
  final T item;
  final String title;
  final String? subtitle;
  final IconData icon;
  final List<LabelValueField> fields;

  /// Unique ID for fetching field history. If provided, history is fetched internally.
  final String? itemId;

  /// Field ID prefix for history lookup (e.g., 'contact.email', 'idCard.number')
  final String? historyFieldId;

  /// Optional pre-fetched history. If not provided, fetched internally via itemId + historyFieldId.
  final FieldHistory? history;
  final EntryActionsConfig actionsConfig;

  /// Optional external callbacks. If provided, these override the internal onEdit/onDelete/onCopy.
  /// Useful when EntryItemWidget is used inside UnifiedFormSection which provides real callbacks.
  final VoidCallback? externalOnEdit;
  final VoidCallback? externalOnDelete;
  final Future<void> Function(String)? externalOnCopy;

  final VoidCallback onEdit;
  final VoidCallback onDelete;

  /// Async copy - allows password verification for restricted fields before copying.
  final Future<void> Function(String formattedText) onCopy;

  /// Converts T item to a map of field values for copying all fields.
  final String Function(T item) formatAllFields;

  const EntryItemWidget({
    super.key,
    required this.item,
    required this.title,
    this.subtitle,
    required this.icon,
    required this.fields,
    this.itemId,
    this.historyFieldId,
    this.history,
    this.actionsConfig = const EntryActionsConfig(),
    this.externalOnEdit,
    this.externalOnDelete,
    this.externalOnCopy,
    required this.onEdit,
    required this.onDelete,
    required this.onCopy,
    required this.formatAllFields,
  });

  @override
  ConsumerState<EntryItemWidget<T>> createState() => _EntryItemWidgetState<T>();
}

class _EntryItemWidgetState<T> extends ConsumerState<EntryItemWidget<T>> {
  bool _historyExpanded = false;

  /// Fetches history internally when itemId and historyFieldId are provided.
  FieldHistory? get _fetchedHistory {
    if (widget.history != null) return widget.history;
    if (widget.itemId == null || widget.historyFieldId == null) return null;
    return ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.itemId!, widget.historyFieldId!);
  }

  Future<void> _handleCopyAll() async {
    final formattedText = widget.formatAllFields(widget.item);
    await widget.onCopy(formattedText);
  }

  Future<void> _handleCopyAllWithVerification() async {
    // Check if any field is restricted - if so, verify password first
    final hasRestricted = widget.fields.any((f) => f.isSensitive);

    if (hasRestricted) {
      final verified = await _verifyPasswordForRestrictedFields();
      if (!verified) return;
    }

    if (!mounted) return;
    await _handleCopyAll();
  }

  Future<bool> _verifyPasswordForRestrictedFields() async {
    // Find a restricted field ID for verification prompt
    String? restrictedFieldId;
    for (final field in widget.fields) {
      if (field.isSensitive && field.fieldId != null) {
        restrictedFieldId = field.fieldId;
        break;
      }
    }

    if (restrictedFieldId == null) return true;

    final settings = ref.read(sensitivitySettingsProvider);
    final level = settings.getFieldLevel(restrictedFieldId);
    if (level != SensitivityLevel.restricted) return true;

    // Check if user was verified within the last 1 minute (password cache)
    final sensitiveAccess = ref.read(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    final hasRecentVerification =
        sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);
    if (hasRecentVerification) return true;

    // Show password dialog
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccount = authNotifier.selectedAccount;
    final password = await showPasswordVerificationDialog(
      context: context,
      ref: ref,
      passwordHint: selectedAccount?.passwordHint,
      onVerify: authNotifier.verifyPasswordForSensitiveData,
    );
    if (password == null) return false;

    ref.read(sensitivePageAccessProvider.notifier).markVerified();
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final history = _fetchedHistory;
    final hasHistory = history != null;

    // Check for external callbacks from EntryActionsContext (provided by UnifiedFormSection)
    final actionsContext = EntryActionsContext.of(context);
    final extOnEdit = actionsContext?.onEdit;
    final extOnDelete = actionsContext?.onDelete;
    final extOnCopy = actionsContext?.onCopy;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Left icon
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Icon(
                  widget.icon,
                  size: 20,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 12),
              // Content: title, subtitle, fields
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Title
                    SelectableText(
                      widget.title,
                      style: theme.textTheme.bodyLarge?.copyWith(
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    // Subtitle if present
                    if (widget.subtitle != null && widget.subtitle!.isNotEmpty)
                      SelectableText(
                        widget.subtitle!,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    // Custom fields
                    if (widget.fields.isNotEmpty) ...[
                      const SizedBox(height: 4),
                      ResponsiveLabelField(
                        fields: widget.fields,
                        labelValueSpacing: 4,
                        layoutAxis: Axis.vertical,
                      ),
                    ],
                  ],
                ),
              ),
              // Action buttons
              if (widget.actionsConfig.showCopy)
                IconButton(
                  icon: const Icon(Icons.copy_all, size: 20),
                  tooltip: 'Copy All',
                  onPressed: extOnCopy != null
                      ? () => extOnCopy(widget.formatAllFields(widget.item))
                      : _handleCopyAllWithVerification,
                  visualDensity: VisualDensity.compact,
                ),
              if (widget.actionsConfig.showEdit)
                IconButton(
                  icon: const Icon(Icons.edit_outlined, size: 20),
                  tooltip: 'Edit',
                  onPressed: () async {
                    // Check if any field is restricted - if so, verify password first
                    final hasRestricted = widget.fields.any((f) => f.isSensitive);
                    if (hasRestricted) {
                      final verified = await _verifyPasswordForRestrictedFields();
                      if (!verified) return;
                    }
                    if (!mounted) return;
                    if (extOnEdit != null) {
                      extOnEdit();
                    } else {
                      widget.onEdit();
                    }
                  },
                  visualDensity: VisualDensity.compact,
                ),
              if (widget.actionsConfig.showDelete)
                IconButton(
                  icon: const Icon(Icons.delete_outline, size: 20),
                  tooltip: 'Delete',
                  onPressed: () async {
                    // Check if any field is restricted - if so, verify password first
                    final hasRestricted = widget.fields.any((f) => f.isSensitive);
                    if (hasRestricted) {
                      final verified = await _verifyPasswordForRestrictedFields();
                      if (!verified) return;
                    }
                    if (!mounted) return;
                    if (extOnDelete != null) {
                      extOnDelete();
                    } else {
                      widget.onDelete();
                    }
                  },
                  visualDensity: VisualDensity.compact,
                ),
              // History button
              if (widget.actionsConfig.showHistory && hasHistory)
                IconButton(
                  icon: Icon(
                    _historyExpanded ? Icons.expand_less : Icons.history,
                    size: 20,
                  ),
                  tooltip: 'History',
                  onPressed: () {
                    setState(() => _historyExpanded = !_historyExpanded);
                  },
                  visualDensity: VisualDensity.compact,
                ),
            ],
          ),
        ),
        // History view
        if (hasHistory && _historyExpanded)
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
}
