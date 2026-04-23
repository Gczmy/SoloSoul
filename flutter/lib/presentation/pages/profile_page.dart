import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, SnackBarType, showOverlaySnackBar;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart'
    show profileNotifierProvider, identityProvider, idCardItemsProvider, contactItemsProvider, addressItemsProvider, fieldHistoriesProvider;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show SensitivityLevel, SensitivityDisplayMode, effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show UnifiedFormSection, FormFieldDef, HistoryRecordingConfig;
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show
        authNotifierProvider,
        sensitivePageAccessProvider,
        isSensitiveAccessGrantedProvider;
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';

/// Standalone helper to verify password for restricted fields.
/// Returns true if field is not restricted OR if verification succeeded.
Future<bool> verifyPasswordForRestrictedField({
  required BuildContext context,
  required WidgetRef ref,
  required String fieldId,
  bool Function()? isMounted,
}) async {
  final level = ref.watch(effectiveSensitivityProvider(fieldId));

  // If not restricted, allow without verification
  if (level != SensitivityLevel.critical) {
    return true;
  }

  // Check if user was verified within the valid duration (password cache)
  if (ref.read(isSensitiveAccessGrantedProvider)) {
    return true;
  }

  // Show password dialog
  final authNotifier = ref.read(authNotifierProvider.notifier);
  final selectedAccount = authNotifier.selectedAccount;
  final password = await showPasswordVerificationDialog(
    context: context,
    ref: ref,
    passwordHint: selectedAccount?.passwordHint,
    onVerify: authNotifier.verifyPasswordForSensitiveData,
  );
  if (password == null) {
    return false;
  }

  // Check mounted before accessing state after await
  if (isMounted != null && !isMounted()) {
    return false;
  }

  // Mark as verified in shared sensitive page access
  ref.read(sensitivePageAccessProvider.notifier).markVerified();
  return true;
}

class ProfilePage extends ConsumerStatefulWidget {
  const ProfilePage({super.key});

  @override
  ConsumerState<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends ConsumerState<ProfilePage> {
  bool _isEditingName = false;
  final _nameController = TextEditingController();
  String? _nameError;
  bool _isSavingName = false;

  Future<void> _persistOperation(String operationDesc) async {
    final accountId = ref
        .read(authNotifierProvider.notifier)
        .selectedAccount
        ?.id;
    if (accountId != null) {
      await ref
          .read(authNotifierProvider.notifier)
          .updateOperation(operationDesc);
    }
  }

  @override
  void initState() {
    super.initState();
    // Profile should be pre-loaded during login. This is a safety net
    // in case profile page is accessed directly without going through login flow,
    // or if the auto-load hasn't completed yet.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final notifier = ref.read(profileNotifierProvider.notifier);
      if (ref.read(profileNotifierProvider) == null) {
        notifier.loadProfile();
      }
    });
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  void _startEditingName(String currentName) {
    _nameController.text = currentName;
    _nameError = null;
    setState(() => _isEditingName = true);
  }

  void _cancelEditingName() {
    setState(() {
      _isEditingName = false;
      _nameError = null;
    });
  }

  Future<void> _saveName(String currentName) async {
    final name = _nameController.text.trim();
    if (name.isEmpty) {
      setState(() => _nameError = 'Name cannot be empty');
      return;
    }
    if (name.length > kMaxNameLength) {
      setState(() => _nameError = 'Maximum $kMaxNameLength characters');
      return;
    }

    setState(() => _isSavingName = true);
    final newIdentity = IdentityData(
      fullName: name,
      givenName: ref.read(identityProvider)?.givenName,
      familyName: ref.read(identityProvider)?.familyName,
      dateOfBirth: ref.read(identityProvider)?.dateOfBirth,
      gender: ref.read(identityProvider)?.gender,
      nationality: ref.read(identityProvider)?.nationality,
      idCards: ref.read(identityProvider)?.idCards,
      contact: ref.read(identityProvider)?.contact,
      addresses: ref.read(identityProvider)?.addresses,
    );

    final success = await ref
        .read(profileNotifierProvider.notifier)
        .updateIdentity(newIdentity);

    if (mounted) {
      setState(() => _isSavingName = false);
      if (success) {
        setState(() => _isEditingName = false);
        // Persist operation to account metadata
        await _persistOperation('Updated Full Name');
        // Show top notification for operation feedback
        if (!mounted) return;
        final isPrivacyMode =
            ref.read(accountStyleProvider).displayMode ==
            SensitivityDisplayMode.hidePrivate;
        OperationNotification.show(
          context,
          message: OperationLogger.createNotification(
            section: LogSection.identity,
            action: LogAction.update,
            itemName: 'Identity',
            fieldName: 'Full Name',
            isPrivacyModeActive: isPrivacyMode,
          ),
        );
      } else {
        setState(() => _nameError = 'Failed to save');
      }
    }
  }

