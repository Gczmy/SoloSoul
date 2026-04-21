import 'dart:async';
import 'dart:convert';
import 'package:collection/collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/rust_vault_service.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/global_sensitivity_defaults.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show FieldRegistry, FieldSensitivity, SensitivityDisplayMode;

// Re-export for single import point
export 'package:solosoul_flutter/core/constants/sensitivity_enums.dart' show SensitivityLevel;
export 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart' show SensitivityDisplayMode;

/// Tag-based sensitivity resolver.
///
/// Resolution order (highest to lowest priority):
/// 1. Field-level explicit setting
/// 2. Tag-level default
/// 3. Global defaults
class TagBasedSensitivityResolver {
  final GlobalSensitivityDefaults _globalDefaults;
  final Map<String, SensitivityLevel> _fieldSettings;
  final Map<String, SensitivityLevel> _tagDefaults;

  const TagBasedSensitivityResolver({
    required GlobalSensitivityDefaults globalDefaults,
    required Map<String, SensitivityLevel> fieldSettings,
    required Map<String, SensitivityLevel> tagDefaults,
  })  : _globalDefaults = globalDefaults,
        _fieldSettings = fieldSettings,
        _tagDefaults = tagDefaults;

  /// Get effective sensitivity level for a field.
  ///
  /// Resolution order:
  /// 1. Check field-level explicit setting
  /// 2. Check tag-based defaults (for each tag)
  /// 3. Fall back to global defaults
  SensitivityLevel getLevel(String fieldId, {List<String> tags = const []}) {
    // Priority 1: Field-level explicit setting
    final fieldLevel = _fieldSettings[fieldId];
    if (fieldLevel != null) {
      return fieldLevel;
    }

    // Priority 2: Tag-based defaults (first tag with a setting wins)
    for (final tag in tags) {
      final tagLevel = _tagDefaults[tag];
      if (tagLevel != null) {
        return tagLevel;
      }
    }

    // Priority 3: Global defaults (untaggedFieldDefault for untagged fields)
    if (tags.isEmpty) {
      return _globalDefaults.untaggedFieldDefault;
    }

    return _globalDefaults.defaultFieldLevel;
  }

  /// Get sensitivity level for a specific tag.
  SensitivityLevel? getTagLevel(String tag) {
    return _tagDefaults[tag];
  }

  /// Check if field has an explicit setting.
  bool hasFieldSetting(String fieldId) {
    return _fieldSettings.containsKey(fieldId);
  }

  /// Check if tag has a default setting.
  bool hasTagSetting(String tag) {
    return _tagDefaults.containsKey(tag);
  }
}

/// Account style data stored in Rust vault.
///
/// Stored as SETTING_{accountId} with encrypted JSON.
class AccountStyle {
  final Map<String, SensitivityLevel> fieldSettings;
  final Map<String, SensitivityLevel> tagDefaults;
  final GlobalSensitivityDefaults globalDefaults;
  final DateTime? lastModified;
  final SensitivityDisplayMode displayMode;
  final Set<String> revealedFields;
  final List<FieldSensitivity> fieldOverrides;

  const AccountStyle({
    this.fieldSettings = const {},
    this.tagDefaults = const {},
    this.globalDefaults = const GlobalSensitivityDefaults(),
    this.lastModified,
    this.displayMode = SensitivityDisplayMode.hidePrivate,
    this.revealedFields = const {},
    this.fieldOverrides = const [],
  });

  AccountStyle copyWith({
    Map<String, SensitivityLevel>? fieldSettings,
    Map<String, SensitivityLevel>? tagDefaults,
    GlobalSensitivityDefaults? globalDefaults,
    DateTime? lastModified,
    SensitivityDisplayMode? displayMode,
    Set<String>? revealedFields,
    List<FieldSensitivity>? fieldOverrides,
  }) {
    return AccountStyle(
      fieldSettings: fieldSettings ?? this.fieldSettings,
      tagDefaults: tagDefaults ?? this.tagDefaults,
      globalDefaults: globalDefaults ?? this.globalDefaults,
      lastModified: lastModified ?? this.lastModified,
      displayMode: displayMode ?? this.displayMode,
      revealedFields: revealedFields ?? this.revealedFields,
      fieldOverrides: fieldOverrides ?? this.fieldOverrides,
    );
  }

