import 'package:flutter/material.dart';

class QuickAction {
  final IconData icon;
  final String label;
  final String route;
  final Color color;
  final bool isCustom;
  const QuickAction({
    required this.icon,
    required this.label,
    required this.route,
    required this.color,
    this.isCustom = false,
  });
}
