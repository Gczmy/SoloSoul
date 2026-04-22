import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/native_crypto_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/header_action_buttons.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';

// Section identifiers for operation logs
enum LogSection {
  identity('identity'),
  contactInformation('contact information'),
  address('address'),
  idCard('ID card'),
  passport('passport'),
  visa('visa'),
  travelHistory('travel history'),
  bankAccount('bank account'),
  card('card'),
  education('education'),
  employment('employment'),
  skill('skill'),
  language('language'),
  travel('travel'),
  financial('financial'),
  professional('professional'),
  sensitivitySettings('sensitivity settings');

  final String value;
  const LogSection(this.value);
}

// Action types for operation logs
enum LogAction {
  create('create'),
  update('update'),
  delete('delete'),
  restore('restore'),
  purge('purge');

  final String value;
  const LogAction(this.value);
}

// Device/Platform types for operation logs
enum LogDevice {
  macos('macOS'),
  ios('iOS'),
  android('Android'),
  windows('Windows'),
  linux('Linux'),
  web('Web'),
  unknown('Unknown');

  final String value;
  const LogDevice(this.value);

  static LogDevice get current {
    try {
      return Platform.operatingSystem.toLowerCase() as LogDevice;
    } catch (_) {
      return LogDevice.unknown;
    }
  }

  static LogDevice fromString(String value) {
    switch (value.toLowerCase()) {
      case 'macos':
        return LogDevice.macos;
      case 'ios':
        return LogDevice.ios;
      case 'android':
        return LogDevice.android;
      case 'windows':
        return LogDevice.windows;
      case 'linux':
        return LogDevice.linux;
      case 'web':
        return LogDevice.web;
      default:
        return LogDevice.unknown;
    }
  }
}

/// Operation log entry model
/// NOTE: The description field should NOT contain sensitive plain text.
/// Example: Use "Modified password field" not "Changed password to 123456"
class OperationEntry {
  final DateTime timestamp;
  final String action; // 'create', 'update', 'delete'
  final String section; // 'identity', 'travel', 'financial', 'professional'
  final String description;
  final String? fieldPath; // Optional field path for more details
  final String device; // Platform: 'macos', 'ios', 'android', etc.
  final SensitivityLevel sensitivityLevel;

  const OperationEntry({
    required this.timestamp,
    required this.action,
    required this.section,
    required this.description,
    this.fieldPath,
    this.device = 'unknown',
    this.sensitivityLevel = SensitivityLevel.public,
  });

  factory OperationEntry.fromJson(Map<String, dynamic> json) {
    return OperationEntry(
      timestamp: DateTime.parse(json['timestamp'] as String),
      action: json['action'] as String,
      section: json['section'] as String,
      description: json['description'] as String,
      fieldPath: json['fieldPath'] as String?,
      device: json['device'] as String? ?? 'unknown',
      sensitivityLevel: SensitivityLevel.values.firstWhere(
        (e) => e.name == json['sensitivityLevel'],
        orElse: () => SensitivityLevel.public,
      ),
    );
  }

  Map<String, dynamic> toJson() => {
    'timestamp': timestamp.toIso8601String(),
    'action': action,
    'section': section,
    'description': description,
    if (fieldPath != null) 'fieldPath': fieldPath,
    'device': device,
    'sensitivityLevel': sensitivityLevel.name,
  };
}

/// Operation log storage service with ENCRYPTED persistence
/// Logs are stored encrypted in the vault, matching profile encryption
/// TTL: 90 days or 500 records max (whichever comes first)
class OperationLogService extends ChangeNotifier {
  static OperationLogService? _instance;
  static OperationLogService get instance =>
      _instance ??= OperationLogService._();

  OperationLogService._();

  final List<OperationEntry> _entries = [];
  bool _initialized = false;
  String? _currentAccountId;
  Uint8List? _encryptionKey;

  /// Future to track pending save operations to prevent race conditions
  Future<void>? _pendingSave;

  // TTL constants
  static const int _maxEntries = 500;
  static const int _ttlDays = 90;

  /// Set encryption key (called after vault unlock)
  void setEncryptionKey(Uint8List key) {
    _encryptionKey = key;
  }

