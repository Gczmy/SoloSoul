import 'dart:async';

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
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/utils/device_utils.dart' show getDeviceName;
import 'package:solosoul_flutter/presentation/widgets/login/account_list_section.dart';
import 'package:solosoul_flutter/presentation/widgets/login/create_account_form.dart';
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

  // Password verification error state
  bool _hasPasswordError = false;
  String? _passwordErrorMessage;

  // Password field focus state
  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  // Biometric unlock state
  bool _biometricsEnabled = false;
  String _biometricType = 'Biometric';

  // Create account form fields
  final _newAccountNameController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();
  final _passwordHintController = TextEditingController();

  /// 若 Vault 数据为空但存在备份，提示用户恢复
  Future<void> _promptRestoreIfEmpty(
    BuildContext context,
    WidgetRef ref,
    String? accountId,
  ) async {
    if (accountId == null) return;
    if (!context.mounted) return;

    final profile = ref.read(profileNotifierProvider).value;
    final unifiedData = ref.read(unifiedObjectProvider);
    final objectCount = unifiedData.objects.length;
    final activeCount = unifiedData.objects.where((o) => !o.isDeleted).length;

    SoloLog.d(
      'LOGIN',
      '_promptRestoreIfEmpty: profile=${profile != null}, '
      'unifiedObjects=${profile?.unifiedObjects != null}, '
      'objects=$objectCount, active=$activeCount',
    );

    // 保守策略：只要 Vault 中有 profile 数据（包括旧格式），就不提示恢复
    // 避免旧账号因迁移时序或格式问题导致误报
    if (profile != null) {
      if (profile.unifiedObjects != null && activeCount > 0) {
        return; // 有有效数据，无需恢复
      }
      if (profile.unifiedObjects != null && activeCount == 0) {
        // unifiedObjects 存在但为空（可能是新账号或空 section），继续检查备份
      }
      if (profile.unifiedObjects == null) {
        // 旧格式数据（无 unified_objects 字段），profile 存在但无法通过新模型读取
        // 为避免误报，不提示恢复（legacy migration 已移除，旧数据无法自动恢复）
        SoloLog.w(
          'LOGIN',
          'Legacy profile detected without unified_objects, skipping restore prompt',
        );
        return;
      }
    }

    final backups = await BackupService.instance.listBackups(accountId);
    if (backups.isEmpty) return; // 无备份，无需恢复

    if (!context.mounted) return;

    final latest = backups.first;
    final shouldRestore = await showSoloGlassDialog<bool>(
      context: context,
      title: 'Data Recovery',
      message:
          'Your vault appears to be empty, but a backup exists from ${latest.displayTime}. '
          'Would you like to restore from this backup?',
      actions: [
        SoloGlassDialogAction(
          label: 'Skip',
          onPressed: () => Navigator.of(context).pop(false),
        ),
        SoloGlassDialogAction(
          label: 'Restore Backup',
          isPrimary: true,
          onPressed: () => Navigator.of(context).pop(true),
        ),
      ],
    );

    if (shouldRestore == true) {
      final success = await BackupService.instance.restoreBackup(
        accountId,
        latest.fileName,
      );
      if (success && context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Restore successful. Please restart the app.'),
            duration: AppTheme.kPasswordHintDelay,
          ),
        );
      } else if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Restore failed')),
        );
      }
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
    final biometric = BiometricService.instance;
    final available = await biometric.isAvailable();
    final settings = SecurityService.instance.settings;
    final availableBiometrics = await biometric.getAvailableBiometrics();

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final accountId = authNotifier.selectedAccountId;
    final hasCredential = accountId != null &&
        await BiometricCredentialService.instance.hasBiometricCredential(accountId) &&
        await BiometricCredentialService.instance.isDeviceKeyAvailable();

    String biometricType = 'Biometric';
    if (availableBiometrics.isNotEmpty) {
      if (availableBiometrics.any((b) => b == BiometricType.face)) {
        biometricType = 'Face ID';
      } else if (availableBiometrics.any(
        (b) => b == BiometricType.fingerprint,
      )) {
        biometricType = 'Touch ID';
      } else if (availableBiometrics.any((b) => b == BiometricType.iris)) {
        biometricType = 'Iris';
      }
    }

    setState(() {
      _biometricsEnabled = (settings.biometricsEnabled || settings.faceIdEnabled) && available && hasCredential;
      _biometricType = biometricType;
    });
  }

  Future<void> _handleBiometricUnlock() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.selectedAccountId == null) return;

    setState(() => _isLoading = true);

    try {
      final success = await BiometricService.instance.authenticate(
        reason: 'Unlock SoloSoul with $_biometricType',
      );

      if (!success) {
        if (mounted) {
          setState(() => _isLoading = false);
          showOverlaySnackBar(
            context,
            content: 'Biometric authentication failed or was cancelled',
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
            content: 'Failed to unlock vault. Please use your master password.',
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
        setState(() => _isLoading = false);
        showOverlaySnackBar(
          context,
          content: 'Biometric unlock error: $e',
          type: SnackBarType.error,
        );
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

  String _formatLastAccessed(DateTime? lastAccessed) {
    if (lastAccessed == null) return 'Never';
    final now = DateTime.now();
    final diff = now.difference(lastAccessed);
    if (diff.inDays == 0) return 'Today';
    if (diff.inDays == 1) return 'Yesterday';
    if (diff.inDays < 7) return '${diff.inDays} days ago';
    return '${lastAccessed.day}/${lastAccessed.month}/${lastAccessed.year}';
  }

  /// Shared post-login setup: load profile, unified objects, register forms,
  /// record metadata, and navigate to home.
  Future<void> _postLoginSetup() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);

    // Pre-load profile before navigating to home
    SoloLog.d('LOGIN', 'Loading profile...');
    try {
      await ref.read(profileNotifierProvider.notifier).loadProfile().timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          SoloLog.e('LOGIN', 'loadProfile timed out');
        },
      );
      SoloLog.d('LOGIN', 'Profile loaded OK');
    } on Exception catch (e, st) {
      SoloLog.e('LOGIN', 'loadProfile exception', e, st);
    }
    SoloLog.d('LOGIN', 'Calling loadFromProfile...');
    try {
      await ref.read(unifiedObjectProvider.notifier).loadFromProfile();
      SoloLog.d('LOGIN', 'loadFromProfile OK');
    } on Exception catch (e, st) {
      SoloLog.e('LOGIN', 'loadFromProfile exception', e, st);
    }
    final unifiedData = ref.read(unifiedObjectProvider);
    SoloLog.d(
      'LOGIN',
      'After loadFromProfile: objects=${unifiedData.objects.length}, customTypes=${unifiedData.customTypes.length}',
    );

    // 首次启动/空数据检测：若 Vault 无数据但存在备份，提示恢复
    final accountId = authNotifier.selectedAccountId;
    if (mounted) {
      await _promptRestoreIfEmpty(context, ref, accountId);
    }

    // Pre-register all form fields for sensitivity settings
    ref.read(formFieldRegistryProvider.notifier).registerAllForms();

    // Record login metadata (lastLoginAt + device)
    if (accountId != null) {
      await authNotifier.updateAccountMetadata(
        lastLoginAt: DateTime.now(),
        device: DeviceInfo(
          deviceName: getDeviceName(),
          lastUsed: DateTime.now(),
        ).toJson(),
      ).timeout(const Duration(seconds: 5), onTimeout: () {
        SoloLog.w('LOGIN', 'updateAccountMetadata timed out');
      });
    }

    if (mounted) {
      context.go(AppRoutes.home);
    }
  }

  Future<void> _handleUnlock() async {
    final formState = _formKey.currentState;
    if (formState == null) return;
    if (!formState.validate()) {
      // Form validator showed the error - sync our flag so label/icon turn red too
      setState(() {
        _hasPasswordError = true;
        _passwordErrorMessage = 'Password must be at least 8 characters';
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
            ? 'Invalid master password'
            : 'Unlock failed: $specificError';
      });
      return;
    }

    await _postLoginSetup();
  }

  Future<void> _handleCreateAccount() async {
    SoloLog.d('LOGIN', '_handleCreateAccount start');

    final name = _newAccountNameController.text.trim();
    final password = _newPasswordController.text;
    final confirm = _confirmPasswordController.text;

    // Validation
    if (name.isEmpty) {
      setState(() => _createError = 'Account name is required');
      return;
    }
    if (password.length < 8) {
      setState(() => _createError = 'Password must be at least 8 characters');
      return;
    }
    if (password != confirm) {
      setState(() => _createError = 'Passwords do not match');
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
          _createError = result.error ?? 'Failed to create account';
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
          _createError = 'Failed to unlock vault. Please try again.';
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
      builder: (ctx) => Positioned(
        top: MediaQuery.of(ctx).padding.top + kToolbarHeight + 8,
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
                      'Password Hint: $hint',
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
                    onPressed: () {
                      _passwordHintOverlayEntry?.remove();
                      _passwordHintOverlayEntry = null;
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(_passwordHintOverlayEntry!);
    // Use explicit Timer so it persists across navigation (not tied to widget lifecycle)
    _passwordHintTimer = Timer(AppTheme.kPasswordHintDelay, () {
      _passwordHintOverlayEntry?.remove();
      _passwordHintOverlayEntry = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
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
            Positioned(
              top: 80,
              left: -80,
              child: Container(
                width: 240,
                height: 240,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: AppTheme.primaryColor.withValues(
                    alpha: isDark ? 0.12 : 0.07,
                  ),
                ),
              ),
            ),
            Positioned(
              bottom: 120,
              right: -100,
              child: Container(
                width: 300,
                height: 300,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: AppTheme.accentColor.withValues(
                    alpha: isDark ? 0.1 : 0.05,
                  ),
                ),
              ),
            ),
            Positioned(
              top: 300,
              right: 20,
              child: Container(
                width: 100,
                height: 100,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: AppTheme.secondaryColor.withValues(
                    alpha: isDark ? 0.08 : 0.04,
                  ),
                ),
              ),
            ),
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
                            // Header Logo — Liquid Glass orb
                            Center(
                              child: GlassButton(
                                icon: const Icon(
                                  Icons.lock_outline,
                                  color: Colors.white,
                                ),
                                onTap: () {},
                                width: 80,
                                height: 80,
                                iconSize: 36,
                                shape: const LiquidRoundedSuperellipse(
                                  borderRadius: 20,
                                ),
                                useOwnLayer: true,
                                settings: const LiquidGlassSettings(
                                  thickness: 30,
                                  blur: 10,
                                  glassColor: Color(0x4D487CA5),
                                  refractiveIndex: 1.3,
                                  lightIntensity: 1.2,
                                ),
                              ),
                            )
                                .animate()
                                .scale(
                                  begin: const Offset(0.8, 0.8),
                                  end: const Offset(1, 1),
                                  duration: 500.ms,
                                  curve: Curves.easeOutBack,
                                )
                                .fadeIn(),

                            const SizedBox(height: 32),

                            // Title
                            Text(
                              'SoloSoul',
                              style: theme.textTheme.headlineMedium?.copyWith(
                                fontWeight: FontWeight.w700,
                              ),
                              textAlign: TextAlign.center,
                            ).animate().fadeIn(
                              delay: 100.ms,
                              duration: 400.ms,
                            ),

                            const SizedBox(height: 8),

                            // Subtitle
                            Text(
                              'Your data, your control',
                              style: theme.textTheme.bodyLarge?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                                fontWeight: FontWeight.w400,
                              ),
                              textAlign: TextAlign.center,
                            ).animate().fadeIn(
                              delay: 150.ms,
                              duration: 400.ms,
                            ),

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
          formatLastAccessed: _formatLastAccessed,
        ),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, _) => Center(child: Text('Error: $error')),
      );
    }
  }
}
