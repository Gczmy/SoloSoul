import 'dart:io';

import 'package:package_info_plus/package_info_plus.dart';

import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/unified_object_model.dart';
import 'package:solosoul_flutter/core/services/language_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_manager.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';
import 'package:solosoul_flutter/core/services/plugin_service.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/core/services/security_service.dart';
import 'package:solosoul_flutter/core/services/user_preferences_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';

// =============================================================================
// LLM Chat Context
// =============================================================================

/// Assembled context for a single AI chat turn.
class LlmChatContext {
  /// The complete system prompt to inject as the first message.
  final String systemPrompt;

  /// Rough token estimate (1 token ≈ 4 chars for CJK, used for logging).
  final int estimatedTokens;

  /// Whether the static portion was served from cache.
  final bool wasCached;

  const LlmChatContext({
    required this.systemPrompt,
    required this.estimatedTokens,
    this.wasCached = false,
  });
}

// =============================================================================
// LLM Context Service
// =============================================================================

/// Collects and assembles contextual information for AI chat sessions.
///
/// **Privacy guarantee:** Only properties with [SensitivityLevel.public]
/// are collected. All [internal], [sensitive], and [critical] fields are
/// unconditionally skipped.
///
/// **Caching:** Static info (user profile, app version, preferences) is cached
/// per-account. Dynamic info (token usage stats) is fetched in real-time.
class LlmContextService {
  LlmContextService._();

  static LlmContextService? _instance;
  static LlmContextService get instance => _instance ??= LlmContextService._();

  // ---------------------------------------------------------------------------
  // Cache
  // ---------------------------------------------------------------------------

  /// Cached static system prompt (without real-time stats).
  String? _cachedStaticPrompt;

  /// Cache key: accountId + profile signature.
  String? _lastCacheKey;

  /// Cached app version (rarely changes).
  String? _cachedAppVersion;

  /// Cached platform name.
  String? _cachedPlatform;

  // ---------------------------------------------------------------------------
  // Limits
  // ---------------------------------------------------------------------------

  static const int _maxObjectsPerType = 3;
  static const int _maxPropertiesPerObject = 8;
  static const int _maxValueLength = 100;
  static const int _maxTotalChars = 2000;

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /// Build the full chat context for the current account.
  ///
  /// [accountId] — current active account.
  /// [modelManager] — the LLM model manager for real-time usage stats.
  Future<LlmChatContext> buildContext({
    required String accountId,
    required LlmModelManager modelManager,
  }) async {
    final stopwatch = Stopwatch()..start();

    // 1. Load profile
    final profile = await ProfileStorageService.instance.loadProfile(accountId);

    // 2. Build cache key from account + profile signature
    final cacheKey = _buildCacheKey(accountId, profile);
    final cacheHit = _cachedStaticPrompt != null && _lastCacheKey == cacheKey;

    String staticPrompt;
    if (cacheHit) {
      staticPrompt = _cachedStaticPrompt!;
      SoloLog.d('LlmContextService', 'Cache hit for account=$accountId');
    } else {
      staticPrompt = await _buildStaticPrompt(accountId, profile);
      _cachedStaticPrompt = staticPrompt;
      _lastCacheKey = cacheKey;
      SoloLog.d('LlmContextService', 'Cache miss, rebuilt static prompt for account=$accountId');
    }

    // 3. Append real-time stats (never cached)
    final statsSection = _buildStatsSection(modelManager);
    final fullPrompt = '$staticPrompt\n\n$statsSection';

    // 4. Enforce total length
    final trimmedPrompt = _trimToLimit(fullPrompt, _maxTotalChars);

    stopwatch.stop();
    final estimatedTokens = _estimateTokens(trimmedPrompt);

    SoloLog.d('LlmContextService',
        'Built context in ${stopwatch.elapsedMilliseconds}ms, '
        'cached=$cacheHit, estTokens=$estimatedTokens, chars=${trimmedPrompt.length}');

    return LlmChatContext(
      systemPrompt: trimmedPrompt,
      estimatedTokens: estimatedTokens,
      wasCached: cacheHit,
    );

  }

  /// Clear the cache. Call when user switches account or vault is locked.
  void clearCache() {
    _cachedStaticPrompt = null;
    _lastCacheKey = null;
    SoloLog.d('LlmContextService', 'Cache cleared');
  }

  // ---------------------------------------------------------------------------
  // Static Prompt Builder
  // ---------------------------------------------------------------------------

