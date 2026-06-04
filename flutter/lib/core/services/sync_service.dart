import 'dart:async';
import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;

/// Service for multi-device synchronization via local network.
///
/// Uses mDNS for device discovery and CRDT-based sync for profile data.
/// All communication is encrypted via Noise_IK protocol.
/// Attachment files are synced over the same encrypted channel.
class SyncService {
  SyncService._();

  static SyncService? _instance;
  static SyncService get instance => _instance ??= SyncService._();

  static const _syncPort = 9900;
  static const _attachmentsDirName = 'solosoul_storage/attachments';

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

  /// Get the attachments directory path for an account.
  Future<String> _getAttachmentsDir(String accountId) async {
    final appDir = await getApplicationDocumentsDirectory();
    final dir = Directory('${appDir.path}/$_attachmentsDirName/$accountId');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir.path;
  }

  /// Initiate sync with a remote device.
  ///
  /// This device sends its state vector first, then applies the remote diff.
  /// Attachment files are synced after CRDT sync completes.
  Future<frb.SyncResult> syncAsInitiator({
    required String accountId,
    required String remoteAddr,
    required List<int> pairingKey,
    required List<int> deviceSalt,
    void Function(String level, String message)? onLog,
  }) async {
    void log(String level, String message) {
      SoloLog.d('SyncService', message);
      onLog?.call(level, message);
    }

    log('INFO', 'Initiating sync with $remoteAddr');
    final timer = SoloLog.startTimer('SyncService', 'syncAsInitiator');
    try {
      final attachmentsDir = await _getAttachmentsDir(accountId);
      log('INFO', 'Attachments dir: $attachmentsDir');
      log('INFO', 'Connecting to $remoteAddr ...');

      final result = await frb.frbSyncInitiator(
        accountId: accountId,
        remoteAddr: remoteAddr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
        attachmentsDir: attachmentsDir,
      ).timeout(const Duration(seconds: 30));

      log('INFO', 'CRDT sync complete: direction=${result.direction}, '
          'bytes=${result.bytesSent} sent / ${result.bytesReceived} received');
      log('INFO', 'Attachment sync: ${result.attachmentsSent} files sent / ${result.attachmentsReceived} files received, '
          '${result.attachmentBytesSent} bytes sent / ${result.attachmentBytesReceived} bytes received, '
          'incomplete=${result.attachmentIncomplete}');
      log('INFO', 'Sync complete with $remoteAddr');
      SoloLog.endTimer(timer);
      return result;
    } on Exception catch (e, st) {
      log('ERROR', 'Sync initiator failed: $e');
      SoloLog.e('SyncService', 'Sync initiator failed', e, st);
      SoloLog.endTimer(timer);
      rethrow;
    }
  }

  /// Respond to an incoming sync request.
  ///
  /// Listens on 0.0.0.0:9900 for an incoming connection, then performs sync.
  /// This device receives the remote state vector first, then sends its diff.
  Future<frb.SyncResult> syncAsResponder({
    required String accountId,
    required List<int> pairingKey,
    required List<int> deviceSalt,
    void Function(String level, String message)? onLog,
  }) async {
    const listenAddr = '0.0.0.0:9900';

    void log(String level, String message) {
      SoloLog.d('SyncService', message);
      onLog?.call(level, message);
    }

    log('INFO', 'Listening for sync on $listenAddr');
    final timer = SoloLog.startTimer('SyncService', 'syncAsResponder');
    try {
      final attachmentsDir = await _getAttachmentsDir(accountId);
      log('INFO', 'Attachments dir: $attachmentsDir');

      final result = await frb.frbSyncResponder(
        accountId: accountId,
        remoteAddr: listenAddr,
        pairingKey: pairingKey,
        deviceSalt: deviceSalt,
        attachmentsDir: attachmentsDir,
      ).timeout(const Duration(seconds: 90));

      log('INFO', 'CRDT sync complete: direction=${result.direction}, '
          'bytes=${result.bytesSent} sent / ${result.bytesReceived} received');
      log('INFO', 'Attachment sync: ${result.attachmentsSent} files sent / ${result.attachmentsReceived} files received, '
          '${result.attachmentBytesSent} bytes sent / ${result.attachmentBytesReceived} bytes received, '
          'incomplete=${result.attachmentIncomplete}');
      log('INFO', 'Sync complete with remote peer');
      SoloLog.endTimer(timer);
      return result;
    } on Exception catch (e, st) {
      log('ERROR', 'Sync responder failed: $e');
      SoloLog.e('SyncService', 'Sync responder failed', e, st);
      SoloLog.endTimer(timer);
      rethrow;
    }
  }

  /// Get local IP addresses, filtering out VPN/virtual interfaces.
  static Future<List<String>> getLocalIps() async {
    final results = <String>[];
    try {
      final interfaces = await NetworkInterface.list();
      for (final interface in interfaces) {
        final name = interface.name.toLowerCase();
        // Skip loopback, VPN, tunnel, and virtual interfaces
        if (name.contains('lo') ||
            name.contains('utun') ||
            name.contains('tun') ||
            name.contains('ppp') ||
            name.contains('vmnet') ||
            name.contains('veth') ||
            name.contains('docker') ||
            name.contains('bridge')) {
          continue;
        }
        for (final addr in interface.addresses) {
          if (addr.type == InternetAddressType.IPv4 &&
              !addr.isLoopback &&
              !addr.address.startsWith('127.')) {
            results.add(addr.address);
          }
        }
      }
    } on Object catch (_) {}
    return results;
  }

  /// Get the best local IP address for display.
  static Future<String?> getLocalIp() async {
    final ips = await getLocalIps();
    return ips.isNotEmpty ? ips.first : null;
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
