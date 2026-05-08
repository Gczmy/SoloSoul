import 'dart:ui' show PlatformDispatcher;

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
    final savedCode = await _service.getLanguage();
    // On first launch (no stored preference), auto-detect from OS locale.
    if (!await _service.hasStoredPreference()) {
      final osLocale = PlatformDispatcher.instance.locale;
      if (osLocale.languageCode == 'zh') {
        await _service.setLanguage('zh');
        return const Locale('zh');
      }
      // For all other OS languages, default to English.
      await _service.setLanguage('en');
      return const Locale('en');
    }
    return Locale(savedCode);
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
