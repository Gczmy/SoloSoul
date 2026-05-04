import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;
  final secureStorageData = <String, String?>{};
  var shouldThrowKeychainError = false;

  setUpAll(() async {
    tempDir = await Directory.systemTemp.createTemp('solosoul_fallback_test_');

    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(pathProviderChannel, (call) async {
      if (call.method == 'getApplicationSupportDirectory') {
        return tempDir.path;
      }
      return null;
    });

    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, (call) async {
      if (shouldThrowKeychainError) {
        throw PlatformException(
          code: 'errSecMissingEntitlement',
          message: '-34018 Keychain access denied',
        );
      }
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
        case 'deleteAll':
          secureStorageData.clear();
          return null;
      }
      return null;
    });
  });

  tearDownAll(() async {
    await tempDir.delete(recursive: true);
    const pathProviderChannel = MethodChannel('plugins.flutter.io/path_provider');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(pathProviderChannel, null);
    const secureStorageChannel = MethodChannel(
      'plugins.it_nomads.com/flutter_secure_storage',
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(secureStorageChannel, null);
  });

  setUp(() {
    secureStorageData.clear();
    shouldThrowKeychainError = false;
    final fallbackDir = Directory('${tempDir.path}/solosoul_fallback_storage');
    if (fallbackDir.existsSync()) {
      fallbackDir.deleteSync(recursive: true);
    }
  });

  group('FallbackSecureStorage', () {
    test('read returns value from secure storage', () async {
      final storage = FallbackSecureStorage();
      secureStorageData['my_key'] = 'my_value';
      final value = await storage.read(key: 'my_key');
      expect(value, 'my_value');
    });

    test('read returns null when key not found', () async {
      final storage = FallbackSecureStorage();
      final value = await storage.read(key: 'missing_key');
      expect(value, isNull);
    });

    test('read falls back to file storage on keychain error', () async {
      shouldThrowKeychainError = true;
      final storage = FallbackSecureStorage();
      await storage.write(key: 'fallback_key', value: 'fallback_value');
      secureStorageData.clear();
      final value = await storage.read(key: 'fallback_key');
      expect(value, 'fallback_value');
    });

    test('write stores value in secure storage', () async {
      final storage = FallbackSecureStorage();
      await storage.write(key: 'test_key', value: 'test_value');
      expect(secureStorageData['test_key'], 'test_value');
    });

    test('write falls back to file storage on keychain error', () async {
      shouldThrowKeychainError = true;
      final storage = FallbackSecureStorage();
      await storage.write(key: 'fb_key', value: 'fb_value');
      expect(secureStorageData['fb_key'], isNull);
      final value = await storage.read(key: 'fb_key');
      expect(value, 'fb_value');
    });

    test('write null deletes fallback file', () async {
      shouldThrowKeychainError = true;
      final storage = FallbackSecureStorage();
      await storage.write(key: 'del_key', value: 'to_delete');
      expect(await storage.read(key: 'del_key'), 'to_delete');
      await storage.write(key: 'del_key', value: null);
      expect(await storage.read(key: 'del_key'), isNull);
    });

    test('delete removes from secure storage', () async {
      final storage = FallbackSecureStorage();
      secureStorageData['del_key'] = 'val';
      await storage.delete(key: 'del_key');
      expect(secureStorageData.containsKey('del_key'), isFalse);
    });

    test('deleteAll clears secure storage and fallback', () async {
      final storage = FallbackSecureStorage();
      await storage.write(key: 'k1', value: 'v1');
      secureStorageData['k2'] = 'v2';
      await storage.deleteAll();
      expect(secureStorageData.isEmpty, isTrue);
      expect(await storage.read(key: 'k1'), isNull);
    });

    test('deleteAll clears fallback even when keychain errors', () async {
      shouldThrowKeychainError = true;
      final storage = FallbackSecureStorage();
      await storage.write(key: 'k1', value: 'v1');
      await storage.write(key: 'k2', value: 'v2');
      await storage.deleteAll();
      // secureStorage not cleared because keychain threw, but fallback is
      expect(await storage.read(key: 'k1'), isNull);
      expect(await storage.read(key: 'k2'), isNull);
    });
  });

  group('AppVersionTracker', () {
    late AppVersionTracker tracker;

    setUp(() {
      tracker = AppVersionTracker.instance;
    });

    test('checkVersion sets pending on first run', () async {
      await tracker.checkVersion('1.0.0');
      expect(tracker.currentVersion, '1.0.0');
      expect(tracker.pendingUpgradeBackup, isTrue);
    });

    test('checkVersion same version clears pending', () async {
      await tracker.checkVersion('1.0.0');
      tracker.clearPendingBackup();
      await tracker.checkVersion('1.0.0');
      expect(tracker.pendingUpgradeBackup, isFalse);
    });

    test('checkVersion different version sets pending', () async {
      await tracker.checkVersion('1.0.0');
      tracker.clearPendingBackup();
      await tracker.checkVersion('1.1.0');
      expect(tracker.currentVersion, '1.1.0');
      expect(tracker.pendingUpgradeBackup, isTrue);
    });

    test('clearPendingBackup resets flag', () async {
      await tracker.checkVersion('2.0.0');
      expect(tracker.pendingUpgradeBackup, isTrue);
      tracker.clearPendingBackup();
      expect(tracker.pendingUpgradeBackup, isFalse);
    });
  });
}