  Future<String> _buildStaticPrompt(String accountId, ProfileData? profile) async {
    // Software info (with lazy init)
    final appVersion = await _getAppVersion();
    final platform = _getPlatform();
    final language = await LanguageService.instance.getLanguage();

    // User public info
    final userPublicInfo = _extractPublicInfo(profile);

    // Preferences
    final preferences = await _collectPreferences(accountId);

    // Installed plugins
    final installedPlugins = await _collectInstalledPlugins();

    // Render via template
    return LlmPromptTemplates.chatSystemPrompt(
      appVersion: appVersion,
      platform: platform,
      language: language,
      userPublicInfo: userPublicInfo,
      preferences: preferences,
      installedPlugins: installedPlugins,
      usageStats: {}, // Stats appended separately
    );
  }

  // ---------------------------------------------------------------------------
  // Software Info
  // ---------------------------------------------------------------------------

  Future<String> _getAppVersion() async {
    if (_cachedAppVersion != null) return _cachedAppVersion!;
    try {
      final info = await PackageInfo.fromPlatform();
      _cachedAppVersion = info.version;
      return _cachedAppVersion!;
    } on Exception catch (e) {
      SoloLog.w('LlmContextService', 'Failed to get app version', e);
      return 'unknown';
    }
  }

  String _getPlatform() {
    _cachedPlatform ??= switch (Platform.operatingSystem) {
      'macos' => 'macOS',
      'windows' => 'Windows',
      'linux' => 'Linux',
      'android' => 'Android',
      'ios' => 'iOS',
      _ => Platform.operatingSystem,
    };
    return _cachedPlatform!;
  }

  // ---------------------------------------------------------------------------
  // User Public Info Extractor
  // ---------------------------------------------------------------------------

  /// Extract only [SensitivityLevel.public] properties from the unified object tree.
  ///
  /// Returns: { typeDisplayName -> [ { propertyLabel -> value }, ... ] }
  Map<String, List<Map<String, String>>> _extractPublicInfo(ProfileData? profile) {
    final result = <String, List<Map<String, String>>>{};
    if (profile == null) return result;

    final objects = profile.unifiedObjects?.objects;
    if (objects == null || objects.isEmpty) return result;

    // Group objects by typeId
    final byType = <String, List<UnifiedObject>>{};
    for (final obj in objects) {
      if (obj.isDeleted) continue;
      final typeId = obj.typeId ?? 'other';
      byType.putIfAbsent(typeId, () => []).add(obj);
    }

    for (final typeEntry in byType.entries) {
      final typeId = typeEntry.key;
      final typeObjects = typeEntry.value;

      // Determine display name for the type
      final typeName = _typeDisplayName(typeId);

      final objectList = <Map<String, String>>[];
      final objectsToProcess = typeObjects.take(_maxObjectsPerType);

      for (final obj in objectsToProcess) {
        final props = <String, String>{};
        var propCount = 0;

        for (final propEntry in obj.properties.entries) {
          if (propCount >= _maxPropertiesPerObject) break;

          final propValue = propEntry.value;
          // **Privacy filter**: only public sensitivity
          if (propValue.sensitivity != SensitivityLevel.public) continue;

          final displayValue = _propertyValueToString(propValue);
          if (displayValue.isEmpty) continue;

          final label = _propertyKeyToLabel(propEntry.key);
          props[label] = _truncate(displayValue, _maxValueLength);
          propCount++;
        }

        if (props.isNotEmpty) {
          objectList.add(props);
        }
      }

      if (objectList.isNotEmpty) {
        result[typeName] = objectList;
      }
    }

    return result;
  }

  // ---------------------------------------------------------------------------
  // Preferences
  // ---------------------------------------------------------------------------

  Future<Map<String, dynamic>> _collectPreferences(String accountId) async {
    final prefs = <String, dynamic>{};

    // Language
    final language = await LanguageService.instance.getLanguage();
    prefs['应用语言'] = language == 'zh' ? '中文' : 'English';

    // Security settings (only non-sensitive, safe-to-share items)
    final security = SecurityService.instance.settings;
    prefs['自动锁定'] = security.autoLockDelayMinutes == -1
        ? '永不'
        : '${security.autoLockDelayMinutes} 分钟';
    prefs['剪贴板自动清除'] = security.clipboardClearDelaySeconds == -1
        ? '永不'
        : '${security.clipboardClearDelaySeconds} 秒';

    // Quick actions (just count, not the actual routes for privacy)
    try {
      final quickActions = await UserPreferencesService.instance.loadQuickActions(accountId);
      if (quickActions.isNotEmpty) {
        prefs['快捷操作数量'] = quickActions.length;
      }
    } on Exception catch (e) {
      SoloLog.w('LlmContextService', 'Failed to load quick actions', e);
    }

    return prefs;
  }

