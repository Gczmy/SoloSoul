import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  group('NativeChannelService', () {
    tearDown(() {
      NativeChannelService.onLockVault = null;
      NativeChannelService.onSystemWillSleep = null;
      NativeChannelService.onSystemDidWake = null;
    });

    test('initialize does not crash', () {
      NativeChannelService.initialize();
      expect(true, isTrue);
    });

    test('setLockCallback sets callback', () {
      var called = false;
      NativeChannelService.setLockCallback(() => called = true);
      NativeChannelService.onLockVault?.call();
      expect(called, isTrue);
    });

    test('setSleepCallback sets callback', () {
      var called = false;
      NativeChannelService.setSleepCallback(() => called = true);
      NativeChannelService.onSystemWillSleep?.call();
      expect(called, isTrue);
    });

    test('setWakeCallback sets callback', () {
      var called = false;
      NativeChannelService.setWakeCallback(() => called = true);
      NativeChannelService.onSystemDidWake?.call();
      expect(called, isTrue);
    });

    test('callbacks are null by default', () {
      expect(NativeChannelService.onLockVault, isNull);
      expect(NativeChannelService.onSystemWillSleep, isNull);
      expect(NativeChannelService.onSystemDidWake, isNull);
    });
  });
}
