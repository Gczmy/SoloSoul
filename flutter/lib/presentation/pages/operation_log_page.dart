import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/operation_log_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_filter_chip.dart';
import 'package:solosoul_flutter/presentation/widgets/operation_tile.dart';

class OperationLogPage extends ConsumerStatefulWidget {
  const OperationLogPage({super.key});

  @override
  ConsumerState<OperationLogPage> createState() => _OperationLogPageState();
}

class _OperationLogPageState extends ConsumerState<OperationLogPage> {
  final _passwordController = TextEditingController();
  bool _isLoading = false;
  bool _obscurePassword = true;
  bool _filterExpanded = false;
  String? _error;

  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  @override
  void initState() {
    super.initState();
    _refreshLogs();
    _passwordFocusNode.addListener(_onPasswordFocusChange);
  }

  void _onPasswordFocusChange() {
    final hasFocus = _passwordFocusNode.hasFocus;
    if (hasFocus != _isPasswordFocused) {
      setState(() => _isPasswordFocused = hasFocus);
    }
  }

  Future<void> _refreshLogs() async {
    await OperationLogService.instance.refreshFromDisk();
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _passwordController.dispose();
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

    final authNotifier = ref.read(authNotifierProvider.notifier);
    final success = await authNotifier.verifyPasswordForSensitiveData(_passwordController.text);

    if (success) {
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    } else {
      setState(() => _error = 'Invalid password');
    }
    _passwordController.clear();
    setState(() => _isLoading = false);
  }

  @override
  Widget build(BuildContext context) {
    if (!ref.watch(isSensitiveAccessGrantedProvider)) {
      return _buildPasswordVerification();
    }
    return _buildLogView();
  }

