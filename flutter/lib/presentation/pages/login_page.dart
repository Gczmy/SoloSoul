import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:go_router/go_router.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show formFieldRegistryProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/fallback_secure_storage.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/utils/device_utils.dart' show getDeviceName;
import 'package:solosoul_flutter/presentation/widgets/login/account_list_section.dart';
import 'package:solosoul_flutter/presentation/widgets/login/create_account_form.dart';
import 'package:solosoul_flutter/presentation/widgets/login/login_background.dart';
import 'package:solosoul_flutter/presentation/widgets/login/login_header.dart';
import 'package:solosoul_flutter/presentation/widgets/login/password_input_section.dart';

class LoginPage extends ConsumerStatefulWidget {
  const LoginPage({super.key});

  @override
  ConsumerState<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends ConsumerState<LoginPage> {
  final _formKey = GlobalKey<FormState>();
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;
  bool _isLoading = false;
  bool _showCreateAccount = false;
  bool _accountsExpanded = false;

  /// Write debug log to file for Release build diagnostics
  void _debugLog(String msg) {
    try {
      final f = File('/tmp/solosoul_dart.log');
      final now = DateTime.now().toIso8601String();
      f.writeAsStringSync('[$now] $msg\n', mode: FileMode.append);
    } on Exception {
      // ignore
    }
  }

  // Password verification error state
  bool _hasPasswordError = false;
  String? _passwordErrorMessage;

  // Password field focus state
  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  // Biometric unlock state
  bool _biometricsEnabled = false;
  // Initialized from l10n in build()
  String _biometricType = '';

  // Create account form fields
  final _newAccountNameController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();
  final _passwordHintController = TextEditingController();

  static const _restoreHandledKeyPrefix = 'restore_prompt_handled_v1';

  String _restoreHandledKey(String accountId) => '${_restoreHandledKeyPrefix}_$accountId';

  /// 若 Vault 数据为空但存在备份，提示用户恢复。
  /// 用户跳过或恢复后记录标志，避免每次登录重复提示。
  Future<void> _promptRestoreIfEmpty(
    BuildContext context,
    WidgetRef ref,
    String? accountId,
  ) async {
    if (accountId == null) return;
    if (!context.mounted) return;

    // 检查是否已处理过恢复提示（跳过或恢复）
    final storage = FallbackSecureStorage();
    String? handled;
    try {
      handled = await storage.read(key: _restoreHandledKey(accountId));
    } on Exception catch (e) {
      SoloLog.w('LOGIN', 'Failed to read restore handled flag: $e');
      handled = null;
    }
    // 如果之前标记为 restored 但数据仍然为空（只有默认结构），重新提示恢复
    final profile = ref.read(profileNotifierProvider).value;
    final unifiedData = ref.read(unifiedObjectProvider);
    final activeCount = unifiedData.objects.where((o) => !o.isDeleted).length;
    final hasRealData = profile?.unifiedObjects != null &&
        activeCount > 0 &&
        unifiedData.objects.any((o) => o.typeId != 'page' && o.typeId != 'collection');

    if (handled == 'skipped') {
      SoloLog.d('LOGIN', 'Restore prompt already skipped, skipping');
      return;
    }
    if (handled == 'restored' && hasRealData) {
      SoloLog.d('LOGIN', 'Restore prompt already restored and data exists, skipping');
      return;
    }
    if (handled == 'restored' && !hasRealData) {
      SoloLog.w('LOGIN', 'Previously restored but data still empty — re-prompting restore');
    }

    final objectCount = unifiedData.objects.length;

    SoloLog.d(
      'LOGIN',
      '_promptRestoreIfEmpty: profile=${profile != null}, '
      'unifiedObjects=${profile?.unifiedObjects != null}, '
      'objects=$objectCount, active=$activeCount',
    );

    if (hasRealData) {
      return; // 有有效用户数据，无需恢复
    }

    SoloLog.w(
      'LOGIN',
      'Data appears empty or corrupted (hasRealData=$hasRealData), checking backups',
    );

    final backups = await BackupService.instance.listAllBackups(accountId);
    if (backups.isEmpty) return; // 无备份，无需恢复

    if (!context.mounted) return;

    final l10n = AppLocalizations.of(context);

    // Show backup list selection dialog using standard AlertDialog
    BackupEntry? selectedBackup;
    final dialogResult = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.loginDataRecoveryTitle),
        content: SizedBox(
          width: double.maxFinite,
          height: 320,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('检测到 ${backups.length} 个可用备份，请选择一个恢复：'),
              const SizedBox(height: 12),
              Expanded(
                child: ListView.builder(
                  shrinkWrap: true,
                  itemCount: backups.length,
                  itemBuilder: (listCtx, index) {
                    final b = backups[index];
                    final label = b.isSpecial ? '特殊备份' : '自动备份';
                    return ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Row(
                        children: [
                          Expanded(
                            child: Text(
                              b.displayName,
                              style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 6),
                          Chip(
                            label: Text(label, style: const TextStyle(fontSize: 10)),
                            padding: EdgeInsets.zero,
                            materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                            backgroundColor: b.isSpecial ? const Color(0xFFE8F5E9) : const Color(0xFFE3F2FD),
                            labelStyle: TextStyle(
                              fontSize: 10,
                              color: b.isSpecial ? const Color(0xFF2E7D32) : const Color(0xFF1565C0),
                            ),
                          ),
                        ],
                      ),
                      subtitle: Text(
                        '${b.displayTime}  ·  ${b.displaySize}',
                        style: const TextStyle(fontSize: 11, color: Colors.grey),
                      ),
                      onTap: () {
                        selectedBackup = b;
                        Navigator.of(listCtx).pop(true);
                      },
                    );
                  },
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(l10n.loginSkip),
          ),
        ],
      ),
    );

