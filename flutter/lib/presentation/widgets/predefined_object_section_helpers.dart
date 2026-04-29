import 'dart:async' show unawaited;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
export 'package:solosoul_flutter/presentation/models/operation_log_models.dart' show LogSection, LogAction;
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart'
    show unifiedObjectProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Builds the shared [onDidDelete] callback for [PredefinedObjectSection].
///
/// Shows an undoable notification and logs the operation.
void Function(UnifiedObject item, int index) buildOnDidDelete(
  BuildContext context, {
  required LogSection logSection,
  required bool isPrivacyMode,
  required WidgetRef ref,
}) {
  return (item, index) {
    OperationNotification.show(
      context,
      message: OperationLogger.createNotification(
        section: logSection,
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
  };
}

/// Builds the shared [onDeleteFailed] callback for [PredefinedObjectSection].
///
/// Shows an error snackbar with the section label.
void Function(UnifiedObject item, int index) buildOnDeleteFailed(
  BuildContext context, {
  required String sectionLabel,
}) {
  return (item, index) {
    showOverlaySnackBar(
      context,
      content: 'Failed to delete $sectionLabel',
      type: SnackBarType.error,
    );
  };
}

/// Builds the shared [onCopyAll] callback for [PredefinedObjectSection].
///
/// Copies text to clipboard and shows a success snackbar.
Future<void> Function(UnifiedObject item, String text) buildOnCopyAll(
  BuildContext context,
) {
  return (item, text) async {
    unawaited(Clipboard.setData(ClipboardData(text: text)));
    unawaited(ClipboardMonitorService.instance.notifySensitiveCopied());
    showOverlaySnackBar(
      context,
      content: 'Copied to clipboard',
      type: SnackBarType.success,
    );
  };
}


