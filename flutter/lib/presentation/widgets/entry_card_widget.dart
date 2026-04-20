import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/field_history_models.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/widgets/universal_entry_card.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_action_builder.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show EntryActionsContext;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show sensitivitySettingsProvider, SensitivityDisplayMode;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show showOverlaySnackBar, SnackBarType;

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
  /// When used inside UnifiedFormSection, EntryActionsContext provides the real callbacks.
  final void Function(T item)? onDelete;
  final void Function(T item)? onEdit;
  final String Function(T item)? formatAllFields;

  const EntryCardWidget({
    super.key,
    required this.item,
    required this.title,
    this.subtitle,
    required this.icon,
    required this.fields,
    this.itemId,
    this.historyFieldId,
    this.isSensitive = false,
    this.isRestricted = false,
    this.onDelete,
    this.onEdit,
    this.formatAllFields,
  });

  @override
  ConsumerState<EntryCardWidget<T>> createState() => _EntryCardWidgetState<T>();
}

class _EntryCardWidgetState<T> extends ConsumerState<EntryCardWidget<T>> {
  bool _historyExpanded = false;

  FieldHistory? get _history {
    if (widget.itemId == null || widget.historyFieldId == null) return null;
    return ref
        .watch(fieldHistoriesProvider.notifier)
        .getHistory(widget.itemId!, widget.historyFieldId!);
  }

  Future<void> _handleCopy(String formattedText) async {
    await Clipboard.setData(ClipboardData(text: formattedText));
    if (mounted) {
      showOverlaySnackBar(
        context,
        content: 'Copied to clipboard',
        type: SnackBarType.success,
      );
    }
  }

  Future<void> _handleHistoryPress(bool isSensitive) async {
    // Non-sensitive items: toggle freely
    if (!isSensitive) {
      setState(() => _historyExpanded = !_historyExpanded);
      return;
    }

    // Restricted items in privacy mode: force collapse if already expanded
    if (widget.isRestricted) {
      final sensitivityMode = ref.read(sensitivitySettingsProvider).displayMode;
      final isPrivacyMode = sensitivityMode != SensitivityDisplayMode.showAll;
      if (isPrivacyMode && _historyExpanded) {
        setState(() => _historyExpanded = false);
        return;
      }
    }

    // Sensitive items: require password verification
    final sensitiveAccess = ref.read(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    final hasRecentVerification = sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);
    if (hasRecentVerification) {
      setState(() => _historyExpanded = !_historyExpanded);
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
      setState(() => _historyExpanded = !_historyExpanded);
    }
  }

  @override
  Widget build(BuildContext context) {
    final history = _history;
    final hasHistory = history != null;
    final isSensitive = widget.isSensitive || widget.fields.any((f) => f.isSensitive);

    // Get EntryActionsContext for use inside UnifiedFormSection
    final actionsContext = EntryActionsContext.of(context);

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
          actions: _buildActions(context, actionsContext, isSensitive),
          bottomActions: [
            TextButton.icon(
              icon: Icon(
                _historyExpanded ? Icons.expand_less : Icons.history,
                size: 16,
              ),
              label: Text('History(${history?.entries.length ?? 0})'),
              onPressed: () => _handleHistoryPress(isSensitive),
            ),
          ],
          children: widget.fields.isNotEmpty
              ? [
                  const SizedBox(height: 4),
                  ResponsiveLabelField(
                    fields: widget.fields,
                    labelValueSpacing: 4,
                    layoutAxis: Axis.vertical,
                  ),
                ]
              : [],
        ),
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

  List<Widget> _buildActions(BuildContext context, EntryActionsContext? ctx, bool isSensitive) {
    // ctx != null means we're inside UnifiedFormSection — use its callbacks directly
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
    );
  }
}
