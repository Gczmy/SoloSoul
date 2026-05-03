import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// Service for multi-device synchronization via local network.
///
/// Uses mDNS for device discovery and CRDT-based sync for profile data.
/// All communication is encrypted via Noise_IK protocol.
class SyncService {
  SyncService._();

  static SyncService? _instance;
  static SyncService get instance => _instance ??= SyncService._();

  static const _syncPort = 9900;

  /// Discover SoloSoul devices on the local network.
  ///
  /// Returns a list of discovered devices after [timeout] milliseconds.
  Future<List<frb.DiscoveredDevice>> discoverDevices({
    int timeoutMs = 3000,
  }) async {
    SoloLog.d('SyncService', 'Starting mDNS discovery (${timeoutMs}ms)');
    try {
      final devices = await frb.frbMdnsDiscover(
        timeoutMs: BigInt.from(timeoutMs),
      );
      SoloLog.d('SyncService', 'Discovered ${devices.length} devices');
      return devices;
    } on Exception catch (e, st) {
      SoloLog.e('SyncService', 'mDNS discovery failed', e, st);
      return [];
    }
  }

  /// Advertise this device on the local network.
  ///
  /// [deviceName] should be unique per device (e.g. "MacBook-Pro").
  Future<bool> advertise({required String deviceName}) async {
    SoloLog.d('SyncService', 'Advertising as "$deviceName" on port $_syncPort');
    try {
      await frb.frbMdnsAdvertise(
        deviceName: deviceName,
        port: _syncPort,
      );
      SoloLog.d('SyncService', 'Advertising started');
      return true;
    } on Exception catch (e, st) {
      SoloLog.e('SyncService', 'mDNS advertise failed', e, st);
      return false;
    }
  }

  /// Initiate sync with a remote device.
  ///
  /// This device sends its state vector first, then applies the remote diff.
  Future<frb.SyncResult> syncAsInitiator({
    required String accountId,
    required String remoteAddr,
    required List<int> pairingKey,
    required List<int> deviceSalt,
  }) async {
    SoloLog.d('SyncService', 'Initiating sync with $remoteAddr');
    final timer = SoloLog.startTimer('SyncService', 'syncAsInitiator');
    try {
      final result = await frb.frbSyncInitiator(
        accountId: accountId,
        remoteAddr: remoteAddr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
      );
      SoloLog.d('SyncService', 'Sync complete: direction=${result.direction}');
      SoloLog.endTimer(timer);
      return result;
    } on Exception catch (e, st) {
      SoloLog.e('SyncService', 'Sync initiator failed', e, st);
      SoloLog.endTimer(timer);
      rethrow;
    }
  }

  /// Respond to an incoming sync request.
  ///
  /// This device receives the remote state vector first, then sends its diff.
  Future<frb.SyncResult> syncAsResponder({
    required String accountId,
    required String remoteAddr,
    required List<int> pairingKey,
    required List<int> deviceSalt,
  }) async {
    SoloLog.d('SyncService', 'Responding to sync from $remoteAddr');
    final timer = SoloLog.startTimer('SyncService', 'syncAsResponder');
    try {
      final result = await frb.frbSyncResponder(
        accountId: accountId,
        remoteAddr: remoteAddr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
      );
      SoloLog.d('SyncService', 'Sync complete: direction=${result.direction}');
      SoloLog.endTimer(timer);
      return result;
    } on Exception catch (e, st) {
      SoloLog.e('SyncService', 'Sync responder failed', e, st);
      SoloLog.endTimer(timer);
      rethrow;
    }
  }

  /// Generate a random pairing key for Noise handshake.
  Future<List<int>> generatePairingKey() async {
    final salt = await frb.frbGenerateSalt(length: 32);
    return salt.toList();
  }

  /// Generate a device-unique salt for key derivation.
  Future<List<int>> generateDeviceSalt() async {
    final salt = await frb.frbGenerateSalt(length: 32);
    return salt.toList();
  }
}