  Widget _buildPasswordVerification() {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(title: const Text('Operation Log')),
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
              Text('Password Required', style: theme.textTheme.headlineSmall),
              const SizedBox(height: 8),
              Text(
                'Enter your master password to view the operation log',
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
                        enabledBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.grey.shade400),
                        ),
                        errorBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.red.shade300),
                        ),
                        focusedErrorBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: Colors.red.shade500, width: 2),
                        ),
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

  Widget _buildLogView() {
    final theme = Theme.of(context);
    final entries = ref.watch(operationLogFilteredEntriesProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Operation Log'),
        actions: [
          const HeaderActionButtons(),
          if (entries.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: () => _confirmClearLog(context),
              tooltip: 'Clear log',
            ),
        ],
      ),
      body: Column(
        children: [
          _buildFilterHeader(),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 300),
            child: _filterExpanded ? _buildFilterSection() : const SizedBox.shrink(),
          ),
          Expanded(
            child: entries.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          Icons.filter_list_off,
                          size: 64,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No matching entries',
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Try adjusting your filters',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.all(16),
                    itemCount: entries.length,
                    separatorBuilder: (_, a) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final entry = entries[index];
                      return OperationTile(entry: entry);
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildFilterHeader() {
    final theme = Theme.of(context);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);
    final sensitivityFilters = ref.watch(logSensitivityFilterProvider);
    final hasActiveFilters =
        actionFilters.isNotEmpty || deviceFilters.isNotEmpty || sensitivityFilters.isNotEmpty;

    return InkWell(
      onTap: () => setState(() => _filterExpanded = !_filterExpanded),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
          border: Border(
            bottom: BorderSide(color: theme.colorScheme.outlineVariant),
          ),
        ),
        child: Row(
          children: [
            Icon(
              Icons.filter_list,
              size: 20,
              color: hasActiveFilters
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Text(
              'Filters',
              style: theme.textTheme.titleSmall?.copyWith(
                color: hasActiveFilters
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (hasActiveFilters) ...[
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  '${actionFilters.length + deviceFilters.length + sensitivityFilters.length}',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.primary,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
            const Spacer(),
            AnimatedRotation(
              turns: _filterExpanded ? 0.5 : 0,
              duration: const Duration(milliseconds: 300),
              child: Icon(
                Icons.keyboard_arrow_down,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFilterSection() {
    final theme = Theme.of(context);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);
    final sensitivityFilters = ref.watch(logSensitivityFilterProvider);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
        border: Border(
          bottom: BorderSide(color: theme.colorScheme.outlineVariant),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                'Action:',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      OperationFilterChip(
                        label: 'Create',
                        icon: Icons.add_circle_outline,
                        isSelected: actionFilters.contains('create'),
                        color: AppTheme.successColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'create', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Update',
                        icon: Icons.edit_outlined,
                        isSelected: actionFilters.contains('update'),
                        color: AppTheme.primaryColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'update', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Delete',
                        icon: Icons.delete_outline,
                        isSelected: actionFilters.contains('delete'),
                        color: Colors.orange.shade700,
                        onSelected: (_) => _toggleFilter(actionFilters, 'delete', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Restore',
                        icon: Icons.restore,
                        isSelected: actionFilters.contains('restore'),
                        color: Colors.blue,
                        onSelected: (_) => _toggleFilter(actionFilters, 'restore', logActionFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Purge',
                        icon: Icons.delete_forever,
                        isSelected: actionFilters.contains('purge'),
                        color: AppTheme.errorColor,
                        onSelected: (_) => _toggleFilter(actionFilters, 'purge', logActionFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Text(
                'Device:',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      OperationFilterChip(
                        label: 'macOS',
                        icon: Icons.laptop_mac,
                        isSelected: deviceFilters.contains('macos'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'macos', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'iOS',
                        icon: Icons.phone_iphone,
                        isSelected: deviceFilters.contains('ios'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'ios', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Android',
                        icon: Icons.phone_android,
                        isSelected: deviceFilters.contains('android'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'android', logDeviceFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Web',
                        icon: Icons.web,
                        isSelected: deviceFilters.contains('web'),
                        color: Colors.grey.shade700,
                        onSelected: (_) => _toggleFilter(deviceFilters, 'web', logDeviceFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Text(
                'Privacy:',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      OperationFilterChip(
                        label: 'Critical',
                        icon: Icons.lock,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.critical),
                        color: Colors.red,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.critical, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Sensitive',
                        icon: Icons.visibility_off,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.sensitive),
                        color: Colors.orange,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.sensitive, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Internal',
                        icon: Icons.folder,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.internal),
                        color: Colors.green,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.internal, logSensitivityFilterProvider),
                      ),
                      const SizedBox(width: 4),
                      OperationFilterChip(
                        label: 'Public',
                        icon: Icons.public,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.public),
                        color: Colors.blue,
                        onSelected: (_) => _toggleFilter(sensitivityFilters, SensitivityLevel.public, logSensitivityFilterProvider),
                      ),
                    ],
                  ),
                ),
              ),
              if (actionFilters.isNotEmpty || deviceFilters.isNotEmpty || sensitivityFilters.isNotEmpty)
                TextButton.icon(
                  onPressed: _clearAllFilters,
                  icon: const Icon(Icons.clear_all, size: 16),
                  label: const Text('Clear'),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    minimumSize: Size.zero,
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }

  void _toggleFilter<T>(
    Set<T> currentFilters,
    T value,
    StateProvider<Set<T>> provider,
  ) {
    final newFilters = Set<T>.from(currentFilters);
    if (newFilters.contains(value)) {
      newFilters.remove(value);
    } else {
      newFilters.add(value);
    }
    ref.read(provider.notifier).state = newFilters;
  }

  void _clearAllFilters() {
    ref.read(logActionFilterProvider.notifier).state = {};
    ref.read(logDeviceFilterProvider.notifier).state = {};
    ref.read(logSensitivityFilterProvider.notifier).state = {};
  }

  void _confirmClearLog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Log'),
        content: const Text(
          'Are you sure you want to clear all operation history?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              OperationLogService.instance.clearEntries();
              Navigator.pop(context);
              setState(() {});
            },
            child: const Text('Clear', style: TextStyle(color: AppTheme.errorColor)),
          ),
        ],
      ),
    );
  }
}
