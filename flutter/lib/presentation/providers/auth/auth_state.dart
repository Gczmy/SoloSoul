import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/presentation/providers/auth/auth_types.dart';

part 'auth_state.g.dart';

/// Pure state machine for authentication state (locked/unlocked/loading)
class AuthStateNotifier extends StateNotifier<AuthState> {
  AuthStateNotifier() : super(AuthState.initial);

  void setInitial() => state = AuthState.initial;
  void setLoading() => state = AuthState.loading;
  void setLocked() => state = AuthState.locked;
  void setUnlocked() => state = AuthState.unlocked;

  bool get isUnlocked => state == AuthState.unlocked;
}

/// Sensitive data access validation timeout (unique constant)
const kSensitiveAccessTimeout = Duration(minutes: 1);

/// Sensitive page access state
class SensitivePageAccessState {
  final DateTime? lastVerified;

  const SensitivePageAccessState({this.lastVerified});

  bool get isValid {
    if (lastVerified == null) return false;
    return DateTime.now().difference(lastVerified!) < kSensitiveAccessTimeout;
  }

  SensitivePageAccessState copyWith({DateTime? lastVerified}) {
    return SensitivePageAccessState(lastVerified: lastVerified ?? this.lastVerified);
  }
}

/// Notifier for sensitive page access
class SensitivePageAccessNotifier extends StateNotifier<SensitivePageAccessState> {
  Timer? _timer;

  SensitivePageAccessNotifier() : super(const SensitivePageAccessState());

  void markVerified() {
    _timer?.cancel();
    state = SensitivePageAccessState(lastVerified: DateTime.now());
    _timer = Timer(kSensitiveAccessTimeout, () {
      state = state.copyWith(lastVerified: state.lastVerified);
    });
  }

  void clear() {
    _timer?.cancel();
    _timer = null;
    state = const SensitivePageAccessState();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}

/// Provider for sensitive page access
final sensitivePageAccessProvider =
    StateNotifierProvider<SensitivePageAccessNotifier, SensitivePageAccessState>(
        (ref) {
  return SensitivePageAccessNotifier();
});

/// Provider that checks if sensitive access is currently granted
@riverpod
class IsSensitiveAccessGranted extends _$IsSensitiveAccessGranted {
  @override
  bool build() {
    final access = ref.watch(sensitivePageAccessProvider);
    return access.isValid;
  }
}
