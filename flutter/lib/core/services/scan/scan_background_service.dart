import 'dart:async';

import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/scan/local_search_service.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';

// =============================================================================
// Scan Background Service
// =============================================================================

/// Singleton service that manages a filesystem scan in the background.
///
/// Unlike [LocalSearchNotifier], this service is NOT tied to any Riverpod
/// provider lifecycle. The scan continues even when the user navigates away
/// from the scan pages. UI layers (pages, banners) subscribe to
/// [stateStream] to display progress.
///
/// Usage:
///   ScanBackgroundService.instance.startScan(config);
///   ScanBackgroundService.instance.stateStream.listen((state) { ... });
///   ScanBackgroundService.instance.cancelScan();
class ScanBackgroundService {
  static final ScanBackgroundService instance = ScanBackgroundService._();
  ScanBackgroundService._();

  StreamSubscription<ScanResult>? _subscription;

  final _stateController = StreamController<LocalSearchState>.broadcast();
  LocalSearchState _currentState = const LocalSearchState();

  /// Whether a scan is currently running.
  bool get isScanning => _subscription != null;

  /// The most recent scan state.
  LocalSearchState get currentState => _currentState;

  /// Broadcast stream of scan state updates.
  Stream<LocalSearchState> get stateStream => _stateController.stream;

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /// Start a new scan with the given configuration.
  ///
  /// If a scan is already running, this call is ignored.
  void startScan({
    required List<String> paths,
    required List<String> extensions,
    required String scanDepth,
    required Map<String, int> maxFileSizeByExtension,
  }) {
    if (_subscription != null) return;

    final results = <ScanResult>[];
    final scannedFiles = <String>[];
    final foundFiles = <String>[];
    final skippedFiles = <String>[];

    // Preserve user config (paths, extensions, etc.) while resetting
    // transient scan state.
    _emit(_currentState.copyWith(
      isScanning: true,
      scanProgress: 0,
      scannedCount: 0,
      foundCount: 0,
      currentPath: '',
      scanResults: [],
      scanError: null,
      scannedFiles: [],
      foundFiles: [],
      skippedFiles: [],
      wasCanceled: false,
      paths: paths,
      extensions: extensions,
      scanDepth: scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension,
    ));

    const progressThrottle = 50;
    _subscription = LocalSearchService.scan(
      paths: paths.isEmpty ? null : paths,
      extensions: extensions.isEmpty ? null : extensions,
      scanDepth: scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension,
      onProgress: (scanned, found, skipped, currentPath) {
        // Throttle state updates to avoid O(n²) list copying overhead.
        // UI updates every 50 files are sufficient for visual feedback.
        if (scanned % progressThrottle != 0) return;
        _emit(_currentState.copyWith(
          scannedCount: scanned,
          foundCount: found,
          currentPath: currentPath,
          scannedFiles: [...scannedFiles],
          foundFiles: [...foundFiles],
          skippedFiles: [...skippedFiles],
        ));
      },
      onScanned: (path) => scannedFiles.add(path),
      onFound: (path) => foundFiles.add(path),
      onSkipped: (path) => skippedFiles.add(path),
    ).listen(
      (result) {
        results.add(result);
        _emit(_currentState.copyWith(
          scanResults: [...results],
          foundCount: results.length,
          foundFiles: [...foundFiles],
        ));
      },
      onDone: () {
        _emit(_currentState.copyWith(
          isScanning: false,
          scanProgress: 100,
          scanResults: results,
          scannedFiles: scannedFiles,
          foundFiles: foundFiles,
          skippedFiles: skippedFiles,
        ));
        _subscription = null;
      },
      onError: (error) {
        _emit(_currentState.copyWith(
          isScanning: false,
          scanError: error.toString(),
        ));
        _subscription = null;
      },
    );
  }

  /// Update config fields without starting a scan.
  /// Used by UI setters to keep _currentState in sync with provider state.
  void updateConfig({
    List<String>? paths,
    List<String>? extensions,
    String? scanDepth,
    Map<String, int>? maxFileSizeByExtension,
  }) {
    _emit(_currentState.copyWith(
      paths: paths,
      extensions: extensions,
      scanDepth: scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension,
    ));
  }

  /// Cancel the currently running scan.
  void cancelScan() {
    _subscription?.cancel();
    _subscription = null;
    _emit(_currentState.copyWith(isScanning: false, wasCanceled: true));
  }

  /// Reset transient scan state (e.g. after import is complete).
  ///
  /// Preserves user configuration (paths, extensions, scan depth, size limits)
  /// so the user can scan again with the same settings.
  void reset() {
    _subscription?.cancel();
    _subscription = null;
    _emit(_currentState.copyWith(
      isScanning: false,
      wasCanceled: false,
      scanProgress: 0,
      scannedCount: 0,
      foundCount: 0,
      currentPath: '',
      scanError: null,
      scanResults: [],
      scannedFiles: [],
      foundFiles: [],
      skippedFiles: [],
      importCandidates: [],
      importConflicts: [],
      importResult: null,
    ));
  }

  // ---------------------------------------------------------------------------
  // Internal
  // ---------------------------------------------------------------------------

  void _emit(LocalSearchState state) {
    _currentState = state;
    if (!_stateController.isClosed) {
      _stateController.add(state);
    }
  }
}
