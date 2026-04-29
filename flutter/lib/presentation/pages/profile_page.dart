import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart'
    show DefaultSectionIds;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, SnackBarType, showOverlaySnackBar;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show SensitivityLevel, SensitivityDisplayMode, effectiveSensitivityProvider;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';
import 'package:solosoul_flutter/core/models/profile_data.dart'
    show kMaxFieldLength;
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart'
    show
        authNotifierProvider,
        sensitivePageAccessProvider,
        isSensitiveAccessGrantedProvider;
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;

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
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Profile'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Identity
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.identity,
              typeId: 'profile_identity',
              title: 'Identity',
              icon: Icons.person_outlined,
              maxVisibleItems: 1,
              displayItemBuilder: (item, itemMap) =>
                  EntryCardWidget<UnifiedObject>(
                    item: item,
                    title:
                        itemMap['fullName']?.isNotEmpty == true
                            ? itemMap['fullName']!
                            : item.name,
                    icon: Icons.person,
                    itemId: item.id,
                    historyFieldId: 'identity',
                    formatAllFields:
                        (i) => 'Identity\n${i.toFormattedString()}',
                    itemData: itemMap,
                    fieldPrefix: 'identity',
                    excludeFields: const {'fullName'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.identity,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete identity',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Contact Information
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.contact,
              typeId: 'profile_contact',
              title: 'Contact Information',
              icon: Icons.contact_mail_outlined,
              maxVisibleItems: 3,
              customFormBuilder:
                  (context, theme, controllers, mode, onSubmit, onCancel,
                      sensitivities) {
                    final selectedType = controllers['type']?.text ?? 'email';
                    final valueSensitivity =
                        sensitivities['value'] ?? SensitivityLevel.public;
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
                          controller: controllers['title'],
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
                          initialValue:
                              selectedType.isEmpty ? 'email' : selectedType,
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
                            controllers['type']?.text = v ?? 'email';
                          },
                        ),
                        const SizedBox(height: 12),
                        TextField(
                          controller: controllers['value'],
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
                          keyboardType:
                              selectedType == 'email'
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
                              child: Text(
                                mode == 'adding' ? 'Add' : 'Save',
                              ),
                            ),
                          ],
                        ),
                      ],
                    );
                  },
              displayItemBuilder: (item, itemMap) =>
                  EntryCardWidget<UnifiedObject>(
                    item: item,
                    title:
                        itemMap['title']?.isNotEmpty == true
                            ? itemMap['title']!
                            : (itemMap['value'] ?? item.name),
                    icon:
                        itemMap['type'] == 'email'
                            ? Icons.email_outlined
                            : Icons.phone_outlined,
                    itemId: item.id,
                    historyFieldId: 'contact',
                    formatAllFields:
                        (c) =>
                            '${itemMap['type'] ?? 'contact'}: ${itemMap['value'] ?? ''}',
                    itemData: itemMap,
                    fieldPrefix: 'contact',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.contactInformation,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete contact',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 100.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Identity Documents
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.idCard,
              typeId: 'profile_id_card',
              title: 'Identity Documents',
              icon: Icons.badge_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) =>
                  EntryCardWidget<UnifiedObject>(
                    item: item,
                    title:
                        itemMap['title']?.isNotEmpty == true
                            ? itemMap['title']!
                            : 'ID Card',
                    icon: Icons.badge_outlined,
                    itemId: item.id,
                    historyFieldId: 'idCard',
                    formatAllFields:
                        (c) => 'ID Card\n${c.toFormattedString()}',
                    itemData: itemMap,
                    fieldPrefix: 'idCard',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.idCard,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete ID card',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 200.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),

            const SizedBox(height: 16),

            // Addresses
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.address,
              typeId: 'profile_address',
              title: 'Addresses',
              icon: Icons.location_on_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) =>
                  EntryCardWidget<UnifiedObject>(
                    item: item,
                    title:
                        itemMap['title']?.isNotEmpty == true
                            ? itemMap['title']!
                            : 'Address',
                    icon: Icons.home_outlined,
                    itemId: item.id,
                    historyFieldId: 'address',
                    itemData: itemMap,
                    fieldPrefix: 'address',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.address,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref
                        .read(unifiedObjectProvider.notifier)
                        .restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete address',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(
                  ClipboardMonitorService.instance.notifySensitiveCopied(),
                );
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
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
