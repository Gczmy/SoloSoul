import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/sync_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/presentation/utils/device_utils.dart';

/// Sync state for the current session
enum SyncStatus {
  idle,
  discovering,
  syncing,
  success,
  error,
}

/// Sync state
class SyncState {
  final SyncStatus status;
  final List<frb.DiscoveredDevice> devices;
  final String? errorMessage;
  final frb.SyncResult? lastResult;
  final bool isAdvertising;
  final bool isListening;

  const SyncState({
    this.status = SyncStatus.idle,
    this.devices = const [],
    this.errorMessage,
    this.lastResult,
    this.isAdvertising = false,
    this.isListening = false,
  });

  SyncState copyWith({
    SyncStatus? status,
    List<frb.DiscoveredDevice>? devices,
    String? errorMessage,
    frb.SyncResult? lastResult,
    bool? isAdvertising,
    bool? isListening,
  }) {
    return SyncState(
      status: status ?? this.status,
      devices: devices ?? this.devices,
      errorMessage: errorMessage,
      lastResult: lastResult ?? this.lastResult,
      isAdvertising: isAdvertising ?? this.isAdvertising,
      isListening: isListening ?? this.isListening,
    );
  }
}

/// Notifier for sync operations
class SyncNotifier extends Notifier<SyncState> {
  bool _shouldListen = false;

  @override
  SyncState build() {
    return const SyncState();
  }

  /// Discover devices on the local network
  Future<void> discoverDevices({int timeoutMs = 3000}) async {
    state = state.copyWith(status: SyncStatus.discovering, errorMessage: null);
    try {
      final devices = await SyncService.instance.discoverDevices(
        timeoutMs: timeoutMs,
      );
      state = state.copyWith(
        status: SyncStatus.idle,
        devices: devices,
      );
    } on Exception catch (e) {
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Discovery failed: $e',
      );
    } on Object catch (e) {
      SoloLog.e('SyncNotifier', 'Unexpected discovery error', e);
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Unexpected discovery error: $e',
      );
    }
  }

  /// Start advertising this device
  Future<void> startAdvertising() async {
    final deviceName = _getDeviceName();
    final success = await SyncService.instance.advertise(deviceName: deviceName);
    state = state.copyWith(isAdvertising: success);
  }

  /// Stop advertising
  void stopAdvertising() {
    state = state.copyWith(isAdvertising: false);
  }

  /// Start listening for incoming sync connections.
  Future<void> startListening({
    required String accountId,
    required List<int> pairingKey,
    required List<int> deviceSalt,
  }) async {
    _shouldListen = true;
    state = state.copyWith(
      isListening: true,
      status: SyncStatus.idle,
      errorMessage: null,
    );
    try {
      final result = await SyncService.instance.syncAsResponder(
        accountId: accountId,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
      );
      if (!_shouldListen) return;
      state = state.copyWith(
        isListening: false,
        status: SyncStatus.success,
        lastResult: result,
      );
    } on Exception catch (e) {
      if (!_shouldListen) return;
      state = state.copyWith(
        isListening: false,
        status: SyncStatus.error,
        errorMessage: 'Sync failed: $e',
      );
    } on Object catch (e) {
      SoloLog.e('SyncNotifier', 'Unexpected sync error', e);
      if (!_shouldListen) return;
      state = state.copyWith(
        isListening: false,
        status: SyncStatus.error,
        errorMessage: 'Unexpected sync error: $e',
      );
    }
  }

  /// Stop listening for incoming connections.
  void stopListening() {
    _shouldListen = false;
    state = state.copyWith(isListening: false);
  }

  /// Sync with a discovered device as initiator
  Future<void> syncWithDevice({
    required String accountId,
    required frb.DiscoveredDevice device,
    required List<int> pairingKey,
    required List<int> deviceSalt,
  }) async {
    state = state.copyWith(status: SyncStatus.syncing, errorMessage: null);
    try {
      final addr = '${device.addresses.first}:${device.port}';
      final result = await SyncService.instance.syncAsInitiator(
        accountId: accountId,
        remoteAddr: addr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
      );
      state = state.copyWith(
        status: SyncStatus.success,
        lastResult: result,
      );
    } on Exception catch (e) {
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Sync failed: $e',
      );
    } on Object catch (e) {
      SoloLog.e('SyncNotifier', 'Unexpected sync error', e);
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Unexpected sync error: $e',
      );
    }
  }

  /// Sync with a remote address as initiator
  Future<void> syncWithAddress({
    required String accountId,
    required String remoteAddr,
    required List<int> pairingKey,
    required List<int> deviceSalt,
  }) async {
    state = state.copyWith(status: SyncStatus.syncing, errorMessage: null);
    try {
      final result = await SyncService.instance.syncAsInitiator(
        accountId: accountId,
        remoteAddr: remoteAddr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
      );
      state = state.copyWith(
        status: SyncStatus.success,
        lastResult: result,
      );
    } on Exception catch (e) {
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Sync failed: $e',
      );
    } on Object catch (e) {
      SoloLog.e('SyncNotifier', 'Unexpected sync error', e);
      state = state.copyWith(
        status: SyncStatus.error,
        errorMessage: 'Unexpected sync error: $e',
      );
    }
  }

  /// Reset state to idle
  void reset() {
    state = const SyncState();
  }

  String _getDeviceName() {
    return getDeviceName();
  }
}

/// Provider for sync state
final syncProvider = NotifierProvider<SyncNotifier, SyncState>(SyncNotifier.new);
