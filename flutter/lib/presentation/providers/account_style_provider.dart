import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/services/operation_logger.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/presentation/models/operation_log_models.dart' show LogAction;
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart'
    show FieldRegistry, FormFieldRegistry, firstWhereOrNull;

// Re-export for single import point
export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart' show SensitivityLevel;
export 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show SensitivityDisplayMode, firstWhereOrNull, effectiveSensitivityProvider;

part 'account_style_provider.g.dart';

/// Sensitivity display mode
enum SensitivityDisplayMode {
  showAll,
  hidePrivate,
  hideAll,
}

/// Tag-based sensitivity defaults.
/// Embedded directly in the resolver to eliminate GlobalSensitivityDefaults.
const _tagDefaults = {
  'work': SensitivityLevel.internal,
  'personal': SensitivityLevel.sensitive,
  'financial': SensitivityLevel.critical,
  'health': SensitivityLevel.critical,
};

/// Unified sensitivity resolver.
///
/// Resolution priority (highest to lowest):
/// 1. Temporary reveal (revealedFields set)  → public
/// 2. User override (fieldSettings map)      → user's chosen level
/// 3. Tag-based default (tagDefaults map)   → level by tag
/// 4. Registry default (FieldRegistry)     → hardcoded default
/// 5. Fallback                             → public
class SensitivityResolver {
  const SensitivityResolver();

  SensitivityLevel resolve({
    required String fieldId,
    required Map<String, SensitivityLevel> fieldSettings,
    required Set<String> revealedFields,
    List<String> tags = const [],
  }) {
    // 1. Temporary reveal
    if (revealedFields.contains(fieldId)) {
      return SensitivityLevel.public;
    }

    // 2. User override
    final userLevel = fieldSettings[fieldId];
    if (userLevel != null) {
      return userLevel;
    }

    // 3. Tag-based default
    for (final tag in tags) {
      final tagLevel = _tagDefaults[tag];
      if (tagLevel != null) {
        return tagLevel;
      }
    }

    // 4. FormFieldRegistry (Single Source of Truth)
    final formFieldLevel =
        FormFieldRegistry.getField(fieldId)?.level;
    if (formFieldLevel != null) {
      return formFieldLevel;
    }

    // 5. Legacy FieldRegistry fallback with warning
    final registryLevel = firstWhereOrNull(
        FieldRegistry.defaultFields, (f) => f.fieldId == fieldId)
        ?.level;
    if (registryLevel != null) {
      return registryLevel;
    }

    // 6. Fallback to public
    return SensitivityLevel.public;
  }
}

/// Singleton resolver instance.
const sensitivityResolver = SensitivityResolver();

/// Account style data stored in Rust vault.
///
/// Stored as SETTING_{accountId} with encrypted JSON.
class AccountStyle {
  final Map<String, SensitivityLevel> fieldSettings;
  final Map<String, SensitivityLevel> tagDefaults;
  final DateTime? lastModified;
  final SensitivityDisplayMode displayMode;
  final Set<String> revealedFields;

  const AccountStyle({
    this.fieldSettings = const {},
    this.tagDefaults = const {},
    this.lastModified,
    this.displayMode = SensitivityDisplayMode.hidePrivate,
    this.revealedFields = const {},
  });

  AccountStyle copyWith({
    Map<String, SensitivityLevel>? fieldSettings,
    Map<String, SensitivityLevel>? tagDefaults,
    DateTime? lastModified,
    SensitivityDisplayMode? displayMode,
    Set<String>? revealedFields,
  }) {
    return AccountStyle(
      fieldSettings: fieldSettings ?? this.fieldSettings,
      tagDefaults: tagDefaults ?? this.tagDefaults,
      lastModified: lastModified ?? this.lastModified,
      displayMode: displayMode ?? this.displayMode,
      revealedFields: revealedFields ?? this.revealedFields,
    );
  }