  /// Initialize with account ID and load persisted (encrypted) logs
  Future<void> initializeForAccount(String accountId) async {
    // If already initialized for this account and entries are in memory, skip
    // But if entries were cleared (e.g., by clearMemoryCache), reload from disk
    if (_currentAccountId == accountId && _initialized && _entries.isNotEmpty) {
      return;
    }
    _currentAccountId = accountId;
    await _loadFromDisk();
    _initialized = true;
  }

  /// Clear logs and encryption key when account is locked
  /// Waits for any pending save to complete first to prevent data loss
  Future<void> clearForCurrentAccount() async {
    // Wait for any pending save to complete before clearing
    if (_pendingSave != null) {
      await _pendingSave;
    }
    _entries.clear();
    _initialized = false;
    _currentAccountId = null;
    _encryptionKey = null;
    _pendingSave = null;
  }

  Future<Directory> get _storageDir async {
    // Place logs in the same directory as profiles (within vault)
    final dir = await ProfileStorageService.instance.storageDir;
    return Directory('${dir.path}/logs');
  }

  Future<String> get _logFilePath async {
    if (_currentAccountId == null) return '';
    final dir = await _storageDir;
    return '${dir.path}/logs_$_currentAccountId.enc';
  }

  Future<File> get _logFile async {
    return File(await _logFilePath);
  }

  Future<void> _ensureDirExists() async {
    final dir = await _storageDir;
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
  }

