import 'package:flutter/material.dart';
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
        // Back button and selected account
        Row(
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
              // Password field
              TextFormField(
                controller: passwordController,
                obscureText: obscurePassword,
                focusNode: passwordFocusNode,
                textInputAction: TextInputAction.done,
                onFieldSubmitted: (_) => onUnlock(),
                decoration: InputDecoration(
                  labelText: 'Master Password',
                  hintText: 'Enter your password',
                  labelStyle: TextStyle(
                    color: hasPasswordError
                        ? Colors.red.shade700
                        : isPasswordFocused
                        ? AppTheme.primaryColor
                        : theme.colorScheme.onSurface,
                  ),
                  floatingLabelStyle: TextStyle(
                    color: hasPasswordError
                        ? Colors.red.shade700
                        : isPasswordFocused
                        ? AppTheme.primaryColor
                        : theme.colorScheme.onSurface,
                  ),
                  prefixIcon: Icon(
                    Icons.key,
                    color: hasPasswordError
                        ? Colors.red.shade700
                        : isPasswordFocused
                        ? AppTheme.primaryColor
                        : theme.colorScheme.onSurfaceVariant,
                  ),
                  errorText: hasPasswordError ? passwordErrorMessage : null,
                  errorStyle: TextStyle(
                    color: Colors.red.shade700,
                    fontWeight: FontWeight.w500,
                  ),
                  border: hasPasswordError
                      ? OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide(
                            color: Colors.red.shade700,
                            width: 2,
                          ),
                        )
                      : null,
                  enabledBorder: hasPasswordError
                      ? OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide(
                            color: Colors.red.shade700,
                            width: 2,
                          ),
                        )
                      : null,
                  focusedBorder: hasPasswordError
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
                          color: hasPasswordError
                              ? Colors.red.shade700
                              : isPasswordFocused
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
                          obscurePassword
                              ? Icons.visibility_outlined
                              : Icons.visibility_off_outlined,
                          size: 20,
                          color: hasPasswordError
                              ? Colors.red.shade700
                              : isPasswordFocused
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
              ),

              const SizedBox(height: 24),

              // Unlock button
              ElevatedButton(
                onPressed: isLoading ? null : onUnlock,
                child: isLoading
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
              if (biometricsEnabled) ...[
                const SizedBox(height: 16),
                OutlinedButton.icon(
                  onPressed: isLoading ? null : onBiometricUnlock,
                  icon: Icon(
                    biometricType == 'Face ID'
                        ? Icons.face
                        : Icons.fingerprint,
                    size: 22,
                  ),
                  label: Text('Use $biometricType'),
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
}
