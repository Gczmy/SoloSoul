import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

/// Minimal unified-object trash coordinator.
/// All operations delegate to [unifiedObjectProvider].
class TrashManager {
  final Ref _ref;

  TrashManager(this._ref);

  /// Soft delete a unified object by [id].
  /// Delegates to [UnifiedObjectNotifier.deleteDefaultItem] first,
  /// then falls back to [UnifiedObjectNotifier.deleteObject].
  Future<void> softDelete(String id) async {
    final notifier = _ref.read(unifiedObjectProvider.notifier);
    var success = await notifier.deleteDefaultItem(id);
    if (!success) {
      success = await notifier.deleteObject(id);
    }

    final object = _getObjectById(id);
    if (object != null) {
      OperationLogger.createNotification(
        section: LogSection.identity,
        action: LogAction.delete,
        itemName: object.name,
      );
    }
  }

  /// Restore a soft-deleted unified object by [id].
  /// Delegates to [UnifiedObjectNotifier.restoreObject] first,
  /// then falls back to [UnifiedObjectNotifier.restoreDefaultItem].
  Future<void> restore(String id) async {
    final notifier = _ref.read(unifiedObjectProvider.notifier);
    var success = await notifier.restoreObject(id);
    if (!success) {
      success = await notifier.restoreDefaultItem(id);
    }

    final object = _getObjectById(id);
    if (object != null) {
      OperationLogger.createNotification(
        section: LogSection.identity,
        action: LogAction.restore,
        itemName: object.name,
      );
    }
  }

  /// Permanently delete a unified object by [id].
  Future<void> permanentDelete(String id) async {
    final notifier = _ref.read(unifiedObjectProvider.notifier);
    final object = _getObjectById(id);

    await notifier.permanentlyDeleteObject(id);

    if (object != null) {
      OperationLogger.createNotification(
        section: LogSection.identity,
        action: LogAction.purge,
        itemName: object.name,
      );
    }
  }

  /// Empty all trash by permanently deleting all soft-deleted objects.
  Future<void> emptyAllTrash() async {
    final objects = _ref.read(unifiedObjectProvider).objects;
    final deletedObjects = objects.where((o) => o.isDeleted).toList();
    final notifier = _ref.read(unifiedObjectProvider.notifier);

    for (final object in deletedObjects) {
      await notifier.permanentlyDeleteObject(object.id);
      OperationLogger.createNotification(
        section: LogSection.identity,
        action: LogAction.purge,
        itemName: object.name,
      );
    }
  }

  UnifiedObject? _getObjectById(String id) {
    final objects = _ref.read(unifiedObjectProvider).objects;
    try {
      return objects.firstWhere((o) => o.id == id);
    } on StateError {
      return null;
    }
  }
}
