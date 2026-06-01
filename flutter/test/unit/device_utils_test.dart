import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/device_utils.dart';

void main() {
  group('getDeviceIcon', () {
    test('returns phone_iphone for iPhone', () {
      expect(getDeviceIcon('iPhone'), Icons.phone_iphone);
      expect(getDeviceIcon('My iOS Device'), Icons.phone_iphone);
    });

    test('returns phone_android for Android', () {
      expect(getDeviceIcon('Android'), Icons.phone_android);
      expect(getDeviceIcon('Samsung Android'), Icons.phone_android);
    });

    test('returns laptop_mac for Mac', () {
      expect(getDeviceIcon('MacBook Pro'), Icons.laptop_mac);
      expect(getDeviceIcon('Darwin'), Icons.laptop_mac);
    });

    test('returns desktop_windows for Windows', () {
      expect(getDeviceIcon('Windows PC'), Icons.desktop_windows);
    });

    test('returns computer for Linux', () {
      expect(getDeviceIcon('Linux Server'), Icons.computer);
    });

    test('returns web for Web', () {
      expect(getDeviceIcon('Web Browser'), Icons.web);
    });

    test('returns devices for unknown', () {
      expect(getDeviceIcon('Unknown'), Icons.devices);
    });
  });

  group('getDevicePlatformLabel', () {
    test('returns [iOS] for iPhone', () {
      expect(getDevicePlatformLabel('iPhone'), '[iOS]');
    });

    test('returns [Android] for Android', () {
      expect(getDevicePlatformLabel('Android'), '[Android]');
    });

    test('returns [macOS] for Mac', () {
      expect(getDevicePlatformLabel('MacBook'), '[macOS]');
      expect(getDevicePlatformLabel('Darwin'), '[macOS]');
    });

    test('returns [Windows] for Windows', () {
      expect(getDevicePlatformLabel('Windows'), '[Windows]');
    });

    test('returns [Linux] for Linux', () {
      expect(getDevicePlatformLabel('Linux'), '[Linux]');
    });

    test('returns [Web] for Web', () {
      expect(getDevicePlatformLabel('Web'), '[Web]');
    });

    test('returns empty for unknown', () {
      expect(getDevicePlatformLabel('Unknown'), '');
    });
  });
}