  Map<String, dynamic> toJson() => {
        'field_settings': fieldSettings.map((k, v) => MapEntry(k, v.name)),
        'tag_defaults': tagDefaults.map((k, v) => MapEntry(k, v.name)),
        'last_modified': lastModified?.toIso8601String(),
        'display_mode': displayMode.index,
        'revealed_fields': revealedFields.toList(),
      };

  factory AccountStyle.fromJson(Map<String, dynamic> json) {
    final fieldJson = json['field_settings'] as Map<String, dynamic>? ?? {};
    final fieldSettings = fieldJson.map(
      (k, v) => MapEntry(k, _levelFromString(v as String)),
    );

    final tagJson = json['tag_defaults'] as Map<String, dynamic>? ?? {};
    final tagDefaults = tagJson.map(
      (k, v) => MapEntry(k, _levelFromString(v as String)),
    );

    return AccountStyle(
      fieldSettings: fieldSettings,
      tagDefaults: tagDefaults,
      lastModified: json['last_modified'] != null
          ? DateTime.tryParse(json['last_modified'] as String)
          : null,
      displayMode: _displayModeFromJson(json['display_mode']),
      revealedFields: Set<String>.from(json['revealed_fields'] as List? ?? []),
    );
  }

  static SensitivityDisplayMode _displayModeFromJson(dynamic value) {
    final index = value as int? ?? 1;
    if (index >= 0 && index < SensitivityDisplayMode.values.length) {
      return SensitivityDisplayMode.values[index];
    }
    return SensitivityDisplayMode.hidePrivate;
  }

  static SensitivityLevel _levelFromString(String name) {
    return SensitivityLevel.values.firstWhere(
      (e) => e.name == name,
      orElse: () => SensitivityLevel.public,
    );
  }
}

/// Account style service - handles persistence via Rust vault.
class AccountStyleService {
  static AccountStyleService? _instance;
  static AccountStyleService get instance =>
      _instance ??= AccountStyleService._();

  AccountStyleService._();

  final RustVaultService _rustVault = RustVaultService.instance;

  /// Load account style from Rust vault.
  Future<AccountStyle?> loadStyle(String accountId) async {
    final decrypted = await _rustVault.loadSettingDecrypted(accountId);
    if (decrypted == null) {
      return null;
    }

    try {
      final json = jsonDecode(decrypted) as Map<String, dynamic>;
      return AccountStyle.fromJson(json);
    } on Object catch (_) {
      return null;
    }
  }

  /// Save account style to Rust vault.
  Future<bool> saveStyle(String accountId, AccountStyle style) async {
    final jsonData = jsonEncode(style.toJson());
    return await _rustVault.saveSettingEncrypted(accountId, jsonData);
  }

  /// Delete account style from Rust vault.
  Future<bool> deleteStyle(String accountId) async {
    return await _rustVault.deleteSetting(accountId);
  }
}

/// State notifier for account style management.
class AccountStyleNotifier extends AsyncNotifier<AccountStyle> {
  String? _currentAccountId;
  Timer? _autoSaveTimer;

  String? get _accountId =>
      ref.read(authNotifierProvider.notifier).selectedAccountId;

  @override
  Future<AccountStyle> build() async {
    ref.onDispose(() {
      _autoSaveTimer?.cancel();
    });

    final accId = _accountId;
    if (accId == null) return const AccountStyle();

    // Skip if already loaded for this account
    if (_currentAccountId == accId && state.hasValue && state.value!.fieldSettings.isNotEmpty) {
      return state.value!;
    }

    _currentAccountId = accId;
    final style = await AccountStyleService.instance.loadStyle(accId);
    return style ?? const AccountStyle();
  }

