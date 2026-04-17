import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, SnackBarType, showOverlaySnackBar;
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show SensitivityLevel, sensitivitySettingsProvider, SensitivityDisplayMode;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/widgets/section_card.dart'
    show CollapsibleSectionCard;
import 'package:solosoul_flutter/presentation/widgets/unified_form_section.dart'
    show showDeleteConfirmationDialog;
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/presentation/widgets/responsive_label_field.dart'
    show ResponsiveLabelField, LabelValueField;
import 'package:solosoul_flutter/core/services/log_section_config.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show authNotifierProvider, sensitivePageAccessProvider;

/// Standalone helper to verify password for restricted fields.
/// Returns true if field is not restricted OR if verification succeeded.
Future<bool> verifyPasswordForRestrictedField({
  required BuildContext context,
  required WidgetRef ref,
  required String fieldId,
}) async {
  final settings = ref.read(sensitivitySettingsProvider);
  final level = settings.getFieldLevel(fieldId);

  // If not restricted, allow without verification
  if (level != SensitivityLevel.restricted) {
    return true;
  }

  // Check if user was verified within the last 1 minute (password cache)
  final sensitiveAccess = ref.read(sensitivePageAccessProvider);
  final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
  final hasRecentVerification = sensitiveAccess.lastVerified != null &&
      sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);

  if (hasRecentVerification) {
    return true;
  }

  // Show password dialog
  final authNotifier = ref.read(authNotifierProvider.notifier);
  final password = await showPasswordVerificationDialog(
    context: context,
    ref: ref,
    onVerify: authNotifier.verifyPasswordForSensitiveData,
  );
  if (password == null) {
    return false;
  }

  // Mark as verified in shared sensitive page access
  ref.read(sensitivePageAccessProvider.notifier).markVerified();
  return true;
}

/// Helper class to track original indices for soft delete
/// When items are filtered (e.g., only showing active items), we need to
/// remember their original indices in the full list to call softDelete correctly
class _EntryWithIndex<T> {
  final T entry;
  final int originalIndex;

