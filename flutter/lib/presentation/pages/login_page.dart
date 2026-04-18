import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/pages/home_page.dart';

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

  // Create account form fields
  final _newAccountNameController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();
  final _passwordHintController = TextEditingController();
  bool _obscureNewPassword = true;
  bool _obscureConfirmPassword = true;
  String? _createError;

  // Password hint overlay tracking
  OverlayEntry? _passwordHintOverlayEntry;
  Timer? _passwordHintTimer;

  @override
  void dispose() {
    _passwordHintTimer?.cancel();
    _passwordHintOverlayEntry?.remove();
    _passwordController.dispose();
    _newAccountNameController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    _passwordHintController.dispose();
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
    if (!_formKey.currentState!.validate()) return;

    final authNotifier = ref.read(authNotifierProvider.notifier);
    if (authNotifier.selectedAccountId == null) return;

    setState(() => _isLoading = true);

    final success = await authNotifier.unlockVault(_passwordController.text);

    if (success && mounted) {
      // Pre-load profile before navigating to home
      // Await directly to ensure load completes before navigation
      await ref.read(profileNotifierProvider.notifier).loadProfile();

      // Record login metadata (lastLoginAt + device)
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
        // Reload selected account info so Settings page shows updated data
        await authNotifier.selectAccount(accountId);
      }

      if (mounted) {
        Navigator.of(
          context,
        ).pushReplacement(MaterialPageRoute(builder: (_) => const HomePage()));
      }
    } else if (mounted) {
      setState(() => _isLoading = false);
      _passwordController.clear();
      showOverlaySnackBar(
        context,
        content: 'Invalid master password',
        type: SnackBarType.error,
      );
    }
  }

  Future<void> _handleCreateAccount() async {
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

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final passwordHint = _passwordHintController.text.trim();
    final result = await authNotifier.createAccount(
      name,
      password,
      passwordHint: passwordHint.isEmpty ? null : passwordHint,
    );

    if (result.success && mounted) {
      // Account created, now unlock
      final success = await authNotifier.unlockVault(password);
      if (success && mounted) {
        // Pre-load profile before navigating to home
        await ref.read(profileNotifierProvider.notifier).loadProfile();
        if (mounted) {
          Navigator.of(context).pushReplacementNamed('/home');
        }
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
                    onPressed: () => _passwordHintOverlayEntry?.remove(),
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
    _passwordHintTimer = Timer(const Duration(seconds: 4), () {
      _passwordHintOverlayEntry?.remove();
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bottomPadding = MediaQuery.of(context).viewInsets.bottom;
    ref.watch(authNotifierProvider); // Watch to rebuild on auth state changes
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final selectedAccountId = authNotifier.selectedAccountId;

    // Use FutureBuilder for accounts
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
              textInputAction: TextInputAction.next,
              decoration: InputDecoration(
                labelText: 'Master Password',
                hintText: 'Create a strong password',
                prefixIcon: const Icon(Icons.key),
                suffixIcon: IconButton(
                  icon: Icon(
                    _obscureNewPassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
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
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => _handleCreateAccount(),
              decoration: InputDecoration(
                labelText: 'Confirm Password',
                hintText: 'Re-enter your password',
                prefixIcon: const Icon(Icons.key),
                suffixIcon: IconButton(
                  icon: Icon(
                    _obscureConfirmPassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
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
                      selectedAccount.name[0].toUpperCase(),
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
        ).animate().fadeIn(duration: 300.ms),

        const SizedBox(height: 32),

        Text(
          'Enter Master Password',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w600,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(delay: 100.ms, duration: 400.ms),

        const SizedBox(height: 8),

        Text(
          'Unlock your vault',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
        ).animate().fadeIn(delay: 150.ms, duration: 400.ms),

        const SizedBox(height: 32),

        Form(
          key: _formKey,
          child: Column(
            children: [
              // Password field
              TextFormField(
                    controller: _passwordController,
                    obscureText: _obscurePassword,
                    autofocus: true,
                    textInputAction: TextInputAction.done,
                    onFieldSubmitted: (_) => _handleUnlock(),
                    decoration: InputDecoration(
                      labelText: 'Master Password',
                      hintText: 'Enter your password',
                      prefixIcon: const Icon(Icons.key),
                      suffixIcon: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Visibility(
                            visible: selectedAccount.passwordHint != null,
                            child: IconButton(
                              icon: const Icon(Icons.help_outline, size: 20),
                              onPressed: () => _showPasswordHint(
                                selectedAccount.passwordHint!,
                              ),
                              tooltip: 'Show password hint',
                            ),
                          ),
                          IconButton(
                            icon: Icon(
                              _obscurePassword
                                  ? Icons.visibility_outlined
                                  : Icons.visibility_off_outlined,
                            ),
                            onPressed: () {
                              setState(
                                () => _obscurePassword = !_obscurePassword,
                              );
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
                  )
                  .animate()
                  .fadeIn(delay: 200.ms, duration: 400.ms)
                  .slideY(begin: 0.2, end: 0),

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
                  )
                  .animate()
                  .fadeIn(delay: 300.ms, duration: 400.ms)
                  .slideY(begin: 0.2, end: 0),
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
              child:
                  Material(
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
                                    account.name[0].toUpperCase(),
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
                      )
                      .animate()
                      .fadeIn(delay: (200 + index * 50).ms, duration: 400.ms)
                      .slideX(begin: 0.1, end: 0),
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
            padding: const EdgeInsets.all(24),
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
          ).animate().fadeIn(delay: 200.ms, duration: 400.ms),
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
            )
            .animate()
            .fadeIn(delay: 300.ms, duration: 400.ms)
            .slideY(begin: 0.1, end: 0),
      ],
    );
  }
}
