import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/language_service.dart';

// =============================================================================
// Language Provider
// =============================================================================

/// Riverpod 封装 for [LanguageService]。
///
/// UI 层通过 `ref.watch(languageProvider)` 实时订阅语言变化，
/// 通过 `ref.read(languageProvider.notifier).setLanguage(...)` 切换语言。
class LanguageNotifier extends AsyncNotifier<Locale> {
  final LanguageService _service = LanguageService.instance;

  @override
  Future<Locale> build() async {
    final code = await _service.getLanguage();
    return Locale(code);
  }

  Future<void> setLanguage(String languageCode) async {
    await _service.setLanguage(languageCode);
    state = AsyncData(Locale(languageCode));
  }
}

final languageProvider =
    AsyncNotifierProvider<LanguageNotifier, Locale>(
  () => LanguageNotifier(),
);