  _EntryWithIndex({required this.entry, required this.originalIndex});
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
      givenName: ref.read(profileNotifierProvider)?.identity?.givenName,
      familyName: ref.read(profileNotifierProvider)?.identity?.familyName,
      dateOfBirth: ref.read(profileNotifierProvider)?.identity?.dateOfBirth,
      gender: ref.read(profileNotifierProvider)?.identity?.gender,
      nationality: ref.read(profileNotifierProvider)?.identity?.nationality,
      idCards: ref.read(profileNotifierProvider)?.identity?.idCards,
      contact: ref.read(profileNotifierProvider)?.identity?.contact,
      addresses: ref.read(profileNotifierProvider)?.identity?.addresses,
    );

    final success = await ref
        .read(profileNotifierProvider.notifier)
        .updateIdentity(newIdentity);

    if (mounted) {
      setState(() => _isSavingName = false);
      if (success) {
        setState(() => _isEditingName = false);
        // Show top notification for operation feedback
        final isPrivacyMode =
            ref.read(sensitivitySettingsProvider).displayMode ==
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
                showOverlaySnackBar(context, content: 'Name copied!');
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
      appBar: AppBar(title: const Text('Profile')),
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
                      fullName.isNotEmpty ? fullName[0].toUpperCase() : '?',
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
                  Icon(
                    Icons.lock_outline,
                    color: AppTheme.primaryColor,
                    size: 24,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
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

class _ContactSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final ContactData? contact;

  const _ContactSection({required this.identity, required this.contact});

  @override
  ConsumerState<_ContactSection> createState() => _ContactSectionState();
}

class _ContactSectionState extends ConsumerState<_ContactSection> {
  // 'idle' | 'adding' | 'editing'
  String _mode = 'idle';
  int _editingIndex = -1;
  late List<_EntryWithIndex<ContactEntry>> _entries;

  // Form controllers for inline editing
  final _labelController = TextEditingController();
  final _valueController = TextEditingController();
  String _selectedType = 'email';
  bool _isSaving = false;

  static const _types = ['email', 'phone'];

  @override
  void initState() {
    super.initState();
    _entries = [
      ...?(widget.contact?.entries.asMap().entries.map(
        (mapEntry) => _EntryWithIndex(
          entry: mapEntry.value.copyWith(),
          originalIndex: mapEntry.key,
        ),
      )),
    ];
  }

  @override
  void didUpdateWidget(_ContactSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Reload data when parent passes updated contact
    // Filter to only active (non-deleted) entries while preserving originalIndex
    if (widget.contact != oldWidget.contact) {
      _entries = [
        for (var i = 0; i < (widget.contact?.entries.length ?? 0); i++)
          if (widget.contact != null && !widget.contact!.entries[i].isDeleted)
            _EntryWithIndex(
              entry: widget.contact!.entries[i].copyWith(),
              originalIndex: i,
            ),
      ];
    }
  }

  @override
  void dispose() {
    _labelController.dispose();
    _valueController.dispose();
    super.dispose();
  }

  void _startAdding() {
    setState(() {
      _mode = 'adding';
      _labelController.clear();
      _valueController.clear();
      _selectedType = 'email';
    });
  }

  void _startEditing(int index) async {
    // Check if the contact being edited contains restricted fields
    final entry = _entries[index].entry;
    final fieldId = entry.type == 'email' ? 'contact.email' : 'contact.phone';

    // Verify password for restricted fields
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: fieldId);
    if (!verified) return;

    if (!mounted) return;
    setState(() {
      _mode = 'editing';
      _editingIndex = index;
      _labelController.text = entry.label;
      // Show empty if stored as "(no value)"
      _valueController.text = entry.value == '(no value)' ? '' : entry.value;
      _selectedType = entry.type;
    });
  }

  void _cancelEdit() {
    setState(() {
      _mode = 'idle';
      _editingIndex = -1;
    });
  }

  void _deleteEntry(int index) async {
    final deleted = _entries[index];
    final fieldId = deleted.entry.type == 'email' ? 'contact.email' : 'contact.phone';

    // Verify password for restricted fields BEFORE showing delete confirmation
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: fieldId);
    if (!verified) return;

    if (!mounted) return;

    final confirm = await showDeleteConfirmationDialog(
      context: context,
      itemName: deleted.entry.label.isNotEmpty
          ? '${deleted.entry.label} - ${deleted.entry.value}'
          : deleted.entry.value,
      itemType: 'Contact',
    );
    if (!confirm) return;

    final itemName = deleted.entry.label.isNotEmpty
        ? '${deleted.entry.label} - ${deleted.entry.value}'
        : deleted.entry.value;

    // Mark as soft deleted and show Undo snackbar
    _softDeleteWithUndo(
      section: 'profile',
      itemType: 'contact',
      index: deleted.originalIndex,
      deletedItem: deleted.entry,
      itemName: itemName,
    );
  }

  void _softDeleteWithUndo({
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
    required String itemName,
  }) async {
    _cancelEdit();

    // Store values needed after await
    final isMounted = mounted;
    final originalIndex = index;

    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: section,
          itemType: itemType,
          index: index,
          deletedItem: deletedItem,
        );

    if (!isMounted) return;

    // Remove from local list (find by originalIndex) - create new list to ensure state change is detected
    setState(() {
      _entries = _entries
          .where((e) => e.originalIndex != originalIndex)
          .toList();
    });

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.delete,
        itemName: itemName,
        fieldName: itemName,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(
              section: section,
              itemType: itemType,
              index: originalIndex,
            );
      },
    );
  }

  Future<void> _saveContacts({
    LogAction operationType = LogAction.update,
    String? itemName,
  }) async {
    setState(() => _isSaving = true);

    final newIdentity = IdentityData(
      fullName: widget.identity?.fullName,
      givenName: widget.identity?.givenName,
      familyName: widget.identity?.familyName,
      dateOfBirth: widget.identity?.dateOfBirth,
      gender: widget.identity?.gender,
      nationality: widget.identity?.nationality,
      idCards: widget.identity?.idCards,
      contact: ContactData(entries: _entries.map((e) => e.entry).toList()),
      addresses: widget.identity?.addresses,
    );

    final success = await ref
        .read(profileNotifierProvider.notifier)
        .updateIdentity(newIdentity);

    if (mounted) {
      setState(() => _isSaving = false);
      if (success) {
        _cancelEdit();
        // Show operation notification
        final isPrivacyMode =
            ref.read(sensitivitySettingsProvider).displayMode ==
            SensitivityDisplayMode.hidePrivate;
        final displayName =
            itemName ??
            (_entries.isNotEmpty ? _entries.last.entry.label : 'Contact');
        OperationNotification.show(
          context,
          message: OperationLogger.createNotification(
            section: LogSection.contactInformation,
            action: operationType,
            itemName: 'Contact',
            fieldName: displayName,
            isPrivacyModeActive: isPrivacyMode,
          ),
        );
      } else {
        showOverlaySnackBar(
          context,
          content: 'Failed to save',
          type: SnackBarType.error,
        );
      }
    }
  }

  void _submitForm() {
    final label = _labelController.text.trim();
    final value = _valueController.text.trim();

    if (label.isEmpty && value.isEmpty) {
      showOverlaySnackBar(
        context,
        content: 'Please fill in at least label or value',
      );
      return;
    }

    final entry = ContactEntry(
      label: label,
      type: _selectedType,
      value: value.isEmpty ? '(no value)' : value,
    );

    // Capture operation type before state reset
    final wasAdding = _mode == 'adding';
    final originalIndex = wasAdding
        ? -1
        : _entries[_editingIndex].originalIndex;

    setState(() {
      if (_mode == 'adding') {
        _entries.add(
          _EntryWithIndex(
            entry: entry,
            originalIndex: widget.contact?.entries.length ?? 0,
          ),
        );
      } else if (_mode == 'editing') {
        _entries[_editingIndex] = _EntryWithIndex(
          entry: entry,
          originalIndex: originalIndex,
        );
      }
      // Sort: phone before email, then by label
      _entries.sort((a, b) {
        final typeOrder = {'phone': 0, 'email': 1}
            .putIfAbsent(a.entry.type, () => 2)
            .compareTo(
              {'phone': 0, 'email': 1}.putIfAbsent(b.entry.type, () => 2),
            );
        if (typeOrder != 0) return typeOrder;
        return a.entry.label.compareTo(b.entry.label);
      });
      _mode = 'idle';
      _editingIndex = -1;
    });

    // Build a descriptive name that includes type and value for better notification
    final contactDisplayName = entry.type.isNotEmpty && entry.value.isNotEmpty
        ? '${entry.type} - ${entry.label.isNotEmpty ? entry.label : entry.value}'
        : (entry.label.isNotEmpty ? entry.label : 'Contact');
    _saveContacts(
      operationType: wasAdding ? LogAction.create : LogAction.update,
      itemName: contactDisplayName,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // Defensive filter: use only non-deleted entries for display
    // This handles edge cases where didUpdateWidget might not have fired
    final displayEntries = _entries.where((e) => !e.entry.isDeleted).toList();
    final hasEntries = displayEntries.isNotEmpty;
    final isEditing = _mode == 'adding' || _mode == 'editing';

    return CollapsibleSectionCard(
      title: 'Contact Information',
      icon: Icons.contact_mail_outlined,
      maxVisibleItems: 3,
      actionIcon: Icons.add,
      onAction: _startAdding,
      footer: isEditing ? _buildInlineForm(theme) : null,
      children: hasEntries
          ? _buildContactTiles(theme, displayEntries)
          : [_buildEmptyState(theme)],
    );
  }

  Widget _buildEmptyState(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            Icon(
              Icons.contact_mail_outlined,
              size: 40,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 8),
            Text(
              'No contacts saved',
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 12),
            TextButton.icon(
              onPressed: _startAdding,
              icon: const Icon(Icons.add),
              label: const Text('Add Contact'),
            ),
          ],
        ),
      ),
    );
  }

  /// Returns individual contact tiles for CollapsibleSectionCard
  List<Widget> _buildContactTiles(
    ThemeData theme,
    List<_EntryWithIndex<ContactEntry>> displayEntries,
  ) {
    return [
      for (var i = 0; i < displayEntries.length; i++)
        Column(
          children: [
            _ContactEntryTile(
              entry: displayEntries[i].entry,
              onCopy: () {
                Clipboard.setData(ClipboardData(text: displayEntries[i].entry.value));
                showOverlaySnackBar(context, content: 'Copied!');
              },
              onEdit: () => _startEditing(i),
              onDelete: () => _deleteEntry(i),
            ),
            if (i < displayEntries.length - 1) const Divider(height: 1),
          ],
        ),
    ];
  }

  Widget _buildInlineForm(ThemeData theme) {
    final isAdding = _mode == 'adding';
    final title = isAdding ? 'Add Contact' : 'Edit Contact';
    final settings = ref.watch(sensitivitySettingsProvider);
    final valueFieldId = _selectedType == 'email'
        ? 'contact.email'
        : 'contact.phone';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: theme.textTheme.titleSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _labelController,
                maxLength: kMaxFieldLength,
                decoration: const InputDecoration(
                  labelText: 'Label',
                  hintText: 'e.g., Gmail, Work',
                  counterText: '',
                  border: OutlineInputBorder(),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: DropdownButtonFormField<String>(
                value: _selectedType,
                decoration: const InputDecoration(
                  labelText: 'Type',
                  border: OutlineInputBorder(),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                ),
                items: _types
                    .map((t) => DropdownMenuItem(value: t, child: Text(t)))
                    .toList(),
                onChanged: (v) => setState(() => _selectedType = v!),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _valueController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: _selectedType == 'email' ? 'Email' : 'Phone',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel(valueFieldId) ??
                    SensitivityLevel.public,
              ),
            ),
          ),
          keyboardType: _selectedType == 'email'
              ? TextInputType.emailAddress
              : TextInputType.phone,
        ),
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: _isSaving ? null : _cancelEdit,
              child: const Text('Cancel'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _isSaving ? null : _submitForm,
              child: _isSaving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Text(isAdding ? 'Add' : 'Save'),
            ),
          ],
        ),
      ],
    );
  }
}

