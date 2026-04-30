import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/services/profile_persistence_notifier.dart';

// Re-export field history types for backward compatibility
export 'package:solosoul_flutter/core/services/field_history_service.dart'
    show fieldHistoriesProvider, FieldHistoriesNotifier;
export 'package:solosoul_flutter/core/models/field_history_models.dart'
    show FieldHistory, FormHistories;

/// Profile notifier - manages encrypted profile load/save.
/// All CRUD operations are delegated to [unifiedObjectProvider] via
/// [UnifiedObjectNotifier]. This class only handles the top-level
/// profile container lifecycle.
class ProfileNotifier extends AsyncNotifier<ProfileData?> {
  late final ProfilePersistenceService _persistence;

  @override
  Future<ProfileData?> build() async {
    _persistence = ProfilePersistenceService(ref);

    ref.onDispose(() {
      _persistence.dispose();
    });

    // Do NOT auto-load here. Profile is only meaningful after vault unlock,
    // and lazy build() race against login_page's explicit loadProfile() call
    // causes "already loading" skip. Leave loading to loadProfile().
    return null;
  }

  Future<ProfileData?> _loadFromStorage() async {
    final profile = await _persistence.loadProfile();
    return profile;
  }

  bool get isLoading => _persistence.isLoading;

  /// Clear profile state (when auth is locked or reset)
  Future<void> clearProfile() async {
    await _persistence.clearProfile();
    state = const AsyncData(null);
  }

  /// Reload profile (for manual refresh or auth state changes)
  Future<void> loadProfile() async {
    state = const AsyncLoading();
    try {
      final profile = await _loadFromStorage();
      state = AsyncData(profile);
    } on Object catch (e, st) {
      state = AsyncError(e, st);
    }
  }

  /// Save profile for the currently unlocked account (debounced)
  Future<bool> saveProfile(ProfileData profile, {bool immediate = false}) async {
    final result = await _persistence.saveProfile(profile, immediate: immediate);
    if (result) {
      state = AsyncData(profile);
    }
    return result;
  }

  /// Force an immediate save (bypasses debounce)
  Future<bool> saveProfileImmediate(ProfileData profile) async {
    final result = await _persistence.saveProfileImmediate(profile);
    if (result) {
      state = AsyncData(profile);
    }
    return result;
  }
}

/// Auto-loading profile provider that loads when auth state is unlocked
final profileNotifierProvider = AsyncNotifierProvider<ProfileNotifier, ProfileData?>(() {
  return ProfileNotifier();
});