  Widget _buildNameDisplayRow(ThemeData theme, String fullName) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Full name centered, taking full width
        SizedBox(
          width: double.infinity,
          child: SelectableText(
            fullName,
            textAlign: TextAlign.center,
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        const SizedBox(height: 4),
        // Buttons centered below the name
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon: const Icon(Icons.copy, size: 20),
              tooltip: 'Copy Name',
              onPressed: () {
                Clipboard.setData(ClipboardData(text: fullName));
                ClipboardMonitorService.instance.notifySensitiveCopied();
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
              visualDensity: VisualDensity.compact,
            ),
            IconButton(
              icon: const Icon(Icons.edit_outlined, size: 20),
              tooltip: 'Edit Name',
              onPressed: () => _startEditingName(fullName),
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildNameEditRow(ThemeData theme, String currentName) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          width: 200,
          child: TextField(
            controller: _nameController,
            maxLength: kMaxNameLength,
            decoration: InputDecoration(
              counterText: '',
              hintText: 'Your name',
              errorText: _nameError,
              border: const OutlineInputBorder(),
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 12,
                vertical: 8,
              ),
            ),
            onChanged: (_) {
              if (_nameError != null) {
                setState(() => _nameError = null);
              }
            },
          ),
        ),
        const SizedBox(height: 8),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          mainAxisSize: MainAxisSize.min,
          children: [
            TextButton(
              onPressed: _isSavingName ? null : _cancelEditingName,
              child: const Text('Cancel'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _isSavingName ? null : () => _saveName(currentName),
              child: _isSavingName
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Save'),
            ),
          ],
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final profile = ref.watch(profileNotifierProvider);
    final identity = profile?.identity;
    final contact = identity?.contact;

    final fullName = identity?.fullName ?? 'Unnamed';

    return Scaffold(
      appBar: AppBar(
        title: const Text('Profile'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Avatar and name header
            Center(
              child: Column(
                children: [
                  CircleAvatar(
                    radius: 48,
                    backgroundColor: AppTheme.primaryColor,
                    child: Text(
                      fullName.isNotEmpty ? fullName.runes.first.toString().toUpperCase() : '?',
                      style: const TextStyle(
                        fontSize: 36,
                        color: Colors.white,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  if (_isEditingName)
                    _buildNameEditRow(theme, fullName)
                  else
                    _buildNameDisplayRow(theme, fullName),
                  const SizedBox(height: 4),
                  Text(
                    'Identity Profile',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ).animate().fadeIn(duration: 400.ms),

            const SizedBox(height: 32),

            // Contact Information
            _ContactSection(identity: identity, contact: contact)
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Identity Documents
            _IdCardSection(identity: identity, idCards: identity?.idCards)
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Addresses
            _AddressSection(identity: identity, addresses: identity?.addresses)
                .animate()
                .fadeIn(delay: 300.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 32),

            // Security notice
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: AppTheme.primaryColor.withValues(alpha: 0.05),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: AppTheme.primaryColor.withValues(alpha: 0.2),
                ),
              ),
              child: Row(
                children: [
                  const Icon(
                    Icons.lock_outline,
                    color: AppTheme.primaryColor,
                    size: 24,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Text(
                          'End-to-End Encrypted',
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                            color: AppTheme.primaryColor,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          'Your data is encrypted with AES-256-GCM',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ).animate().fadeIn(delay: 400.ms, duration: 400.ms),
          ],
        ),
      ),
    );
  }
}

// ============ Contact Section (using UnifiedFormSection) ============

class _ContactSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final ContactData? contact;

  const _ContactSection({required this.identity, required this.contact});

  @override
  ConsumerState<_ContactSection> createState() => _ContactSectionState();
}

class _ContactSectionState extends ConsumerState<_ContactSection> {
  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _loadData() {
    // No-op: contact items are now derived via ref.watch(contactItemsProvider)
    // in build(). Kept for compatibility with mixin/interface expectations.
  }

  ContactEntry _createContactFromValues(
    Map<String, String> values, {
    String? id,
  }) {
    return ContactEntry(
      id: id ?? generateEntryId(),
      title: values['contact.title']?.isEmpty == true
          ? ''
          : values['contact.title']!,
      type: values['contact.type']?.isEmpty == true
          ? 'email'
          : values['contact.type']!,
      value: values['contact.value']?.isEmpty == true
          ? '(no value)'
          : values['contact.value']!,
    );
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onContactDelete(ContactEntry contact) async {
    final contacts = ref.read(contactItemsProvider);
    final index = contacts.indexWhere((c) => c.id == contact.id);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'profile',
          itemType: 'contact',
          index: index,
          deletedItem: contact,
        );
  }

  void _onContactDidDelete(ContactEntry contact, int index) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = contact.id;
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.contactInformation,
        action: LogAction.delete,
        itemName: contact.title.isNotEmpty ? contact.title : contact.value,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(section: 'profile', itemType: 'contact', id: deletedId);
        _loadData();
      },
    );
  }

  void _onContactDeleteFailed(ContactEntry contact, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete contact',
      type: SnackBarType.error,
    );
  }

  Future<void> _onContactSave(
    ContactEntry? newItem,
    Map<String, String> values,
    ContactEntry? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final ContactEntry contactToSave;
    if (wasAdding) {
      contactToSave = newItem!;
    } else {
      contactToSave = _createContactFromValues(values, id: editingItem.id);
    }
    final itemName = contactToSave.title.isNotEmpty
        ? contactToSave.title
        : contactToSave.value;

    // Get current contacts from provider for building new identity
    final currentContacts = ref.read(contactItemsProvider);
    final updatedContacts = wasAdding
        ? [...currentContacts, contactToSave]
        : currentContacts.map((c) => c.id == editingItem.id ? contactToSave : c).toList();

    // Persist via provider
    try {
      final newIdentity = IdentityData(
        fullName: widget.identity?.fullName,
        givenName: widget.identity?.givenName,
        familyName: widget.identity?.familyName,
        dateOfBirth: widget.identity?.dateOfBirth,
        gender: widget.identity?.gender,
        nationality: widget.identity?.nationality,
        idCards: widget.identity?.idCards,
        contact: ContactData(entries: updatedContacts),
        addresses: widget.identity?.addresses,
      );
      await ref
          .read(profileNotifierProvider.notifier)
          .updateIdentity(newIdentity);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save contact: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.contactInformation,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: 'Contact',
          fieldName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final contacts = ref.watch(contactItemsProvider);
    final formSection = UnifiedFormSection<ContactEntry>(
      title: 'Contact Information',
      icon: Icons.contact_mail_outlined,
      items: contacts,
      maxVisibleItems: 3,
      itemFactory: _createContactFromValues,
      fieldDefs: [
        FormFieldDef(
          fieldId: 'contact.title',
          label: 'Title',
          hintText: 'e.g., Gmail, Work',
          sensitivity: ref.watch(effectiveSensitivityProvider('contact.title')),
        ),
        FormFieldDef(
          fieldId: 'contact.type',
          label: 'Type',
          sensitivity: ref.watch(effectiveSensitivityProvider('contact.type')),
        ),
        FormFieldDef(
          fieldId: 'contact.value',
          label: 'Value',
          sensitivity: ref.watch(effectiveSensitivityProvider('contact.value')),
        ),
      ],
      customFormBuilder:
          (context, theme, controllers, mode, onSubmit, onCancel, sensitivities) {
            final selectedType = controllers['contact.type']?.text ?? 'email';
            final valueSensitivity = sensitivities['contact.value'] ?? SensitivityLevel.public;
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  mode == 'adding' ? 'Add Contact' : 'Edit Contact',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['contact.title'],
                  maxLength: kMaxFieldLength,
                  decoration: const InputDecoration(
                    labelText: 'Title',
                    hintText: 'e.g., Gmail, Work',
                    counterText: '',
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                  ),
                ),
                const SizedBox(height: 12),
                DropdownButtonFormField<String>(
                  initialValue: selectedType.isEmpty ? 'email' : selectedType,
                  decoration: const InputDecoration(
                    labelText: 'Type',
                    border: OutlineInputBorder(),
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: 'email',
                      child: Text('email'),
                    ),
                    DropdownMenuItem(
                      value: 'phone',
                      child: Text('phone'),
                    ),
                  ],
                  onChanged: (v) {
                    controllers['contact.type']?.text = v ?? 'email';
                  },
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: controllers['contact.value'],
                  maxLength: kMaxFieldLength,
                  decoration: InputDecoration(
                    labelText: 'Value',
                    counterText: '',
                    border: const OutlineInputBorder(),
                    suffixIcon: Padding(
                      padding: const EdgeInsets.only(right: 8),
                      child: SensitivityTag(level: valueSensitivity),
                    ),
                  ),
                  keyboardType: selectedType == 'email'
                      ? TextInputType.emailAddress
                      : TextInputType.phone,
                ),
                const SizedBox(height: 16),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed: onCancel,
                      child: const Text('Cancel'),
                    ),
                    const SizedBox(width: 8),
                    FilledButton(
                      onPressed: onSubmit,
                      child: Text(mode == 'adding' ? 'Add' : 'Save'),
                    ),
                  ],
                ),
              ],
            );
          },
      onDelete: _onContactDelete,
      onSave: _onContactSave,
      // Note: contact.title is included in itemToMap for history but excluded from display via excludeFields.
      // Keys WITHOUT prefix - _autoBuildFields adds prefix; _populateControllersFromItem strips prefix.
      itemToMap: (c) => {'title': c.title, 'type': c.type, 'value': c.value},
      onCopyAll: (contact, text) async {
        unawaited(Clipboard.setData(ClipboardData(text: text)));
        unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
        showOverlaySnackBar(
          context,
          content: 'Copied to clipboard',
          type: SnackBarType.success,
        );
      },
      historyConfig: HistoryRecordingConfig<ContactEntry>(
        itemIdExtractor: (c) => c.id,
        fieldIdPrefix: 'contact',
      ),
      historyAwareOnSave: (newItem, values, editingItem, [oldValues]) async {
        if (editingItem == null) return;
        final accountId = ref
            .read(authNotifierProvider.notifier)
            .selectedAccountId;
        if (accountId == null) return;
        await ref
            .read(fieldHistoriesProvider.notifier)
            .recordSnapshot(
              accountId: accountId,
              itemId: editingItem.id,
              fieldIdPrefix: 'contact',
              allFieldValues: oldValues ?? {},
            );
      },
      showHistoryExpansion: true,
      historyFieldIdPrefix: 'contact',
      itemIdExtractor: (c) => c.id,
      onDidDelete: _onContactDidDelete,
      onDeleteFailed: _onContactDeleteFailed,
      displayItemBuilder: (contact, itemMap) => EntryCardWidget<ContactEntry>(
        item: contact,
        title: contact.title.isNotEmpty ? contact.title : contact.value,
        icon: contact.type == 'email'
            ? Icons.email_outlined
            : Icons.phone_outlined,
        itemId: contact.id,
        historyFieldId: 'contact',
        itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
        fieldPrefix: 'contact',
        excludeFields: const {'title'}, // title already used as card title
        formatAllFields: (c) => '${c.type}: ${c.value}',
        onDelete: (c) => _onContactDelete(c),
      ),
    );
    return formSection;
  }
}

