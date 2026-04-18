import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/pages/splash_page.dart';
import 'package:solosoul_flutter/presentation/pages/login_page.dart';
import 'package:solosoul_flutter/presentation/pages/home_page.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';
import 'package:solosoul_flutter/presentation/pages/financial_page.dart';
import 'package:solosoul_flutter/presentation/pages/professional_page.dart';
import 'package:solosoul_flutter/presentation/pages/settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/pages/sensitivity_settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/trash_page.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/core/utils/global_error_handler.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/presentation/widgets/lock_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize native channel service (for macOS menu bar callbacks)
  if (Platform.isMacOS) {
    NativeChannelService.initialize();
  }

  // Initialize Rust vault with app support directory
  // This must happen BEFORE any vault operations
  if (!Platform.isAndroid) {
    final appSupport = await getApplicationSupportDirectory();
    RustVaultService.instance.initAccountManager(appSupport.path);
  }

  // Initialize debug logger for troubleshooting account issues
  await DebugLogger.instance.init();

  runApp(
    const ProviderScope(
      child: SoloSoulApp(),
    ),
  );
}

class SoloSoulApp extends ConsumerStatefulWidget {
  const SoloSoulApp({super.key});

  @override
  ConsumerState<SoloSoulApp> createState() => _SoloSoulAppState();
}

class _SoloSoulAppState extends ConsumerState<SoloSoulApp> with WidgetsBindingObserver {
  DateTime? _pausedAt;
  static const _autoLockDuration = Duration(minutes: 5);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    // Set up native lock callback for macOS menu bar
    if (Platform.isMacOS) {
      NativeChannelService.setLockCallback(() {
        ref.read(authNotifierProvider.notifier).lockVault();
      });
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    super.didChangeAppLifecycleState(state);

    switch (state) {
      case AppLifecycleState.paused:
      case AppLifecycleState.inactive:
        _pausedAt = DateTime.now();
        break;
      case AppLifecycleState.resumed:
        _checkAutoLock();
        break;
      case AppLifecycleState.detached:
      case AppLifecycleState.hidden:
        break;
    }
  }

  void _checkAutoLock() {
    if (_pausedAt == null) return;

    final elapsed = DateTime.now().difference(_pausedAt!);
    if (elapsed >= _autoLockDuration) {
      final authNotifier = ref.read(authNotifierProvider.notifier);
      if (authNotifier.isUnlocked) {
        // Wipe sensitive state before locking
        _wipeSensitiveState();
        authNotifier.lockVault();
        // Lock overlay will automatically appear via builder pattern
        if (mounted) {
          GlobalErrorHandler.showSnackBar(
            context,
            'Vault auto-locked after ${_autoLockDuration.inMinutes} minutes of inactivity',
            isWarning: true,
          );
        }
      }
    }
    _pausedAt = null;
  }

  void _wipeSensitiveState() {
    // Clear any cached profile data from providers
    // This ensures no sensitive data leaks through back gesture or app switcher
    if (mounted) {
      // Clear profile state to prevent sensitive data leaks
      ref.read(profileNotifierProvider.notifier).clearProfile();
    }
  }

  @override
  Widget build(BuildContext context) {
    // Wrap with ScaffoldMessenger at root level so SnackBars persist across navigation.
    // Without this, each Scaffold has its own ScaffoldMessengerState and when that
    // Scaffold is removed (on navigation), all SnackBar timers are cancelled.
    return ScaffoldMessenger(
      child: MaterialApp(
        title: 'SoloSoul',
        debugShowCheckedModeBanner: false,
        theme: AppTheme.lightTheme,
        darkTheme: AppTheme.darkTheme,
        themeMode: ThemeMode.system,
        home: const SplashPage(),
        builder: (context, child) {
          return Stack(
            children: [
              child ?? const SizedBox(),
              // LockScreen overlay - shown when vault is locked
              Consumer(
                builder: (context, ref, _) {
                  final authState = ref.watch(authNotifierProvider);
                  return AnimatedOpacity(
                    opacity: authState == AuthState.locked ? 1.0 : 0.0,
                    duration: const Duration(milliseconds: 300),
                    child: authState == AuthState.locked
                        ? const LockScreen()
                        : const SizedBox.shrink(),
                  );
                },
              ),
            ],
          );
        },
        routes: {
          '/login': (context) => const LoginPage(),
          '/home': (context) => const HomePage(),
          '/profile': (context) => const ProfilePage(),
          '/travel': (context) => const TravelPage(),
          '/financial': (context) => const FinancialPage(),
          '/professional': (context) => const ProfessionalPage(),
          '/settings': (context) => const SettingsPage(),
          '/operation_log': (context) => const OperationLogPage(),
          '/sensitivity_settings': (context) => const SensitivitySettingsPage(),
          '/trash': (context) => const TrashPage(),
        },
      ),
    );
  }
}
