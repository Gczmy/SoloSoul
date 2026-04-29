import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:go_router/go_router.dart';
import 'package:path_provider/path_provider.dart';
import 'package:solosoul_flutter/core/router/app_router.dart' show AppRoutes;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show formFieldRegistryProvider;
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/services/backup_service.dart';
import 'package:solosoul_flutter/core/services/biometric_credential_service.dart';
import 'package:solosoul_flutter/core/services/biometric_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/debug_logger.dart';

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

    final unifiedData = ref.read(unifiedObjectProvider);
    final hasData = unifiedData.objects.any((o) => !o.isDeleted);
    if (hasData) return; // 数据非空，无需恢复

    final backups = await BackupService.instance.listBackups(accountId);
    if (backups.isEmpty) return; // 无备份，无需恢复

    if (!context.mounted) return;

    final latest = backups.first;
    final shouldRestore = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => AlertDialog(
        title: const Text('Data Recovery'),
        content: Text(
          'Your vault appears to be empty, but a backup exists from ${latest.displayTime}. '
          'Would you like to restore from this backup?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Skip'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Restore Backup'),
          ),
        ],
      ),
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
        await SecureAccountStorage.instance.updateAccountMetadata(
          accountId,
          lastLoginAt: DateTime.now(),
          device: DeviceInfo(
            deviceName: Platform.isMacOS
                ? 'Mac'
                : Platform.isIOS
                ? 'iPhone'
                : Platform.isAndroid
                ? 'Android'
                : Platform.isLinux
                ? 'Linux'
                : Platform.isWindows
                ? 'Windows'
                : 'Flutter Device',
            lastUsed: DateTime.now(),
          ).toJson(),
        );
        await authNotifier.selectAccount(accountId);
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

  Future<void> _handleUnlock() async {
    if (!_formKey.currentState!.validate()) {
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

    DebugLogger.instance.logInfo('LOGIN', 'Starting unlockVault for account: ${authNotifier.selectedAccountId}');
    bool success;
    try {
      success = await authNotifier.unlockVault(_passwordController.text);
    } on Exception catch (e, st) {
      DebugLogger.instance.logError('LOGIN', 'unlockVault exception: $e\n$st');
      success = false;
    }
    DebugLogger.instance.logInfo('LOGIN', 'unlockVault completed, success: $success');

    if (success && mounted) {
      // Pre-load profile before navigating to home
      // Use timeout to prevent hanging if OperationLogService initialization is slow
      await ref.read(profileNotifierProvider.notifier).loadProfile().timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          DebugLogger.instance.logError('LOGIN', 'loadProfile timed out');
        },
      );
      await ref.read(unifiedObjectProvider.notifier).loadFromProfile();

      // 首次启动/空数据检测：若 Vault 无数据但存在备份，提示恢复
      final accountId = authNotifier.selectedAccountId;
      if (!mounted) return;
      await _promptRestoreIfEmpty(context, ref, accountId);

      // Pre-register all form fields for sensitivity settings
      ref.read(formFieldRegistryProvider.notifier).registerAllForms();

      // Record login metadata (lastLoginAt + device)
      if (accountId != null) {
        await SecureAccountStorage.instance.updateAccountMetadata(
          accountId,
          lastLoginAt: DateTime.now(),
          device: DeviceInfo(
            deviceName: Platform.isMacOS
                ? 'Mac'
                : Platform.isIOS
                ? 'iPhone'
                : Platform.isAndroid
                ? 'Android'
                : Platform.isLinux
                ? 'Linux'
                : Platform.isWindows
                ? 'Windows'
                : 'Flutter Device',
            lastUsed: DateTime.now(),
          ).toJson(),
        ).timeout(const Duration(seconds: 5), onTimeout: () {
          DebugLogger.instance.logWarning('LOGIN', 'updateAccountMetadata timed out');
        });
        // Reload selected account info so Settings page shows updated data
        await authNotifier.selectAccount(accountId);
      }

      if (mounted) {
        context.go(AppRoutes.home);
      }
    } else if (mounted) {
      setState(() {
        _isLoading = false;
        _hasPasswordError = true;
        _passwordErrorMessage = 'Invalid master password';
      });
    }
  }

  Future<void> _handleCreateAccount() async {
    // Use path_provider for cross-platform log directory (debug build only)
    File? traceLog;
    if (kDebugMode) {
      try {
        final appDocDir = await getApplicationDocumentsDirectory();
        final logDir = Directory('${appDocDir.path}/logs');
        if (!await logDir.exists()) {
          await logDir.create(recursive: true);
        }
        traceLog = File('${logDir.path}/flutter_native_vault.log');
        await traceLog.writeAsString('${DateTime.now().toIso8601String()} [LOGIN] _handleCreateAccount start\n', mode: FileMode.append);
      } on Exception {
        // Silently fail if logging fails - not critical path
      }
    }

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

    await traceLog?.writeAsString('${DateTime.now().toIso8601String()} [LOGIN] calling authNotifier.createAccount\n', mode: FileMode.append);

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final passwordHint = _passwordHintController.text.trim();
    final result = await authNotifier.createAccount(
      name,
      password,
      passwordHint: passwordHint.isEmpty ? null : passwordHint,
    );

    await traceLog?.writeAsString('${DateTime.now().toIso8601String()} [LOGIN] createAccount returned, success=${result.success}\n', mode: FileMode.append);

    if (result.success && mounted) {
      // Account created, now unlock
      await traceLog?.writeAsString('${DateTime.now().toIso8601String()} [LOGIN] calling authNotifier.unlockVault\n', mode: FileMode.append);
      final success = await authNotifier.unlockVault(password);
      await traceLog?.writeAsString('${DateTime.now().toIso8601String()} [LOGIN] unlockVault returned, success=$success\n', mode: FileMode.append);
      if (success && mounted) {
        // Pre-load profile before navigating to home
        await ref.read(profileNotifierProvider.notifier).loadProfile();
        await ref.read(unifiedObjectProvider.notifier).loadFromProfile();
        // Pre-register all form fields for sensitivity settings
        ref.read(formFieldRegistryProvider.notifier).registerAllForms();
        if (mounted) {
          context.go(AppRoutes.home);
        }
      } else if (mounted) {
        setState(() {
          _createError = 'Failed to unlock vault. Please try again.';
          _isLoading = false;
        });
      }
    } else if (mounted) {
      setState(() {
        _createError = result.error ?? 'Failed to create account';
        _isLoading = false;
      });
    }
  }

  Future<void> _selectAccount(String accountId) async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    await authNotifier.selectAccount(accountId);
    setState(() {});
  }

  Future<void> _backToAccountList() async {
    final authNotifier = ref.read(authNotifierProvider.notifier);
    await authNotifier.selectAccount(null);
    _passwordController.clear();
    setState(() {
      _accountsExpanded = false;
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
    // Use ref.read to avoid rebuilds when auth state changes during setState
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccountId = authNotifier.selectedAccountId;

    // Use ref.watch to reactively rebuild when accounts change
    final accountsAsync = ref.watch(accountsProvider);

    return Scaffold(
      body: SafeArea(
        child: SingleChildScrollView(
          padding: EdgeInsets.only(
            left: 24,
            right: 24,
            top: 60,
            bottom: bottomPadding + 24,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header Logo
              Center(
                    child: Container(
                      width: 80,
                      height: 80,
                      decoration: BoxDecoration(
                        gradient: const LinearGradient(
                          colors: [
                            AppTheme.primaryColor,
                            AppTheme.secondaryColor,
                          ],
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        ),
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child: const Icon(
                        Icons.lock_outline,
                        size: 40,
                        color: Colors.white,
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
              ).animate().fadeIn(delay: 100.ms, duration: 400.ms),

              const SizedBox(height: 8),

              // Content based on state
              if (_showCreateAccount) ...[
                _buildCreateAccountForm(theme),
              ] else if (selectedAccountId != null) ...[
                accountsAsync.when(
                  data: (accounts) {
                    final selectedAccount = accounts
                        .cast<AccountInfo?>()
                        .firstWhere(
                          (a) => a?.id == selectedAccountId,
                          orElse: () => null,
                        );
                    if (selectedAccount != null) {
                      return _buildPasswordInput(theme, selectedAccount);
                    }
                    return const SizedBox.shrink();
                  },
                  loading: () =>
                      const Center(child: CircularProgressIndicator()),
                  error: (_, __) => const SizedBox.shrink(),
                ),
              ] else ...[
                accountsAsync.when(
                  data: (accounts) => _buildAccountList(theme, accounts),
                  loading: () =>
                      const Center(child: CircularProgressIndicator()),
                  error: (error, _) => Center(child: Text('Error: $error')),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildCreateAccountForm(ThemeData theme) {
    return Column(
      children: [
        Text(
          'Create New Account',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w600,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(delay: 150.ms, duration: 400.ms),

        const SizedBox(height: 32),

        // Account Name Field
        TextFormField(
              controller: _newAccountNameController,
              textInputAction: TextInputAction.next,
              decoration: const InputDecoration(
                labelText: 'Account Name',
                hintText: 'e.g., Personal, Work',
                prefixIcon: Icon(Icons.person_outline),
              ),
            )
            .animate()
            .fadeIn(delay: 200.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // New Password Field
        TextFormField(
              controller: _newPasswordController,
              obscureText: _obscureNewPassword,
              focusNode: _newPasswordFocusNode,
              textInputAction: TextInputAction.next,
              decoration: InputDecoration(
                labelText: 'Master Password',
                hintText: 'Create a strong password',
                labelStyle: TextStyle(
                  color: _isNewPasswordFocused ? AppTheme.primaryColor : null,
                ),
                prefixIcon: Icon(
                  Icons.key,
                  color: _isNewPasswordFocused ? AppTheme.primaryColor : null,
                ),
                suffixIcon: IconButton(
                  icon: Icon(
                    _obscureNewPassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
                    color: _isNewPasswordFocused ? AppTheme.primaryColor : null,
                  ),
                  onPressed: () {
                    setState(() => _obscureNewPassword = !_obscureNewPassword);
                  },
                ),
              ),
            )
            .animate()
            .fadeIn(delay: 250.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // Confirm Password Field
        TextFormField(
              controller: _confirmPasswordController,
              obscureText: _obscureConfirmPassword,
              focusNode: _confirmPasswordFocusNode,
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => _handleCreateAccount(),
              decoration: InputDecoration(
                labelText: 'Confirm Password',
                hintText: 'Re-enter your password',
                labelStyle: TextStyle(
                  color: _isConfirmPasswordFocused
                      ? AppTheme.primaryColor
                      : null,
                ),
                prefixIcon: Icon(
                  Icons.key,
                  color: _isConfirmPasswordFocused
                      ? AppTheme.primaryColor
                      : null,
                ),
                suffixIcon: IconButton(
                  icon: Icon(
                    _obscureConfirmPassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
                    color: _isConfirmPasswordFocused
                        ? AppTheme.primaryColor
                        : null,
                  ),
                  onPressed: () {
                    setState(
                      () => _obscureConfirmPassword = !_obscureConfirmPassword,
                    );
                  },
                ),
              ),
            )
            .animate()
            .fadeIn(delay: 300.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // Password Hint Field (Optional)
        TextFormField(
              controller: _passwordHintController,
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => _handleCreateAccount(),
              decoration: const InputDecoration(
                labelText: 'Password Hint (Optional)',
                hintText: 'A hint to help you remember',
                prefixIcon: Icon(Icons.help_outline),
              ),
            )
            .animate()
            .fadeIn(delay: 350.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        if (_createError != null) ...[
          const SizedBox(height: 16),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.red.shade50,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.red.shade200),
            ),
            child: Row(
              children: [
                Icon(Icons.error_outline, color: Colors.red.shade700, size: 20),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    _createError!,
                    style: TextStyle(color: Colors.red.shade700),
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(duration: 300.ms),
        ],

        const SizedBox(height: 24),

        // Warning
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Colors.orange.shade50,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: Colors.orange.shade200),
          ),
          child: Row(
            children: [
              Icon(
                Icons.warning_amber,
                color: Colors.orange.shade700,
                size: 24,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'There is no password recovery. If you forget your master password, your data cannot be accessed.',
                  style: TextStyle(color: Colors.orange.shade900, fontSize: 13),
                ),
              ),
            ],
          ),
        ).animate().fadeIn(delay: 350.ms, duration: 400.ms),

        const SizedBox(height: 24),

        // Create Button
        ElevatedButton(
              onPressed: _isLoading ? null : _handleCreateAccount,
              child: _isLoading
                  ? const SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                      ),
                    )
                  : const Text('Create Account'),
            )
            .animate()
            .fadeIn(delay: 400.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 12),

        // Back to Account List
        TextButton(
          onPressed: () {
            setState(() {
              _showCreateAccount = false;
              _createError = null;
              _newAccountNameController.clear();
              _newPasswordController.clear();
              _confirmPasswordController.clear();
            });
          },
          child: const Text('Back to Account List'),
        ).animate().fadeIn(delay: 450.ms, duration: 400.ms),
      ],
    );
  }

  Widget _buildPasswordInput(ThemeData theme, AccountInfo selectedAccount) {
    return Column(
      children: [
        // Back button and selected account
        Row(
          children: [
            IconButton(
              onPressed: _backToAccountList,
              icon: const Icon(Icons.arrow_back),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(),
            ),
            const SizedBox(width: 12),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: AppTheme.primaryColor.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  CircleAvatar(
                    radius: 14,
                    backgroundColor: AppTheme.primaryColor,
                    child: Text(
                      selectedAccount.name.isNotEmpty
                          ? selectedAccount.name[0].toUpperCase()
                          : '?',
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    selectedAccount.name,
                    style: const TextStyle(
                      fontWeight: FontWeight.w600,
                      color: AppTheme.primaryColor,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),

        const SizedBox(height: 32),

        Text(
          'Enter Master Password',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w600,
          ),
          textAlign: TextAlign.center,
        ),

        const SizedBox(height: 8),

        Text(
          'Unlock your vault',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
        ),

        const SizedBox(height: 32),

        Form(
          key: _formKey,
          child: Column(
            children: [
              // Password field
              TextFormField(
                controller: _passwordController,
                obscureText: _obscurePassword,
                focusNode: _passwordFocusNode,
                textInputAction: TextInputAction.done,
                onFieldSubmitted: (_) => _handleUnlock(),
                decoration: InputDecoration(
                  labelText: 'Master Password',
                  hintText: 'Enter your password',
                  labelStyle: TextStyle(
                    color: _hasPasswordError
                        ? Colors.red.shade700
                        : _isPasswordFocused
                        ? AppTheme.primaryColor
                        : Theme.of(context).colorScheme.onSurface,
                  ),
                  floatingLabelStyle: TextStyle(
                    color: _hasPasswordError
                        ? Colors.red.shade700
                        : _isPasswordFocused
                        ? AppTheme.primaryColor
                        : Theme.of(context).colorScheme.onSurface,
                  ),
                  prefixIcon: Icon(
                    Icons.key,
                    color: _hasPasswordError
                        ? Colors.red.shade700
                        : _isPasswordFocused
                        ? AppTheme.primaryColor
                        : Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                  errorText: _hasPasswordError ? _passwordErrorMessage : null,
                  errorStyle: TextStyle(
                    color: Colors.red.shade700,
                    fontWeight: FontWeight.w500,
                  ),
                  border: _hasPasswordError
                      ? OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide(
                            color: Colors.red.shade700,
                            width: 2,
                          ),
                        )
                      : null,
                  enabledBorder: _hasPasswordError
                      ? OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide(
                            color: Colors.red.shade700,
                            width: 2,
                          ),
                        )
                      : null,
                  focusedBorder: _hasPasswordError
                      ? OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide(
                            color: Colors.red.shade700,
                            width: 2,
                          ),
                        )
                      : null,
                  focusedErrorBorder: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(12),
                    borderSide: BorderSide(
                      color: Colors.red.shade700,
                      width: 2,
                    ),
                  ),
                  suffixIcon: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        constraints: const BoxConstraints(),
                        padding: const EdgeInsets.all(8),
                        icon: Icon(
                          Icons.help_outline,
                          size: 20,
                          color: _hasPasswordError
                              ? Colors.red.shade700
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(
                                  context,
                                ).colorScheme.onSurfaceVariant,
                        ),
                        onPressed: () => _showPasswordHint(
                          selectedAccount.passwordHint ?? 'No password hint available',
                        ),
                        tooltip: 'Show password hint',
                      ),
                      IconButton(
                        constraints: const BoxConstraints(),
                        padding: const EdgeInsets.all(8),
                        icon: Icon(
                          _obscurePassword
                              ? Icons.visibility_outlined
                              : Icons.visibility_off_outlined,
                          size: 20,
                          color: _hasPasswordError
                              ? Colors.red.shade700
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                        onPressed: () {
                          setState(() => _obscurePassword = !_obscurePassword);
                        },
                      ),
                    ],
                  ),
                ),
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return 'Please enter your password';
                  }
                  if (value.length < 8) {
                    return 'Password must be at least 8 characters';
                  }
                  return null;
                },
              ),

              const SizedBox(height: 24),

              // Unlock button
              ElevatedButton(
                onPressed: _isLoading ? null : _handleUnlock,
                child: _isLoading
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          valueColor: AlwaysStoppedAnimation<Color>(
                            Colors.white,
                          ),
                        ),
                      )
                    : const Text('Unlock'),
              ),

              // Face ID / Touch ID button
              if (_biometricsEnabled) ...[
                const SizedBox(height: 16),
                OutlinedButton.icon(
                  onPressed: _isLoading ? null : _handleBiometricUnlock,
                  icon: Icon(
                    _biometricType == 'Face ID'
                        ? Icons.face
                        : Icons.fingerprint,
                    size: 22,
                  ),
                  label: Text('Use $_biometricType'),
                  style: OutlinedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 24,
                      vertical: 12,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildAccountList(ThemeData theme, List<AccountInfo> accounts) {
    final displayAccounts = _accountsExpanded || accounts.length <= 3
        ? accounts
        : accounts.sublist(0, 3);

    return Column(
      children: [
        Text(
          'Select an account to unlock',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w600,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(delay: 150.ms, duration: 400.ms),

        const SizedBox(height: 32),

        if (accounts.isNotEmpty) ...[
          ...displayAccounts.asMap().entries.map((entry) {
            final index = entry.key;
            final account = entry.value;
            final isRecent = index == 0;

            return Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Material(
                color: Colors.transparent,
                child: InkWell(
                  onTap: () => _selectAccount(account.id),
                  borderRadius: BorderRadius.circular(12),
                  child: Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: isRecent
                            ? AppTheme.primaryColor
                            : theme.dividerColor,
                        width: isRecent ? 2 : 1,
                      ),
                      borderRadius: BorderRadius.circular(12),
                      color: isRecent
                          ? AppTheme.primaryColor.withValues(
                              alpha: 0.05,
                            )
                          : null,
                    ),
                    child: Row(
                      children: [
                        CircleAvatar(
                          radius: 22,
                          backgroundColor: AppTheme.primaryColor,
                          child: Text(
                            account.name.isNotEmpty
                                ? account.name[0].toUpperCase()
                                : '?',
                            style: const TextStyle(
                              color: Colors.white,
                              fontWeight: FontWeight.w600,
                              fontSize: 16,
                            ),
                          ),
                        ),
                        const SizedBox(width: 16),
                        Expanded(
                          child: Column(
                            crossAxisAlignment:
                                CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  Text(
                                    account.name,
                                    style: const TextStyle(
                                      fontWeight: FontWeight.w600,
                                      fontSize: 16,
                                    ),
                                  ),
                                  if (isRecent) ...[
                                    const SizedBox(width: 8),
                                    Container(
                                      padding:
                                          const EdgeInsets.symmetric(
                                        horizontal: 8,
                                        vertical: 2,
                                      ),
                                      decoration: BoxDecoration(
                                        color: AppTheme.primaryColor,
                                        borderRadius:
                                            BorderRadius.circular(4),
                                      ),
                                      child: const Text(
                                        'Recent',
                                        style: TextStyle(
                                          color: Colors.white,
                                          fontSize: 10,
                                          fontWeight: FontWeight.w600,
                                        ),
                                      ),
                                    ),
                                  ],
                                ],
                              ),
                              const SizedBox(height: 4),
                              Text(
                                'Last accessed: ${_formatLastAccessed(account.lastAccessed)}',
                                style: TextStyle(
                                  color: theme
                                      .colorScheme
                                      .onSurfaceVariant,
                                  fontSize: 13,
                                ),
                              ),
                            ],
                          ),
                        ),
                        Icon(
                          Icons.chevron_right,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            );
          }),

          // Expand/Collapse button when > 3 accounts
          if (accounts.length > 3) ...[
            const SizedBox(height: 8),
            TextButton.icon(
              onPressed: () {
                setState(() => _accountsExpanded = !_accountsExpanded);
              },
              icon: Icon(
                _accountsExpanded ? Icons.expand_less : Icons.expand_more,
              ),
              label: Text(
                _accountsExpanded
                    ? 'Show less'
                    : 'Show all ${accounts.length} accounts',
              ),
            ),
          ],
        ] else ...[
          Container(
            padding: AppTheme.kPagePadding,
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Column(
              children: [
                Icon(
                  Icons.account_circle_outlined,
                  size: 48,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(height: 12),
                Text(
                  'No accounts yet',
                  style: TextStyle(
                    color: theme.colorScheme.onSurfaceVariant,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
          ),
        ],

        const SizedBox(height: 24),

        // Create New Account Button
        OutlinedButton.icon(
          onPressed: () {
            setState(() {
              _showCreateAccount = true;
              _accountsExpanded = false;
            });
          },
          icon: const Icon(Icons.add),
          label: const Text('Create New Account'),
          style: OutlinedButton.styleFrom(
            padding: const EdgeInsets.symmetric(
              horizontal: 24,
              vertical: 12,
            ),
          ),
        ),
      ],
    );
  }
}
