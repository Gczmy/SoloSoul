import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

/// Account list section for the login page.
/// Displays available accounts for selection and a "Create New Account" button.
class AccountListSection extends StatelessWidget {
  final List<AccountInfo> accounts;
  final bool accountsExpanded;
  final ValueChanged<String> onSelectAccount;
  final VoidCallback onToggleExpanded;
  final VoidCallback onCreateAccount;
  final String Function(DateTime?) formatLastAccessed;

  const AccountListSection({
    super.key,
    required this.accounts,
    required this.accountsExpanded,
    required this.onSelectAccount,
    required this.onToggleExpanded,
    required this.onCreateAccount,
    required this.formatLastAccessed,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;
    final displayAccounts = accountsExpanded || accounts.length <= 3
        ? accounts
        : accounts.sublist(0, 3);

    return Column(
      children: [
        Text(
          AppLocalizations.of(context).loginSelectAccountToUnlock,
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
              child: _AccountListItem(
                account: account,
                isRecent: isRecent,
                onTap: () => onSelectAccount(account.id),
                formatLastAccessed: formatLastAccessed,
              ),
            );
          }),

          // Expand/Collapse button when > 3 accounts
          if (accounts.length > 3) ...[
            const SizedBox(height: 8),
            TextButton.icon(
              onPressed: onToggleExpanded,
              icon: Icon(
                accountsExpanded ? Icons.expand_less : Icons.expand_more,
              ),
              label: Text(
                accountsExpanded
                    ? l10n.loginShowLess
                    : l10n.loginShowAllAccounts(accounts.length),
              ),
            ),
          ],
        ] else ...[
          GlassCard(
            useOwnLayer: true,
            padding: AppTheme.kPagePadding,
            child: Column(
              children: [
                Icon(
                  Icons.account_circle_outlined,
                  size: 48,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(height: 12),
                Text(
                  l10n.loginNoAccountsYet,
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

        // Create New Account Button — Liquid Glass
        _HoverableCreateAccountButton(
          onTap: onCreateAccount,
          isDark: isDark,
        ),
      ],
    );
  }
}

/// Hoverable account list item with scale and shadow feedback.
class _AccountListItem extends StatefulWidget {
  final AccountInfo account;
  final bool isRecent;
  final VoidCallback onTap;
  final String Function(DateTime?) formatLastAccessed;

  const _AccountListItem({
    required this.account,
    required this.isRecent,
    required this.onTap,
    required this.formatLastAccessed,
  });

  @override
  State<_AccountListItem> createState() => _AccountListItemState();
}

class _AccountListItemState extends State<_AccountListItem> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final isDark = MediaQuery.platformBrightnessOf(context) == Brightness.dark;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _isHovered ? 1.015 : 1.0,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(12),
              boxShadow: _isHovered
                  ? [
                      BoxShadow(
                        color: AppTheme.primaryColor.withValues(
                          alpha: isDark ? 0.3 : 0.15,
                        ),
                        blurRadius: 20,
                        offset: const Offset(0, 8),
                        spreadRadius: 2,
                      ),
                    ]
                  : [],
            ),
            child: GlassCard(
              useOwnLayer: true,
              padding: const EdgeInsets.all(16),
              margin: EdgeInsets.zero,
              shape: const LiquidRoundedSuperellipse(borderRadius: 12),
              settings: widget.isRecent
                  ? LiquidGlassSettings(
                      thickness: isDark ? 35 : 22,
                      blur: isDark ? 12 : 10,
                      glassColor: isDark
                          ? const Color(0x33487CA5)
                          : const Color(0x1A487CA5),
                      refractiveIndex: 1.2,
                      lightIntensity: isDark ? 1.2 : 1.0,
                    )
                  : null,
              child: Row(
                children: [
                  CircleAvatar(
                    radius: 22,
                    backgroundColor: AppTheme.primaryColor,
                    child: Text(
                      widget.account.name.isNotEmpty
                          ? widget.account.name[0].toUpperCase()
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
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(
                              widget.account.name,
                              style: const TextStyle(
                                fontWeight: FontWeight.w600,
                                fontSize: 16,
                              ),
                            ),
                            if (widget.isRecent) ...[
                              const SizedBox(width: 8),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 8,
                                  vertical: 2,
                                ),
                                decoration: BoxDecoration(
                                  color: AppTheme.primaryColor,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  l10n.loginRecent,
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
                          l10n.loginLastAccessed(widget.formatLastAccessed(widget.account.lastAccessed)),
                          style: TextStyle(
                            color: theme.colorScheme.onSurfaceVariant,
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
      ),
    );
  }
}

/// Hoverable create account button with scale and shadow feedback.
class _HoverableCreateAccountButton extends StatefulWidget {
  final VoidCallback onTap;
  final bool isDark;

  const _HoverableCreateAccountButton({
    required this.onTap,
    required this.isDark,
  });

  @override
  State<_HoverableCreateAccountButton> createState() =>
      _HoverableCreateAccountButtonState();
}

class _HoverableCreateAccountButtonState
    extends State<_HoverableCreateAccountButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: AnimatedScale(
        scale: _isHovered ? 1.03 : 1.0,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 200),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: AppTheme.primaryColor.withValues(
                        alpha: widget.isDark ? 0.35 : 0.2,
                      ),
                      blurRadius: 20,
                      offset: const Offset(0, 8),
                      spreadRadius: 2,
                    ),
                  ]
                : [],
          ),
          child: SizedBox(
            width: double.infinity,
            height: 48,
            child: GlassButton.custom(
              onTap: widget.onTap,
              width: double.infinity,
              height: 48,
              shape: const LiquidRoundedSuperellipse(borderRadius: 12),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    Icons.add,
                    size: 20,
                    color: widget.isDark ? Colors.white : const Color(0xFF1F1F1F),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    l10n.loginCreateNewAccount,
                    style: TextStyle(
                      color: widget.isDark ? Colors.white : const Color(0xFF1F1F1F),
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
