import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
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

  group('SecurityService', () {
    test('instance returns singleton', () {
      final a = SecurityService.instance;
      final b = SecurityService.instance;
      expect(identical(a, b), isTrue);
    });

    test('isInitialized is false before loadSettings', () {
      expect(SecurityService.instance.isInitialized, isFalse);
    });

    test('settings returns default before loadSettings', () {
      final settings = SecurityService.instance.settings;
      expect(settings.autoLockDelayMinutes, 5);
      expect(settings.clipboardClearDelaySeconds, 60);
      expect(settings.lockOnWindowBlur, isTrue);
      expect(settings.biometricsEnabled, isFalse);
      expect(settings.faceIdEnabled, isFalse);
      expect(settings.privacyScreenEnabled, isTrue);
    });

    test('loadSettings with no data uses defaults and marks initialized', () async {
      await SecurityService.instance.loadSettings();
      expect(SecurityService.instance.isInitialized, isTrue);
      expect(SecurityService.instance.settings.autoLockDelayMinutes, 5);
    });

    test('loadSettings reads saved settings', () async {
      secureStorageData['security_settings'] =
          '{"autoLockDelayMinutes":15,"clipboardClearDelaySeconds":120,"lockOnWindowBlur":false,"biometricsEnabled":true,"faceIdEnabled":true,"privacyScreenEnabled":false}';
      await SecurityService.instance.loadSettings();
      expect(SecurityService.instance.settings.autoLockDelayMinutes, 15);
      expect(SecurityService.instance.settings.clipboardClearDelaySeconds, 120);
      expect(SecurityService.instance.settings.lockOnWindowBlur, isFalse);
      expect(SecurityService.instance.settings.biometricsEnabled, isTrue);
    });

    test('setAutoLockDelay updates setting', () async {
      await SecurityService.instance.setAutoLockDelay(30);
      expect(SecurityService.instance.settings.autoLockDelayMinutes, 30);
    });

    test('setClipboardClearDelay updates setting', () async {
      await SecurityService.instance.setClipboardClearDelay(-1);
      expect(SecurityService.instance.settings.clipboardClearDelaySeconds, -1);
    });

    test('setLockOnWindowBlur updates setting', () async {
      await SecurityService.instance.setLockOnWindowBlur(false);
      expect(SecurityService.instance.settings.lockOnWindowBlur, isFalse);
    });

    test('resetToDefaults restores initial values', () async {
      await SecurityService.instance.setAutoLockDelay(30);
      await SecurityService.instance.resetToDefaults();
      expect(SecurityService.instance.settings.autoLockDelayMinutes, 5);
      expect(SecurityService.instance.settings.clipboardClearDelaySeconds, 60);
    });
  });

  group('SecuritySettings', () {
    test('default constructor has correct values', () {
      const settings = SecuritySettings();
      expect(settings.autoLockDelayMinutes, 5);
      expect(settings.clipboardClearDelaySeconds, 60);
      expect(settings.lockOnWindowBlur, isTrue);
      expect(settings.biometricsEnabled, isFalse);
      expect(settings.faceIdEnabled, isFalse);
      expect(settings.privacyScreenEnabled, isTrue);
    });

    test('custom constructor', () {
      const settings = SecuritySettings(
        autoLockDelayMinutes: 15,
        clipboardClearDelaySeconds: 120,
        lockOnWindowBlur: false,
        biometricsEnabled: true,
        faceIdEnabled: true,
        privacyScreenEnabled: false,
      );
      expect(settings.autoLockDelayMinutes, 15);
      expect(settings.clipboardClearDelaySeconds, 120);
      expect(settings.lockOnWindowBlur, isFalse);
      expect(settings.biometricsEnabled, isTrue);
      expect(settings.faceIdEnabled, isTrue);
      expect(settings.privacyScreenEnabled, isFalse);
    });

    group('copyWith', () {
      test('copies with no changes', () {
        const original = SecuritySettings();
        final copy = original.copyWith();
        expect(copy.autoLockDelayMinutes, original.autoLockDelayMinutes);
        expect(
          copy.clipboardClearDelaySeconds,
          original.clipboardClearDelaySeconds,
        );
        expect(copy.lockOnWindowBlur, original.lockOnWindowBlur);
        expect(copy.biometricsEnabled, original.biometricsEnabled);
        expect(copy.faceIdEnabled, original.faceIdEnabled);
        expect(copy.privacyScreenEnabled, original.privacyScreenEnabled);
      });

      test('copies with changes', () {
        const original = SecuritySettings();
        final copy = original.copyWith(
          autoLockDelayMinutes: 30,
          biometricsEnabled: true,
        );
        expect(copy.autoLockDelayMinutes, 30);
        expect(copy.biometricsEnabled, isTrue);
        // Unchanged fields
        expect(
          copy.clipboardClearDelaySeconds,
          original.clipboardClearDelaySeconds,
        );
        expect(copy.lockOnWindowBlur, original.lockOnWindowBlur);
      });
    });

    group('JSON serialization', () {
      test('toJson produces correct map', () {
        const settings = SecuritySettings(
          autoLockDelayMinutes: 15,
          clipboardClearDelaySeconds: 120,
          lockOnWindowBlur: false,
          biometricsEnabled: true,
          faceIdEnabled: true,
          privacyScreenEnabled: false,
        );
        final json = settings.toJson();
        expect(json['autoLockDelayMinutes'], 15);
        expect(json['clipboardClearDelaySeconds'], 120);
        expect(json['lockOnWindowBlur'], false);
        expect(json['biometricsEnabled'], true);
        expect(json['faceIdEnabled'], true);
        expect(json['privacyScreenEnabled'], false);
      });

      test('fromJson round-trips correctly', () {
        const original = SecuritySettings(
          autoLockDelayMinutes: 30,
          clipboardClearDelaySeconds: 120,
          lockOnWindowBlur: false,
          biometricsEnabled: true,
          faceIdEnabled: true,
          privacyScreenEnabled: false,
        );
        final json = original.toJson();
        final restored = SecuritySettings.fromJson(json);
        expect(restored.autoLockDelayMinutes, original.autoLockDelayMinutes);
        expect(
          restored.clipboardClearDelaySeconds,
          original.clipboardClearDelaySeconds,
        );
        expect(restored.lockOnWindowBlur, original.lockOnWindowBlur);
        expect(restored.biometricsEnabled, original.biometricsEnabled);
        expect(restored.faceIdEnabled, original.faceIdEnabled);
        expect(restored.privacyScreenEnabled, original.privacyScreenEnabled);
      });

      test('fromJson uses defaults for missing fields', () {
        final restored = SecuritySettings.fromJson({});
        expect(restored.autoLockDelayMinutes, 5);
        expect(restored.clipboardClearDelaySeconds, 60);
        expect(restored.lockOnWindowBlur, isTrue);
        expect(restored.biometricsEnabled, isFalse);
        expect(restored.faceIdEnabled, isFalse);
        expect(restored.privacyScreenEnabled, isTrue);
      });

      test('fromJson handles partial data', () {
        final json = {
          'autoLockDelayMinutes': 15,
          'biometricsEnabled': true,
        };
        final restored = SecuritySettings.fromJson(json);
        expect(restored.autoLockDelayMinutes, 15);
        expect(restored.biometricsEnabled, isTrue);
        // Defaults for missing
        expect(restored.clipboardClearDelaySeconds, 60);
        expect(restored.lockOnWindowBlur, isTrue);
      });
    });

    group('autoLockDelayLabel', () {
      test('returns minutes label', () {
        expect(
          const SecuritySettings(autoLockDelayMinutes: 5).autoLockDelayLabel,
          '5 min',
        );
        expect(
          const SecuritySettings(autoLockDelayMinutes: 15).autoLockDelayLabel,
          '15 min',
        );
      });

      test('returns Never for -1', () {
        expect(
          const SecuritySettings(autoLockDelayMinutes: -1).autoLockDelayLabel,
          'Never',
        );
      });
    });

    group('clipboardClearDelayLabel', () {
      test('returns seconds label', () {
        expect(
          const SecuritySettings(
            clipboardClearDelaySeconds: 60,
          ).clipboardClearDelayLabel,
          '60 sec',
        );
      });

      test('returns Never for -1', () {
        expect(
          const SecuritySettings(
            clipboardClearDelaySeconds: -1,
          ).clipboardClearDelayLabel,
          'Never',
        );
      });
    });

    test('autoLockDelayOptions contains expected values', () {
      expect(SecuritySettings.autoLockDelayOptions, [1, 5, 15, 30, -1]);
    });

    test('clipboardClearDelayOptions contains expected values', () {
      expect(SecuritySettings.clipboardClearDelayOptions, [30, 60, 120, -1]);
    });
  });
}