  Map<String, dynamic> toJson() => {
        'field_settings': fieldSettings.map((k, v) => MapEntry(k, v.name)),
        'tag_defaults': tagDefaults.map((k, v) => MapEntry(k, v.name)),
        'global_defaults': globalDefaults.toJson(),
        'last_modified': lastModified?.toIso8601String(),
        'display_mode': displayMode.index,
        'revealed_fields': revealedFields.toList(),
        'field_overrides': fieldOverrides.map((f) => f.toJson()).toList(),
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
      globalDefaults: json['global_defaults'] != null
          ? GlobalSensitivityDefaults.fromJson(
              json['global_defaults'] as Map<String, dynamic>)
          : const GlobalSensitivityDefaults(),
      lastModified: json['last_modified'] != null
          ? DateTime.tryParse(json['last_modified'] as String)
          : null,
      displayMode: SensitivityDisplayMode.values[json['display_mode'] as int? ?? 1],
      revealedFields: Set<String>.from(json['revealed_fields'] as List? ?? []),
      fieldOverrides: (json['field_overrides'] as List?)
              ?.map((f) => FieldSensitivity.fromJson(f as Map<String, dynamic>))
              .toList() ??
          [],
    );
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
    } catch (_) {
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
class AccountStyleNotifier extends StateNotifier<AccountStyle> {
  final Ref _ref;
  String? _currentAccountId;
  Timer? _autoSaveTimer;

  AccountStyleNotifier(this._ref) : super(const AccountStyle());

  String? get _accountId =>
      _ref.read(authNotifierProvider.notifier).selectedAccountId;

  /// Load style for account (call after unlock).
  Future<void> loadStyle([String? accountId]) async {
    final accId = accountId ?? _accountId;
    if (accId == null) return;

    // Skip if already loaded for this account
    if (_currentAccountId == accId && state.fieldSettings.isNotEmpty) {
      return;
    }

    _currentAccountId = accId;
    final style = await AccountStyleService.instance.loadStyle(accId);
    if (style != null) {
      state = style;
    } else {
      // Use defaults if no stored style
      state = const AccountStyle();
    }
  }

  /// Reload style from disk.
  Future<void> reloadStyle(String accountId) async {
    _currentAccountId = accountId;
    final style = await AccountStyleService.instance.loadStyle(accountId);
    if (style != null) {
      state = style;
    }
  }

  /// Update field sensitivity level.
  Future<bool> setFieldLevel(String fieldId, SensitivityLevel level) async {
    if (_currentAccountId == null) return false;

    final newFieldSettings = Map<String, SensitivityLevel>.from(state.fieldSettings);
    newFieldSettings[fieldId] = level;

    final updated = state.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    );

    final saved = await AccountStyleService.instance.saveStyle(
      _currentAccountId!,
      updated,
    );

    if (saved) {
      state = updated;
    }

    return saved;
  }

  /// Remove field sensitivity override (revert to tag/global default).
  Future<bool> clearFieldLevel(String fieldId) async {
    if (_currentAccountId == null) return false;

    final newFieldSettings = Map<String, SensitivityLevel>.from(state.fieldSettings);
    newFieldSettings.remove(fieldId);

    final updated = state.copyWith(
      fieldSettings: newFieldSettings,
      lastModified: DateTime.now(),
    );

    final saved = await AccountStyleService.instance.saveStyle(
      _currentAccountId!,
      updated,
    );

    if (saved) {
      state = updated;
    }

    return saved;
  }

  /// Update tag default sensitivity level.
  Future<bool> setTagLevel(String tag, SensitivityLevel level) async {
    if (_currentAccountId == null) return false;

    final newTagDefaults = Map<String, SensitivityLevel>.from(state.tagDefaults);
    newTagDefaults[tag] = level;

    final updated = state.copyWith(
      tagDefaults: newTagDefaults,
      lastModified: DateTime.now(),
    );

    final saved = await AccountStyleService.instance.saveStyle(
      _currentAccountId!,
      updated,
    );

    if (saved) {
      state = updated;
    }

    return saved;
  }

  /// Remove tag default (revert to global default).
  Future<bool> clearTagLevel(String tag) async {
    if (_currentAccountId == null) return false;

    final newTagDefaults = Map<String, SensitivityLevel>.from(state.tagDefaults);
    newTagDefaults.remove(tag);

    state = state.copyWith(
      tagDefaults: newTagDefaults,
      lastModified: DateTime.now(),
    );
    _autoSave();

    return true;
  }

  /// Debounced auto-save with 300ms timer.
  void _autoSave() {
    _autoSaveTimer?.cancel();
    _autoSaveTimer = Timer(const Duration(milliseconds: 300), () {
      if (_currentAccountId != null) {
        AccountStyleService.instance.saveStyle(_currentAccountId!, state);
      }
    });
  }

  /// Set the sensitivity display mode.
  void setDisplayMode(SensitivityDisplayMode mode) {
    state = state.copyWith(displayMode: mode);
    _autoSave();
  }

  /// Reveal a specific field temporarily.
  void revealField(String fieldId) {
    state = state.copyWith(
      revealedFields: {...state.revealedFields, fieldId},
    );
    _autoSave();
  }

  /// Hide a specific field.
  void hideField(String fieldId) {
    state = state.copyWith(
      revealedFields: state.revealedFields.where((id) => id != fieldId).toSet(),
    );
    _autoSave();
  }

  /// Toggle field visibility.
  void toggleField(String fieldId) {
    if (state.revealedFields.contains(fieldId)) {
      hideField(fieldId);
    } else {
      revealField(fieldId);
    }
  }

  /// Hide all revealed private fields.
  void hideAllPrivate() {
    state = state.copyWith(revealedFields: {});
    _autoSave();
  }

  /// Upgrade field to a higher sensitivity level.
  void upgradeField(String fieldId) {
    _moveField(fieldId, 1);
  }

  /// Downgrade field to a lower sensitivity level.
  void downgradeField(String fieldId) {
    _moveField(fieldId, -1);
  }

  void _moveField(String fieldId, int direction) {
    final fieldIndex = state.fieldOverrides.indexWhere((f) => f.fieldId == fieldId);
    if (fieldIndex == -1) return;

    final field = state.fieldOverrides[fieldIndex];
    final newLevel = SensitivityLevel.values[field.level.index + direction];

    if (newLevel.index < 0 || newLevel.index > 3) return;

    final updatedFields = List<FieldSensitivity>.from(state.fieldOverrides);
    updatedFields[fieldIndex] = field.copyWith(level: newLevel);

    state = state.copyWith(fieldOverrides: updatedFields);
    _autoSave();
  }

  /// Clear style state (on lock).
  void clear() {
    _autoSaveTimer?.cancel();
    if (_currentAccountId != null) {
      AccountStyleService.instance.saveStyle(_currentAccountId!, state);
    }
    state = const AccountStyle();
    _currentAccountId = null;
  }

  @override
  void dispose() {
    _autoSaveTimer?.cancel();
    if (_currentAccountId != null) {
      AccountStyleService.instance.saveStyle(_currentAccountId!, state);
    }
    super.dispose();
  }
}