// ============ ID Card Section (using UnifiedFormSection) ============

class _IdCardSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final List<IdCardData>? idCards;

  const _IdCardSection({required this.identity, required this.idCards});

  @override
  ConsumerState<_IdCardSection> createState() => _IdCardSectionState();
}

class _IdCardSectionState extends ConsumerState<_IdCardSection> {
  IdCardData _createIdCardFromValues(Map<String, String> values, {String? id}) {
    return IdCardData(
      id: id ?? generateEntryId(),
      title: values['idCard.title']?.isEmpty == true
          ? null
          : values['idCard.title'],
      number: values['idCard.number']?.isEmpty == true
          ? null
          : values['idCard.number'],
      holderName: values['idCard.holderName']?.isEmpty == true
          ? null
          : values['idCard.holderName'],
      country: values['idCard.country']?.isEmpty == true
          ? null
          : values['idCard.country'],
      issueDate: values['idCard.issueDate']?.isEmpty == true
          ? null
          : values['idCard.issueDate'],
      expiryDate: values['idCard.expiryDate']?.isEmpty == true
          ? null
          : values['idCard.expiryDate'],
    );
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onIdCardDelete(IdCardData card) async {
    final idCards = ref.read(idCardItemsProvider);
    final index = idCards.indexOf(card);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'profile',
          itemType: 'idCard',
          index: index,
          deletedItem: card,
        );
  }

