
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
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

import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_document_button.dart';

class ProfilePage extends ConsumerStatefulWidget {
  const ProfilePage({super.key});

  @override
  ConsumerState<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends ConsumerState<ProfilePage> {
  static final _dummyValueNotifier = ValueNotifier<TextEditingValue>(const TextEditingValue());

  @override
  Widget build(BuildContext context) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    final l10n = AppLocalizations.of(context);
    return ObjectCategoryPage(
      title: l10n.profileTitle,
      sections: [
        const ScanDocumentButton(parentId: DefaultSectionIds.contact),
        const SizedBox(height: 16),
        _IdentitySection(isPrivacyMode: isPrivacyMode),
        const SizedBox(height: 16),
        _ContactSection(isPrivacyMode: isPrivacyMode, dummyValueNotifier: _dummyValueNotifier),
        const SizedBox(height: 16),
        _IdentityDocumentsSection(isPrivacyMode: isPrivacyMode),
        const SizedBox(height: 16),
        _AddressesSection(isPrivacyMode: isPrivacyMode),
        const SizedBox(height: 32),
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
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ).animate().fadeIn(delay: 400.ms, duration: 400.ms),
      ],
    );
  }
}

class _IdentitySection extends ConsumerWidget {
  const _IdentitySection({required this.isPrivacyMode});
  final bool isPrivacyMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return PredefinedObjectSection(
      key: const ValueKey(DefaultSectionIds.identity),
      sectionId: DefaultSectionIds.identity,
      typeId: 'profile_identity',
      title: l10n.profileIdentity,
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
        .slideX(begin: 0.05, end: 0);
  }
}

class _ContactSection extends ConsumerWidget {
  const _ContactSection({
    required this.isPrivacyMode,
    required this.dummyValueNotifier,
  });
  final bool isPrivacyMode;
  final ValueNotifier<TextEditingValue> dummyValueNotifier;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return PredefinedObjectSection(
      key: const ValueKey(DefaultSectionIds.contact),
      sectionId: DefaultSectionIds.contact,
      typeId: 'profile_contact',
      title: l10n.profileContactInfo,
      icon: Icons.contact_mail_outlined,
      maxVisibleItems: 3,
      customFormBuilder: (context, theme, controllers, mode, onSubmit, onCancel, sensitivities) {
        return _ContactForm(
          controllers: controllers,
          mode: mode,
          onSubmit: onSubmit,
          onCancel: onCancel,
          sensitivities: sensitivities,
          dummyValueNotifier: dummyValueNotifier,
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
        .slideX(begin: 0.05, end: 0);
  }
}

class _CountedTextField extends StatelessWidget {
  const _CountedTextField({
    required this.controller,
    required this.label,
    this.hint,
    this.suffixIcon,
    this.keyboardType,
    required this.dummyValueNotifier,
  });
  final TextEditingController? controller;
  final String label;
  final String? hint;
  final Widget? suffixIcon;
  final TextInputType? keyboardType;
  final ValueNotifier<TextEditingValue> dummyValueNotifier;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    const effectiveMaxLength = kMaxFieldLength;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: TextField(
            controller: controller,
            maxLength: effectiveMaxLength,
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
            valueListenable: controller ?? dummyValueNotifier,
            builder: (context, val, child) {
              final len = val.text.length;
              const max = effectiveMaxLength;
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
}

class _IdentityDocumentsSection extends ConsumerWidget {
  const _IdentityDocumentsSection({required this.isPrivacyMode});
  final bool isPrivacyMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return PredefinedObjectSection(
      key: const ValueKey(DefaultSectionIds.idCard),
      sectionId: DefaultSectionIds.idCard,
      typeId: 'profile_id_card',
      title: l10n.profileIdentityDocuments,
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
        .slideX(begin: 0.05, end: 0);
  }
}

class _AddressesSection extends ConsumerWidget {
  const _AddressesSection({required this.isPrivacyMode});
  final bool isPrivacyMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return PredefinedObjectSection(
      key: const ValueKey(DefaultSectionIds.address),
      sectionId: DefaultSectionIds.address,
      typeId: 'profile_address',
      title: l10n.profileAddresses,
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
        .slideX(begin: 0.05, end: 0);
  }
}

class _ContactForm extends StatelessWidget {
  final Map<String, TextEditingController?> controllers;
  final String mode;
  final VoidCallback onSubmit;
  final VoidCallback onCancel;
  final Map<String, SensitivityLevel> sensitivities;
  final ValueNotifier<TextEditingValue> dummyValueNotifier;

  const _ContactForm({
    required this.controllers,
    required this.mode,
    required this.onSubmit,
    required this.onCancel,
    required this.sensitivities,
    required this.dummyValueNotifier,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final typeText = controllers['type']?.text ?? '';
    final selectedType = typeText.isEmpty ? 'email' : typeText;
    if (typeText.isEmpty) {
      controllers['type']?.text = 'email';
    }
    final valueSensitivity = sensitivities['value'] ?? SensitivityLevel.public;

    final l10n = AppLocalizations.of(context);
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
    _CountedTextField(
      controller: controllers['title'],
      label: l10n.profileTitleLabel,
      hint: 'e.g., Gmail, Work',
      dummyValueNotifier: dummyValueNotifier,
    ),
    const SizedBox(height: 12),
    DropdownButtonFormField<String>(
      initialValue:
          selectedType.isEmpty ? 'email' : selectedType,
      decoration: InputDecoration(
        labelText: l10n.profileTypeLabel,
        border: const OutlineInputBorder(),
        contentPadding: EdgeInsets.symmetric(
          horizontal: 12,
          vertical: 8,
        ),
      ),
      items: [
        DropdownMenuItem(
          value: 'email',
          child: Text(AppLocalizations.of(context).profileTypeEmail),
        ),
        DropdownMenuItem(
          value: 'phone',
          child: Text(AppLocalizations.of(context).profileTypePhone),
        ),
      ],
      onChanged: (v) {
        controllers['type']?.text = v ?? 'email';
      },
    ),
    const SizedBox(height: 12),
    _CountedTextField(
      controller: controllers['value'],
      label: l10n.profileValueLabel,
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
      dummyValueNotifier: dummyValueNotifier,
    ),
    const SizedBox(height: 16),
    Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        TextButton(
          onPressed: onCancel,
          child: Text(AppLocalizations.of(context).commonCancel),
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

  }
}