  /// Load and decrypt logs from disk
  Future<void> _loadFromDisk() async {
    if (_encryptionKey == null) return;

    try {
      await _ensureDirExists();
      final logFile = await _logFile;
      if (!await logFile.exists()) {
        _entries.clear();
        return;
      }

      final encryptedContent = await logFile.readAsString();
      if (encryptedContent.isEmpty) {
        _entries.clear();
        return;
      }

      final decrypted = _decrypt(encryptedContent);
      if (decrypted == null) {
        _entries.clear();
        return;
      }

      final List<dynamic> jsonList = jsonDecode(decrypted);
      _entries.clear();
      _entries.addAll(
        jsonList
            .map((e) => OperationEntry.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

      // Sort by timestamp descending (newest first)
      _entries.sort((a, b) => b.timestamp.compareTo(a.timestamp));

      // Apply TTL cleanup
      _applyTTL();
    } catch (_) {
      _entries.clear();
    }
  }

  /// Encrypt and save logs to disk
  /// Waits for any pending save to complete before starting new save
  Future<void> _saveToDisk() async {
    // Wait for any pending save operation to complete first
    // This ensures saves happen in order and don't overwrite each other
    if (_pendingSave != null) {
      await _pendingSave;
    }

    if (_encryptionKey == null) return;

    _pendingSave = _doSave();
    await _pendingSave;
    _pendingSave =
        null; // Reset after completion to allow next save to proceed immediately
  }

  /// Actual save implementation
  Future<void> _doSave() async {
    try {
      await _ensureDirExists();
      final jsonList = _entries.map((e) => e.toJson()).toList();
      final jsonString = jsonEncode(jsonList);

      final encrypted = _encrypt(jsonString);
      if (encrypted != null) {
        final logFile = await _logFile;
        await logFile.writeAsString(encrypted);
      }
    } catch (_) {
      // Silently fail on save errors to not disrupt user workflow
    }
  }

  /// Encrypt using AES-256-GCM (same as ProfileStorageService)
  String? _encrypt(String plaintext) {
    if (_encryptionKey == null) return null;

    try {
      final salt = NativeCryptoService.instance.generateSalt();
      if (salt == null) return null;

      // Use first 12 bytes of 32-byte salt as nonce
      final nonce = Uint8List.fromList(salt.sublist(0, 12));

      final encrypted = NativeCryptoService.instance.encrypt(
        data: Uint8List.fromList(utf8.encode(plaintext)),
        key: _encryptionKey!,
        nonce: nonce,
      );
      if (encrypted == null) return null;

      // Combine nonce + ciphertext and encode as base64
      final combined = Uint8List(12 + encrypted.length);
      combined.setRange(0, 12, nonce);
      combined.setRange(12, combined.length, encrypted);

      return base64Encode(combined);
    } catch (_) {
      return null;
    }
  }

  /// Decrypt using AES-256-GCM (same as ProfileStorageService)
  String? _decrypt(String ciphertext) {
    if (_encryptionKey == null) return null;

    try {
      final combined = base64Decode(ciphertext);
      if (combined.length < 13) return null;

      final nonce = combined.sublist(0, 12);
      final encryptedData = combined.sublist(12);

      final decrypted = NativeCryptoService.instance.decrypt(
        encrypted: encryptedData,
        key: _encryptionKey!,
        nonce: Uint8List.fromList(nonce),
      );

      if (decrypted == null) return null;

      return utf8.decode(decrypted);
    } catch (_) {
      return null;
    }
  }

  /// Apply TTL: remove entries older than 90 days and keep max 500 entries
  void _applyTTL() {
    final now = DateTime.now();
    final cutoffDate = now.subtract(const Duration(days: _ttlDays));

    _entries.removeWhere((entry) => entry.timestamp.isBefore(cutoffDate));

    // Also enforce max entries limit
    if (_entries.length > _maxEntries) {
      _entries.removeRange(_maxEntries, _entries.length);
    }
  }

  /// Flush pending saves to disk and wait for completion
  /// Call this after batch operations to ensure all entries are persisted
  Future<void> flush() async {
    await _saveToDisk();
  }

  void addEntry(OperationEntry entry) {
    // Automatically capture current device platform
    final devicePlatform = _getCurrentDevice();
    final entryWithDevice = devicePlatform != entry.device
        ? OperationEntry(
            timestamp: entry.timestamp,
            action: entry.action,
            section: entry.section,
            description: entry.description,
            fieldPath: entry.fieldPath,
            device: devicePlatform,
            sensitivityLevel: entry.sensitivityLevel,
          )
        : entry;

    _entries.insert(0, entryWithDevice); // Most recent first
    _applyTTL(); // Ensure TTL limits before saving
    _saveToDisk();
    notifyListeners(); // Notify Riverpod providers to refresh
  }

  String _getCurrentDevice() {
    try {
      return Platform.operatingSystem.toLowerCase();
    } catch (_) {
      return 'unknown';
    }
  }

  List<OperationEntry> getEntries() => List.unmodifiable(_entries);

  /// Filter entries by action, device, and sensitivity level
  List<OperationEntry> getFilteredEntries({
    Set<String>? actionFilters,
    Set<String>? deviceFilters,
    Set<SensitivityLevel>? sensitivityFilters,
  }) {
    return _entries.where((entry) {
      // Action filter
      if (actionFilters != null && actionFilters.isNotEmpty) {
        if (!actionFilters.contains(entry.action)) return false;
      }
      // Device filter
      if (deviceFilters != null && deviceFilters.isNotEmpty) {
        if (!deviceFilters.contains(entry.device)) return false;
      }
      // Sensitivity filter
      if (sensitivityFilters != null && sensitivityFilters.isNotEmpty) {
        if (!sensitivityFilters.contains(entry.sensitivityLevel)) return false;
      }
      return true;
    }).toList();
  }

  /// Reload logs from disk (for explicit refresh)
  Future<void> refreshFromDisk() async {
    if (_currentAccountId != null) {
      await _loadFromDisk();
      notifyListeners(); // Notify after reload so UI rebuilds
    }
  }

  void clearEntries() {
    _entries.clear();
    _saveToDisk();
    notifyListeners(); // Notify so UI rebuilds
  }

  /// Clear only in-memory cache (burn after reading)
  /// Keeps the encrypted disk storage intact
  void clearMemoryCache() {
    _entries.clear();
  }
}

// Provider for operation log - uses ChangeNotifierProvider to react to addEntry calls
final operationLogProvider = ChangeNotifierProvider<OperationLogService>((ref) {
  return OperationLogService.instance;
});

// Derived provider that returns the entries list and rebuilds when service notifies
final operationLogEntriesProvider = Provider<List<OperationEntry>>((ref) {
  ref.watch(
    operationLogProvider,
  ); // depend on the service so we rebuild when notified
  return OperationLogService.instance.getEntries();
});

// Filter state providers
final logActionFilterProvider = StateProvider<Set<String>>((ref) => {});
final logDeviceFilterProvider = StateProvider<Set<String>>((ref) => {});
final logSensitivityFilterProvider = StateProvider<Set<SensitivityLevel>>((ref) => {});

// Filtered entries provider
final operationLogFilteredEntriesProvider = Provider<List<OperationEntry>>((
  ref,
) {
  ref.watch(operationLogProvider);
  final actionFilters = ref.watch(logActionFilterProvider);
  final deviceFilters = ref.watch(logDeviceFilterProvider);
  final sensitivityFilters = ref.watch(logSensitivityFilterProvider);

  return OperationLogService.instance.getFilteredEntries(
    actionFilters: actionFilters.isEmpty ? null : actionFilters,
    deviceFilters: deviceFilters.isEmpty ? null : deviceFilters,
    sensitivityFilters: sensitivityFilters.isEmpty ? null : sensitivityFilters,
  );
});

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

  // Password field focus state
  final _passwordFocusNode = FocusNode();
  bool _isPasswordFocused = false;

  @override
  void initState() {
    super.initState();
    // Refresh logs from disk when page is shown
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
    // Note: We do NOT clear the in-memory cache here like "burn after reading"
    // because it causes data loss. Entries are kept in memory and properly
    // cleared when vault is locked via clearForCurrentAccount().
    super.dispose();
  }

  void _showPasswordHint(String hint) {
    // Use Overlay so the timer persists across navigation
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
      // Mark as verified in shared sensitive page access
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    } else {
      setState(() => _error = 'Invalid password');
    }
    _passwordController.clear();
    setState(() => _isLoading = false);
  }