  /// Reload style from disk.
  Future<void> reloadStyle(String accountId) async {
    _currentAccountId = accountId;
    state = const AsyncLoading();
    try {
      final style = await AccountStyleService.instance.loadStyle(accountId);
      state = AsyncData(style ?? const AccountStyle());
    } on Exception catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  /// Update field sensitivity level.
  Future<void> setFieldLevel(String fieldId, SensitivityLevel level) async {
    if (_currentAccountId == null) return;
    if (!state.hasValue) return;

    final currentStyle = state.value!;
    final oldLevel = currentStyle.fieldSettings[fieldId];
    final newFieldSettings = Map<String, SensitivityLevel>.from(currentStyle.fieldSettings);
    newFieldSettings[fieldId] = level;

    final updated = currentStyle.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    );

    state = AsyncData(updated);
    _autoSave();
    unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Changed sensitivity level'));

    // Log the sensitivity change
    final description = oldLevel != null
        ? 'Changed "$fieldId" sensitivity from ${oldLevel.label} to ${level.label}'
        : 'Set "$fieldId" sensitivity to ${level.label}';
    OperationLogger.logSensitivitySettings(
      action: oldLevel != null ? LogAction.update : LogAction.create,
      description: description,
      fieldPath: fieldId,
      sensitivityLevel: level,
    );
  }

  /// Remove field sensitivity override (revert to tag/global default).
  Future<bool> clearFieldLevel(String fieldId) async {
    if (_currentAccountId == null) return false;
    if (!state.hasValue) return false;

    final currentStyle = state.value!;
    final oldLevel = currentStyle.fieldSettings[fieldId];
    final newFieldSettings = Map<String, SensitivityLevel>.from(currentStyle.fieldSettings);
    newFieldSettings.remove(fieldId);

    final updated = currentStyle.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    );

    final saved = await AccountStyleService.instance.saveStyle(
      _currentAccountId!,
      updated,
    );

    if (saved) {
      state = AsyncData(updated);
      // Log the sensitivity revert
      if (oldLevel != null) {
        OperationLogger.logSensitivitySettings(
          action: LogAction.delete,
          description: 'Reverted "$fieldId" sensitivity to default (was ${oldLevel.label})',
          fieldPath: fieldId,
          sensitivityLevel: oldLevel,
        );
      }
    }

