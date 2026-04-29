import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/providers/account_style_provider.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/widgets/entry_card_widget.dart';
import 'package:solosoul_flutter/presentation/widgets/predefined_object_section.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';

class TravelPage extends ConsumerStatefulWidget {
  const TravelPage({super.key});

  @override
  ConsumerState<TravelPage> createState() => _TravelPageState();
}

class _TravelPageState extends ConsumerState<TravelPage> {
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
        title: const Text('Travel'),
        actions: const [HeaderActionButtons()],
      ),
      body: SingleChildScrollView(
        padding: AppTheme.kPagePadding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const SizedBox(height: 8),
            PredefinedObjectSection(
              sectionId: DefaultSectionIds.passport,
              typeId: 'travel_passport',
              title: 'Passports',
              icon: Icons.flight_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (passport, itemMap) => EntryCardWidget<UnifiedObject>(
                item: passport,
                title: passport.name,
                icon: Icons.book,
                itemId: passport.id,
                historyFieldId: 'passport',
                isRestricted: true,
                formatAllFields: (p) => 'Passport\n${p.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'passport',
                excludeFields: const {'title'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.travel,
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
                  content: 'Failed to delete passport',
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
              sectionId: DefaultSectionIds.visa,
              typeId: 'travel_visa',
              title: 'Visas',
              icon: Icons.assignment_ind_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (visa, itemMap) => EntryCardWidget<UnifiedObject>(
                item: visa,
                title: visa.name,
                icon: Icons.article,
                itemId: visa.id,
                historyFieldId: 'visa',
                isRestricted: true,
                formatAllFields: (v) => 'Visa\n${v.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'visa',
                excludeFields: const {'title'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.travel,
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
                  content: 'Failed to delete visa',
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
              sectionId: DefaultSectionIds.travelHistory,
              typeId: 'travel_history',
              title: 'Travel History',
              icon: Icons.history_outlined,
              maxVisibleItems: 3,
              displayItemBuilder: (item, itemMap) => EntryCardWidget<UnifiedObject>(
                item: item,
                title: item.name,
                icon: Icons.place,
                itemId: item.id,
                historyFieldId: 'travel',
                formatAllFields: (t) => 'Travel History\n${t.toFormattedString()}',
                itemData: itemMap,
                fieldPrefix: 'travel',
                excludeFields: const {'destination'},
              ),
              onDidDelete: (item, index) {
                OperationNotification.show(
                  context,
                  message: OperationLogger.createNotification(
                    section: LogSection.travel,
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
                  content: 'Failed to delete travel history',
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
          ],
        ),
      ),
    );
  }
}