  void _onIdCardDidDelete(IdCardData card, int index) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = card.id;
    final itemName = card.title ?? card.number ?? 'ID Card';
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.idCard,
        action: LogAction.delete,
        itemName: itemName,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(section: 'profile', itemType: 'idCard', id: deletedId);
      },
    );
  }

  void _onIdCardDeleteFailed(IdCardData card, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete ID card',
      type: SnackBarType.error,
    );
  }

  Future<void> _onIdCardSave(
    IdCardData? newItem,
    Map<String, String> values,
    IdCardData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final IdCardData cardToSave;
    if (wasAdding) {
      cardToSave = newItem!;
    } else {
      cardToSave = _createIdCardFromValues(values, id: editingItem.id);
    }
    final itemName = cardToSave.title ?? cardToSave.number ?? 'ID Card';

    // Persist via provider - UI will update automatically via idCardItemsProvider watch
    try {
      final idCards = ref.read(idCardItemsProvider);
      final newIdCards = wasAdding
          ? [...idCards, cardToSave]
          : idCards.map((c) => c.id == editingItem.id ? cardToSave : c).toList();

      final identity = IdentityData(
        fullName: widget.identity?.fullName,
        givenName: widget.identity?.givenName,
        familyName: widget.identity?.familyName,
        dateOfBirth: widget.identity?.dateOfBirth,
        gender: widget.identity?.gender,
        nationality: widget.identity?.nationality,
        idCards: newIdCards,
        contact: widget.identity?.contact,
        addresses: widget.identity?.addresses,
      );
      await ref.read(profileNotifierProvider.notifier).updateIdentity(identity);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save ID card: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.idCard,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: 'ID Card',
          fieldName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final idCards = ref.watch(idCardItemsProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UnifiedFormSection<IdCardData>(
          title: 'Identity Documents',
          icon: Icons.badge_outlined,
          items: idCards,
          maxVisibleItems: 3,
          itemFactory: _createIdCardFromValues,
          fieldDefs: [
            FormFieldDef(
              fieldId: 'idCard.title',
              label: 'Title',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.title')),
            ),
            FormFieldDef(
              fieldId: 'idCard.number',
              label: 'ID Number',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.number')),
            ),
            FormFieldDef(
              fieldId: 'idCard.holderName',
              label: 'Holder Name',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.holderName')),
            ),
            FormFieldDef(
              fieldId: 'idCard.country',
              label: 'Country',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.country')),
            ),
            FormFieldDef(
              fieldId: 'idCard.issueDate',
              label: 'Issue Date',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.issueDate')),
            ),
            FormFieldDef(
              fieldId: 'idCard.expiryDate',
              label: 'Expiry Date',
              sensitivity: ref.watch(effectiveSensitivityProvider('idCard.expiryDate')),
            ),
          ],
          displayItemBuilder: (card, itemMap) => EntryCardWidget<IdCardData>(
            item: card,
            title: card.title ?? 'ID Card',
            icon: Icons.badge_outlined,
            itemId: card.id,
            historyFieldId: 'idCard',
            itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
            fieldPrefix: 'idCard',
            excludeFields: const {'title'},
            formatAllFields: (c) => '${c.entryType}\n${c.toFormattedString()}',
            onDelete: (c) => _onIdCardDelete(c),
          ),
          onDelete: _onIdCardDelete,
          onSave: _onIdCardSave,
          // Keys without prefix - _autoBuildFields will add 'idCard.' prefix via fieldPrefix.
          itemToMap: (c) => {
            'title': c.title ?? '',
            'number': c.number ?? '',
            'holderName': c.holderName ?? '',
            'country': c.country ?? '',
            'issueDate': c.issueDate ?? '',
            'expiryDate': c.expiryDate ?? '',
          },
          onCopyAll: (card, text) async {
            unawaited(Clipboard.setData(ClipboardData(text: text)));
            unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
            showOverlaySnackBar(
              context,
              content: 'Copied to clipboard',
              type: SnackBarType.success,
            );
          },
          historyConfig: HistoryRecordingConfig<IdCardData>(
            itemIdExtractor: (c) => c.id,
            fieldIdPrefix: 'idCard',
          ),
          historyAwareOnSave:
              (newItem, values, editingItem, [oldValues]) async {
                if (editingItem == null) return;
                final accountId = ref
                    .read(authNotifierProvider.notifier)
                    .selectedAccountId;
                if (accountId == null) return;
                await ref
                    .read(fieldHistoriesProvider.notifier)
                    .recordSnapshot(
                      accountId: accountId,
                      itemId: editingItem.id,
                      fieldIdPrefix: 'idCard',
                      allFieldValues: oldValues ?? {},
                    );
              },
          showHistoryExpansion: true,
          historyFieldIdPrefix: 'idCard',
          onDidDelete: _onIdCardDidDelete,
          onDeleteFailed: _onIdCardDeleteFailed,
        ),
      ],
    );
  }
}

