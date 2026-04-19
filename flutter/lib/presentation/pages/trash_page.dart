import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/profile_provider.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/operation_notification.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/widgets/field_history_view.dart';

class TrashPage extends ConsumerStatefulWidget {
  const TrashPage({super.key});

  @override
  ConsumerState<TrashPage> createState() => _TrashPageState();
}

class _TrashPageState extends ConsumerState<TrashPage> {
  final _passwordController = TextEditingController();
  final _searchController = TextEditingController();
  bool _isLoading = false;
  bool _obscurePassword = true;
  String? _error;
  String _searchQuery = '';

  // Password field focus state
  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  @override
  void initState() {
    super.initState();
    // Load profile and field histories if not already loaded
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(profileNotifierProvider.notifier).loadProfile();
      ref.read(fieldHistoriesProvider.notifier).loadHistories();
    });
    _passwordFocusNode.addListener(_onPasswordFocusChange);
  }

  void _onPasswordFocusChange() {
    final hasFocus = _passwordFocusNode.hasFocus;
    if (hasFocus != _isPasswordFocused) {
      setState(() => _isPasswordFocused = hasFocus);
    }
  }

  @override
  void dispose() {
    _passwordController.dispose();
    _searchController.dispose();
    _passwordFocusNode.removeListener(_onPasswordFocusChange);
    _passwordFocusNode.dispose();
    super.dispose();
  }

  void _showPasswordHint(String hint) {
    final overlay = Overlay.of(context);
    late OverlayEntry entry;

    entry = OverlayEntry(
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
                    icon: const Icon(Icons.close, color: Colors.white70, size: 18),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    onPressed: () => entry.remove(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    overlay.insert(entry);
    Timer(const Duration(seconds: 4), () {
      if (entry.mounted) {
        entry.remove();
      }
    });
  }

  Future<void> _verifyPassword() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    // Use verifyPasswordForSensitiveData instead of unlockVault
    // to avoid changing auth state when vault is already unlocked
    final authNotifier = ref.read(authNotifierProvider.notifier);
    final success = await authNotifier.verifyPasswordForSensitiveData(_passwordController.text);

    if (success) {
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
      if (mounted) {
        _passwordController.clear();
        setState(() => _isLoading = false);
      }
    } else {
      if (mounted) {
        setState(() {
          _error = 'Invalid password';
          _isLoading = false;
        });
      }
      _passwordController.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final accessState = ref.watch(sensitivePageAccessProvider);

    if (!accessState.isVerified) {
      return _buildPasswordVerification();
    }
    return _buildTrashView();
  }

  Widget _buildPasswordVerification() {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Trash'),
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.lock_outline,
                size: 64,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(height: 24),
              Text(
                'Password Required',
                style: theme.textTheme.headlineSmall,
              ),
              const SizedBox(height: 8),
              Text(
                'Enter your master password to view the trash',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 32),
              SizedBox(
                width: 300,
                child: Builder(
                  builder: (ctx) {
                    final authNotifier = ref.read(authNotifierProvider.notifier);
                    final hint = authNotifier.selectedAccount?.passwordHint;
                    final hasError = _error != null;
                    final errorColor = Colors.red.shade700;
                    final normalColor = Theme.of(ctx).colorScheme.onSurfaceVariant;
                    return TextField(
                      controller: _passwordController,
                      obscureText: _obscurePassword,
                      focusNode: _passwordFocusNode,
                      autofocus: true,
                      onSubmitted: (_) => _verifyPassword(),
                      decoration: InputDecoration(
                        labelText: 'Master Password',
                        labelStyle: TextStyle(
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(ctx).colorScheme.onSurface,
                        ),
                        floatingLabelStyle: TextStyle(
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : Theme.of(ctx).colorScheme.onSurface,
                        ),
                        errorText: _error,
                        errorStyle: TextStyle(
                          color: errorColor,
                          fontWeight: FontWeight.w500,
                        ),
                        prefixIcon: Icon(
                          Icons.key,
                          color: hasError
                              ? errorColor
                              : _isPasswordFocused
                              ? AppTheme.primaryColor
                              : normalColor,
                        ),
                        suffixIcon: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            if (hint != null)
                              IconButton(
                                icon: Icon(
                                  Icons.help_outline,
                                  size: 20,
                                  color: hasError
                                      ? errorColor
                                      : _isPasswordFocused
                                      ? AppTheme.primaryColor
                                      : normalColor,
                                ),
                                onPressed: () => _showPasswordHint(hint),
                                tooltip: 'Show password hint',
                              ),
                            IconButton(
                              icon: Icon(
                                _obscurePassword
                                    ? Icons.visibility_outlined
                                    : Icons.visibility_off_outlined,
                                size: 20,
                                color: hasError
                                    ? errorColor
                                    : _isPasswordFocused
                                    ? AppTheme.primaryColor
                                    : normalColor,
                              ),
                              onPressed: () {
                                setState(() => _obscurePassword = !_obscurePassword);
                              },
                            ),
                          ],
                        ),
                        enabledBorder: AppTheme.passwordFieldEnabledBorder,
                        errorBorder: AppTheme.passwordFieldErrorBorder,
                        focusedErrorBorder: AppTheme.passwordFieldFocusedErrorBorder,
                      ),
                    );
                  },
                ),
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _isLoading ? null : _verifyPassword,
                child: _isLoading
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Verify'),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildTrashView() {
    final theme = Theme.of(context);
    final profile = ref.watch(profileNotifierProvider);

    if (profile == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Trash')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    final deletedItems = ProfileStorageService.instance.getDeletedItems(profile);

    // Filter items based on search query
    final filteredItems = _searchQuery.isEmpty
        ? deletedItems
        : deletedItems.where((item) {
            final query = _searchQuery.toLowerCase();
            return item.itemLabel.toLowerCase().contains(query) ||
                item.section.toLowerCase().contains(query) ||
                item.itemType.toLowerCase().contains(query);
          }).toList();

    return Scaffold(
      appBar: AppBar(
        title: const Text('Trash'),
        actions: [
          const HeaderActionButtons(),
          if (filteredItems.isNotEmpty)
            TextButton.icon(
              onPressed: () => _confirmEmptyTrash(context, filteredItems.length),
              icon: const Icon(Icons.delete_forever, color: AppTheme.errorColor),
              label: const Text(
                'Empty Trash',
                style: TextStyle(color: AppTheme.errorColor),
              ),
            ),
        ],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              controller: _searchController,
              onChanged: (value) => setState(() => _searchQuery = value),
              decoration: InputDecoration(
                hintText: 'Search trash...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchQuery.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          setState(() => _searchQuery = '');
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
                contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              ),
            ),
          ),

          // Results count
          if (_searchQuery.isNotEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Row(
                children: [
                  Text(
                    filteredItems.isNotEmpty
                        ? 'Found ${filteredItems.length} result(s)'
                        : 'No results found',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: filteredItems.isNotEmpty
                              ? Theme.of(context).colorScheme.onSurfaceVariant
                              : Colors.orange,
                            ),
                  ),
                  const Spacer(),
                  Text(
                    '${deletedItems.length} total items in trash',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                  ),
                ],
              ),
            ),

          const SizedBox(height: 8),

          // Trash info banner
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 16),
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Colors.orange.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Icons.warning_amber, color: Colors.orange.shade700),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Items in trash are permanently deleted after 30 days',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Colors.orange.shade700,
                        ),
                  ),
                ),
              ],
            ),
          ).animate().fadeIn(duration: 400.ms),

          const SizedBox(height: 16),

          // Items list
          Expanded(
            child: filteredItems.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          _searchQuery.isEmpty
                              ? Icons.delete_outline
                              : Icons.search_off,
                          size: 64,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          _searchQuery.isEmpty
                              ? 'Trash is empty'
                              : 'No matching items',
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          _searchQuery.isEmpty
                              ? 'Deleted items will appear here'
                              : 'Try adjusting your search',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.all(16),
                    itemCount: filteredItems.length,
                    separatorBuilder: (context, index) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final item = filteredItems[index];
                      final hasHistory = _itemHasHistory(item);
                      return _TrashItemCard(
                        item: item,
                        hasHistory: hasHistory,
                        onRestore: (item) => _confirmRestore(item),
                        onPurge: (item) => _confirmPurge(context, item),
                        onDetail: () => _showDetail(context, item),
                        onHistory: hasHistory ? () => _showHistoryForItem(context, item) : null,
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Future<void> _restoreItem(DeletedItemInfo item) async {
    await ref.read(profileNotifierProvider.notifier).restore(
          section: item.section,
          itemType: item.itemType,
          id: item.id,
        );

    if (mounted) {
      // Trigger rebuild to remove item from list immediately
      setState(() {});

      OperationNotification.show(
        context,
        message: OperationLogger.createNotification(
          section: _getLogSection(item.section),
          action: LogAction.restore,
          itemName: item.itemLabel,
        ),
        duration: const Duration(seconds: 3),
      );

      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('${item.itemLabel} restored'),
          behavior: SnackBarBehavior.floating,
          backgroundColor: Colors.blue,
        ),
      );
    }
  }

  Future<void> _confirmRestore(DeletedItemInfo item) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.restore, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            const Text('Confirm Restore'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to restore "${item.itemLabel}"?',
              style: Theme.of(ctx).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.blue.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.blue.shade200),
              ),
              child: Row(
                children: [
                  Icon(Icons.info_outline, color: Colors.blue.shade700, size: 20),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'The item will be moved back to its original location.',
                      style: TextStyle(
                        color: Colors.blue.shade900,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Restore'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await _restoreItem(item);
    }
  }

  Future<void> _confirmPurge(BuildContext context, DeletedItemInfo item) async {
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            const Text('Confirm Permanent Delete'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to permanently delete "${item.itemLabel}"?',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(Icons.info_outline, color: Colors.red.shade700, size: 20),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'This action cannot be undone. The item will be permanently removed.',
                      style: TextStyle(
                        color: Colors.red.shade900,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(
              backgroundColor: AppTheme.errorColor,
            ),
            child: const Text('Delete Forever'),
          ),
        ],
      ),
    ).then((confirmed) async {
      if (confirmed == true) {
        await _purgeItem(item);
      }
    });
  }

  Future<void> _purgeItem(DeletedItemInfo item) async {
    await ref.read(profileNotifierProvider.notifier).permanentDelete(
          section: item.section,
          itemType: item.itemType,
          id: item.id,
        );

    if (mounted) {
      // Trigger rebuild to remove item from list immediately
      setState(() {});
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('${item.itemLabel} permanently deleted'),
          behavior: SnackBarBehavior.floating,
          backgroundColor: AppTheme.errorColor,
        ),
      );
    }
  }

  void _confirmEmptyTrash(BuildContext context, int itemCount) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(Icons.warning_amber, color: Colors.orange.shade700),
            const SizedBox(width: 8),
            const Text('Empty Trash'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Are you sure you want to permanently delete all $itemCount items in trash?',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.red.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.shade200),
              ),
              child: Row(
                children: [
                  Icon(Icons.info_outline, color: Colors.red.shade700, size: 20),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'This action cannot be undone. All items will be permanently removed.',
                      style: TextStyle(
                        color: Colors.red.shade900,
                        fontSize: 13,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              final scaffoldMessenger = ScaffoldMessenger.of(context);
              Navigator.pop(context);
              await ref.read(profileNotifierProvider.notifier).emptyAllTrash();
              if (mounted) {
                // Trigger rebuild to remove all items immediately
                setState(() {});
                scaffoldMessenger.showSnackBar(
                  SnackBar(
                    content: Text('All $itemCount items permanently deleted'),
                    behavior: SnackBarBehavior.floating,
                    backgroundColor: AppTheme.errorColor,
                  ),
                );
              }
            },
            style: FilledButton.styleFrom(
              backgroundColor: AppTheme.errorColor,
            ),
            child: const Text('Empty Trash'),
          ),
        ],
      ),
    );
  }

  void _showDetail(BuildContext context, DeletedItemInfo item) {
    final theme = Theme.of(context);
    final profile = ref.read(profileNotifierProvider);
    if (profile == null) return;

    String detailText = '';
    DateTime? deletedAt;

    switch (item.section) {
      case 'travel':
        if (item.itemType == 'passport') {
          final idx = profile.travel?.passports.indexWhere((p) => p.id == item.id) ?? -1;
          if (idx >= 0) {
            final p = profile.travel!.passports[idx];
            detailText = 'Country: ${p.country ?? "N/A"}\n'
                'Number: ${p.number ?? "N/A"}\n'
                'Expiry: ${p.expiryDate ?? "N/A"}';
            deletedAt = p.deletedAt;
          }
        } else if (item.itemType == 'visa') {
          final idx = profile.travel?.visas.indexWhere((v) => v.id == item.id) ?? -1;
          if (idx >= 0) {
            final v = profile.travel!.visas[idx];
            detailText = 'Country: ${v.country ?? "N/A"}\n'
                'Type: ${v.visaType ?? "N/A"}\n'
                'Number: ${v.number ?? "N/A"}\n'
                'Expiry: ${v.expiryDate ?? "N/A"}';
            deletedAt = v.deletedAt;
          }
        } else if (item.itemType == 'travel_history') {
          final idx = profile.travel?.travelHistory.indexWhere((t) => t.id == item.id) ?? -1;
          if (idx >= 0) {
            final t = profile.travel!.travelHistory[idx];
            detailText = 'Destination: ${t.destination}\n'
                'Date: ${t.date ?? "N/A"}';
            deletedAt = t.deletedAt;
          }
        }
        break;
      case 'financial':
        if (item.itemType == 'bank_account') {
          final idx = profile.financial?.bankAccounts.indexWhere((b) => b.id == item.id) ?? -1;
          if (idx >= 0) {
            final b = profile.financial!.bankAccounts[idx];
            detailText = 'Bank: ${b.bankName ?? "N/A"}\n'
                'Account: ${b.accountNumber ?? "N/A"}\n'
                'Currency: ${b.currency ?? "N/A"}';
            deletedAt = b.deletedAt;
          }
        } else if (item.itemType == 'card') {
          final idx = profile.financial?.cards.indexWhere((c) => c.id == item.id) ?? -1;
          if (idx >= 0) {
            final c = profile.financial!.cards[idx];
            detailText = 'Type: ${c.cardType ?? "N/A"}\n'
                'Number: ${c.cardNumber ?? "N/A"}\n'
                'Expiry: ${c.expiryDate ?? "N/A"}';
            deletedAt = c.deletedAt;
          }
        }
        break;
      case 'professional':
        if (item.itemType == 'education') {
          final idx = profile.professional?.education.indexWhere((e) => e.id == item.id) ?? -1;
          if (idx >= 0) {
            final e = profile.professional!.education[idx];
            detailText = 'Institution: ${e.institution ?? "N/A"}\n'
                'Degree: ${e.degree ?? "N/A"}\n'
                'Field: ${e.field ?? "N/A"}';
            deletedAt = e.deletedAt;
          }
        } else if (item.itemType == 'employment') {
          final idx = profile.professional?.employment.indexWhere((emp) => emp.id == item.id) ?? -1;
          if (idx >= 0) {
            final emp = profile.professional!.employment[idx];
            detailText = 'Company: ${emp.company ?? "N/A"}\n'
                'Position: ${emp.position ?? "N/A"}\n'
                'Period: ${emp.startDate ?? "N/A"} - ${emp.endDate ?? "N/A"}';
            deletedAt = emp.deletedAt;
          }
        } else if (item.itemType == 'skill') {
          final idx = profile.professional?.skills.indexWhere((s) => s.id == item.id) ?? -1;
          if (idx >= 0) {
            final s = profile.professional!.skills[idx];
            detailText = 'Name: ${s.name}\n'
                'Level: ${s.level ?? "N/A"}';
            deletedAt = s.deletedAt;
          }
        } else if (item.itemType == 'language') {
          final idx = profile.professional?.languages.indexWhere((l) => l.id == item.id) ?? -1;
          if (idx >= 0) {
            final l = profile.professional!.languages[idx];
            detailText = 'Name: ${l.name}\n'
                'Proficiency: ${l.proficiency ?? "N/A"}';
            deletedAt = l.deletedAt;
          }
        }
        break;
      case 'profile':
        if (item.itemType == 'contact') {
          final idx = profile.identity?.contact?.entries.indexWhere((e) => e.id == item.id) ?? -1;
          if (idx >= 0) {
            final e = profile.identity!.contact!.entries[idx];
            detailText = 'Label: ${e.label}\n'
                'Type: ${e.type}\n'
                'Value: ${e.value}';
            deletedAt = e.deletedAt;
          }
        } else if (item.itemType == 'idCard') {
          final idx = profile.identity?.idCards?.indexWhere((c) => c.id == item.id) ?? -1;
          if (idx >= 0) {
            final c = profile.identity!.idCards![idx];
            detailText = 'Label: ${c.label ?? "N/A"}\n'
                'Number: ${c.number ?? "N/A"}\n'
                'Country: ${c.country ?? "N/A"}';
            deletedAt = c.deletedAt;
          }
        } else if (item.itemType == 'address') {
          final idx = profile.identity?.addresses?.indexWhere((a) => a.id == item.id) ?? -1;
          if (idx >= 0) {
            final a = profile.identity!.addresses![idx];
            detailText = 'Label: ${a.label ?? "N/A"}\n'
                'Street: ${a.street ?? "N/A"}\n'
                'City: ${a.city ?? "N/A"}\n'
                'Country: ${a.country ?? "N/A"}';
            deletedAt = a.deletedAt;
          }
        }
        break;
    }

    final deletedAtDate = deletedAt ?? item.deletedAt;
    final daysRemaining = 30 - DateTime.now().difference(deletedAtDate).inDays;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(_getSectionIcon(item.section), color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Expanded(child: Text(item.itemLabel)),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '${_getItemTypeLabel(item.itemType)} - ${_getSectionLabel(item.section)}',
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                          color: AppTheme.primaryColor,
                        ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    detailText.isNotEmpty ? detailText : 'No details available',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Icon(Icons.access_time, size: 16, color: Colors.grey.shade600),
                const SizedBox(width: 8),
                Text(
                  'Deleted: ${_formatDate(deletedAtDate)}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Colors.grey.shade600,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Row(
              children: [
                Icon(Icons.timer, size: 16, color: daysRemaining <= 7 ? Colors.orange : Colors.grey.shade600),
                const SizedBox(width: 8),
                Text(
                  '$daysRemaining days until permanent deletion',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: daysRemaining <= 7 ? Colors.orange : Colors.grey.shade600,
                        fontWeight: daysRemaining <= 7 ? FontWeight.w600 : FontWeight.normal,
                      ),
                ),
              ],
            ),
          ],
        ),
        actions: [
          OutlinedButton.icon(
            onPressed: () => _showHistoryForItem(context, item),
            icon: const Icon(Icons.history, size: 18),
            label: const Text('History'),
            style: OutlinedButton.styleFrom(
              foregroundColor: AppTheme.primaryColor,
              side: const BorderSide(color: AppTheme.primaryColor),
            ),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  bool _itemHasHistory(DeletedItemInfo item) {
    String? fieldIdPrefix;
    switch (item.itemType) {
      case 'contact':
        final profile = ref.read(profileNotifierProvider);
        if (profile?.identity?.contact != null) {
          final entries = profile!.identity!.contact!.entries;
          final contactIdx = entries.indexWhere((e) => e.id == item.id);
          if (contactIdx >= 0) {
            final contactType = entries[contactIdx].type;
            fieldIdPrefix = contactType == 'phone' ? 'contact.phone' : 'contact.email';
          }
        }
        break;
      case 'idCard':
        fieldIdPrefix = 'idCard.number';
        break;
      case 'address':
        fieldIdPrefix = 'address.postalCode';
        break;
      case 'passport':
        fieldIdPrefix = 'travel.passport';
        break;
      case 'visa':
        fieldIdPrefix = 'travel.visa';
        break;
      case 'bank_account':
        fieldIdPrefix = 'financial.bankAccount';
        break;
      case 'card':
        fieldIdPrefix = 'financial.card';
        break;
      case 'education':
        fieldIdPrefix = 'professional.education';
        break;
      case 'employment':
        fieldIdPrefix = 'professional.employment';
        break;
      case 'skill':
        fieldIdPrefix = 'professional.skill';
        break;
      case 'language':
        fieldIdPrefix = 'professional.language';
        break;
      default:
        fieldIdPrefix = null;
    }

    if (fieldIdPrefix == null) return false;

    final history = ref.read(fieldHistoriesProvider.notifier).getHistory(item.id, fieldIdPrefix);
    return history != null && history.entries.isNotEmpty;
  }

  LogSection _getLogSection(String section) {
    switch (section) {
      case 'travel':
        return LogSection.travel;
      case 'financial':
        return LogSection.financial;
      case 'professional':
        return LogSection.professional;
      case 'profile':
        return LogSection.identity;
      default:
        return LogSection.identity;
    }
  }

  IconData _getSectionIcon(String section) {
    switch (section) {
      case 'travel':
        return Icons.flight;
      case 'financial':
        return Icons.account_balance;
      case 'professional':
        return Icons.work;
      case 'profile':
        return Icons.person;
      default:
        return Icons.help_outline;
    }
  }

  String _getSectionLabel(String section) {
    switch (section) {
      case 'travel':
        return 'Travel';
      case 'financial':
        return 'Financial';
      case 'professional':
        return 'Professional';
      case 'profile':
        return 'Profile';
      default:
        return section;
    }
  }

  String _getItemTypeLabel(String itemType) {
    switch (itemType) {
      case 'passport':
        return 'Passport';
      case 'visa':
        return 'Visa';
      case 'travel_history':
        return 'Travel History';
      case 'bank_account':
        return 'Bank Account';
      case 'card':
        return 'Card';
      case 'education':
        return 'Education';
      case 'employment':
        return 'Employment';
      case 'skill':
        return 'Skill';
      case 'language':
        return 'Language';
      case 'contact':
        return 'Contact';
      case 'idCard':
        return 'ID Card';
      case 'address':
        return 'Address';
      default:
        return itemType;
    }
  }

  String _formatDate(DateTime date) {
    return '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';
  }

  void _showHistoryForItem(BuildContext context, DeletedItemInfo item) {
    // Determine the fieldId prefix based on item type
    String? fieldIdPrefix;
    switch (item.itemType) {
      case 'contact':
        // Contact can be email or phone - look up the actual type
        final profile = ref.read(profileNotifierProvider);
        if (profile?.identity?.contact != null) {
          final entries = profile!.identity!.contact!.entries;
          final contactIdx = entries.indexWhere((e) => e.id == item.id);
          if (contactIdx >= 0) {
            final contactType = entries[contactIdx].type;
            fieldIdPrefix = contactType == 'phone' ? 'contact.phone' : 'contact.email';
          }
        }
        break;
      case 'idCard':
        fieldIdPrefix = 'idCard.number';
        break;
      case 'address':
        fieldIdPrefix = 'address.postalCode';
        break;
      case 'passport':
        fieldIdPrefix = 'travel.passport';
        break;
      case 'visa':
        fieldIdPrefix = 'travel.visa';
        break;
      case 'bank_account':
        fieldIdPrefix = 'financial.bankAccount';
        break;
      case 'card':
        fieldIdPrefix = 'financial.card';
        break;
      case 'education':
        fieldIdPrefix = 'professional.education';
        break;
      case 'employment':
        fieldIdPrefix = 'professional.employment';
        break;
      case 'skill':
        fieldIdPrefix = 'professional.skill';
        break;
      case 'language':
        fieldIdPrefix = 'professional.language';
        break;
      default:
        fieldIdPrefix = null;
    }

    if (fieldIdPrefix == null) {
      showOverlaySnackBar(
        context,
        content: 'History not available for this item type',
        type: SnackBarType.info,
      );
      return;
    }

    final history = ref.read(fieldHistoriesProvider.notifier).getHistory(item.id, fieldIdPrefix);

    if (history == null || history.entries.isEmpty) {
      showOverlaySnackBar(
        context,
        content: 'No history available for this item',
        type: SnackBarType.info,
      );
      return;
    }

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            const Icon(Icons.history, color: AppTheme.primaryColor),
            const SizedBox(width: 8),
            Expanded(child: Text('${item.itemLabel} - History')),
          ],
        ),
        content: SizedBox(
          width: double.maxFinite,
          child: FieldHistoryView(
            fieldName: fieldIdPrefix!,
            history: history,
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}

class _TrashItemCard extends StatefulWidget {
  final DeletedItemInfo item;
  final bool hasHistory;
  final Future<void> Function(DeletedItemInfo item) onRestore;
  final Future<void> Function(DeletedItemInfo item) onPurge;
  final VoidCallback onDetail;
  final VoidCallback? onHistory;

  const _TrashItemCard({
    required this.item,
    required this.hasHistory,
    required this.onRestore,
    required this.onPurge,
    required this.onDetail,
    this.onHistory,
  });

  @override
  State<_TrashItemCard> createState() => _TrashItemCardState();
}

class _TrashItemCardState extends State<_TrashItemCard> {
  bool _isRestoring = false;
  bool _isPurging = false;

  bool get _isProcessing => _isRestoring || _isPurging;

  Future<void> _handleRestore() async {
    if (_isProcessing) return;

    setState(() {
      _isRestoring = true;
    });

    try {
      await widget.onRestore(widget.item);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to restore ${widget.item.itemLabel}'),
            behavior: SnackBarBehavior.floating,
            backgroundColor: Colors.orange,
          ),
        );
      }
    } finally {
      // Reset flag after operation completes (success or failure)
      if (mounted) {
        setState(() {
          _isRestoring = false;
        });
      }
    }
  }

  Future<void> _handlePurge() async {
    if (_isProcessing) return;

    setState(() {
      _isPurging = true;
    });

    try {
      // Await the entire purge flow: dialog confirmation + actual deletion
      await widget.onPurge(widget.item);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to purge ${widget.item.itemLabel}'),
            behavior: SnackBarBehavior.floating,
            backgroundColor: Colors.orange,
          ),
        );
      }
    } finally {
      // Reset flag after operation completes (success or failure)
      if (mounted) {
        setState(() {
          _isPurging = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final daysRemaining = 30 - DateTime.now().difference(widget.item.deletedAt).inDays;
    final isExpiringSoon = daysRemaining <= 7;

    return Card(
      child: InkWell(
        onTap: _isProcessing ? null : widget.onDetail,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: Colors.orange.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      _getSectionIcon(widget.item.section),
                      color: Colors.orange,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          widget.item.itemLabel,
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${_getItemTypeLabel(widget.item.itemType)} - ${_getSectionLabel(widget.item.section)}',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (isExpiringSoon)
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: Colors.orange.shade100,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Text(
                        '$daysRemaining days',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: Colors.orange.shade800,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Icon(
                    Icons.access_time,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    'Deleted ${_formatTimeAgo(widget.item.deletedAt)}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const Spacer(),
                  if (widget.hasHistory && widget.onHistory != null)
                    TextButton.icon(
                      onPressed: widget.onHistory,
                      icon: const Icon(Icons.history, size: 16),
                      label: const Text('History'),
                      style: TextButton.styleFrom(
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        minimumSize: Size.zero,
                        foregroundColor: AppTheme.primaryColor,
                      ),
                    ),
                  if (widget.hasHistory && widget.onHistory != null)
                    const SizedBox(width: 4),
                  TextButton.icon(
                    onPressed: _isRestoring ? null : _handleRestore,
                    icon: _isRestoring
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.restore, size: 16),
                    label: Text(_isRestoring ? 'Restoring...' : 'Restore'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                    ),
                  ),
                  const SizedBox(width: 4),
                  TextButton.icon(
                    onPressed: _isPurging ? null : _handlePurge,
                    icon: _isPurging
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.delete_forever, size: 16),
                    label: Text(_isPurging ? 'Purging...' : 'Purge'),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: Size.zero,
                      foregroundColor: AppTheme.errorColor,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    ).animate().fadeIn(duration: 300.ms);
  }

  IconData _getSectionIcon(String section) {
    switch (section) {
      case 'travel':
        return Icons.flight;
      case 'financial':
        return Icons.account_balance;
      case 'professional':
        return Icons.work;
      case 'profile':
        return Icons.person;
      default:
        return Icons.help_outline;
    }
  }

  String _getSectionLabel(String section) {
    switch (section) {
      case 'travel':
        return 'Travel';
      case 'financial':
        return 'Financial';
      case 'professional':
        return 'Professional';
      case 'profile':
        return 'Profile';
      default:
        return section;
    }
  }

  String _getItemTypeLabel(String itemType) {
    switch (itemType) {
      case 'passport':
        return 'Passport';
      case 'visa':
        return 'Visa';
      case 'travel_history':
        return 'Travel History';
      case 'bank_account':
        return 'Bank Account';
      case 'card':
        return 'Card';
      case 'education':
        return 'Education';
      case 'employment':
        return 'Employment';
      case 'skill':
        return 'Skill';
      case 'language':
        return 'Language';
      case 'contact':
        return 'Contact';
      case 'idCard':
        return 'ID Card';
      case 'address':
        return 'Address';
      default:
        return itemType;
    }
  }

  String _formatTimeAgo(DateTime date) {
    final diff = DateTime.now().difference(date);
    if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'Just now';
    }
  }
}
