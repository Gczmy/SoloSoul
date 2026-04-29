import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'settings_common.dart';

/// Version info bottom sheet with hidden debug mode activation.
class VersionSheet extends ConsumerStatefulWidget {
  final AsyncValue<PackageInfo> packageInfo;
  final Future<void> Function() onDebugActivationRequested;

  const VersionSheet({
    super.key,
    required this.packageInfo,
    required this.onDebugActivationRequested,
  });

  @override
  ConsumerState<VersionSheet> createState() => VersionSheetState();
}

class VersionSheetState extends ConsumerState<VersionSheet> {
  int _tapCount = 0;
  DateTime? _lastTapTime;

  void _handleCurrentVersionTap() {
    final now = DateTime.now();
    // Reset if more than 2 seconds between taps
    if (_lastTapTime != null && now.difference(_lastTapTime!).inSeconds > 2) {
      _tapCount = 0;
    }
    _lastTapTime = now;
    _tapCount++;

    if (_tapCount >= 5) {
      _tapCount = 0;
      widget.onDebugActivationRequested();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    const latestVersion = '1.0.0';

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 12),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 24),
          Flexible(
            child: SingleChildScrollView(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Column(
                children: [
                  // App icon
                  Container(
                    width: 72,
                    height: 72,
                    decoration: BoxDecoration(
                      color: AppTheme.primaryColor.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(20),
                    ),
                    child: const Icon(
                      Icons.shield_outlined,
                      size: 40,
                      color: AppTheme.primaryColor,
                    ),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    'SoloSoul',
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 24),

                  // Version info items
                  GestureDetector(
                    onTap: _handleCurrentVersionTap,
                    child: VersionInfoTile(
                      icon: Icons.info_outline,
                      title: 'Current Version',
                      value: widget.packageInfo.when(
                        data: (info) => '${info.version}${_tapCount > 0 ? ' ($_tapCount)' : ''}',
                        loading: () => '...',
                        error: (_, __) => '1.0.0',
                      ),
                      trailing: _tapCount > 0
                          ? Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                              decoration: BoxDecoration(
                                color: AppTheme.primaryColor.withValues(alpha: 0.1),
                                borderRadius: BorderRadius.circular(10),
                              ),
                              child: const Text(
                                '/5',
                                style: TextStyle(
                                  color: AppTheme.primaryColor,
                                  fontSize: 11,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            )
                          : null,
                    ),
                  ),
                  const Divider(height: 1),
                  const VersionInfoTile(
                    icon: Icons.cloud_download_outlined,
                    title: 'Latest Version',
                    value: latestVersion,
                  ),
                  const Divider(height: 1),
                  const VersionInfoTile(
                    icon: Icons.check_circle_outline,
                    title: 'Update Status',
                    value: 'Up to date',
                  ),
                  const Divider(height: 1),
                  VersionInfoTile(
                    icon: Icons.phone_android,
                    title: 'Platform',
                    value:
                        Platform.isMacOS ? 'macOS' : Platform.operatingSystem[0].toUpperCase() + Platform.operatingSystem.substring(1),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }
}
