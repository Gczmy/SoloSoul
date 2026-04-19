import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
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
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/privacy_blur_overlay.dart';

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

class SoloSoulApp extends ConsumerStatefulWidget {
  const SoloSoulApp({super.key});

  @override
  ConsumerState<SoloSoulApp> createState() => _SoloSoulAppState();
}

class _SoloSoulAppState extends ConsumerState<SoloSoulApp> with WidgetsBindingObserver {
  DateTime? _pausedAt;
  Timer? _autoLockTimer;
  bool _isInBackground = false;
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
        ref.read(authNotifierProvider.notifier).lockVault();
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
        _isInBackground = true;
        _startAutoLockTimer();
        break;
      case AppLifecycleState.resumed:
        _isInBackground = false;
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
      // Defer navigation using microtask to avoid frame conflict during widget rebuild/dispose
      Future.microtask(() {
        final navContext = _navigatorKey.currentContext;
        if (navContext != null) {
          ScaffoldMessenger.of(navContext).showSnackBar(
            const SnackBar(
              content: Text('Vault auto-locked after leaving the app'),
              behavior: SnackBarBehavior.floating,
            ),
          );
          _navigatorKey.currentState?.pushNamedAndRemoveUntil(
            '/login',
            (route) => false,
          );
        }
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
        builder: (context, child) {
          return Stack(
            children: [
              child ?? const SizedBox(),
              // PrivacyBlurOverlay - shown when app is inactive/paused AND vault is unlocked
              Consumer(
                builder: (context, ref, _) {
                  final authState = ref.watch(authNotifierProvider);
                  final showBlur = _isInBackground && authState == AuthState.unlocked;
                  return PrivacyBlurOverlay(visible: showBlur);
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
          '/security_settings': (context) => const SecuritySettingsPage(),
          '/operation_log': (context) => const OperationLogPage(),
          '/sensitivity_settings': (context) => const SensitivitySettingsPage(),
          '/trash': (context) => const TrashPage(),
          '/search': (context) => const SearchPage(),
        },
      ),
    );
  }
}
