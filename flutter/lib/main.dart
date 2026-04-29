import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize native channel service (for macOS menu bar callbacks)
  if (Platform.isMacOS) {
    NativeChannelService.initialize();
  }

  runApp(
    ProviderScope(
      child: const SoloSoulApp(),
      retry: (retryCount, error) => null,
    ),
  );
}

class SoloSoulApp extends ConsumerStatefulWidget {
  const SoloSoulApp({super.key});

  @override
  ConsumerState<SoloSoulApp> createState() => _SoloSoulAppState();
}

class _SoloSoulAppState extends ConsumerState<SoloSoulApp>
    with WidgetsBindingObserver {
  DateTime? _pausedAt;
  Timer? _autoLockTimer;
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    // Load security settings at startup
    SecurityService.instance.loadSettings();

    // 检测 App 版本变化，若升级则标记待备份
    _checkAppVersion();

    // Create router after initState (needs ref)
    _router = createRouter(ref);

    // Set up native lock callback for macOS menu bar
    if (Platform.isMacOS) {
      NativeChannelService.setLockCallback(() {
        try {
          ref.read(authNotifierProvider.notifier).lockVault();
        } on Exception catch (e) {
          DebugLogger.instance.logError('MAIN', 'Lock callback error: $e');
        }
      });
    }
  }

  Future<void> _checkAppVersion() async {
    try {
      final info = await PackageInfo.fromPlatform();
      await AppVersionTracker.instance.checkVersion(info.version);
    } on Exception catch (e) {
      DebugLogger.instance.logError('MAIN', 'Version check failed: $e');
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

    final delayMinutes =
        SecurityService.instance.settings.autoLockDelayMinutes;
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
    }
    _pausedAt = null;
  }

  void _checkAutoLock() {
    if (_pausedAt == null) return;

    final delayMinutes =
        SecurityService.instance.settings.autoLockDelayMinutes;
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
    // Watch auth state to trigger redirect when it changes (e.g., after lockVault)
    ref.listen<AsyncValue<AuthState>>(authNotifierProvider, (previous, next) {
      final wasUnlocked = previous?.value == AuthState.unlocked;
      final isUnlocked = next.value == AuthState.unlocked;
      if (wasUnlocked && !isUnlocked) {
        // Auth state changed from unlocked to locked - navigate to login
        _router.go(AppRoutes.login);
      }
    });

    // Wrap with ScaffoldMessenger at root level so SnackBars persist across navigation.
    // Without this, each Scaffold has its own ScaffoldMessengerState and when that
    // Scaffold is removed (on navigation), all SnackBar timers are cancelled.
    return ScaffoldMessenger(
      child: MaterialApp.router(
        title: 'SoloSoul',
        debugShowCheckedModeBanner: false,
        theme: AppTheme.lightTheme,
        darkTheme: AppTheme.darkTheme,
        themeMode: ThemeMode.system,
        routerConfig: _router,
      ),
    );
  }
}
