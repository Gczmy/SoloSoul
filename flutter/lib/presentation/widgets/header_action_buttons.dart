import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' show AppTheme, SnackBarType, showOverlaySnackBar;

/// Non-permanent lock sensitivity button shown only when sensitive access is granted.
class HeaderActionButtons extends ConsumerWidget {
  const HeaderActionButtons({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    // Only show when sensitive access is currently granted
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      return const SizedBox.shrink();
    }

    return IconButton(
      icon: const Icon(
        Icons.lock_open_outlined,
        color: AppTheme.successColor,
      ),
      onPressed: () {
        ref.read(sensitivePageAccessProvider.notifier).clear();
        showOverlaySnackBar(
          context,
          content: l10n.headerSensitiveAccessLocked,
          type: SnackBarType.info,
        );
      },
      tooltip: l10n.headerLockSensitivity,
    );
  }
}
