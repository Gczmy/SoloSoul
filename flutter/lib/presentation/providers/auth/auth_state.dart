import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'auth_state.g.dart';

/// Sensitive data access validation timeout (unique constant)
const kSensitiveAccessTimeout = Duration(minutes: 1);

/// Backoff state for brute-force protection
class BackoffState {
  final int remainingSeconds;
  final bool isLockedOut;

  const BackoffState({this.remainingSeconds = 0, this.isLockedOut = false});

}

/// Notifier for backoff state
class BackoffNotifier extends Notifier<BackoffState> {
  Timer? _timer;

  @override
  BackoffState build() {
    ref.onDispose(() {
      _timer?.cancel();
    });
    return const BackoffState();
  }

  /// Call after catching PasswordBackoffException to sync state
  void onBackoffException(int remainingSeconds, bool isLockedOut) {
    _timer?.cancel();
    state = BackoffState(
      remainingSeconds: remainingSeconds,
      isLockedOut: isLockedOut,
    );
    if (!isLockedOut && remainingSeconds > 0) {
      _startCountdown(remainingSeconds);
    }
  }

  void _startCountdown(int seconds) {
    _timer = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (state.remainingSeconds <= 1) {
        timer.cancel();
        state = const BackoffState();
      } else {
        state = BackoffState(
          remainingSeconds: state.remainingSeconds - 1,
          isLockedOut: false,
        );
      }
    });
  }

  /// Call after successful login to clear backoff
  void clear() {
    _timer?.cancel();
    state = const BackoffState();
  }
}

/// Provider for backoff state
final backoffProvider = NotifierProvider<BackoffNotifier, BackoffState>(() {
  return BackoffNotifier();
});

/// Sensitive page access state
class SensitivePageAccessState {
  final DateTime? lastVerified;

  const SensitivePageAccessState({this.lastVerified});

  bool get isValid {
    final lastVerified = this.lastVerified;
    if (lastVerified == null) return false;
    return DateTime.now().difference(lastVerified) < kSensitiveAccessTimeout;
  }

  SensitivePageAccessState copyWith({DateTime? lastVerified}) {
    return SensitivePageAccessState(lastVerified: lastVerified ?? this.lastVerified);
  }
}

/// Notifier for sensitive page access
class SensitivePageAccessNotifier extends Notifier<SensitivePageAccessState> {
  Timer? _timer;

  @override
  SensitivePageAccessState build() {
    ref.onDispose(() {
      _timer?.cancel();
    });
    return const SensitivePageAccessState();
  }

  void markVerified() {
    _timer?.cancel();
    state = SensitivePageAccessState(lastVerified: DateTime.now());
    _timer = Timer(kSensitiveAccessTimeout, () {
      state = const SensitivePageAccessState();
    });
  }

  void clear() {
    _timer?.cancel();
    _timer = null;
    state = const SensitivePageAccessState();
  }
}

/// Provider for sensitive page access
final sensitivePageAccessProvider =
    NotifierProvider<SensitivePageAccessNotifier, SensitivePageAccessState>(() {
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