    return saved;
  }

  /// Update tag default sensitivity level.
  Future<bool> setTagLevel(String tag, SensitivityLevel level) async {
    if (_currentAccountId == null) return false;
    if (!state.hasValue) return false;

    final currentStyle = state.value!;
    final newTagDefaults = Map<String, SensitivityLevel>.from(currentStyle.tagDefaults);
    newTagDefaults[tag] = level;

    final updated = currentStyle.copyWith(
      tagDefaults: newTagDefaults,
      lastModified: DateTime.now(),
    );

    final saved = await AccountStyleService.instance.saveStyle(
      _currentAccountId!,
      updated,
    );

    if (saved) {
      state = AsyncData(updated);
    }

    return saved;
  }

  /// Remove tag default (revert to global default).
  Future<bool> clearTagLevel(String tag) async {
    if (_currentAccountId == null) return false;
    if (!state.hasValue) return false;

    final currentStyle = state.value!;
    final newTagDefaults = Map<String, SensitivityLevel>.from(currentStyle.tagDefaults);
    newTagDefaults.remove(tag);

    state = AsyncData(currentStyle.copyWith(
      tagDefaults: newTagDefaults,
      lastModified: DateTime.now(),
    ));
    _autoSave();

    return true;
  }

  /// Debounced auto-save with 300ms timer.
  void _autoSave() {
    _autoSaveTimer?.cancel();
    _autoSaveTimer = Timer(const Duration(milliseconds: 300), () async {
      if (_currentAccountId != null && state.hasValue) {
        await AccountStyleService.instance.saveStyle(_currentAccountId!, state.value!);
      }
    });
  }

  /// Set the sensitivity display mode.
  void setDisplayMode(SensitivityDisplayMode mode) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(displayMode: mode));
    _autoSave();
  }

  /// Reveal a specific field temporarily.
  void revealField(String fieldId) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      revealedFields: {...state.value!.revealedFields, fieldId},
    ));
    _autoSave();
  }

  /// Hide a specific field.
  void hideField(String fieldId) {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(
      revealedFields: state.value!.revealedFields.where((id) => id != fieldId).toSet(),
    ));
    _autoSave();
  }

  /// Toggle field visibility.
  void toggleField(String fieldId) {
    if (!state.hasValue) return;
    if (state.value!.revealedFields.contains(fieldId)) {
      hideField(fieldId);
    } else {
      revealField(fieldId);
    }
  }

  /// Hide all revealed private fields.
  void hideAllPrivate() {
    if (!state.hasValue) return;
    state = AsyncData(state.value!.copyWith(revealedFields: {}));
    _autoSave();
  }

  /// Upgrade field to a higher sensitivity level.
  void upgradeField(String fieldId) {
    if (!state.hasValue || _currentAccountId == null) return;

    final currentStyle = state.value!;
    // Resolve effective level: user's override takes priority, else use FormFieldRegistry (preferred) or FieldRegistry fallback
    final effectiveLevel = currentStyle.fieldSettings[fieldId] ??
        FormFieldRegistry.getField(fieldId)?.level ??
        firstWhereOrNull(
            FieldRegistry.defaultFields, (f) => f.fieldId == fieldId)
            ?.level ??
        SensitivityLevel.public;
    if (effectiveLevel.index >= SensitivityLevel.critical.index) return;
    final newLevel = SensitivityLevel.values[effectiveLevel.index + 1];
    final newFieldSettings = Map<String, SensitivityLevel>.from(currentStyle.fieldSettings);
    newFieldSettings[fieldId] = newLevel;
    state = AsyncData(currentStyle.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    ));
    _autoSave();
    unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Upgraded field sensitivity'));

    // Log the sensitivity upgrade
    OperationLogger.logSensitivitySettings(
      action: LogAction.update,
      description: 'Upgraded "$fieldId" sensitivity from ${effectiveLevel.label} to ${newLevel.label}',
      fieldPath: fieldId,
      sensitivityLevel: newLevel,
    );
  }

  /// Downgrade field to a lower sensitivity level.
  void downgradeField(String fieldId) {
    if (!state.hasValue || _currentAccountId == null) return;

    final currentStyle = state.value!;
    // Resolve effective level: user's override takes priority, else use FormFieldRegistry (preferred) or FieldRegistry fallback
    final effectiveLevel = currentStyle.fieldSettings[fieldId] ??
        FormFieldRegistry.getField(fieldId)?.level ??
        firstWhereOrNull(
            FieldRegistry.defaultFields, (f) => f.fieldId == fieldId)
            ?.level ??
        SensitivityLevel.public;
    if (effectiveLevel.index <= SensitivityLevel.public.index) return;
    final newLevel = SensitivityLevel.values[effectiveLevel.index - 1];
    final newFieldSettings = Map<String, SensitivityLevel>.from(currentStyle.fieldSettings);
    newFieldSettings[fieldId] = newLevel;
    state = AsyncData(currentStyle.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    ));
    _autoSave();
    unawaited(ref.read(authNotifierProvider.notifier).updateOperation('Downgraded field sensitivity'));

    // Log the sensitivity downgrade
    OperationLogger.logSensitivitySettings(
      action: LogAction.update,
      description: 'Downgraded "$fieldId" sensitivity from ${effectiveLevel.label} to ${newLevel.label}',
      fieldPath: fieldId,
      sensitivityLevel: newLevel,
    );
  }

  /// Clear style state (on lock).
  void clear() {
    _autoSaveTimer?.cancel();
    if (_currentAccountId != null && state.hasValue) {
      AccountStyleService.instance.saveStyle(_currentAccountId!, state.value!);
    }
    state = const AsyncData(AccountStyle());
    _currentAccountId = null;
  }
}

/// Provider for account style.
final accountStyleProvider =
    AsyncNotifierProvider<AccountStyleNotifier, AccountStyle>(() {
  return AccountStyleNotifier();
});

/// Provider for display mode (reuses existing sensitivity settings).
@riverpod
class DisplayMode extends _$DisplayMode {
  @override
  SensitivityDisplayMode build() {
    return SensitivityDisplayMode.hidePrivate;
  }
}