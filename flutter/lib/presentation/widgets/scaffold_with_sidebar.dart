import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/widgets/app_sidebar.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_progress_banner.dart';

/// Persistent shell layout with sidebar + main content area.
/// Used as the builder for GoRouter ShellRoute.
class ScaffoldWithSidebar extends StatelessWidget {
  final Widget child;

  const ScaffoldWithSidebar({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Column(
        children: [
          const ScanProgressBanner(),
          Expanded(
            child: Row(
              children: [
                const AppSidebar(),
                const VerticalDivider(width: 1),
                Expanded(child: child),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
