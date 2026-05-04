import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/utils/device_utils.dart';

void main() {
  group('getDeviceIcon', () {
    test('returns phone_iphone for iPhone', () {
      expect(getDeviceIcon('iPhone'), Icons.phone_iphone);
      expect(getDeviceIcon('iphone'), Icons.phone_iphone);
      expect(getDeviceIcon('iOS Device'), Icons.phone_iphone);
    });

    test('returns phone_android for Android', () {
      expect(getDeviceIcon('Android'), Icons.phone_android);
      expect(getDeviceIcon('android phone'), Icons.phone_android);
    });

    test('returns laptop_mac for Mac', () {
      expect(getDeviceIcon('Mac'), Icons.laptop_mac);
      expect(getDeviceIcon('MacBook Pro'), Icons.laptop_mac);
      expect(getDeviceIcon('darwin'), Icons.laptop_mac);
    });

    test('returns desktop_windows for Windows', () {
      expect(getDeviceIcon('Windows'), Icons.desktop_windows);
      expect(getDeviceIcon('windows pc'), Icons.desktop_windows);
    });

    test('returns computer for Linux', () {
      expect(getDeviceIcon('Linux'), Icons.computer);
      expect(getDeviceIcon('linux'), Icons.computer);
    });

    test('returns web for browser/web', () {
      expect(getDeviceIcon('Web'), Icons.web);
      expect(getDeviceIcon('browser'), Icons.web);
    });

    test('returns devices for unknown', () {
      expect(getDeviceIcon('Unknown'), Icons.devices);
      expect(getDeviceIcon(''), Icons.devices);
      expect(getDeviceIcon('Fuchsia'), Icons.devices);
    });
  });
}
