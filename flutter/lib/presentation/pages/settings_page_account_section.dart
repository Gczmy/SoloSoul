part of 'settings_page.dart';

/// Account settings section showing current account, all accounts, and data management.
/// Extracted from SettingsPage to reduce file length.
class _AccountSettingsSection extends ConsumerWidget {
  final String totalDataSize;

  const _AccountSettingsSection({required this.totalDataSize});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final accountsAsync = ref.watch(accountsProvider);
    final authNotifier = ref.read(authNotifierProvider.notifier);

    return SectionCard(
      title: l10n.settingsAccount,
      icon: Icons.account_circle_outlined,
      children: [
        accountsAsync.when(
          data: (accounts) {
            final selectedId = authNotifier.selectedAccountId;
            final currentAccount = accounts
                .cast<AccountInfo?>()
                .firstWhere(
                  (a) => a?.id == selectedId,
                  orElse: () => null,
                );
            return Column(
              children: [
                SettingsTile(
                  icon: Icons.person_outline,
                  title: l10n.settingsCurrentAccount,
                  subtitle: currentAccount?.name ?? selectedId ?? l10n.settingsUnknown,
                  trailing: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: AppTheme.successColor.withValues(
                        alpha: 0.1,
                      ),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      l10n.settingsActive,
                      style: const TextStyle(
                        color: AppTheme.successColor,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  onTap: () {
                    if (currentAccount == null) return;
                    if (!context.mounted) return;
                    showModalBottomSheet(
                      context: context,
                      isScrollControlled: true,
                      backgroundColor: Colors.transparent,
                      builder: (context) => CurrentAccountSheet(account: currentAccount),
                    );
                  },
                ),
                const Divider(height: 1),
                SettingsTile(
                  icon: Icons.people_outline,
                  title: l10n.settingsAllAccounts,
                  subtitle: l10n.settingsAccountCount(accounts.length),
                  onTap: () {
                    showModalBottomSheet(
                      context: context,
                      isScrollControlled: true,
                      backgroundColor: Colors.transparent,
                      builder: (sheetContext) => AllAccountsSheet(
                        accounts: accounts,
                        selectedAccountId: ref
                            .read(authNotifierProvider.notifier)
                            .selectedAccountId,
                        onSelectAccount: (accountId) async {
                          final authNotifier = ref.read(authNotifierProvider.notifier);
                          await authNotifier.lockVault();
                          await authNotifier.selectAccount(accountId);
                          if (context.mounted) {
                            context.go(AppRoutes.login);
                          }
                        },
                      ),
                    );
                  },
                ),
                const Divider(height: 1),
                SettingsTile(
                  icon: Icons.storage_outlined,
                  title: l10n.settingsDataManagement,
                  subtitle: totalDataSize.isEmpty ? l10n.commonLoading : totalDataSize,
                  onTap: () => context.push(AppRoutes.dataManagement),
                ),
              ],
            );
          },
          loading: () => const Padding(
            padding: EdgeInsets.all(16),
            child: Center(child: CircularProgressIndicator()),
          ),
          error: (_, __) => SettingsTile(
            icon: Icons.error_outline,
            title: l10n.settingsErrorLoadingAccounts,
            subtitle: l10n.settingsPleaseRestart,
          ),
        ),
      ],
    ).animate().fadeIn(duration: 400.ms).slideX(begin: 0.05, end: 0);
  }
}