class _ContactEntryTile extends ConsumerWidget {
  final ContactEntry entry;
  final VoidCallback onCopy;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _ContactEntryTile({
    required this.entry,
    required this.onCopy,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final fieldId = entry.type == 'email' ? 'contact.email' : 'contact.phone';

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(
              entry.type == 'email'
                  ? Icons.email_outlined
                  : Icons.phone_outlined,
              size: 20,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: ResponsiveLabelField(
              fields: [
                LabelValueField(
                  label: 'Label',
                  value: entry.label,
                ),
                LabelValueField(
                  label: 'Type',
                  value: entry.type,
                ),
                LabelValueField(
                  label: 'Value',
                  value: entry.value,
                  fieldId: fieldId,
                  isSensitive: true,
                ),
              ],
              labelValueSpacing: 4,
              layoutAxis: Axis.vertical,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.copy, size: 20),
            tooltip: 'Copy',
            onPressed: onCopy,
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined, size: 20),
            tooltip: 'Edit',
            onPressed: onEdit,
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 20),
            tooltip: 'Delete',
            onPressed: onDelete,
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
    );
  }
}

class _IdCardSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final List<IdCardData>? idCards;

  const _IdCardSection({required this.identity, required this.idCards});

  @override
  ConsumerState<_IdCardSection> createState() => _IdCardSectionState();
}

