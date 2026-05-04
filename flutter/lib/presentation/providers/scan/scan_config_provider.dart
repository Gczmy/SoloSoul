import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth.dart'
    show authNotifierProvider;

// =============================================================================
// Scan Config Model
// =============================================================================

/// Persistent scan configuration stored in Rust vault.
///
/// Stored as SCAN_CONFIG_{accountId} with encrypted JSON.
/// Per-extension default size limits (MB).
const _kDefaultSizeLimits = {
  '.pdf': 5,
  '.docx': 1,
  '.xlsx': 1,
  '.csv': 1,
  '.json': 1,
  '.txt': 1,
  '.md': 1,
};

class ScanConfig {
  final List<String> paths;
  final List<String> extensions;
  final String scanDepth;
  /// Per-extension max file size in MB. Extension keys include the leading dot.
  final Map<String, int> maxFileSizeByExtension;
  final DateTime? lastModified;

  const ScanConfig({
    this.paths = const [],
    this.extensions = const ['.txt', '.md', '.json', '.csv', '.pdf', '.docx', '.xlsx'],
    this.scanDepth = 'fingerprint',
    this.maxFileSizeByExtension = _kDefaultSizeLimits,
    this.lastModified,
  });

  ScanConfig copyWith({
    List<String>? paths,
    List<String>? extensions,
    String? scanDepth,
    Map<String, int>? maxFileSizeByExtension,
    DateTime? lastModified,
  }) {
    return ScanConfig(
      paths: paths ?? this.paths,
      extensions: extensions ?? this.extensions,
      scanDepth: scanDepth ?? this.scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension ?? this.maxFileSizeByExtension,
      lastModified: lastModified ?? this.lastModified,
    );
  }

  Map<String, dynamic> toJson() => {
        'paths': paths,
        'extensions': extensions,
        'scan_depth': scanDepth,
        'max_file_size_by_ext': maxFileSizeByExtension,
        'last_modified': lastModified?.toIso8601String(),
      };

  factory ScanConfig.fromJson(Map<String, dynamic> json) {
    // Backward-compat: migrate old global max_file_size_mb to per-extension map
    final Map<String, int> sizeLimits;
    final byExt = json['max_file_size_by_ext'] as Map<String, dynamic>?;
    if (byExt != null) {
      sizeLimits = byExt.map((k, v) => MapEntry(k, (v as num).toInt()));
    } else {
      final oldGlobal = (json['max_file_size_mb'] as num?)?.toInt();
      sizeLimits = oldGlobal != null
          ? _kDefaultSizeLimits.map((k, _) => MapEntry(k, oldGlobal))
          : _kDefaultSizeLimits;
    }

    return ScanConfig(
      paths: List<String>.from(json['paths'] as List? ?? []),
      extensions: List<String>.from(
        json['extensions'] as List? ??
            ['.txt', '.md', '.json', '.csv', '.pdf', '.docx', '.xlsx'],
      ),
      scanDepth: json['scan_depth'] as String? ?? 'fingerprint',
      maxFileSizeByExtension: sizeLimits,
      lastModified: json['last_modified'] != null
          ? DateTime.tryParse(json['last_modified'] as String)
          : null,
    );
  }
}

// =============================================================================
// Scan Config Service
// =============================================================================

/// Scan config service - handles persistence via Rust vault.
class ScanConfigService {
  static ScanConfigService? _instance;
  static ScanConfigService get instance =>
      _instance ??= ScanConfigService._();

  ScanConfigService._();

  final RustVaultService _rustVault = RustVaultService.instance;

  /// Load scan config from Rust vault.
  Future<ScanConfig?> loadConfig(String accountId) async {
    final decrypted = await _rustVault.loadScanConfigDecrypted(accountId);
    if (decrypted == null) {
      return null;
    }

    try {
      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      return ScanConfig.fromJson(json);
    } on Object catch (_) {
      return null;
    }
  }

  /// Save scan config to Rust vault.
  Future<bool> saveConfig(String accountId, ScanConfig config) async {
    final jsonData = jsonEncode(config.toJson());
    return await _rustVault.saveScanConfigEncrypted(accountId, jsonData);
  }

  /// Delete scan config from Rust vault.
  Future<bool> deleteConfig(String accountId) async {
    return await _rustVault.deleteScanConfig(accountId);
  }
}

