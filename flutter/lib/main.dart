import 'dart:async' show Future, Timer, unawaited;
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:go_router/go_router.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:solosoul_flutter/core/services/app_version_tracker.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/native_channel_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart'
    show AppTheme;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/core/services/clipboard_monitor_service.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/frb/frb_generated.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/language_provider.dart';

void main() {
  // 仅做最基本的 Flutter 绑定，立即显示启动画面
  WidgetsFlutterBinding.ensureInitialized();

  // 配置全局错误捕获（不依赖 Rust）
  FlutterError.onError = (FlutterErrorDetails details) {
    // Suppress known Flutter framework bugs that don't affect functionality.
    final exception = details.exception.toString();
    if (exception.contains('HardwareKeyboard') &&
        exception.contains('KeyDownEvent is dispatched')) {
      DebugLogger.instance.logWarning(
        'FLUTTER_KEYBOARD',
        'Ignored known HardwareKeyboard assertion: $exception',
      );
      return;
    }
    DebugLogger.instance.logError(
      'FLUTTER_ERROR',
      '${details.exception}\n${details.stack}',
    );
    FlutterError.presentError(details);
  };

  ErrorWidget.builder = (details) {
    DebugLogger.instance.logError(
      'ERROR_WIDGET',
      '${details.exception}\n${details.stack}',
    );
    return const Material(
      child: Center(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Text(
            'Something went wrong. Please restart the app.',
            textAlign: TextAlign.center,
            style: TextStyle(color: Colors.grey),
          ),
        ),
      ),
    );
  };

  // 立即显示启动引导页，所有重量级初始化在后台异步执行
  runApp(const AppBootstrap());
}

// ============================================================================
// 启动引导页：先显示动画，后台异步初始化，完成后进入主应用
// ============================================================================

class AppBootstrap extends StatefulWidget {
  const AppBootstrap({super.key});

  @override
  State<AppBootstrap> createState() => _AppBootstrapState();
}

class _AppBootstrapState extends State<AppBootstrap> {
  bool _initialized = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      // 1. Liquid Glass shader 预热
      await LiquidGlassWidgets.initialize();

      // 2. Rust FFI 初始化
      ExternalLibrary? externalLibrary;
      if (Platform.isMacOS) {
        final exeDir = File(Platform.resolvedExecutable).parent.path;
        final dylibPath = '$exeDir/../Frameworks/libsolosoul_core.dylib';
        final absPath = File(dylibPath).absolute.path;
        if (File(absPath).existsSync()) {
          externalLibrary = ExternalLibrary.open(absPath);
        }
      }
      await RustLib.init(externalLibrary: externalLibrary);

      // 3. macOS 原生通道
      if (Platform.isMacOS) {
        NativeChannelService.initialize();
      }

      // 4. OCR 引擎预初始化（后台异步，失败不阻塞启动）
      unawaited(_prewarmOcrEngine());

