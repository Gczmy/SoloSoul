import 'package:flutter/material.dart';

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