// =============================================================================
// Scan Config Notifier
// =============================================================================

/// State notifier for scan config management.
///
/// Follows the same pattern as [AccountStyleNotifier]:
/// - 300ms debounced auto-save
/// - Loads from vault on first access
/// - Flushes pending save on clear (lock)
class ScanConfigNotifier extends AsyncNotifier<ScanConfig> {
  String? _currentAccountId;
  Timer? _autoSaveTimer;

  String? get _accountId =>
      ref.read(authNotifierProvider.notifier).selectedAccountId;

  @override
  Future<ScanConfig> build() async {
    ref.onDispose(() {
      _autoSaveTimer?.cancel();
    });

    final accId = _accountId;
    if (accId == null) return const ScanConfig();

    // Skip if already loaded for this account
    if (_currentAccountId == accId &&
        state.hasValue &&
        state.value!.paths.isNotEmpty) {
      return state.value!;
    }

    _currentAccountId = accId;
    final config = await ScanConfigService.instance.loadConfig(accId);
    return config ?? const ScanConfig();
  }

  /// Reload config from disk.
  Future<void> reloadConfig(String accountId) async {
    _currentAccountId = accountId;
    state = const AsyncLoading();
    try {
      final config = await ScanConfigService.instance.loadConfig(accountId);
      state = AsyncData(config ?? const ScanConfig());
    } on Exception catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  // ---------------------------------------------------------------------------
  // Mutable setters (trigger auto-save)
  // ---------------------------------------------------------------------------

  void setPaths(List<String> paths) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      paths: paths,
      lastModified: DateTime.now(),
    ));
    _autoSave();
  }

  void setExtensions(List<String> extensions) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      extensions: extensions,
      lastModified: DateTime.now(),
    ));
    _autoSave();
  }

  void setScanDepth(String depth) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      scanDepth: depth,
      lastModified: DateTime.now(),
    ));
    _autoSave();
  }

  void setMaxFileSizeForExtension(String ext, int mb) {
    if (!state.hasValue) return;
    final updated = Map<String, int>.from(state.value!.maxFileSizeByExtension);
    updated[ext] = mb;
    state = AsyncData(state.value!.copyWith(
      maxFileSizeByExtension: updated,
      lastModified: DateTime.now(),
    ));
    _autoSave();
  }

  /// Update all config fields at once (e.g. from UI form).
  void updateConfig({
    List<String>? paths,
    List<String>? extensions,
    String? scanDepth,
    Map<String, int>? maxFileSizeByExtension,
  }) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      paths: paths,
      extensions: extensions,
      scanDepth: scanDepth,
      maxFileSizeByExtension: maxFileSizeByExtension,
      lastModified: DateTime.now(),
    ));
    _autoSave();
  }

  // ---------------------------------------------------------------------------
  // Persistence
  // ---------------------------------------------------------------------------

  /// Debounced auto-save with 300ms timer.
  void _autoSave() {
    _autoSaveTimer?.cancel();
    _autoSaveTimer = Timer(const Duration(milliseconds: 300), () async {
      if (!_isVaultUnlocked) return;
      if (_currentAccountId != null && state.hasValue) {
        await ScanConfigService.instance.saveConfig(
          _currentAccountId!,
          state.value!,
        );
      }
    });
  }

  /// Force immediate save (e.g. before scan starts).
  Future<bool> flush() async {
    _autoSaveTimer?.cancel();
    if (!_isVaultUnlocked) return false;
    if (_currentAccountId != null && state.hasValue) {
      return await ScanConfigService.instance.saveConfig(
        _currentAccountId!,
        state.value!,
      );
    }
    return false;
  }

  /// Clear config state (on lock). Cancels pending save; do NOT attempt to
  /// save when the vault is already locked.
  void clear() {
    _autoSaveTimer?.cancel();
    state = const AsyncData(ScanConfig());
    _currentAccountId = null;
  }

  bool get _isVaultUnlocked =>
      ref.read(authNotifierProvider.notifier).isUnlocked;
}

// =============================================================================
// Provider
// =============================================================================

/// Provider for scan configuration.
final scanConfigProvider =
    AsyncNotifierProvider<ScanConfigNotifier, ScanConfig>(() {
  return ScanConfigNotifier();
});
