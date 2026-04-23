import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/presentation/pages/splash_page.dart';
import 'package:solosoul_flutter/presentation/pages/login_page.dart';
import 'package:solosoul_flutter/presentation/pages/home_page.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';
import 'package:solosoul_flutter/presentation/pages/financial_page.dart';
import 'package:solosoul_flutter/presentation/pages/professional_page.dart';
import 'package:solosoul_flutter/presentation/pages/settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/security_settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/pages/sensitivity_settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/trash_page.dart';
import 'package:solosoul_flutter/presentation/pages/search_page.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme, showOverlaySnackBar, SnackBarType;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize native channel service (for macOS menu bar callbacks)
  if (Platform.isMacOS) {
    NativeChannelService.initialize();
  }

  runApp(
    const ProviderScope(
      child: SoloSoulApp(),
    ),
  );
}

/// Centralized route constants to avoid hardcoded strings across the codebase
class AppRoutes {
  AppRoutes._();

  static const String login = '/login';
  static const String home = '/home';
  static const String profile = '/profile';
  static const String travel = '/travel';
  static const String financial = '/financial';
  static const String professional = '/professional';
  static const String settings = '/settings';
  static const String securitySettings = '/security_settings';
  static const String operationLog = '/operation_log';
  static const String sensitivitySettings = '/sensitivity_settings';
  static const String trash = '/trash';
  static const String search = '/search';
}

class SoloSoulApp extends ConsumerStatefulWidget {
  const SoloSoulApp({super.key});

  @override
  ConsumerState<SoloSoulApp> createState() => _SoloSoulAppState();
}

class _SoloSoulAppState extends ConsumerState<SoloSoulApp> with WidgetsBindingObserver {
  DateTime? _pausedAt;
  Timer? _autoLockTimer;
  final GlobalKey<NavigatorState> _navigatorKey = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    // Load security settings at startup
    SecurityService.instance.loadSettings();

    // Set up native lock callback for macOS menu bar
    if (Platform.isMacOS) {
      NativeChannelService.setLockCallback(() {
        try {
          ref.read(authNotifierProvider.notifier).lockVault();
        } catch (e) {
          DebugLogger.instance.logError('MAIN', 'Lock callback error: $e');
        }
      });
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _autoLockTimer?.cancel();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    super.didChangeAppLifecycleState(state);

    switch (state) {
      case AppLifecycleState.paused:
      case AppLifecycleState.inactive:
        _startAutoLockTimer();
        break;
      case AppLifecycleState.resumed:
        _cancelAutoLockTimer();
        _checkAutoLock();
        break;
      case AppLifecycleState.detached:
      case AppLifecycleState.hidden:
        break;
    }
  }

  void _startAutoLockTimer() {
    if (!SecurityService.instance.settings.lockOnWindowBlur) return;

    final delayMinutes = SecurityService.instance.settings.autoLockDelayMinutes;
    if (delayMinutes == -1) return; // Never auto-lock

    _pausedAt = DateTime.now();
    final duration = Duration(minutes: delayMinutes);

    _autoLockTimer?.cancel();
    _autoLockTimer = Timer(duration, () {
      _triggerAutoLock();
    });
  }

  void _cancelAutoLockTimer() {
    _autoLockTimer?.cancel();
    _autoLockTimer = null;
  }

  void _triggerAutoLock() {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.isUnlocked) {
      _wipeSensitiveState();
      authNotifier.lockVault();
      // Use addPostFrameCallback so the overlay exists (runs after first frame builds MaterialApp)
      WidgetsBinding.instance.addPostFrameCallback((_) {
        try {
          final navContext = _navigatorKey.currentContext;
          if (navContext != null) {
            showOverlaySnackBar(
              navContext,
              content: 'Vault auto-locked after leaving the app',
              type: SnackBarType.info,
            );
          }
        } catch (_) {
          // SnackBar failed (no overlay), continue with navigation
        }
        _navigatorKey.currentState?.pushNamedAndRemoveUntil(
          AppRoutes.login,
          (route) => false,
        );
      });
    }
    _pausedAt = null;
  }

  void _checkAutoLock() {
    if (_pausedAt == null) return;

    final delayMinutes = SecurityService.instance.settings.autoLockDelayMinutes;
    if (delayMinutes == -1) return; // Never auto-lock

    final elapsedMinutes = DateTime.now().difference(_pausedAt!).inMinutes;

    if (elapsedMinutes >= delayMinutes) {
      _triggerAutoLock();
    }
    _pausedAt = null;
  }

  void _wipeSensitiveState() {
    if (mounted) {
      ref.read(profileNotifierProvider.notifier).clearProfile();
      ref.read(fieldHistoriesProvider.notifier).clearHistories();
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
        navigatorKey: _navigatorKey,
        home: const SplashPage(),
        routes: {
          AppRoutes.login: (context) => const LoginPage(),
          AppRoutes.home: (context) => const HomePage(),
          AppRoutes.profile: (context) => const ProfilePage(),
          AppRoutes.travel: (context) => const TravelPage(),
          AppRoutes.financial: (context) => const FinancialPage(),
          AppRoutes.professional: (context) => const ProfessionalPage(),
          AppRoutes.settings: (context) => const SettingsPage(),
          AppRoutes.securitySettings: (context) => const SecuritySettingsPage(),
          AppRoutes.operationLog: (context) => const OperationLogPage(),
          AppRoutes.sensitivitySettings: (context) => const SensitivitySettingsPage(),
          AppRoutes.trash: (context) => const TrashPage(),
          AppRoutes.search: (context) => const SearchPage(),
        },
      ),
    );
  }
}
