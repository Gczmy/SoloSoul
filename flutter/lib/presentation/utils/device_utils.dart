import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// Returns the current device's human-readable name.
///
/// Uses the system hostname on desktop platforms so that multiple Macs
/// (or Windows / Linux machines) are distinguishable in the device list.
/// Falls back to generic platform name on mobile.
String getDeviceName() {
  try {
    final hostname = Platform.localHostname;
    // Strip .local suffix from mDNS hostnames, return human-friendly name.
    final name = hostname.endsWith('.local')
        ? hostname.substring(0, hostname.length - 6)
        : hostname;
    if (name.isNotEmpty) {
      final lower = name.toLowerCase();
      // Windows/Linux hostnames usually don't contain platform keywords,
      // append a platform tag so getDevicePlatformLabel / getDeviceIcon
      // can correctly identify them later.
      if (Platform.isWindows && !lower.contains('windows')) {
        return '$name (Windows)';
      }
      if (Platform.isLinux && !lower.contains('linux')) {
        return '$name (Linux)';
      }
      return name;
    }
  } on Object catch (e) {
    if (kDebugMode) {
      debugPrint('[DeviceUtils] Failed to get hostname: $e');
    }
  }
  // Fallback to generic platform name.
  if (Platform.isMacOS) return 'Mac';
  if (Platform.isIOS) return 'iPhone';
  if (Platform.isAndroid) return 'Android';
  if (Platform.isLinux) return 'Linux';
  if (Platform.isWindows) return 'Windows';
  if (Platform.isFuchsia) return 'Fuchsia';
  return 'Unknown';
}

/// Returns a display-friendly device name.
/// If the raw device name already contains a platform identifier
/// (e.g. "DESKTOP-ABC (Windows)"), returns it as-is.
/// Otherwise prepends the platform label (e.g. "[macOS] MacBook-Pro").
String getDisplayDeviceName(String deviceName) {
  final label = getDevicePlatformLabel(deviceName);
  if (label.isEmpty) return deviceName;

  // Check whether the device name already contains the platform keyword
  // (e.g. "(Windows)" or "[Windows]").
  final tag = label.replaceAll('[', '').replaceAll(']', '').toLowerCase();
  if (deviceName.toLowerCase().contains(tag)) return deviceName;

  return '$label $deviceName';
}

/// Returns an appropriate device icon based on the device name.
IconData getDeviceIcon(String deviceName) {
  final lower = deviceName.toLowerCase();
  if (lower.contains('iphone') || lower.contains('ios')) {
    return Icons.phone_iphone;
  }
  if (lower.contains('android')) return Icons.phone_android;
  if (lower.contains('mac') || lower.contains('darwin')) {
    return Icons.laptop_mac;
  }
  if (lower.contains('windows')) return Icons.desktop_windows;
  if (lower.contains('linux')) return Icons.computer;
  if (lower.contains('web') || lower.contains('browser')) return Icons.web;
  return Icons.devices;
}

/// Returns a short platform label for the device (e.g. "[macOS]", "[iOS]").
String getDevicePlatformLabel(String deviceName) {
  final lower = deviceName.toLowerCase();
  if (lower.contains('iphone') || lower.contains('ios')) return '[iOS]';
  if (lower.contains('android')) return '[Android]';
  if (lower.contains('mac') || lower.contains('darwin')) return '[macOS]';
  if (lower.contains('windows')) return '[Windows]';
  if (lower.contains('linux')) return '[Linux]';
  if (lower.contains('web') || lower.contains('browser')) return '[Web]';
  return '';
}
