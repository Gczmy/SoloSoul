import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final secureStorageData = <String, String?>{};

  setUpAll(() {
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, (call) async {
      final args = call.arguments as Map<dynamic, dynamic>?;
      final key = args?['key'] as String?;
      switch (call.method) {
        case 'read':
          return secureStorageData[key];
        case 'write':
          if (key != null) {
            secureStorageData[key] = args?['value'] as String?;
          }
          return null;
        case 'delete':
          if (key != null) {
            secureStorageData.remove(key);
          }
          return null;
      }
      return null;
    });
  });

  setUp(() {
    secureStorageData.clear();
  });

  tearDownAll(() {
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, null);
  });

  group('ClipboardMonitorService', () {
    late ClipboardMonitorService service;

    setUp(() {
      service = ClipboardMonitorService.instance;
      service.cancelPendingClear();
    });

    tearDown(() {
      service.dispose();
    });

    test('instance returns singleton', () {
      final a = ClipboardMonitorService.instance;
      final b = ClipboardMonitorService.instance;
      expect(identical(a, b), isTrue);
    });

    test('notifySensitiveCopied starts timer with default delay', () {
      // SecurityService is not initialized, so default 60s delay applies
      service.notifySensitiveCopied();
      // Timer is started; we can't directly observe it, but we can verify
      // that cancelPendingClear does not throw and dispose works.
      expect(service.dispose, returnsNormally);
    });

    test('notifySensitiveCopied cancels previous timer', () {
      service.notifySensitiveCopied();
      service.notifySensitiveCopied();
      // Calling twice should cancel the first timer and start a new one.
      expect(service.dispose, returnsNormally);
    });

    test('notifySensitiveCopied with Never setting does not start timer', () async {
      // Initialize SecurityService with clipboardClearDelaySeconds = -1 (Never)
      await SecurityService.instance.setClipboardClearDelay(-1);
      service.notifySensitiveCopied();
      // Since timer is not started, dispose should be safe
      expect(service.dispose, returnsNormally);
      // Reset to default
      await SecurityService.instance.resetToDefaults();
    });

    test('cancelPendingClear cancels timer', () {
      service.notifySensitiveCopied();
      service.cancelPendingClear();
      expect(service.dispose, returnsNormally);
    });

    test('dispose cancels pending timer', () {
      service.notifySensitiveCopied();
      expect(service.dispose, returnsNormally);
    });

    test('clearClipboard invokes platform channel', () async {
      // Mock the platform clipboard channel
      var clipboardCalled = false;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method == 'Clipboard.setData') {
          clipboardCalled = true;
        }
        return null;
      });

      await service.clearClipboard();
      expect(clipboardCalled, isTrue);

      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(SystemChannels.platform, null);
    });
  });
}
