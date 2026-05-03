import 'package:flutter/material.dart';

class QuickAction {
  final IconData icon;
  final String label;
  final String route;
  final Color color;
  const QuickAction({
    required this.icon,
    required this.label,
    required this.route,
    required this.color,
  });
}