/// Provider for account style.
final accountStyleProvider =
    StateNotifierProvider<AccountStyleNotifier, AccountStyle>((ref) {
  return AccountStyleNotifier(ref);
});

/// Provider for tag-based sensitivity resolver.
final sensitivityResolverProvider = Provider<TagBasedSensitivityResolver>((ref) {
  final style = ref.watch(accountStyleProvider);
  return TagBasedSensitivityResolver(
    globalDefaults: style.globalDefaults,
    fieldSettings: style.fieldSettings,
    tagDefaults: style.tagDefaults,
  );
});

/// Provider for display mode (reuses existing sensitivity settings).
final displayModeProvider = StateProvider<SensitivityDisplayMode>((ref) {
  return SensitivityDisplayMode.hidePrivate;
});

/// Convenience provider for getting field sensitivity level.
/// Returns: user override (fieldSettings) > FieldRegistry default > public fallback
final fieldLevelProvider = Provider.family<SensitivityLevel, String>((ref, fieldId) {
  final style = ref.watch(accountStyleProvider);

  // Priority 1: User override in fieldSettings (set by SensitivitySettingsPage)
  final settingsLevel = style.fieldSettings[fieldId];
  if (settingsLevel != null) return settingsLevel;

  // Priority 2: FieldRegistry default
  return FieldRegistry.defaultFields
      .firstWhereOrNull((f) => f.fieldId == fieldId)
      ?.level ?? SensitivityLevel.public;
});