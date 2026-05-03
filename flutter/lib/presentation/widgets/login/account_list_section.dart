import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';
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
    final theme = Theme.of(context);
    final displayAccounts = accountsExpanded || accounts.length <= 3
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
                  onTap: () => onSelectAccount(account.id),
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
                          ? AppTheme.primaryColor.withValues(alpha: 0.05)
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
                            crossAxisAlignment: CrossAxisAlignment.start,
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
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 8,
                                        vertical: 2,
                                      ),
                                      decoration: BoxDecoration(
                                        color: AppTheme.primaryColor,
                                        borderRadius: BorderRadius.circular(4),
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
                                'Last accessed: ${formatLastAccessed(account.lastAccessed)}',
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
          onPressed: onCreateAccount,
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
