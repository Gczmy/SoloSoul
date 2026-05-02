import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart';

part 'operation_log_provider.g.dart';

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

  /// Future to track pending save operations to prevent race conditions
  Future<void>? _pendingSave;

  // TTL constants
  static const int _maxEntries = 500;
  static const int _ttlDays = 90;

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
    // Capture nullable Future to local variable for proper null analysis
    final pending = _pendingSave;
    if (pending != null) {
      try {
        await pending.timeout(
          const Duration(seconds: 2),
          onTimeout: () => null,
        );
      } on Exception catch (_) {
        // Ignore save errors - we're clearing anyway
      }
    }
    _entries.clear();
    _initialized = false;
    _currentAccountId = null;
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

      final decrypted = await _decrypt(encryptedContent);
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
    } on Exception catch (_) {
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

      final encrypted = await _encrypt(jsonString);
      if (encrypted != null) {
        final logFile = await _logFile;
        await logFile.writeAsString(encrypted);
      }
    } on Exception catch (_) {
      // Silently fail on save errors to not disrupt user workflow
    }
  }

  /// Encrypt via Rust FRB (SOLO blob format)
  Future<String?> _encrypt(String plaintext) async {
    try {
      final data = Uint8List.fromList(utf8.encode(plaintext));
      final encrypted = await frb.frbEncryptBytes(data: data);
      if (encrypted.isEmpty) return null;
      return base64Encode(encrypted);
    } on Exception catch (_) {
      return null;
    }
  }

  /// Decrypt via Rust FRB (auto-detects SOLO blob or legacy format)
  Future<String?> _decrypt(String ciphertext) async {
    try {
      final combined = base64Decode(ciphertext);
      final decrypted = await frb.frbDecryptBytes(data: combined);
      return utf8.decode(decrypted);
    } on Exception catch (_) {
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

  Future<void> addEntry(OperationEntry entry) async {
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
    await _saveToDisk();
    notifyListeners(); // Notify Riverpod providers to refresh
  }

  String _getCurrentDevice() {
    try {
      return Platform.operatingSystem.toLowerCase();
    } on Exception catch (_) {
      return 'unknown';
    }
  }

  List<OperationEntry> getEntries() => List.unmodifiable(_entries);

  /// Filter entries by action and device.
  List<OperationEntry> getFilteredEntries({
    Set<String>? actionFilters,
    Set<String>? deviceFilters,
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

/// Provider that tracks [OperationLogService] mutation via a version counter.
/// Each time the service calls [notifyListeners], the version increments,
/// causing all watchers to rebuild.
final operationLogProvider = NotifierProvider<OperationLogServiceNotifier, int>(() {
  return OperationLogServiceNotifier();
});

class OperationLogServiceNotifier extends Notifier<int> {
  @override
  int build() {
    final service = OperationLogService.instance;
    void onChanged() => state = state + 1;
    service.addListener(onChanged);
    ref.onDispose(() => service.removeListener(onChanged));
    return 0;
  }
}

// Derived provider that returns the entries list and rebuilds when service notifies
@riverpod
class OperationLogEntries extends _$OperationLogEntries {
  @override
  List<OperationEntry> build() {
    ref.watch(operationLogProvider);
    return OperationLogService.instance.getEntries();
  }
}

// Filter state providers
@riverpod
class LogActionFilter extends _$LogActionFilter {
  @override
  Set<String> build() => {};

  void setFilters(Set<String> filters) => state = filters;
  void clear() => state = {};
}

@riverpod
class LogDeviceFilter extends _$LogDeviceFilter {
  @override
  Set<String> build() => {};

  void setFilters(Set<String> filters) => state = filters;
  void clear() => state = {};
}

// Filtered entries provider
@riverpod
class OperationLogFilteredEntries extends _$OperationLogFilteredEntries {
  @override
  List<OperationEntry> build() {
    ref.watch(operationLogProvider);
    final actionFilters = ref.watch(logActionFilterProvider);
    final deviceFilters = ref.watch(logDeviceFilterProvider);

    return OperationLogService.instance.getFilteredEntries(
      actionFilters: actionFilters.isEmpty ? null : actionFilters,
      deviceFilters: deviceFilters.isEmpty ? null : deviceFilters,
    );
  }
}