// ============ Address Section (using UnifiedFormSection) ============

class _AddressSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final List<AddressData>? addresses;

  const _AddressSection({required this.identity, required this.addresses});

  @override
  ConsumerState<_AddressSection> createState() => _AddressSectionState();
}

class _AddressSectionState extends ConsumerState<_AddressSection> {
  AddressData _createAddressFromValues(
    Map<String, String> values, {
    String? id,
  }) {
    return AddressData(
      id: id ?? generateEntryId(),
      title: values['address.title']?.isEmpty == true
          ? null
          : values['address.title'],
      street: values['address.street']?.isEmpty == true
          ? null
          : values['address.street'],
      city: values['address.city']?.isEmpty == true
          ? null
          : values['address.city'],
      postalCode: values['address.postalCode']?.isEmpty == true
          ? null
          : values['address.postalCode'],
      country: values['address.country']?.isEmpty == true
          ? null
          : values['address.country'],
    );
  }

  /// Thin passthrough - softDelete is the only persistence operation.
  /// Optimistic UI, rollback, and notification are handled by handleDelete via callbacks.
  Future<void> _onAddressDelete(AddressData address) async {
    final addresses = ref.read(addressItemsProvider);
    final index = addresses.indexOf(address);
    if (index == -1) return;
    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: 'profile',
          itemType: 'address',
          index: index,
          deletedItem: address,
        );
  }

  void _onAddressDidDelete(AddressData address, int index) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;
    final deletedId = address.id;
    final itemName = address.title ?? 'Address';
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSection.address,
        action: LogAction.delete,
        itemName: itemName,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(section: 'profile', itemType: 'address', id: deletedId);
      },
    );
  }

  void _onAddressDeleteFailed(AddressData address, int index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete address',
      type: SnackBarType.error,
    );
  }

  Future<void> _onAddressSave(
    AddressData? newItem,
    Map<String, String> values,
    AddressData? editingItem,
  ) async {
    final wasAdding = editingItem == null;
    final AddressData addressToSave;
    if (wasAdding) {
      addressToSave = newItem!;
    } else {
      addressToSave = _createAddressFromValues(values, id: editingItem.id);
    }
    final itemName = addressToSave.title ?? 'Address';

    // Get current addresses from provider for building new list
    final currentAddresses = ref.read(addressItemsProvider);

    // Persist via provider - UnifiedFormSection handles optimistic UI via ref.watch
    try {
      final identity = IdentityData(
        fullName: widget.identity?.fullName,
        givenName: widget.identity?.givenName,
        familyName: widget.identity?.familyName,
        dateOfBirth: widget.identity?.dateOfBirth,
        gender: widget.identity?.gender,
        nationality: widget.identity?.nationality,
        idCards: widget.identity?.idCards,
        contact: widget.identity?.contact,
        addresses: currentAddresses,
      );
      await ref.read(profileNotifierProvider.notifier).updateIdentity(identity);
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(context, content: 'Failed to save address: $e', type: SnackBarType.error);
      }
      return;
    }

    if (mounted) {
      final isPrivacyMode =
          ref.read(accountStyleProvider).displayMode ==
          SensitivityDisplayMode.hidePrivate;
      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: LogSection.address,
          action: wasAdding ? LogAction.create : LogAction.update,
          itemName: 'Address',
          fieldName: itemName,
          isPrivacyModeActive: isPrivacyMode,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final addresses = ref.watch(addressItemsProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        UnifiedFormSection<AddressData>(
          title: 'Addresses',
          icon: Icons.location_on_outlined,
          items: addresses,
          maxVisibleItems: 3,
          itemFactory: _createAddressFromValues,
          fieldDefs: [
            FormFieldDef(
              fieldId: 'address.title',
              label: 'Title',
              sensitivity: ref.watch(effectiveSensitivityProvider('address.title')),
            ),
            FormFieldDef(
              fieldId: 'address.street',
              label: 'Street',
              sensitivity: ref.watch(effectiveSensitivityProvider('address.street')),
            ),
            FormFieldDef(
              fieldId: 'address.city',
              label: 'City',
              sensitivity: ref.watch(effectiveSensitivityProvider('address.city')),
            ),
            FormFieldDef(
              fieldId: 'address.postalCode',
              label: 'Postal Code',
              sensitivity: ref.watch(effectiveSensitivityProvider('address.postalCode')),
            ),
            FormFieldDef(
              fieldId: 'address.country',
              label: 'Country',
              sensitivity: ref.watch(effectiveSensitivityProvider('address.country')),
            ),
          ],
          displayItemBuilder: (address, itemMap) =>
              EntryCardWidget<AddressData>(
                item: address,
                title: address.title ?? 'Address',
                icon: Icons.home_outlined,
                itemId: address.id,
                historyFieldId: 'address',
                itemData: itemMap.map((k, v) => MapEntry(k, v as dynamic)),
                fieldPrefix: 'address',
                excludeFields: const {
                  'title',
                }, // title already used as card title
                onDelete: (a) => _onAddressDelete(a),
              ),
          onDelete: _onAddressDelete,
          onSave: _onAddressSave,
          // Keys without prefix - _autoBuildFields will add 'address.' prefix via fieldPrefix.
          itemToMap: (a) => {
            'title': a.title ?? '',
            'street': a.street ?? '',
            'city': a.city ?? '',
            'postalCode': a.postalCode ?? '',
            'country': a.country ?? '',
          },
          onCopyAll: (address, text) async {
            unawaited(Clipboard.setData(ClipboardData(text: text)));
            unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
            showOverlaySnackBar(
              context,
              content: 'Copied to clipboard',
              type: SnackBarType.success,
            );
          },
          historyConfig: HistoryRecordingConfig<AddressData>(
            itemIdExtractor: (a) => a.id,
            fieldIdPrefix: 'address',
          ),
          historyAwareOnSave:
              (newItem, values, editingItem, [oldValues]) async {
                if (editingItem == null) return;
                final accountId = ref
                    .read(authNotifierProvider.notifier)
                    .selectedAccountId;
                if (accountId == null) return;
                await ref
                    .read(fieldHistoriesProvider.notifier)
                    .recordSnapshot(
                      accountId: accountId,
                      itemId: editingItem.id,
                      fieldIdPrefix: 'address',
                      allFieldValues: oldValues ?? {},
                    );
              },
          showHistoryExpansion: true,
          historyFieldIdPrefix: 'address',
          itemIdExtractor: (a) => a.id,
          onDidDelete: _onAddressDidDelete,
          onDeleteFailed: _onAddressDeleteFailed,
        ),
      ],
    );
  }
}