class _IdCardSectionState extends ConsumerState<_IdCardSection> {
  // 'idle' | 'adding' | 'editing'
  String _mode = 'idle';
  int _editingIndex = -1;
  late List<_EntryWithIndex<IdCardData>> _idCards;

  final _labelController = TextEditingController();
  final _numberController = TextEditingController();
  final _holderNameController = TextEditingController();
  final _countryController = TextEditingController();
  final _issueDateController = TextEditingController();
  final _expiryDateController = TextEditingController();
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _idCards = [
      ...?(widget.idCards?.asMap().entries.map(
        (mapEntry) => _EntryWithIndex(
          entry: mapEntry.value.copyWith(),
          originalIndex: mapEntry.key,
        ),
      )),
    ];
  }

  @override
  void didUpdateWidget(_IdCardSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Reload data when parent passes updated idCards
    // Filter to only active (non-deleted) entries while preserving originalIndex
    if (widget.idCards != oldWidget.idCards) {
      _idCards = [
        for (var i = 0; i < (widget.idCards?.length ?? 0); i++)
          if (widget.idCards != null && !widget.idCards![i].isDeleted)
            _EntryWithIndex(
              entry: widget.idCards![i].copyWith(),
              originalIndex: i,
            ),
      ];
    }
  }

  @override
  void dispose() {
    _labelController.dispose();
    _numberController.dispose();
    _holderNameController.dispose();
    _countryController.dispose();
    _issueDateController.dispose();
    _expiryDateController.dispose();
    super.dispose();
  }

  void _startAdding() {
    _clearControllers();
    setState(() {
      _mode = 'adding';
      _editingIndex = -1;
    });
  }

  void _startEditing(int index) async {
    final card = _idCards[index].entry;

    // Verify password for restricted fields (idCard.number is restricted)
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: 'idCard.number');
    if (!verified) return;

    if (!mounted) return;
    _labelController.text = card.label ?? '';
    _numberController.text = card.number ?? '';
    _holderNameController.text = card.holderName ?? '';
    _countryController.text = card.country ?? '';
    _issueDateController.text = card.issueDate ?? '';
    _expiryDateController.text = card.expiryDate ?? '';
    setState(() {
      _mode = 'editing';
      _editingIndex = index;
    });
  }

  void _cancelEdit() {
    setState(() {
      _mode = 'idle';
      _editingIndex = -1;
    });
  }

  void _clearControllers() {
    _labelController.clear();
    _numberController.clear();
    _holderNameController.clear();
    _countryController.clear();
    _issueDateController.clear();
    _expiryDateController.clear();
  }

  void _deleteEntry(int index) async {
    final deletedCard = _idCards[index];

    // Verify password for restricted fields BEFORE showing delete confirmation
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: 'idCard.number');
    if (!verified) return;

    if (!mounted) return;

    final confirm = await showDeleteConfirmationDialog(
      context: context,
      itemName: deletedCard.entry.label ?? deletedCard.entry.number ?? 'ID Card',
      itemType: 'ID Card',
    );
    if (!confirm) return;

    final itemName =
        deletedCard.entry.label ?? deletedCard.entry.number ?? 'ID Card';

    // Mark as soft deleted and show Undo snackbar
    _softDeleteWithUndo(
      section: 'profile',
      itemType: 'idCard',
      index: deletedCard.originalIndex,
      deletedItem: deletedCard.entry,
      itemName: itemName,
    );
  }

  void _softDeleteWithUndo({
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
    required String itemName,
  }) async {
    _cancelEdit();

    // Store values needed after await
    final isMounted = mounted;
    final originalIndex = index;

    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: section,
          itemType: itemType,
          index: index,
          deletedItem: deletedItem,
        );

    if (!isMounted) return;

    // Remove from local list (find by originalIndex)
    setState(() {
      _idCards = _idCards
          .where((c) => c.originalIndex != originalIndex)
          .toList();
    });

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.delete,
        itemName: itemName,
        fieldName: itemName,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(
              section: section,
              itemType: itemType,
              index: originalIndex,
            );
      },
    );
  }

  Future<void> _saveIdCards({
    LogAction operationType = LogAction.update,
    String? itemName,
  }) async {
    setState(() => _isSaving = true);

    final identity = IdentityData(
      fullName: widget.identity?.fullName,
      givenName: widget.identity?.givenName,
      familyName: widget.identity?.familyName,
      dateOfBirth: widget.identity?.dateOfBirth,
      gender: widget.identity?.gender,
      nationality: widget.identity?.nationality,
      idCards: _idCards.isEmpty ? null : _idCards.map((c) => c.entry).toList(),
      contact: widget.identity?.contact,
      addresses: widget.identity?.addresses,
    );

    final success = await ref
        .read(profileNotifierProvider.notifier)
        .updateIdentity(identity);

    if (mounted) {
      setState(() => _isSaving = false);
      if (success) {
        _cancelEdit();
        // Show operation notification
        final isPrivacyMode =
            ref.read(sensitivitySettingsProvider).displayMode ==
            SensitivityDisplayMode.hidePrivate;
        OperationNotification.show(
          context,
          message: OperationLogger.createNotification(
            section: LogSection.idCard,
            action: operationType,
            itemName: 'ID Card',
            fieldName: itemName ?? 'Document',
            isPrivacyModeActive: isPrivacyMode,
          ),
        );
      } else {
        showOverlaySnackBar(
          context,
          content: 'Failed to save',
          type: SnackBarType.error,
        );
      }
    }
  }

  void _submitForm() {
    final card = IdCardData(
      label: _labelController.text.isEmpty ? null : _labelController.text,
      number: _numberController.text.isEmpty ? null : _numberController.text,
      holderName: _holderNameController.text.isEmpty
          ? null
          : _holderNameController.text,
      country: _countryController.text.isEmpty ? null : _countryController.text,
      issueDate: _issueDateController.text.isEmpty
          ? null
          : _issueDateController.text,
      expiryDate: _expiryDateController.text.isEmpty
          ? null
          : _expiryDateController.text,
    );

    // Capture operation type before state reset
    final wasAdding = _mode == 'adding';
    final cardLabel = card.label ?? 'ID Card';
    final originalIndex = wasAdding
        ? -1
        : _idCards[_editingIndex].originalIndex;

    setState(() {
      if (_mode == 'adding') {
        _idCards.add(
          _EntryWithIndex(
            entry: card,
            originalIndex: widget.idCards?.length ?? 0,
          ),
        );
      } else if (_mode == 'editing') {
        _idCards[_editingIndex] = _EntryWithIndex(
          entry: card,
          originalIndex: originalIndex,
        );
      }
      _mode = 'idle';
      _editingIndex = -1;
    });

    _saveIdCards(
      operationType: wasAdding ? LogAction.create : LogAction.update,
      itemName: cardLabel,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // Defensive filter: use only non-deleted entries for display
    final displayIdCards = _idCards.where((c) => !c.entry.isDeleted).toList();
    final hasEntries = displayIdCards.isNotEmpty;
    final isEditing = _mode == 'adding' || _mode == 'editing';

    return CollapsibleSectionCard(
      title: 'Identity Documents',
      icon: Icons.badge_outlined,
      maxVisibleItems: 3,
      actionIcon: Icons.add,
      onAction: _startAdding,
      footer: isEditing ? _buildInlineForm(theme) : null,
      children: hasEntries
          ? _buildIdCardTiles(theme, displayIdCards)
          : [_buildEmptyState(theme)],
    );
  }

  Widget _buildEmptyState(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            Icon(
              Icons.badge_outlined,
              size: 40,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 8),
            Text(
              'No ID cards saved',
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 12),
            TextButton.icon(
              onPressed: _startAdding,
              icon: const Icon(Icons.add),
              label: const Text('Add ID Card'),
            ),
          ],
        ),
      ),
    );
  }

  /// Returns individual ID card tiles for CollapsibleSectionCard
  List<Widget> _buildIdCardTiles(
    ThemeData theme,
    List<_EntryWithIndex<IdCardData>> displayIdCards,
  ) {
    return [
      for (var i = 0; i < displayIdCards.length; i++)
        Column(
          children: [
            _IdCardTile(
              card: displayIdCards[i].entry,
              onEdit: () => _startEditing(i),
              onDelete: () => _deleteEntry(i),
            ),
            if (i < displayIdCards.length - 1) const Divider(height: 1),
          ],
        ),
    ];
  }

  Widget _buildInlineForm(ThemeData theme) {
    final isAdding = _mode == 'adding';
    final title = isAdding ? 'Add ID Card' : 'Edit ID Card';
    final settings = ref.watch(sensitivitySettingsProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: theme.textTheme.titleSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _labelController,
          maxLength: kMaxFieldLength,
          decoration: const InputDecoration(
            labelText: 'Label (e.g., National ID, Driver\'s License)',
            counterText: '',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _numberController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: 'ID Number',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel('idCard.number') ??
                    SensitivityLevel.public,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _holderNameController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: 'Holder Name',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel('idCard.holderName') ??
                    SensitivityLevel.public,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _countryController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: 'Country',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel('idCard.country') ??
                    SensitivityLevel.public,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _issueDateController,
                maxLength: kMaxFieldLength,
                decoration: InputDecoration(
                  labelText: 'Issue Date',
                  hintText: 'YYYY-MM-DD',
                  counterText: '',
                  border: const OutlineInputBorder(),
                  suffixIcon: Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: SensitivityTag(
                      level:
                          settings.getFieldLevel('idCard.issueDate') ??
                          SensitivityLevel.public,
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: _expiryDateController,
                maxLength: kMaxFieldLength,
                decoration: InputDecoration(
                  labelText: 'Expiry Date',
                  hintText: 'YYYY-MM-DD',
                  counterText: '',
                  border: const OutlineInputBorder(),
                  suffixIcon: Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: SensitivityTag(
                      level:
                          settings.getFieldLevel('idCard.expiryDate') ??
                          SensitivityLevel.public,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: _isSaving ? null : _cancelEdit,
              child: const Text('Cancel'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _isSaving ? null : _submitForm,
              child: _isSaving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Text(isAdding ? 'Add' : 'Save'),
            ),
          ],
        ),
      ],
    );
  }
}

class _IdCardTile extends ConsumerWidget {
  final IdCardData card;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _IdCardTile({
    required this.card,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final hasLabel = card.label != null && card.label!.isNotEmpty;

    // Build list of fields to display
    final fields = <LabelValueField>[];

    if (hasLabel) {
      fields.add(LabelValueField(label: 'Label', value: card.label!));
    }
    if (card.number != null && card.number!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'ID Number',
        value: card.number!,
        fieldId: 'idCard.number',
        isSensitive: true,
      ));
    }
    if (card.holderName != null && card.holderName!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Holder Name',
        value: card.holderName!,
        fieldId: 'idCard.holderName',
        isSensitive: true,
      ));
    }
    if (card.country != null && card.country!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Country',
        value: card.country!,
        fieldId: 'idCard.country',
      ));
    }
    if (card.issueDate != null && card.issueDate!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Issue Date',
        value: card.issueDate!,
        fieldId: 'idCard.issueDate',
      ));
    }
    if (card.expiryDate != null && card.expiryDate!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Expiry Date',
        value: card.expiryDate!,
        fieldId: 'idCard.expiryDate',
      ));
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(
              Icons.credit_card_outlined,
              size: 20,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: ResponsiveLabelField(
              fields: fields,
              labelValueSpacing: 4,
              layoutAxis: Axis.vertical,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined, size: 20),
            tooltip: 'Edit',
            onPressed: onEdit,
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 20),
            tooltip: 'Delete',
            onPressed: onDelete,
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
    );
  }
}

