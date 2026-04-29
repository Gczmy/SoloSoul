import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart'
    show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';

class ProfessionalPage extends ConsumerStatefulWidget {
  const ProfessionalPage({super.key});

  @override
  ConsumerState<ProfessionalPage> createState() => _ProfessionalPageState();
}

class _ProfessionalPageState extends ConsumerState<ProfessionalPage> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final isPrivacyMode =
        ref.read(accountStyleProvider).value?.displayMode ==
        SensitivityDisplayMode.hidePrivate;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Professional'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Upload CV button at top
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () => _showCVUploadDialog(context),
                icon: const Icon(Icons.upload_file_outlined),
                label: const Text('Upload CV / Resume'),
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                ),
              ),
            ).animate().fadeIn(duration: 400.ms),
            const SizedBox(height: 24),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.education,
              typeId: 'professional_education',
              title: 'Education',
              icon: Icons.school_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.school,
                itemId: item.id,
                historyFieldId: 'education',
                formatAllFields: (e) => 'Education\n${e.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'education',
                excludeFields: const {'institution'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.professional,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref.read(unifiedObjectProvider.notifier).restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete education',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
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
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.employment,
              typeId: 'professional_employment',
              title: 'Employment',
              icon: Icons.work_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.work,
                itemId: item.id,
                historyFieldId: 'employment',
                formatAllFields: (e) => 'Employment\n${e.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'employment',
                excludeFields: const {'company'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.professional,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref.read(unifiedObjectProvider.notifier).restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete employment',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
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
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.award,
              typeId: 'professional_award',
              title: 'Awards',
              icon: Icons.emoji_events_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.emoji_events,
                itemId: item.id,
                historyFieldId: 'award',
                formatAllFields: (e) => 'Award\n${e.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'award',
                excludeFields: const {'title'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.professional,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref.read(unifiedObjectProvider.notifier).restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete award',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
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
            const SizedBox(height: 16),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.skill,
              typeId: 'professional_skill',
              title: 'Skills',
              icon: Icons.star_outline,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.star,
                itemId: item.id,
                historyFieldId: 'skill',
                formatAllFields: (e) => 'Skill\n${e.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'skill',
                excludeFields: const {'name'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.professional,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref.read(unifiedObjectProvider.notifier).restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete skill',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 400.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
            const SizedBox(height: 16),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.language,
              typeId: 'professional_language',
              title: 'Languages',
              icon: Icons.translate,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.translate,
                itemId: item.id,
                historyFieldId: 'language',
                formatAllFields: (e) => 'Language\n${e.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'language',
                excludeFields: const {'name'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.professional,
                    action: LogAction.delete,
                    itemName: item.name,
                    isPrivacyModeActive: isPrivacyMode,
                  ),
                  duration: AppTheme.kNotificationDuration,
                  onUndo: () async {
                    await ref.read(unifiedObjectProvider.notifier).restoreDefaultItem(item.id);
                  },
                );
              },
              onDeleteFailed: (item, index) {
                showOverlaySnackBar(
                  context,
                  content: 'Failed to delete language',
                  type: SnackBarType.error,
                );
              },
              onCopyAll: (item, text) async {
                unawaited(Clipboard.setData(ClipboardData(text: text)));
                unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
                showOverlaySnackBar(
                  context,
                  content: 'Copied to clipboard',
                  type: SnackBarType.success,
                );
              },
            )
                .animate()
                .fadeIn(delay: 500.ms, duration: 400.ms)
                .slideX(begin: 0.05, end: 0),
          ],
        ),
      ),
    );
  }

  void _showCVUploadDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Upload CV'),
        content: const Text(
          'CV upload and parsing will be available in a future update.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }
}
