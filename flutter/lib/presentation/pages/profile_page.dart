
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart'
    show DefaultSectionIds;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart'
    show accountStyleProvider;
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show SensitivityLevel, SensitivityDisplayMode;
import 'package:solosoul_flutter/core/models/profile_data.dart'
    show kMaxFieldLength;
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart'
    show SensitivityTag;
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section_helpers.dart';

import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';

class ProfilePage extends ConsumerStatefulWidget {
  const ProfilePage({super.key});

  @override
  ConsumerState<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends ConsumerState<ProfilePage> {
  static final _dummyValueNotifier = ValueNotifier<TextEditingValue>(const TextEditingValue());

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
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.identity,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'identity',
              ),
              onCopyAll: buildOnCopyAll(context),
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
                    final typeText = controllers['type']?.text ?? '';
                    final selectedType = typeText.isEmpty ? 'email' : typeText;
                    // Sync controller with resolved default so save reads correct value
                    if (typeText.isEmpty) {
                      controllers['type']?.text = 'email';
                    }
                    final valueSensitivity =
                        sensitivities['value'] ?? SensitivityLevel.public;

                    Widget buildCountedField({
                      required TextEditingController? controller,
                      required String label,
                      String? hint,
                      int? maxLength,
                      Widget? suffixIcon,
                      TextInputType? keyboardType,
                    }) {
                      maxLength ??= kMaxFieldLength;
                      return Row(
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          Expanded(
                            child: TextField(
                              controller: controller,
                              maxLength: maxLength,
                              buildCounter: (context, {required currentLength, required isFocused, maxLength}) => null,
                              decoration: InputDecoration(
                                labelText: label,
                                hintText: hint,
                                border: const OutlineInputBorder(),
                                contentPadding: const EdgeInsets.symmetric(
                                  horizontal: 12,
                                  vertical: 8,
                                ),
                                suffixIcon: suffixIcon,
                              ),
                              keyboardType: keyboardType,
                            ),
                          ),
                          const SizedBox(width: 8),
                          SizedBox(
                            width: 64,
                            child: ValueListenableBuilder<TextEditingValue>(
                              valueListenable: controller ?? _dummyValueNotifier,
                              builder: (context, val, child) {
                                final len = val.text.length;
                                final max = maxLength!;
                                return Row(
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    SizedBox(
                                      width: 28,
                                      child: Text(
                                        '$len',
                                        textAlign: TextAlign.right,
                                        style: theme.textTheme.bodySmall?.copyWith(
                                          color: len >= max
                                              ? theme.colorScheme.error
                                              : theme.colorScheme.onSurfaceVariant,
                                        ),
                                      ),
                                    ),
                                    Text(
                                      '/',
                                      style: theme.textTheme.bodySmall?.copyWith(
                                        color: len >= max
                                            ? theme.colorScheme.error
                                            : theme.colorScheme.onSurfaceVariant,
                                      ),
                                    ),
                                    SizedBox(
                                      width: 28,
                                      child: Text(
                                        '$max',
                                        textAlign: TextAlign.left,
                                        style: theme.textTheme.bodySmall?.copyWith(
                                          color: len >= max
                                              ? theme.colorScheme.error
                                              : theme.colorScheme.onSurfaceVariant,
                                        ),
                                      ),
                                    ),
                                  ],
                                );
                              },
                            ),
                          ),
                        ],
                      );
                    }

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
                        buildCountedField(
                          controller: controllers['title'],
                          label: 'Title',
                          hint: 'e.g., Gmail, Work',
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
                        buildCountedField(
                          controller: controllers['value'],
                          label: 'Value',
                          suffixIcon: Padding(
                            padding: const EdgeInsets.only(right: 12),
                            child: Align(
                              alignment: Alignment.centerRight,
                              widthFactor: 1,
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
                    formatAllFields: (c) {
                      final typeStr = itemMap['type'];
                      final type = typeStr?.trim().isNotEmpty == true
                          ? typeStr!
                          : 'contact';
                      return '$type: ${itemMap['value'] ?? ''}';
                    },
                    itemData: itemMap,
                    fieldPrefix: 'contact',
                    excludeFields: const {'title'},
                  ),
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.contactInformation,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'contact',
              ),
              onCopyAll: buildOnCopyAll(context),
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
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.idCard,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'ID card',
              ),
              onCopyAll: buildOnCopyAll(context),
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
              onDidDelete: buildOnDidDelete(
                context,
                logSection: LogSection.address,
                isPrivacyMode: isPrivacyMode,
                ref: ref,
              ),
              onDeleteFailed: buildOnDeleteFailed(
                context,
                sectionLabel: 'address',
              ),
              onCopyAll: buildOnCopyAll(context),
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