      if (mounted) {
        setState(() => _initialized = true);
      }
    } on Exception catch (e, stack) {
      DebugLogger.instance.logError('BOOTSTRAP', 'Initialization failed: $e\n$stack');
      if (mounted) {
        setState(() => _errorMessage = e.toString());
      }
    }
  }

  /// 后台预热 OCR 引擎，避免用户首次使用时等待模型加载
  Future<void> _prewarmOcrEngine() async {
    try {
      await OcrService.initialize();
      SoloLog.d('BOOTSTRAP', 'OCR engine prewarmed successfully');
    } on Exception catch (e) {
      // OCR 预热失败不阻塞应用启动，用户首次使用时会重试
      SoloLog.w('BOOTSTRAP', 'OCR prewarm failed (will retry on first use): $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    // 初始化失败
    if (_errorMessage != null) {
      return MaterialApp(
        debugShowCheckedModeBanner: false,
        home: Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(32),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(Icons.error_outline, size: 48, color: Colors.red),
                  const SizedBox(height: 16),
                  const Text(
                    'Launch failed',
                    style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    _errorMessage!,
                    textAlign: TextAlign.center,
                    style: const TextStyle(color: Colors.grey),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
    }

    // 初始化完成 → 进入主应用
    if (_initialized) {
      return ProviderScope(
        child: const SoloSoulApp(),
        retry: (retryCount, error) {
          DebugLogger.instance.logError(
            'PROVIDER_ERROR',
            'Provider error (retry $retryCount): $error',
          );
          return null;
        },
      );
    }

    // 初始化中 → 显示启动画面
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(
        backgroundColor: Colors.white,
        body: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // Logo
              Container(
                width: 80,
                height: 80,
                decoration: BoxDecoration(
                  gradient: const LinearGradient(
                    colors: [AppTheme.primaryColor, AppTheme.secondaryColor],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
                  borderRadius: BorderRadius.circular(20),
                  boxShadow: [
                    BoxShadow(
                      color: AppTheme.primaryColor.withValues(alpha: 0.3),
                      blurRadius: 20,
                      offset: const Offset(0, 8),
                    ),
                  ],
                ),
                child: const Center(
                  child: Text(
                    'S',
                    style: TextStyle(
                      fontSize: 40,
                      fontWeight: FontWeight.bold,
                      color: Colors.white,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              // App name
              const Text(
                'SoloSoul',
                style: TextStyle(
                  fontSize: 28,
                  fontWeight: FontWeight.w600,
                  letterSpacing: -0.5,
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                'Orchestrate your life data, reshape your digital origin',
                style: TextStyle(fontSize: 14, color: Colors.grey),
              ),
              const SizedBox(height: 40),
              // Loading indicator
              const SizedBox(
                width: 32,
                height: 32,
                child: CircularProgressIndicator(strokeWidth: 3),
              ),
            ],
          ),
        ),
      ),
    );
  }
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
  DateTime _lastActivityTime = DateTime.now();
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    // Load security settings and initialize biometric credential service at startup
    unawaited(SecurityService.instance.loadSettings());
    unawaited(BiometricCredentialService.instance.initialize());

    // 检测 App 版本变化，若升级则标记待备份
    _checkAppVersion();

    // Create router after initState (needs ref)
    _router = createRouter(ref);

    // Set up native lock callback for macOS menu bar
    if (Platform.isMacOS) {
      NativeChannelService.setLockCallback(() {
        try {
          unawaited(ref.read(authNotifierProvider.notifier).lockVault());
        } on Exception catch (e) {
          DebugLogger.instance.logError('MAIN', 'Lock callback error: $e');
        }
      });

      // Lock vault before system sleeps to clear sensitive keys from memory
      NativeChannelService.setSleepCallback(() {
        try {
          unawaited(ref.read(authNotifierProvider.notifier).lockVault());
        } on Exception catch (e) {
          DebugLogger.instance.logError('MAIN', 'Sleep callback error: $e');
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
        ClipboardMonitorService.instance.dispose();
        break;
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
    final delay = Duration(minutes: delayMinutes);
    final inactiveSince = DateTime.now().difference(_lastActivityTime);
    final remaining = delay - inactiveSince;

    _autoLockTimer?.cancel();
    if (remaining.isNegative || remaining == Duration.zero) {
      // User has already been inactive longer than the delay — lock now
      _triggerAutoLock();
    } else {
      _autoLockTimer = Timer(remaining, () {
        _triggerAutoLock();
      });
    }
  }

  void _cancelAutoLockTimer() {
    _autoLockTimer?.cancel();
    _autoLockTimer = null;
  }

  void _triggerAutoLock() {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.isUnlocked) {
      _wipeSensitiveState();
      unawaited(authNotifier.lockVault());
    }
    _pausedAt = null;
  }

  void _checkAutoLock() {
    if (_pausedAt == null) return;

    final delayMinutes =
        SecurityService.instance.settings.autoLockDelayMinutes;
    if (delayMinutes == -1) return; // Never auto-lock

    final inactiveDuration = DateTime.now().difference(_lastActivityTime);
    if (inactiveDuration.inMinutes >= delayMinutes) {
      _triggerAutoLock();
    }
    _pausedAt = null;
  }

  /// Called on user interaction to reset the inactivity timer.
  void _recordActivity() {
    _lastActivityTime = DateTime.now();
  }

  void _wipeSensitiveState() {
    if (mounted) {
      ref.read(profileNotifierProvider.notifier).clearProfile();
      ref.read(fieldHistoriesProvider.notifier).clearHistories();
      ref.read(unifiedObjectProvider.notifier).reset();
    }
  }

  @override
  Widget build(BuildContext context) {
    // Watch auth state to trigger redirect when it changes (e.g., after lockVault)
    ref.listen<AsyncValue<AuthState>>(authNotifierProvider, (previous, next) {
      final wasLocked = previous?.value == AuthState.locked;
      final isLocked = next.value == AuthState.locked;
      if (!wasLocked && isLocked) {
        // Auth state changed to locked - wipe all sensitive state and navigate to login
        _wipeSensitiveState();
        _router.go(AppRoutes.login);
      }
    });

    // Wrap with ScaffoldMessenger at root level so SnackBars persist across navigation.
    // Without this, each Scaffold has its own ScaffoldMessengerState and when that
    // Scaffold is removed (on navigation), all SnackBar timers are cancelled.
    //
    // Liquid Glass wrapping order:
    // 1. LiquidGlassWidgets.wrap() — installs GlassBackdropScope + optional GlassAdaptiveScope
    // 2. GlassTheme — provides centralized glass theme configuration
    // 3. MaterialApp.router — the app itself
    return Listener(
      onPointerDown: (_) => _recordActivity(),
      onPointerMove: (_) => _recordActivity(),
      child: ScaffoldMessenger(
        child: LiquidGlassWidgets.wrap(
          adaptiveQuality: true,
          child: GlassTheme(
            data: AppTheme.glassThemeData,
            child: MaterialApp.router(
              title: 'SoloSoul',
              debugShowCheckedModeBanner: false,
              theme: AppTheme.lightTheme,
              darkTheme: AppTheme.darkTheme,
              themeMode: ThemeMode.system,
              routerConfig: _router,
              localizationsDelegates: const [
                AppLocalizations.delegate,
                GlobalMaterialLocalizations.delegate,
                GlobalWidgetsLocalizations.delegate,
                GlobalCupertinoLocalizations.delegate,
              ],
              supportedLocales: const [
                Locale('en'),
                Locale('zh'),
              ],
              locale: ref.watch(languageProvider).value,
            ),
          ),
        ),
      ),
    );
  }
}