class _AddressSection extends ConsumerStatefulWidget {
  final IdentityData? identity;
  final List<AddressData>? addresses;

  const _AddressSection({required this.identity, required this.addresses});

  @override
  ConsumerState<_AddressSection> createState() => _AddressSectionState();
}

class _AddressSectionState extends ConsumerState<_AddressSection> {
  // 'idle' | 'adding' | 'editing'
  String _mode = 'idle';
  int _editingIndex = -1;
  late List<_EntryWithIndex<AddressData>> _addresses;

  final _labelController = TextEditingController();
  final _streetController = TextEditingController();
  final _cityController = TextEditingController();
  final _postalController = TextEditingController();
  final _countryController = TextEditingController();
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _addresses = [
      ...?(widget.addresses?.asMap().entries.map(
        (mapEntry) => _EntryWithIndex(
          entry: mapEntry.value.copyWith(),
          originalIndex: mapEntry.key,
        ),
      )),
    ];
  }

  @override
  void didUpdateWidget(_AddressSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Reload data when parent passes updated addresses
    // Filter to only active (non-deleted) entries while preserving originalIndex
    if (widget.addresses != oldWidget.addresses) {
      _addresses = [
        for (var i = 0; i < (widget.addresses?.length ?? 0); i++)
          if (widget.addresses != null && !widget.addresses![i].isDeleted)
            _EntryWithIndex(
              entry: widget.addresses![i].copyWith(),
              originalIndex: i,
            ),
      ];
    }
  }

  @override
  void dispose() {
    _labelController.dispose();
    _streetController.dispose();
    _cityController.dispose();
    _postalController.dispose();
    _countryController.dispose();
    super.dispose();
  }

  void _startAdding() {
    _clearControllers();
    setState(() {
      _mode = 'adding';
      _editingIndex = -1;
    });
  }

  void _startEditing(int index) async {
    final addr = _addresses[index].entry;

    // Verify password for restricted fields (address.postalCode is restricted)
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: 'address.postalCode');
    if (!verified) return;

    if (!mounted) return;
    _labelController.text = addr.label ?? '';
    _streetController.text = addr.street ?? '';
    _cityController.text = addr.city ?? '';
    _postalController.text = addr.postalCode ?? '';
    _countryController.text = addr.country ?? '';
    setState(() {
      _mode = 'editing';
      _editingIndex = index;
    });
  }

  void _cancelEdit() {
    setState(() {
      _mode = 'idle';
      _editingIndex = -1;
    });
  }

  void _clearControllers() {
    _labelController.clear();
    _streetController.clear();
    _cityController.clear();
    _postalController.clear();
    _countryController.clear();
  }

  void _deleteEntry(int index) async {
    final deletedAddr = _addresses[index];

    // Verify password for restricted fields BEFORE showing delete confirmation
    final verified = await verifyPasswordForRestrictedField(context: context, ref: ref, fieldId: 'address.postalCode');
    if (!verified) return;

    if (!mounted) return;

    final confirm = await showDeleteConfirmationDialog(
      context: context,
      itemName: deletedAddr.entry.label ?? deletedAddr.entry.street ?? 'Address',
      itemType: 'Address',
    );
    if (!confirm) return;

    final itemName = deletedAddr.entry.label ?? 'Address';

    // Mark as soft deleted and show Undo snackbar
    _softDeleteWithUndo(
      section: 'profile',
      itemType: 'address',
      index: deletedAddr.originalIndex,
      deletedItem: deletedAddr.entry,
      itemName: itemName,
    );
  }

  void _softDeleteWithUndo({
    required String section,
    required String itemType,
    required int index,
    required dynamic deletedItem,
    required String itemName,
  }) async {
    _cancelEdit();

    // Store values needed after await
    final isMounted = mounted;
    final originalIndex = index;

    await ref
        .read(profileNotifierProvider.notifier)
        .softDelete(
          section: section,
          itemType: itemType,
          index: index,
          deletedItem: deletedItem,
        );

    if (!isMounted) return;

    // Remove from local list (find by originalIndex)
    setState(() {
      _addresses = _addresses
          .where((a) => a.originalIndex != originalIndex)
          .toList();
    });

    final isPrivacyMode =
        ref.read(sensitivitySettingsProvider).displayMode ==
        SensitivityDisplayMode.hidePrivate;

    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: LogSectionConfig.getLogSection(section, itemType),
        action: LogAction.delete,
        itemName: itemName,
        fieldName: itemName,
        isPrivacyModeActive: isPrivacyMode,
      ),
      duration: const Duration(seconds: 5),
      onUndo: () async {
        await ref
            .read(profileNotifierProvider.notifier)
            .restore(
              section: section,
              itemType: itemType,
              index: originalIndex,
            );
      },
    );
  }

  Future<void> _saveAddresses({
    LogAction operationType = LogAction.update,
    String? itemName,
  }) async {
    setState(() => _isSaving = true);

    final identity = IdentityData(
      fullName: widget.identity?.fullName,
      givenName: widget.identity?.givenName,
      familyName: widget.identity?.familyName,
      dateOfBirth: widget.identity?.dateOfBirth,
      gender: widget.identity?.gender,
      nationality: widget.identity?.nationality,
      idCards: widget.identity?.idCards,
      contact: widget.identity?.contact,
      addresses: _addresses.isEmpty
          ? null
          : _addresses.map((a) => a.entry).toList(),
    );

    final success = await ref
        .read(profileNotifierProvider.notifier)
        .updateIdentity(identity);

    if (mounted) {
      setState(() => _isSaving = false);
      if (success) {
        _cancelEdit();
        // Show operation notification
        final isPrivacyMode =
            ref.read(sensitivitySettingsProvider).displayMode ==
            SensitivityDisplayMode.hidePrivate;
        OperationNotification.show(
          context,
          message: OperationLogger.createNotification(
            section: LogSection.address,
            action: operationType,
            itemName: 'Address',
            fieldName: itemName ?? 'Location',
            isPrivacyModeActive: isPrivacyMode,
          ),
        );
      } else {
        showOverlaySnackBar(
          context,
          content: 'Failed to save',
          type: SnackBarType.error,
        );
      }
    }
  }

  void _submitForm() {
    final addr = AddressData(
      label: _labelController.text.isEmpty ? null : _labelController.text,
      street: _streetController.text.isEmpty ? null : _streetController.text,
      city: _cityController.text.isEmpty ? null : _cityController.text,
      postalCode: _postalController.text.isEmpty
          ? null
          : _postalController.text,
      country: _countryController.text.isEmpty ? null : _countryController.text,
    );

    // Capture operation type before state reset
    final wasAdding = _mode == 'adding';
    final addrLabel = addr.label ?? 'Address';
    final originalIndex = wasAdding
        ? -1
        : _addresses[_editingIndex].originalIndex;

    setState(() {
      if (_mode == 'adding') {
        _addresses.add(
          _EntryWithIndex(
            entry: addr,
            originalIndex: widget.addresses?.length ?? 0,
          ),
        );
      } else if (_mode == 'editing') {
        _addresses[_editingIndex] = _EntryWithIndex(
          entry: addr,
          originalIndex: originalIndex,
        );
      }
      _mode = 'idle';
      _editingIndex = -1;
    });

    _saveAddresses(
      operationType: wasAdding ? LogAction.create : LogAction.update,
      itemName: addrLabel,
    );
  }

  String _displayAddress(AddressData addr) {
    final parts = [
      addr.street,
      addr.city,
      addr.country,
    ].where((e) => e != null && e.isNotEmpty).join(', ');
    return parts.isEmpty ? '' : parts;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // Defensive filter: use only non-deleted entries for display
    final displayAddresses = _addresses
        .where((a) => !a.entry.isDeleted)
        .toList();
    final hasAddresses = displayAddresses.isNotEmpty;
    final isEditing = _mode == 'adding' || _mode == 'editing';

    return CollapsibleSectionCard(
      title: 'Addresses',
      icon: Icons.location_on_outlined,
      maxVisibleItems: 3,
      actionIcon: Icons.add,
      onAction: _startAdding,
      footer: isEditing ? _buildInlineForm(theme) : null,
      children: hasAddresses
          ? _buildAddressTiles(theme, displayAddresses)
          : [_buildEmptyState(theme)],
    );
  }

  Widget _buildEmptyState(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            Icon(
              Icons.location_off_outlined,
              size: 40,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 8),
            Text(
              'No addresses saved',
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 12),
            TextButton.icon(
              onPressed: _startAdding,
              icon: const Icon(Icons.add),
              label: const Text('Add Address'),
            ),
          ],
        ),
      ),
    );
  }

  /// Returns individual address tiles for CollapsibleSectionCard
  List<Widget> _buildAddressTiles(
    ThemeData theme,
    List<_EntryWithIndex<AddressData>> displayAddresses,
  ) {
    return [
      for (var i = 0; i < displayAddresses.length; i++)
        Column(
          children: [
            _AddressTile(
              address: displayAddresses[i].entry,
              displayText: _displayAddress(displayAddresses[i].entry),
              onEdit: () => _startEditing(i),
              onDelete: () => _deleteEntry(i),
            ),
            if (i < displayAddresses.length - 1) const Divider(height: 1),
          ],
        ),
    ];
  }

  Widget _buildInlineForm(ThemeData theme) {
    final isAdding = _mode == 'adding';
    final settings = ref.watch(sensitivitySettingsProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          isAdding ? 'Add Address' : 'Edit Address',
          style: theme.textTheme.titleSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _labelController,
          maxLength: kMaxFieldLength,
          decoration: const InputDecoration(
            labelText: 'Label (e.g., Home, Work)',
            counterText: '',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _streetController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: 'Street',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel('address.street') ??
                    SensitivityLevel.public,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _cityController,
                maxLength: kMaxFieldLength,
                decoration: InputDecoration(
                  labelText: 'City',
                  counterText: '',
                  border: const OutlineInputBorder(),
                  suffixIcon: Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: SensitivityTag(
                      level:
                          settings.getFieldLevel('address.city') ??
                          SensitivityLevel.public,
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: _postalController,
                maxLength: kMaxFieldLength,
                decoration: InputDecoration(
                  labelText: 'Postal Code',
                  counterText: '',
                  border: const OutlineInputBorder(),
                  suffixIcon: Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: SensitivityTag(
                      level:
                          settings.getFieldLevel('address.postalCode') ??
                          SensitivityLevel.public,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _countryController,
          maxLength: kMaxFieldLength,
          decoration: InputDecoration(
            labelText: 'Country',
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: SensitivityTag(
                level:
                    settings.getFieldLevel('address.country') ??
                    SensitivityLevel.public,
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: _isSaving ? null : _cancelEdit,
              child: const Text('Cancel'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _isSaving ? null : _submitForm,
              child: _isSaving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Text(isAdding ? 'Add' : 'Save'),
            ),
          ],
        ),
      ],
    );
  }
}

class _AddressTile extends ConsumerWidget {
  final AddressData address;
  final String displayText;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _AddressTile({
    required this.address,
    required this.displayText,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    // Build list of fields to display
    final fields = <LabelValueField>[];

    if (address.label != null && address.label!.isNotEmpty) {
      fields.add(LabelValueField(label: 'Label', value: address.label!));
    }
    if (address.street != null && address.street!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Street',
        value: address.street!,
        fieldId: 'address.street',
      ));
    }
    if (address.city != null && address.city!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'City',
        value: address.city!,
        fieldId: 'address.city',
      ));
    }
    if (address.postalCode != null && address.postalCode!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Postal Code',
        value: address.postalCode!,
        fieldId: 'address.postalCode',
        isSensitive: true,
      ));
    }
    if (address.country != null && address.country!.isNotEmpty) {
      fields.add(LabelValueField(
        label: 'Country',
        value: address.country!,
        fieldId: 'address.country',
      ));
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(
              Icons.home_outlined,
              size: 20,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: fields.isEmpty
                ? SelectableText(
                    'Tap to add',
                    style: theme.textTheme.bodyLarge?.copyWith(
                      color: theme.colorScheme.primary,
                    ),
                  )
                : ResponsiveLabelField(
                    fields: fields,
                    labelValueSpacing: 4,
                    layoutAxis: Axis.vertical,
                  ),
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined, size: 20),
            tooltip: 'Edit',
            onPressed: onEdit,
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 20),
            tooltip: 'Delete',
            onPressed: onDelete,
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
    );
  }
}