    if (dialogResult == true && selectedBackup != null) {
      final b = selectedBackup!;
      SoloLog.d('LOGIN', 'Restoring backup: ${b.fileName} (special=${b.isSpecial})');
      final success = b.isSpecial
          ? await BackupService.instance.restoreSpecialBackup(accountId, b.fileName)
          : await BackupService.instance.restoreBackup(accountId, b.fileName);
      if (success && context.mounted) {
        await ref.read(profileNotifierProvider.notifier).loadProfile();
        await ref.read(unifiedObjectProvider.notifier).loadFromProfile();
        await storage.write(key: _restoreHandledKey(accountId), value: 'restored');
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(l10n.loginRestoreSuccess),
              duration: AppTheme.kPasswordHintDelay,
            ),
          );
        }
      } else if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l10n.loginRestoreFailed)),
        );
      }
    } else if (dialogResult == false) {
      await storage.write(key: _restoreHandledKey(accountId), value: 'skipped');
    }
  }

  bool _obscureNewPassword = true;
  bool _obscureConfirmPassword = true;
  String? _createError;

  // Create account form focus states
  final _newPasswordFocusNode = FocusNode();
  bool _isNewPasswordFocused = false;
  final _confirmPasswordFocusNode = FocusNode();
  bool _isConfirmPasswordFocused = false;

  // Password hint overlay tracking
  OverlayEntry? _passwordHintOverlayEntry;
  Timer? _passwordHintTimer;

  @override
  void initState() {
    super.initState();
    _checkBiometrics();
    _passwordController.addListener(_onPasswordTyping);
    _passwordFocusNode.addListener(_onPasswordFocusChange);
    _newPasswordFocusNode.addListener(_onNewPasswordFocusChange);
    _confirmPasswordFocusNode.addListener(_onConfirmPasswordFocusChange);
  }

  void _onNewPasswordFocusChange() {
    final hasFocus = _newPasswordFocusNode.hasFocus;
    if (hasFocus != _isNewPasswordFocused) {
      setState(() => _isNewPasswordFocused = hasFocus);
    }
  }

  void _onConfirmPasswordFocusChange() {
    final hasFocus = _confirmPasswordFocusNode.hasFocus;
    if (hasFocus != _isConfirmPasswordFocused) {
      setState(() => _isConfirmPasswordFocused = hasFocus);
    }
  }

  void _onPasswordFocusChange() {
    final hasFocus = _passwordFocusNode.hasFocus;
    if (hasFocus != _isPasswordFocused) {
      setState(() => _isPasswordFocused = hasFocus);
    }
  }

  void _onPasswordTyping() {
    // Only clear error when field is completely empty
    if (_passwordController.text.isEmpty) {
      if (_hasPasswordError) {
        setState(() {
          _hasPasswordError = false;
          _passwordErrorMessage = null;
        });
      }
    }
  }

  Future<void> _checkBiometrics() async {
    try {
      // Ensure security settings and biometric credential service are ready
      await SecurityService.instance.loadSettings();
      await BiometricCredentialService.instance.initialize();

      final biometric = BiometricService.instance;
      final available = await biometric.isAvailable();
      final settings = SecurityService.instance.settings;
      final availableBiometrics = await biometric.getAvailableBiometrics();

      final authNotifier = ref.read(authNotifierProvider.notifier);
      final accountId = authNotifier.selectedAccountId;
      final hasCredential = accountId != null &&
          await BiometricCredentialService.instance.hasBiometricCredential(accountId) &&
          await BiometricCredentialService.instance.isDeviceKeyAvailable();

      if (!mounted) return;
      final l10n = AppLocalizations.of(context);
      String biometricType = l10n.loginBiometricGeneric;
      if (availableBiometrics.isNotEmpty) {
        if (availableBiometrics.any((b) => b == BiometricType.face)) {
          biometricType = l10n.loginBiometricFaceId;
        } else if (availableBiometrics.any(
          (b) => b == BiometricType.fingerprint,
        )) {
          biometricType = l10n.loginBiometricTouchId;
        } else if (availableBiometrics.any((b) => b == BiometricType.iris)) {
          biometricType = l10n.loginBiometricIris;
        }
      }

      setState(() {
        _biometricsEnabled = (settings.biometricsEnabled || settings.faceIdEnabled) && available && hasCredential;
        _biometricType = biometricType;
      });
    } on Exception catch (e, st) {
      SoloLog.w('LOGIN', 'Biometric check failed: $e');
      if (mounted) {
        setState(() => _biometricsEnabled = false);
      }
    }
  }

  Future<void> _handleBiometricUnlock() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.selectedAccountId == null) return;

    setState(() => _isLoading = true);

    try {
      final success = await BiometricService.instance.authenticate(
        reason: AppLocalizations.of(context).loginUnlockReason(_biometricType),
      );

      if (!success) {
        if (mounted) {
          setState(() => _isLoading = false);
          showOverlaySnackBar(
            context,
            content: AppLocalizations.of(context).loginBiometricFailed,
            type: SnackBarType.error,
          );
        }
        return;
      }

      if (!mounted) return;

      // Unlock vault with biometric credential (session key)
      final unlockSuccess = await authNotifier.unlockVaultWithBiometric();
      if (!unlockSuccess) {
        if (mounted) {
          setState(() => _isLoading = false);
          showOverlaySnackBar(
            context,
            content: AppLocalizations.of(context).loginUnlockFailedUsePassword,
            type: SnackBarType.error,
          );
        }
        return;
      }

      if (!mounted) return;

      await ref.read(profileNotifierProvider.notifier).loadProfile();
      await ref.read(unifiedObjectProvider.notifier).loadFromProfile();
      // Pre-register all form fields for sensitivity settings
      ref.read(formFieldRegistryProvider.notifier).registerAllForms();

      final accountId = authNotifier.selectedAccountId;
      if (accountId != null) {
        await authNotifier.updateAccountMetadata(
          lastLoginAt: DateTime.now(),
          device: DeviceInfo(
            deviceName: getDeviceName(),
            lastUsed: DateTime.now(),
          ).toJson(),
        );
      }

      if (mounted) {
        context.go(AppRoutes.home);
      }
    } on Exception catch (e) {
      if (mounted) {
        showOverlaySnackBar(
          context,
          content: '${AppLocalizations.of(context).commonError}: $e',
          type: SnackBarType.error,
        );
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  @override
  void dispose() {
    _passwordHintTimer?.cancel();
    _passwordHintOverlayEntry?.remove();
    _passwordHintOverlayEntry = null;
    _passwordController.removeListener(_onPasswordTyping);
    _passwordController.dispose();
    _passwordFocusNode.removeListener(_onPasswordFocusChange);
    _passwordFocusNode.dispose();
    _newAccountNameController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    _passwordHintController.dispose();
    _newPasswordFocusNode.removeListener(_onNewPasswordFocusChange);
    _newPasswordFocusNode.dispose();
    _confirmPasswordFocusNode.removeListener(_onConfirmPasswordFocusChange);
    _confirmPasswordFocusNode.dispose();
    super.dispose();
  }

  String _formatLastAccessed(DateTime? lastAccessed, AppLocalizations l10n) {
    if (lastAccessed == null) return l10n.loginNever;
    final now = DateTime.now();
    final diff = now.difference(lastAccessed);
    if (diff.inDays == 0) return l10n.loginToday;
    if (diff.inDays == 1) return l10n.loginYesterday;
    if (diff.inDays < 7) return l10n.loginDaysAgo(diff.inDays);
    return '${lastAccessed.day}/${lastAccessed.month}/${lastAccessed.year}';
  }

  /// Shared post-login setup: load profile, unified objects, register forms,
  /// record metadata, and navigate to home.
  Future<void> _postLoginSetup() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    _debugLog('_postLoginSetup start');

    // Pre-load profile before navigating to home
    _debugLog('calling loadProfile...');
    try {
      await ref.read(profileNotifierProvider.notifier).loadProfile().timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          _debugLog('loadProfile timed out');
        },
      );
      _debugLog('loadProfile done');
    } on Exception catch (e, st) {
      _debugLog('loadProfile exception: $e');
    }

    _debugLog('calling loadFromProfile...');
    try {
      await ref.read(unifiedObjectProvider.notifier).loadFromProfile();
      _debugLog('loadFromProfile done');
    } on Exception catch (e, st) {
      _debugLog('loadFromProfile exception: $e');
    }

    final unifiedData = ref.read(unifiedObjectProvider);
    _debugLog('unifiedData objects=${unifiedData.objects.length}');

    // 首次启动/空数据检测：若 Vault 无数据但存在备份，提示恢复
    final accountId = authNotifier.selectedAccountId;
    _debugLog('accountId=$accountId, mounted=$mounted');
    if (mounted) {
      _debugLog('calling _promptRestoreIfEmpty...');
      try {
        await _promptRestoreIfEmpty(context, ref, accountId);
        _debugLog('_promptRestoreIfEmpty done');
      } on Exception catch (e, st) {
        _debugLog('_promptRestoreIfEmpty exception: $e');
      }
    }

    _debugLog('registering forms...');
    ref.read(formFieldRegistryProvider.notifier).registerAllForms();

    // Record login metadata (lastLoginAt + device)
    if (accountId != null) {
      _debugLog('updating account metadata...');
      await authNotifier.updateAccountMetadata(
        lastLoginAt: DateTime.now(),
        device: DeviceInfo(
          deviceName: getDeviceName(),
          lastUsed: DateTime.now(),
        ).toJson(),
      ).timeout(const Duration(seconds: 5), onTimeout: () {
        _debugLog('updateAccountMetadata timed out');
      });
    }

    _debugLog('navigating to home...');
    if (mounted) {
      context.go(AppRoutes.home);
    }
    _debugLog('_postLoginSetup end');
  }

  Future<void> _handleUnlock() async {
    final formState = _formKey.currentState;
    if (formState == null) return;
    if (!formState.validate()) {
      // Form validator showed the error - sync our flag so label/icon turn red too
      setState(() {
        _hasPasswordError = true;
        _passwordErrorMessage = AppLocalizations.of(context).loginPasswordMinLength;
      });
      return;
    }

    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.selectedAccountId == null) return;

    setState(() => _isLoading = true);

    SoloLog.d('LOGIN', 'Starting unlockVault for account: ${authNotifier.selectedAccountId}');
    bool success;
    try {
      success = await authNotifier.unlockVault(_passwordController.text);
      SoloLog.d('LOGIN', 'unlockVault returned: $success');
    } on Exception catch (e, st) {
      SoloLog.e('LOGIN', 'unlockVault threw exception', e, st);
      success = false;
    }
    SoloLog.d('LOGIN', 'unlockVault completed, success: $success');

    if (!success || !mounted) {
      final specificError = ref.read(authNotifierProvider.notifier).lastUnlockError;
      final isPasswordError = specificError == null ||
          specificError.toLowerCase().contains('invalid password') ||
          specificError.toLowerCase().contains('invalid master password');
      SoloLog.w('LOGIN', 'Unlock failed: specificError=$specificError, isPasswordError=$isPasswordError');
      setState(() {
        _isLoading = false;
        _hasPasswordError = isPasswordError;
        _passwordErrorMessage = isPasswordError
            ? AppLocalizations.of(context).loginInvalidPassword
            : AppLocalizations.of(context).loginUnlockFailed(specificError);
      });
      return;
    }

    try {
      await _postLoginSetup();
    } on Exception catch (e, st) {
      SoloLog.e('LOGIN', '_postLoginSetup failed', e, st);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('登录后初始化失败: $e')),
        );
      }
    } finally {
      // Always reset loading state so UI doesn't get stuck
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _handleCreateAccount() async {
    SoloLog.d('LOGIN', '_handleCreateAccount start');

    final name = _newAccountNameController.text.trim();
    final password = _newPasswordController.text;
    final confirm = _confirmPasswordController.text;

    // Validation
    if (name.isEmpty) {
      setState(() => _createError = AppLocalizations.of(context).loginAccountNameRequired);
      return;
    }
    if (password.length < 8) {
      setState(() => _createError = AppLocalizations.of(context).loginPasswordMinLength);
      return;
    }
    if (password != confirm) {
      setState(() => _createError = AppLocalizations.of(context).loginPasswordsDoNotMatch);
      return;
    }

    setState(() {
      _createError = null;
      _isLoading = true;
    });

    SoloLog.d('LOGIN', 'calling authNotifier.createAccount');

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final passwordHint = _passwordHintController.text.trim();
    final result = await authNotifier.createAccount(
      name,
      password,
      passwordHint: passwordHint.isEmpty ? null : passwordHint,
    );

    SoloLog.d('LOGIN', 'createAccount returned, success=${result.success}');

    if (!result.success || !mounted) {
      if (mounted) {
        setState(() {
          _createError = result.error ?? AppLocalizations.of(context).loginCreateAccountFailed;
          _isLoading = false;
        });
      }
      return;
    }

    // Account created, now unlock
    SoloLog.d('LOGIN', 'calling authNotifier.unlockVault');
    final unlockSuccess = await authNotifier.unlockVault(password);
    SoloLog.d('LOGIN', 'unlockVault returned, success=$unlockSuccess');

    if (!unlockSuccess || !mounted) {
      if (mounted) {
        setState(() {
          _createError = AppLocalizations.of(context).loginUnlockVaultFailed;
          _isLoading = false;
        });
      }
      return;
    }

    await _postLoginSetup();
  }

  Future<void> _selectAccount(String accountId) async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    await authNotifier.selectAccount(accountId);
    // Rebuild to show password input for the selected account
    // (build() uses ref.read for authNotifier, not ref.watch)
    if (mounted) setState(() {});
    // Recheck biometric availability for the newly selected account
    await _checkBiometrics();
  }

  Future<void> _backToAccountList() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    await authNotifier.selectAccount(null);
    _passwordController.clear();
    setState(() {
      _accountsExpanded = false;
    });
  }

  void _backToAccountListFromCreate() {
    setState(() {
      _showCreateAccount = false;
      _createError = null;
      _newAccountNameController.clear();
      _newPasswordController.clear();
      _confirmPasswordController.clear();
    });
  }

  void _showPasswordHint(String hint) {
    // Guard against stale context
    if (!mounted) return;

    // Dismiss any existing hint overlay before showing a new one
    _passwordHintTimer?.cancel();
    _passwordHintOverlayEntry?.remove();

    // Use Overlay instead of ScaffoldMessenger.showSnackBar so the timer persists
    // across navigation. SnackBar's built-in timer is cancelled when the widget
    // tree is unmounted (e.g. pushReplacementNamed to home).
    final overlay = Overlay.of(context);

    _passwordHintOverlayEntry = OverlayEntry(
      builder: (ctx) => _PasswordHintOverlay(
        hint: hint,
        onDismiss: () {
          _passwordHintOverlayEntry?.remove();
          _passwordHintOverlayEntry = null;
        },
      ),
    );

    final entry = _passwordHintOverlayEntry;
    if (entry != null) overlay.insert(entry);
    // Use explicit Timer so it persists across navigation (not tied to widget lifecycle)
    _passwordHintTimer = Timer(AppTheme.kPasswordHintDelay, () {
      _passwordHintOverlayEntry?.remove();
      _passwordHintOverlayEntry = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final bottomPadding = MediaQuery.of(context).viewInsets.bottom;
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;
    // Use ref.read to avoid rebuilds when auth state changes during setState
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccountId = authNotifier.selectedAccountId;

    // Use ref.watch to reactively rebuild when accounts change
    final accountsAsync = ref.watch(accountsProvider);

    return Scaffold(
      backgroundColor: Colors.transparent,
      resizeToAvoidBottomInset: true,
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: isDark
                ? [
                    const Color(0xFF0B1220),
                    const Color(0xFF131D2E),
                    const Color(0xFF0B1220),
                  ]
                : [
                    const Color(0xFFF8FAFD),
                    const Color(0xFFEDF2F9),
                    const Color(0xFFF8FAFD),
                  ],
          ),
        ),
        child: Stack(
          children: [
            // Decorative background orbs
            const LoginBackground(),
            // Main content
            SafeArea(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  return SingleChildScrollView(
                    physics: const AlwaysScrollableScrollPhysics(),
                    child: ConstrainedBox(
                      constraints: BoxConstraints(
                        minHeight: constraints.maxHeight,
                      ),
                      child: Padding(
                        padding: EdgeInsets.only(
                          left: 24,
                          right: 24,
                          top: 24,
                          bottom: bottomPadding + 24,
                        ),
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            // Header with logo, title, and subtitle
                            const LoginHeader(),

                            const SizedBox(height: 32),

                            // Content based on state — wrapped in GlassCard for depth
                            GlassCard(
                              useOwnLayer: true,
                              padding: const EdgeInsets.all(24),
                              settings: isDark
                                  ? const LiquidGlassSettings(
                                      thickness: 30,
                                      blur: 12,
                                      glassColor: Color(0x26FFFFFF),
                                      refractiveIndex: 1.2,
                                      lightIntensity: 1.1,
                                    )
                                  : const LiquidGlassSettings(
                                      thickness: 20,
                                      blur: 10,
                                      glassColor: Color(0x18D2DCF0),
                                      refractiveIndex: 1.15,
                                      lightIntensity: 0.95,
                                    ),
                              child: _buildContent(
                                selectedAccountId,
                                accountsAsync,
                                isDark,
                              ),
                            ).animate().fadeIn(
                              delay: 200.ms,
                              duration: 400.ms,
                            ),
                          ],
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(
    String? selectedAccountId,
    AsyncValue<List<AccountInfo>> accountsAsync,
    bool isDark,
  ) {
    if (_showCreateAccount) {
      return CreateAccountForm(
        nameController: _newAccountNameController,
        passwordController: _newPasswordController,
        confirmPasswordController: _confirmPasswordController,
        hintController: _passwordHintController,
        obscurePassword: _obscureNewPassword,
        obscureConfirmPassword: _obscureConfirmPassword,
        passwordFocusNode: _newPasswordFocusNode,
        isPasswordFocused: _isNewPasswordFocused,
        confirmPasswordFocusNode: _confirmPasswordFocusNode,
        isConfirmPasswordFocused: _isConfirmPasswordFocused,
        isLoading: _isLoading,
        createError: _createError,
        onCreateAccount: _handleCreateAccount,
        onBack: _backToAccountListFromCreate,
        onToggleObscurePassword: () {
          setState(() => _obscureNewPassword = !_obscureNewPassword);
        },
        onToggleObscureConfirmPassword: () {
          setState(
            () => _obscureConfirmPassword = !_obscureConfirmPassword,
          );
        },
      );
    } else if (selectedAccountId != null) {
      return accountsAsync.when(
        data: (accounts) {
          final selectedAccount = accounts
              .cast<AccountInfo?>()
              .firstWhere(
                (a) => a?.id == selectedAccountId,
                orElse: () => null,
              );
          if (selectedAccount != null) {
            return PasswordInputSection(
              formKey: _formKey,
              passwordController: _passwordController,
              obscurePassword: _obscurePassword,
              passwordFocusNode: _passwordFocusNode,
              isPasswordFocused: _isPasswordFocused,
              hasPasswordError: _hasPasswordError,
              passwordErrorMessage: _passwordErrorMessage,
              isLoading: _isLoading,
              biometricsEnabled: _biometricsEnabled,
              biometricType: _biometricType,
              selectedAccount: selectedAccount,
              onBack: _backToAccountList,
              onUnlock: _handleUnlock,
              onBiometricUnlock: _handleBiometricUnlock,
              onToggleObscure: () {
                setState(() => _obscurePassword = !_obscurePassword);
              },
              onShowPasswordHint: _showPasswordHint,
            );
          }
          return const SizedBox.shrink();
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (_, __) => const SizedBox.shrink(),
      );
    } else {
      return accountsAsync.when(
        data: (accounts) => AccountListSection(
          accounts: accounts,
          accountsExpanded: _accountsExpanded,
          onSelectAccount: _selectAccount,
          onToggleExpanded: () {
            setState(() => _accountsExpanded = !_accountsExpanded);
          },
          onCreateAccount: () {
            setState(() {
              _showCreateAccount = true;
              _accountsExpanded = false;
            });
          },
          formatLastAccessed: (lastAccessed) => _formatLastAccessed(lastAccessed, AppLocalizations.of(context)),
        ),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => Center(child: Text('${AppLocalizations.of(context).commonError}: $error')),
      );
    }
  }
}

class _PasswordHintOverlay extends StatelessWidget {
  final String hint;
  final VoidCallback onDismiss;

  const _PasswordHintOverlay({required this.hint, required this.onDismiss});

  @override
  Widget build(BuildContext context) {
    return Positioned(
      top: MediaQuery.of(context).padding.top + kToolbarHeight + 8,
      left: 16,
      right: 16,
      child: SafeArea(
        child: Material(
          color: Colors.transparent,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
            decoration: BoxDecoration(
              color: AppTheme.primaryColor,
              borderRadius: BorderRadius.circular(12),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.15),
                  blurRadius: 10,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Row(
              children: [
                const Icon(Icons.help_outline, color: Colors.white, size: 22),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    AppLocalizations.of(context).loginPasswordHint(hint),
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                IconButton(
                  icon: const Icon(
                    Icons.close,
                    color: Colors.white70,
                    size: 18,
                  ),
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                  onPressed: onDismiss,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
