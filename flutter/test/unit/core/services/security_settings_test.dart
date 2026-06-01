import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';

void main() {
  group('SecuritySettings', () {
    test('has correct defaults', () {
      const settings = SecuritySettings();
      expect(settings.autoLockDelayMinutes, 5);
      expect(settings.clipboardClearDelaySeconds, 60);
      expect(settings.lockOnWindowBlur, isTrue);
      expect(settings.biometricsEnabled, isFalse);
      expect(settings.faceIdEnabled, isFalse);
      expect(settings.privacyScreenEnabled, isTrue);
    });

    test('copyWith updates single field', () {
      const settings = SecuritySettings();
      final copy = settings.copyWith(autoLockDelayMinutes: 15);
      expect(copy.autoLockDelayMinutes, 15);
      expect(copy.clipboardClearDelaySeconds, settings.clipboardClearDelaySeconds);
      expect(copy.lockOnWindowBlur, settings.lockOnWindowBlur);
    });

    test('copyWith updates multiple fields', () {
      const settings = SecuritySettings();
      final copy = settings.copyWith(
        biometricsEnabled: true,
        faceIdEnabled: true,
      );
      expect(copy.biometricsEnabled, isTrue);
      expect(copy.faceIdEnabled, isTrue);
      expect(copy.autoLockDelayMinutes, settings.autoLockDelayMinutes);
    });

    test('copyWith preserves fields when null', () {
      const settings = SecuritySettings(autoLockDelayMinutes: 30);
      final copy = settings.copyWith();
      expect(copy.autoLockDelayMinutes, 30);
    });

    test('toJson serializes all fields', () {
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
      expect(json['lockOnWindowBlur'], isFalse);
      expect(json['biometricsEnabled'], isTrue);
      expect(json['faceIdEnabled'], isTrue);
      expect(json['privacyScreenEnabled'], isFalse);
    });

    test('fromJson deserializes all fields', () {
      final json = {
        'autoLockDelayMinutes': 30,
        'clipboardClearDelaySeconds': 30,
        'lockOnWindowBlur': false,
        'biometricsEnabled': true,
        'faceIdEnabled': false,
        'privacyScreenEnabled': true,
      };
      final restored = SecuritySettings.fromJson(json);
      expect(restored.autoLockDelayMinutes, 30);
      expect(restored.clipboardClearDelaySeconds, 30);
      expect(restored.lockOnWindowBlur, isFalse);
      expect(restored.biometricsEnabled, isTrue);
      expect(restored.faceIdEnabled, isFalse);
      expect(restored.privacyScreenEnabled, isTrue);
    });

    test('fromJson uses defaults for missing fields', () {
      final json = <String, dynamic>{};
      final restored = SecuritySettings.fromJson(json);
      expect(restored.autoLockDelayMinutes, 5);
      expect(restored.clipboardClearDelaySeconds, 60);
      expect(restored.lockOnWindowBlur, isTrue);
      expect(restored.biometricsEnabled, isFalse);
      expect(restored.faceIdEnabled, isFalse);
      expect(restored.privacyScreenEnabled, isTrue);
    });

    test('fromJson uses defaults for null values', () {
      final json = {
        'autoLockDelayMinutes': null,
        'clipboardClearDelaySeconds': null,
        'lockOnWindowBlur': null,
        'biometricsEnabled': null,
        'faceIdEnabled': null,
        'privacyScreenEnabled': null,
      };
      final restored = SecuritySettings.fromJson(json);
      expect(restored.autoLockDelayMinutes, 5);
      expect(restored.clipboardClearDelaySeconds, 60);
      expect(restored.lockOnWindowBlur, isTrue);
      expect(restored.biometricsEnabled, isFalse);
      expect(restored.faceIdEnabled, isFalse);
      expect(restored.privacyScreenEnabled, isTrue);
    });

    test('autoLockDelayOptions contains expected values', () {
      expect(SecuritySettings.autoLockDelayOptions, contains(1));
      expect(SecuritySettings.autoLockDelayOptions, contains(5));
      expect(SecuritySettings.autoLockDelayOptions, contains(15));
      expect(SecuritySettings.autoLockDelayOptions, contains(30));
      expect(SecuritySettings.autoLockDelayOptions, contains(-1));
    });

    test('clipboardClearDelayOptions contains expected values', () {
      expect(SecuritySettings.clipboardClearDelayOptions, contains(30));
      expect(SecuritySettings.clipboardClearDelayOptions, contains(60));
      expect(SecuritySettings.clipboardClearDelayOptions, contains(120));
      expect(SecuritySettings.clipboardClearDelayOptions, contains(-1));
    });

    test('autoLockDelayLabel formats correctly', () {
      expect(const SecuritySettings(autoLockDelayMinutes: 5).autoLockDelayLabel, '5 min');
      expect(const SecuritySettings(autoLockDelayMinutes: 15).autoLockDelayLabel, '15 min');
    });

    test('autoLockDelayLabel shows Never for -1', () {
      expect(const SecuritySettings(autoLockDelayMinutes: -1).autoLockDelayLabel, 'Never');
    });

    test('clipboardClearDelayLabel formats correctly', () {
      expect(const SecuritySettings(clipboardClearDelaySeconds: 30).clipboardClearDelayLabel, '30 sec');
      expect(const SecuritySettings(clipboardClearDelaySeconds: 60).clipboardClearDelayLabel, '60 sec');
    });

    test('clipboardClearDelayLabel shows Never for -1', () {
      expect(const SecuritySettings(clipboardClearDelaySeconds: -1).clipboardClearDelayLabel, 'Never');
    });

    test('round-trip serialization', () {
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
      expect(restored.clipboardClearDelaySeconds, original.clipboardClearDelaySeconds);
      expect(restored.lockOnWindowBlur, original.lockOnWindowBlur);
      expect(restored.biometricsEnabled, original.biometricsEnabled);
      expect(restored.faceIdEnabled, original.faceIdEnabled);
      expect(restored.privacyScreenEnabled, original.privacyScreenEnabled);
    });
  });
}
