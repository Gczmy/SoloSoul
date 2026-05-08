import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// Create account form for the login page.
/// Displays account name, password, confirm password, hint fields and action buttons.
class CreateAccountForm extends StatelessWidget {
  final TextEditingController nameController;
  final TextEditingController passwordController;
  final TextEditingController confirmPasswordController;
  final TextEditingController hintController;
  final bool obscurePassword;
  final bool obscureConfirmPassword;
  final FocusNode passwordFocusNode;
  final bool isPasswordFocused;
  final FocusNode confirmPasswordFocusNode;
  final bool isConfirmPasswordFocused;
  final bool isLoading;
  final String? createError;
  final VoidCallback onCreateAccount;
  final VoidCallback onBack;
  final VoidCallback onToggleObscurePassword;
  final VoidCallback onToggleObscureConfirmPassword;

  const CreateAccountForm({
    super.key,
    required this.nameController,
    required this.passwordController,
    required this.confirmPasswordController,
    required this.hintController,
    required this.obscurePassword,
    required this.obscureConfirmPassword,
    required this.passwordFocusNode,
    required this.isPasswordFocused,
    required this.confirmPasswordFocusNode,
    required this.isConfirmPasswordFocused,
    required this.isLoading,
    this.createError,
    required this.onCreateAccount,
    required this.onBack,
    required this.onToggleObscurePassword,
    required this.onToggleObscureConfirmPassword,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

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
              controller: nameController,
              textInputAction: TextInputAction.next,
              decoration: InputDecoration(
                labelText: l10n.loginAccountName,
                hintText: l10n.loginAccountNameHint,
                prefixIcon: const Icon(Icons.person_outline),
              ),
            )
            .animate()
            .fadeIn(delay: 200.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // New Password Field
        TextFormField(
              controller: passwordController,
              obscureText: obscurePassword,
              focusNode: passwordFocusNode,
              textInputAction: TextInputAction.next,
              decoration: InputDecoration(
                labelText: l10n.loginMasterPassword,
                hintText: l10n.loginCreateStrongPassword,
                labelStyle: TextStyle(
                  color: isPasswordFocused ? AppTheme.primaryColor : null,
                ),
                prefixIcon: Icon(
                  Icons.key,
                  color: isPasswordFocused ? AppTheme.primaryColor : null,
                ),
                suffixIcon: IconButton(
                  icon: Icon(
                    obscurePassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
                    color: isPasswordFocused ? AppTheme.primaryColor : null,
                  ),
                  onPressed: onToggleObscurePassword,
                ),
              ),
            )
            .animate()
            .fadeIn(delay: 250.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // Confirm Password Field
        TextFormField(
              controller: confirmPasswordController,
              obscureText: obscureConfirmPassword,
              focusNode: confirmPasswordFocusNode,
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => onCreateAccount(),
              decoration: InputDecoration(
                labelText: l10n.loginConfirmPassword,
                hintText: l10n.loginReenterPassword,
                labelStyle: TextStyle(
                  color: isConfirmPasswordFocused
                      ? AppTheme.primaryColor
                      : null,
                ),
                prefixIcon: Icon(
                  Icons.key,
                  color: isConfirmPasswordFocused
                      ? AppTheme.primaryColor
                      : null,
                ),
                suffixIcon: IconButton(
                  icon: Icon(
                    obscureConfirmPassword
                        ? Icons.visibility_outlined
                        : Icons.visibility_off_outlined,
                    color: isConfirmPasswordFocused
                        ? AppTheme.primaryColor
                        : null,
                  ),
                  onPressed: onToggleObscureConfirmPassword,
                ),
              ),
            )
            .animate()
            .fadeIn(delay: 300.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 16),

        // Password Hint Field (Optional)
        TextFormField(
              controller: hintController,
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => onCreateAccount(),
              decoration: InputDecoration(
                labelText: l10n.loginPasswordHintOptional,
                hintText: l10n.loginPasswordHintHelp,
                prefixIcon: const Icon(Icons.help_outline),
              ),
            )
            .animate()
            .fadeIn(delay: 350.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        if (createError != null) ...[
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
                    createError!,
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

        // Create Button — Liquid Glass
        _HoverableCreateButton(
          isLoading: isLoading,
          isDark: isDark,
          onCreateAccount: onCreateAccount,
        )
            .animate()
            .fadeIn(delay: 400.ms, duration: 400.ms)
            .slideY(begin: 0.2, end: 0),

        const SizedBox(height: 12),

        // Back to Account List
        TextButton(
          onPressed: onBack,
          child: Text(l10n.loginBackToAccountList),
        ).animate().fadeIn(delay: 450.ms, duration: 400.ms),
      ],
    );
  }
}

/// Hoverable create account button with scale and shadow feedback.
class _HoverableCreateButton extends StatefulWidget {
  final bool isLoading;
  final bool isDark;
  final VoidCallback onCreateAccount;

  const _HoverableCreateButton({
    required this.isLoading,
    required this.isDark,
    required this.onCreateAccount,
  });

  @override
  State<_HoverableCreateButton> createState() => _HoverableCreateButtonState();
}

class _HoverableCreateButtonState extends State<_HoverableCreateButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: AnimatedScale(
        scale: _isHovered && !widget.isLoading ? 1.03 : 1.0,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 200),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            boxShadow: _isHovered && !widget.isLoading
                ? [
                    BoxShadow(
                      color: AppTheme.primaryColor.withValues(
                        alpha: widget.isDark ? 0.4 : 0.25,
                      ),
                      blurRadius: 24,
                      offset: const Offset(0, 8),
                      spreadRadius: 2,
                    ),
                  ]
                : [],
          ),
          child: SizedBox(
            width: double.infinity,
            height: 52,
            child: GlassButton.custom(
              onTap: widget.isLoading ? () {} : widget.onCreateAccount,
              width: double.infinity,
              height: 52,
              shape: const LiquidRoundedSuperellipse(borderRadius: 12),
              child: widget.isLoading
                  ? const SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                      ),
                    )
                  : Text(
                      'Create Account',
                      style: TextStyle(
                        color: widget.isDark
                            ? Colors.white
                            : const Color(0xFF1F1F1F),
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
            ),
          ),
        ),
      ),
    );
  }
}