  Future<List<String>> _collectInstalledPlugins() async {
    try {
      final pluginService = PluginService();
      await pluginService.initialize();
      final plugins = await pluginService.loadInstalledPlugins();
      return plugins.map((p) => p.name).toList();
    } on Exception catch (e) {
      SoloLog.w('LlmContextService', 'Failed to load installed plugins', e);
      return [];
    }
  }

  // ---------------------------------------------------------------------------
  // Real-Time Stats
  // ---------------------------------------------------------------------------

  String _buildStatsSection(LlmModelManager modelManager) {
    final buffer = StringBuffer();
    buffer.writeln('## AI 使用统计（实时）');
    buffer.writeln('- 当前模型：${modelManager.currentModelName}（${modelManager.currentProvider}）');
    buffer.writeln('- 本次会话：${modelManager.sessionUsageCount} 次调用，${modelManager.sessionTotalTokens} tokens');
    buffer.writeln('- 账户累计：${modelManager.accountUsageCount} 次调用，${modelManager.accountTotalTokens} tokens');
    return buffer.toString();
  }

  // ---------------------------------------------------------------------------
  // Cache Key
  // ---------------------------------------------------------------------------

  String _buildCacheKey(String accountId, ProfileData? profile) {
    if (profile?.unifiedObjects == null) return '${accountId}_null';

    final objects = profile!.unifiedObjects!.objects;
    // Use object count + sum of updatedAt as a lightweight signature
    var signature = 0;
    for (final obj in objects) {
      signature += obj.updatedAt;
    }
    return '${accountId}_${objects.length}_$signature';
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  String _typeDisplayName(String typeId) {
    // Strip __preset_ prefix for built-in types
    final clean = typeId.startsWith('__preset_')
        ? typeId.substring(9)
        : typeId;
    // Convert snake_case / camelCase to readable title
    return clean
        .replaceAll('_', ' ')
        .replaceAllMapped(
          RegExp(r'([a-z])([A-Z])'),
          (m) => '${m[1]} ${m[2]}',
        )
        .split(' ')
        .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
        .join(' ');
  }

  String _propertyKeyToLabel(String key) {
    // Simple camelCase / snake_case to Title Case
    return key
        .replaceAll('_', ' ')
        .replaceAllMapped(
          RegExp(r'([a-z])([A-Z])'),
          (m) => '${m[1]} ${m[2]}',
        )
        .split(' ')
        .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
        .join(' ');
  }

  String _propertyValueToString(PropertyValue value) {
    return switch (value) {
      TextProperty(:final text) => text,
      NumberProperty(:final value) => value?.toString() ?? '',
      DateProperty(:final isoDate) => isoDate ?? '',
      CheckboxProperty(:final checked) => checked ? '是' : '否',
      SelectProperty(:final selectedId) => selectedId ?? '',
      MultiSelectProperty(:final selectedIds) => selectedIds.join(', '),
      RelationProperty(:final targetObjectId) => targetObjectId ?? '',
      UrlProperty(:final url) => url ?? '',
    };
  }

  String _truncate(String value, int maxLength) {
    if (value.length <= maxLength) return value;
    return '${value.substring(0, maxLength)}...';
  }

  String _trimToLimit(String text, int maxChars) {
    if (text.length <= maxChars) return text;
    // Try to cut at a line boundary near the limit
    final cutoff = text.lastIndexOf('\n', maxChars);
    if (cutoff > maxChars * 0.8) {
      return '${text.substring(0, cutoff)}\n\n（部分信息已被截断以控制长度）';
    }
    return '${text.substring(0, maxChars)}...（部分信息已被截断）';
  }

  int _estimateTokens(String text) {
    // Rough estimate: CJK chars ≈ 1 token each, Latin words ≈ 0.75 tokens each
    // This is a conservative over-estimate for logging purposes.
    var cjkCount = 0;
    var otherCount = 0;
    for (final char in text.runes) {
      if ((char >= 0x4E00 && char <= 0x9FFF) ||
          (char >= 0x3400 && char <= 0x4DBF) ||
          (char >= 0x3000 && char <= 0x303F) ||
          (char >= 0xFF00 && char <= 0xFFEF)) {
        cjkCount++;
      } else if (!char.isWhitespace) {
        otherCount++;
      }
    }
    return cjkCount + (otherCount * 0.75).ceil();
  }
}

// Helper extension for code point whitespace check
extension on int {
  bool get isWhitespace {
    return this == 0x20 || this == 0x09 || this == 0x0A || this == 0x0D;
  }
}
