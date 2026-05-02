import 'package:flutter/services.dart';

/// Native channel service for receiving events from macOS native code
class NativeChannelService {
  static const _channel = MethodChannel('com.solosoul/native');

  static Function()? onLockVault;
  static Function()? onSystemWillSleep;
  static Function()? onSystemDidWake;

  /// Initialize native channel handlers
  static void initialize() {
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'lockVault':
          onLockVault?.call();
          return null;
        case 'onSystemWillSleep':
          onSystemWillSleep?.call();
          return null;
        case 'onSystemDidWake':
          onSystemDidWake?.call();
          return null;
        default:
          throw MissingPluginException('Not implemented: ${call.method}');
      }
    });
  }

  /// Set the method channel for lock vault callback
  static void setLockCallback(Function() callback) {
    onLockVault = callback;
  }

  /// Set callback for system sleep event — should lock vault and clear sensitive memory
  static void setSleepCallback(Function() callback) {
    onSystemWillSleep = callback;
  }

  /// Set callback for system wake event — should re-validate session
  static void setWakeCallback(Function() callback) {
    onSystemDidWake = callback;
  }
}
