import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/scan/scan_background_service.dart';
import 'package:solosoul_flutter/core/services/scan/scan_import_service.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';
import 'package:solosoul_flutter/presentation/providers/scan/scan_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

part 'local_search_provider.g.dart';

// =============================================================================
// Local Search Provider
// =============================================================================

@riverpod
class LocalSearchNotifier extends _$LocalSearchNotifier {
  StreamSubscription<LocalSearchState>? _bgSubscription;

  bool _initialized = false;

  @override
  LocalSearchState build() {
    // Subscribe to the background service so state updates survive page navigation.
    _bgSubscription = ScanBackgroundService.instance.stateStream.listen(
      (bgState) => state = bgState,
    );

    ref.onDispose(() {
      _bgSubscription?.cancel();
    });

    return ScanBackgroundService.instance.currentState;
  }

  // ---------------------------------------------------------------------------
  // Configuration (synced with persistent ScanConfig)
  // ---------------------------------------------------------------------------

  /// Initialize scan config from persistent storage.
  /// Should be called once when the config page is first built.
  /// Awaits the async provider so the config is loaded even on cold start.
  Future<void> initFromConfig() async {
    if (_initialized) return;
    _initialized = true;

    try {
      final config = await ref.read(scanConfigProvider.future);
      state = state.copyWith(
        paths: config.paths,
        extensions: config.extensions,
        scanDepth: config.scanDepth,
        maxFileSizeByExtension: config.maxFileSizeByExtension,
      );
    } on Exception catch (_) {
      // If loading fails, keep current state (defaults or background service state)
    }
  }

  void setPaths(List<String> paths) {
    state = state.copyWith(paths: paths);
    ScanBackgroundService.instance.updateConfig(paths: paths);
    ref.read(scanConfigProvider.notifier).setPaths(paths);
  }

  void setExtensions(List<String> extensions) {
    state = state.copyWith(extensions: extensions);
    ScanBackgroundService.instance.updateConfig(extensions: extensions);
    ref.read(scanConfigProvider.notifier).setExtensions(extensions);
  }

  void setScanDepth(String depth) {
    state = state.copyWith(scanDepth: depth);
    ScanBackgroundService.instance.updateConfig(scanDepth: depth);
    ref.read(scanConfigProvider.notifier).setScanDepth(depth);
  }

  void setMaxFileSizeForExtension(String ext, int mb) {
    final updated = Map<String, int>.from(state.maxFileSizeByExtension)..[ext] = mb;
    state = state.copyWith(maxFileSizeByExtension: updated);
    ScanBackgroundService.instance.updateConfig(maxFileSizeByExtension: updated);
    ref.read(scanConfigProvider.notifier).setMaxFileSizeForExtension(ext, mb);
  }

  // ---------------------------------------------------------------------------
  // Scan Execution (delegated to background service)
  // ---------------------------------------------------------------------------

  void startScan() {
    ScanBackgroundService.instance.startScan(
      paths: state.paths,
      extensions: state.extensions,
      scanDepth: state.scanDepth,
      maxFileSizeByExtension: state.maxFileSizeByExtension,
    );
  }

  void cancelScan() {
    ScanBackgroundService.instance.cancelScan();
  }

  // ---------------------------------------------------------------------------
  // Preview / Import
  // ---------------------------------------------------------------------------

  Future<void> prepareImport() async {
    final importService = ScanImportService(
      ref.read(unifiedObjectProvider.notifier),
      ref.read(unifiedObjectProvider).objects,
    );

    final allCandidates = <ImportCandidate>[];
    for (final result in state.scanResults) {
      final candidates = importService.mapScanResult(result);
      allCandidates.addAll(candidates);
    }

    // Detect conflicts
    final conflicts = importService.detectConflicts(allCandidates);

    state = state.copyWith(
      importCandidates: allCandidates,
      importConflicts: conflicts,
    );
  }

  void setCandidateSelected(int index, bool selected) {
    final candidates = [...state.importCandidates];
    if (index < 0 || index >= candidates.length) return;
    candidates[index] = ImportCandidate(
      source: candidates[index].source,
      existingObjectId: candidates[index].existingObjectId,
      fields: candidates[index].fields,
      isSelected: selected,
    );
    state = state.copyWith(importCandidates: candidates);
  }

  void setFieldAction(int candidateIndex, int fieldIndex, ImportAction action) {
    final candidates = [...state.importCandidates];
    if (candidateIndex < 0 || candidateIndex >= candidates.length) return;
    final fields = [...candidates[candidateIndex].fields];
    if (fieldIndex < 0 || fieldIndex >= fields.length) return;

    fields[fieldIndex] = ImportFieldCandidate(
      source: fields[fieldIndex].source,
      targetPropertyId: fields[fieldIndex].targetPropertyId,
      suggestedAction: fields[fieldIndex].suggestedAction,
      userAction: action,
    );

    candidates[candidateIndex] = ImportCandidate(
      source: candidates[candidateIndex].source,
      existingObjectId: candidates[candidateIndex].existingObjectId,
      fields: fields,
      isSelected: candidates[candidateIndex].isSelected,
    );

    state = state.copyWith(importCandidates: candidates);
  }

  Future<ScanImportResult> executeImport() async {
    final importService = ScanImportService(
      ref.read(unifiedObjectProvider.notifier),
      ref.read(unifiedObjectProvider).objects,
    );

    final confirmed = state.importCandidates
        .where((c) => c.isSelected)
        .toList();

    final result = await importService.executeImport(confirmed);
    state = state.copyWith(importResult: result);
    return result;
  }

  void reset() {
    ScanBackgroundService.instance.reset();
    _initialized = false;
  }
}
