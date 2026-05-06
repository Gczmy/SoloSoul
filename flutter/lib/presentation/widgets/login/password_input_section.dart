import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Password input section for the login page.
/// Displays the selected account, password field, unlock button, and biometric option.
class PasswordInputSection extends StatelessWidget {
  final GlobalKey<FormState> formKey;
  final TextEditingController passwordController;
  final bool obscurePassword;
  final FocusNode passwordFocusNode;
  final bool isPasswordFocused;
  final bool hasPasswordError;
  final String? passwordErrorMessage;
  final bool isLoading;
  final bool biometricsEnabled;
  final String biometricType;
  final AccountInfo selectedAccount;
  final VoidCallback onBack;
  final VoidCallback onUnlock;
  final VoidCallback onBiometricUnlock;
  final VoidCallback onToggleObscure;
  final ValueChanged<String> onShowPasswordHint;

  const PasswordInputSection({
    super.key,
    required this.formKey,
    required this.passwordController,
    required this.obscurePassword,
    required this.passwordFocusNode,
    required this.isPasswordFocused,
    required this.hasPasswordError,
    this.passwordErrorMessage,
    required this.isLoading,
    required this.biometricsEnabled,
    required this.biometricType,
    required this.selectedAccount,
    required this.onBack,
    required this.onUnlock,
    required this.onBiometricUnlock,
    required this.onToggleObscure,
    required this.onShowPasswordHint,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      children: [
        _AccountHeader(
          selectedAccount: selectedAccount,
          onBack: onBack,
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
          key: formKey,
          child: Column(
            children: [
              _PasswordField(
                controller: passwordController,
                obscureText: obscurePassword,
                focusNode: passwordFocusNode,
                isFocused: isPasswordFocused,
                hasError: hasPasswordError,
                errorMessage: passwordErrorMessage,
                selectedAccount: selectedAccount,
                onUnlock: onUnlock,
                onToggleObscure: onToggleObscure,
                onShowPasswordHint: onShowPasswordHint,
              ),
              const SizedBox(height: 24),
              _UnlockButton(
                isLoading: isLoading,
                onUnlock: onUnlock,
              ),
              if (biometricsEnabled) ...[
                const SizedBox(height: 16),
                _BiometricButton(
                  biometricType: biometricType,
                  isLoading: isLoading,
                  onBiometricUnlock: onBiometricUnlock,
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

class _AccountHeader extends StatelessWidget {
  final AccountInfo selectedAccount;
  final VoidCallback onBack;

  const _AccountHeader({
    required this.selectedAccount,
    required this.onBack,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        IconButton(
          onPressed: onBack,
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
    );
  }
}

class _PasswordField extends StatelessWidget {
  final TextEditingController controller;
  final bool obscureText;
  final FocusNode focusNode;
  final bool isFocused;
  final bool hasError;
  final String? errorMessage;
  final AccountInfo selectedAccount;
  final VoidCallback onUnlock;
  final VoidCallback onToggleObscure;
  final ValueChanged<String> onShowPasswordHint;

  const _PasswordField({
    required this.controller,
    required this.obscureText,
    required this.focusNode,
    required this.isFocused,
    required this.hasError,
    this.errorMessage,
    required this.selectedAccount,
    required this.onUnlock,
    required this.onToggleObscure,
    required this.onShowPasswordHint,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return TextFormField(
      controller: controller,
      obscureText: obscureText,
      focusNode: focusNode,
      textInputAction: TextInputAction.done,
      onFieldSubmitted: (_) => onUnlock(),
      decoration: InputDecoration(
        labelText: 'Master Password',
        hintText: 'Enter your password',
        labelStyle: TextStyle(
          color: hasError
              ? Colors.red.shade700
              : isFocused
                  ? AppTheme.primaryColor
                  : theme.colorScheme.onSurface,
        ),
        floatingLabelStyle: TextStyle(
          color: hasError
              ? Colors.red.shade700
              : isFocused
                  ? AppTheme.primaryColor
                  : theme.colorScheme.onSurface,
        ),
        prefixIcon: Icon(
          Icons.key,
          color: hasError
              ? Colors.red.shade700
              : isFocused
                  ? AppTheme.primaryColor
                  : theme.colorScheme.onSurfaceVariant,
        ),
        errorText: hasError ? errorMessage : null,
        errorStyle: TextStyle(
          color: Colors.red.shade700,
          fontWeight: FontWeight.w500,
        ),
        border: hasError
            ? OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
                borderSide: BorderSide(
                  color: Colors.red.shade700,
                  width: 2,
                ),
              )
            : null,
        enabledBorder: hasError
            ? OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
                borderSide: BorderSide(
                  color: Colors.red.shade700,
                  width: 2,
                ),
              )
            : null,
        focusedBorder: hasError
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
                color: hasError
                    ? Colors.red.shade700
                    : isFocused
                        ? AppTheme.primaryColor
                        : theme.colorScheme.onSurfaceVariant,
              ),
              onPressed: () => onShowPasswordHint(
                selectedAccount.passwordHint ?? 'No password hint available',
              ),
              tooltip: 'Show password hint',
            ),
            IconButton(
              constraints: const BoxConstraints(),
              padding: const EdgeInsets.all(8),
              icon: Icon(
                obscureText
                    ? Icons.visibility_outlined
                    : Icons.visibility_off_outlined,
                size: 20,
                color: hasError
                    ? Colors.red.shade700
                    : isFocused
                        ? AppTheme.primaryColor
                        : theme.colorScheme.onSurfaceVariant,
              ),
              onPressed: onToggleObscure,
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
    );
  }
}

class _UnlockButton extends StatelessWidget {
  final bool isLoading;
  final VoidCallback onUnlock;

  const _UnlockButton({
    required this.isLoading,
    required this.onUnlock,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return SizedBox(
      width: double.infinity,
      height: 52,
      child: GlassButton.custom(
        onTap: isLoading ? () {} : onUnlock,
        width: double.infinity,
        height: 52,
        shape: const LiquidRoundedSuperellipse(borderRadius: 12),
        child: isLoading
            ? const SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                ),
              )
            : Text(
                'Unlock',
                style: TextStyle(
                  color: isDark ? Colors.white : const Color(0xFF1F1F1F),
                  fontSize: 16,
                  fontWeight: FontWeight.w600,
                ),
              ),
      ),
    );
  }
}

class _BiometricButton extends StatelessWidget {
  final String biometricType;
  final bool isLoading;
  final VoidCallback onBiometricUnlock;

  const _BiometricButton({
    required this.biometricType,
    required this.isLoading,
    required this.onBiometricUnlock,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return SizedBox(
      width: double.infinity,
      height: 48,
      child: GlassButton.custom(
        onTap: isLoading ? () {} : onBiometricUnlock,
        width: double.infinity,
        height: 48,
        shape: const LiquidRoundedSuperellipse(borderRadius: 12),
        style: GlassButtonStyle.transparent,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              biometricType == 'Face ID' ? Icons.face : Icons.fingerprint,
              size: 22,
              color: isDark ? Colors.white70 : const Color(0xFF6B6B6B),
            ),
            const SizedBox(width: 8),
            Text(
              'Use $biometricType',
              style: TextStyle(
                color: isDark ? Colors.white70 : const Color(0xFF6B6B6B),
                fontSize: 15,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