  @override
  Widget build(BuildContext context) {
    // If not verified, show password verification
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
          // Filter section header with toggle button
          _buildFilterHeader(),
          // Collapsible filter content with animation
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 300),
            child: _filterExpanded ? _buildFilterSection() : const SizedBox.shrink(),
          ),
          // Entry list
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
                    separatorBuilder: (_, _a) => const SizedBox(height: 8),
                    itemBuilder: (context, index) {
                      final entry = entries[index];
                      return _OperationTile(entry: entry);
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
          // Action type filters
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
                      _FilterChip(
                        label: 'Create',
                        icon: Icons.add_circle_outline,
                        isSelected: actionFilters.contains('create'),
                        color: AppTheme.successColor,
                        onSelected: (selected) {
                          _toggleFilter(
                            actionFilters,
                            'create',
                            logActionFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Update',
                        icon: Icons.edit_outlined,
                        isSelected: actionFilters.contains('update'),
                        color: AppTheme.primaryColor,
                        onSelected: (selected) {
                          _toggleFilter(
                            actionFilters,
                            'update',
                            logActionFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Delete',
                        icon: Icons.delete_outline,
                        isSelected: actionFilters.contains('delete'),
                        color: Colors.orange.shade700,
                        onSelected: (selected) {
                          _toggleFilter(
                            actionFilters,
                            'delete',
                            logActionFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Restore',
                        icon: Icons.restore,
                        isSelected: actionFilters.contains('restore'),
                        color: Colors.blue,
                        onSelected: (selected) {
                          _toggleFilter(
                            actionFilters,
                            'restore',
                            logActionFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Purge',
                        icon: Icons.delete_forever,
                        isSelected: actionFilters.contains('purge'),
                        color: AppTheme.errorColor,
                        onSelected: (selected) {
                          _toggleFilter(
                            actionFilters,
                            'purge',
                            logActionFilterProvider,
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Device filters
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
                      _FilterChip(
                        label: 'macOS',
                        icon: Icons.laptop_mac,
                        isSelected: deviceFilters.contains('macos'),
                        color: Colors.grey.shade700,
                        onSelected: (selected) {
                          _toggleFilter(
                            deviceFilters,
                            'macos',
                            logDeviceFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'iOS',
                        icon: Icons.phone_iphone,
                        isSelected: deviceFilters.contains('ios'),
                        color: Colors.grey.shade700,
                        onSelected: (selected) {
                          _toggleFilter(
                            deviceFilters,
                            'ios',
                            logDeviceFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Android',
                        icon: Icons.phone_android,
                        isSelected: deviceFilters.contains('android'),
                        color: Colors.grey.shade700,
                        onSelected: (selected) {
                          _toggleFilter(
                            deviceFilters,
                            'android',
                            logDeviceFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Web',
                        icon: Icons.web,
                        isSelected: deviceFilters.contains('web'),
                        color: Colors.grey.shade700,
                        onSelected: (selected) {
                          _toggleFilter(
                            deviceFilters,
                            'web',
                            logDeviceFilterProvider,
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Sensitivity filters
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
                      _FilterChip(
                        label: 'Critical',
                        icon: Icons.lock,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.critical),
                        color: Colors.red,
                        onSelected: (selected) {
                          _toggleFilter(
                            sensitivityFilters,
                            SensitivityLevel.critical,
                            logSensitivityFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Sensitive',
                        icon: Icons.visibility_off,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.sensitive),
                        color: Colors.orange,
                        onSelected: (selected) {
                          _toggleFilter(
                            sensitivityFilters,
                            SensitivityLevel.sensitive,
                            logSensitivityFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Internal',
                        icon: Icons.folder,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.internal),
                        color: Colors.green,
                        onSelected: (selected) {
                          _toggleFilter(
                            sensitivityFilters,
                            SensitivityLevel.internal,
                            logSensitivityFilterProvider,
                          );
                        },
                      ),
                      const SizedBox(width: 4),
                      _FilterChip(
                        label: 'Public',
                        icon: Icons.public,
                        isSelected: sensitivityFilters.contains(SensitivityLevel.public),
                        color: Colors.blue,
                        onSelected: (selected) {
                          _toggleFilter(
                            sensitivityFilters,
                            SensitivityLevel.public,
                            logSensitivityFilterProvider,
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ),
              // Clear all filters button
              if (actionFilters.isNotEmpty ||
                  deviceFilters.isNotEmpty ||
                  sensitivityFilters.isNotEmpty)
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
              setState(() {}); // Refresh view
            },
            child: Text('Clear', style: TextStyle(color: AppTheme.errorColor)),
          ),
        ],
      ),
    );
  }
}

class _FilterChip extends StatelessWidget {
  final String label;
  final IconData icon;
  final bool isSelected;
  final Color color;
  final ValueChanged<bool> onSelected;

  const _FilterChip({
    required this.label,
    required this.icon,
    required this.isSelected,
    required this.color,
    required this.onSelected,
  });

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      label: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: isSelected ? Colors.white : color),
          const SizedBox(width: 4),
          Text(label),
        ],
      ),
      selected: isSelected,
      onSelected: onSelected,
      backgroundColor: color.withValues(alpha: 0.1),
      selectedColor: color,
      checkmarkColor: Colors.white,
      labelStyle: TextStyle(
        color: isSelected ? Colors.white : color,
        fontSize: 12,
        fontWeight: FontWeight.w500,
      ),
      padding: const EdgeInsets.symmetric(horizontal: 4),
      visualDensity: VisualDensity.compact,
      side: BorderSide(color: color.withValues(alpha: 0.3)),
    );
  }
}

class _OperationTile extends StatelessWidget {
  final OperationEntry entry;

  const _OperationTile({required this.entry});

  void _showDetailDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Row(
          children: [
            Icon(_actionIcon, color: _actionColor(context)),
            const SizedBox(width: 8),
            Expanded(child: Text('Operation Details')),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _DetailRow(label: 'Timestamp', value: _formatFullTimestamp(entry.timestamp)),
            const SizedBox(height: 12),
            _DetailRow(label: 'Action', value: _actionLabel),
            const SizedBox(height: 12),
            _DetailRow(label: 'Section', value: entry.section.toUpperCase()),
            if (entry.fieldPath != null) ...[
              const SizedBox(height: 12),
              _DetailRow(label: 'Field Path', value: entry.fieldPath!),
            ],
            const SizedBox(height: 12),
            _DetailRow(label: 'Description', value: entry.description),
            const SizedBox(height: 12),
            _DetailRow(label: 'Device', value: _getDeviceLabel(entry.device)),
            const SizedBox(height: 12),
            _DetailRow(
              label: 'Sensitivity Level',
              value: entry.sensitivityLevel.label,
              valueColor: _sensitivityColor(entry.sensitivityLevel),
            ),
          ],
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

  String _formatFullTimestamp(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
  }

  IconData get _actionIcon {
    switch (entry.action) {
      case 'create':
        return Icons.add_circle_outline;
      case 'update':
        return Icons.edit_outlined;
      case 'delete':
        return Icons.delete_outline;
      case 'restore':
        return Icons.restore;
      case 'purge':
        return Icons.delete_forever;
      default:
        return Icons.info_outline;
    }
  }

  Color _actionColor(BuildContext context) {
    switch (entry.action) {
      case 'create':
        return AppTheme.successColor;
      case 'update':
        return AppTheme.primaryColor;
      case 'delete':
        return Colors.orange.shade700;
      case 'restore':
        return Colors.blue;
      case 'purge':
        return AppTheme.errorColor;
      default:
        return Theme.of(context).colorScheme.onSurfaceVariant;
    }
  }

  String get _actionLabel {
    switch (entry.action) {
      case 'create':
        return 'Created';
      case 'update':
        return 'Updated';
      case 'delete':
        return 'Deleted';
      case 'restore':
        return 'Restored';
      case 'purge':
        return 'Purged';
      default:
        return entry.action;
    }
  }

  IconData _deviceIcon(String device) {
    switch (device.toLowerCase()) {
      case 'macos':
        return Icons.laptop_mac;
      case 'ios':
        return Icons.phone_iphone;
      case 'android':
        return Icons.phone_android;
      case 'windows':
        return Icons.desktop_windows;
      case 'linux':
        return Icons.computer;
      case 'web':
        return Icons.web;
      default:
        return Icons.devices;
    }
  }

  Color _sensitivityColor(SensitivityLevel level) {
    switch (level) {
      case SensitivityLevel.critical:
        return Colors.red;
      case SensitivityLevel.sensitive:
        return Colors.orange;
      case SensitivityLevel.internal:
        return Colors.green;
      case SensitivityLevel.public:
        return Colors.blue;
    }
  }

  IconData _sensitivityIcon(SensitivityLevel level) {
    switch (level) {
      case SensitivityLevel.critical:
        return Icons.lock;
      case SensitivityLevel.sensitive:
        return Icons.visibility_off;
      case SensitivityLevel.internal:
        return Icons.folder;
      case SensitivityLevel.public:
        return Icons.public;
    }
  }

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';
    return '${dt.day}/${dt.month}/${dt.year}';
  }

  String _getDeviceLabel(String device) {
    switch (device.toLowerCase()) {
      case 'macos':
        return 'macOS';
      case 'ios':
        return 'iOS';
      case 'android':
        return 'Android';
      case 'windows':
        return 'Windows';
      case 'linux':
        return 'Linux';
      case 'web':
        return 'Web';
      default:
        return device;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sensitivityColor = _sensitivityColor(entry.sensitivityLevel);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: _actionColor(context).withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(_actionIcon, size: 18, color: _actionColor(context)),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // First row: action type, section, and time
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: _actionColor(context).withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          _actionLabel,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: _actionColor(context),
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        entry.section.toUpperCase(),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const Spacer(),
                      Text(
                        _formatTime(entry.timestamp),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  // Second row: description
                  Text(entry.description, style: theme.textTheme.bodyMedium),
                  const SizedBox(height: 8),
                  // Third row: tags
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      // Device tag
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: Colors.grey.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: Colors.grey.withValues(alpha: 0.3),
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              _deviceIcon(entry.device),
                              size: 12,
                              color: Colors.grey.shade700,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              _getDeviceLabel(entry.device),
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: Colors.grey.shade700,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                      // Sensitivity tag
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: sensitivityColor.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: sensitivityColor.withValues(alpha: 0.3),
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              _sensitivityIcon(entry.sensitivityLevel),
                              size: 12,
                              color: sensitivityColor,
                            ),
                            const SizedBox(width: 4),
                            Text(
                              entry.sensitivityLevel.label,
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: sensitivityColor,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            IconButton(
              icon: const Icon(Icons.info_outline, size: 20),
              onPressed: () => _showDetailDialog(context),
              tooltip: 'View details',
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
      ),
    );
  }
}

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;

  const _DetailRow({
    required this.label,
    required this.value,
    this.valueColor,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 2),
        Text(
          value,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: valueColor,
          ),
        ),
      ],
    );
  }
}
