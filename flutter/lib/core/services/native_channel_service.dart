import 'package:flutter/services.dart';

/// Native channel service for receiving events from macOS native code
class NativeChannelService {
  static const _channel = MethodChannel('com.solosoul/native');

  static Function()? onLockVault;

  /// Initialize native channel handlers
  static void initialize() {
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'lockVault':
          onLockVault?.call();
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
}
