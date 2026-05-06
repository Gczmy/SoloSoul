import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/services/llm/llm_field_mapping_parser.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/services/scan/scan_background_service.dart';
import 'package:solosoul_flutter/core/services/scan/scan_import_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';
import 'package:solosoul_flutter/core/services/llm/llm_model_state.dart';
import 'package:solosoul_flutter/presentation/providers/scan/scan_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/unified_object_provider.dart';

part 'local_search_provider.g.dart';

// =============================================================================
// Local Search Provider
// =============================================================================

@riverpod
class LocalSearchNotifier extends _$LocalSearchNotifier {
  StreamSubscription<LocalSearchState>? _bgSubscription;

  bool _initialized = false;

  @override
  LocalSearchState build() {
    // Subscribe to the background service so state updates survive page navigation.
    _bgSubscription = ScanBackgroundService.instance.stateStream.listen(
      (bgState) => state = bgState,
    );

    ref.onDispose(() {
      _bgSubscription?.cancel();
    });

    return ScanBackgroundService.instance.currentState;
  }

  // ---------------------------------------------------------------------------
  // Configuration (synced with persistent ScanConfig)
  // ---------------------------------------------------------------------------

  /// Initialize scan config from persistent storage.
  /// Should be called once when the config page is first built.
  /// Awaits the async provider so the config is loaded even on cold start.
  Future<void> initFromConfig() async {
    if (_initialized) return;
    _initialized = true;

    try {
      final config = await ref.read(scanConfigProvider.future);
      state = state.copyWith(
        paths: config.paths,
        extensions: config.extensions,
        scanDepth: config.scanDepth,
        maxFileSizeByExtension: config.maxFileSizeByExtension,
      );
    } on Exception catch (_) {
      // If loading fails, keep current state (defaults or background service state)
    }
  }

  void setPaths(List<String> paths) {
    state = state.copyWith(paths: paths);
    ScanBackgroundService.instance.updateConfig(paths: paths);
    ref.read(scanConfigProvider.notifier).setPaths(paths);
  }

  void setExtensions(List<String> extensions) {
    state = state.copyWith(extensions: extensions);
    ScanBackgroundService.instance.updateConfig(extensions: extensions);
    ref.read(scanConfigProvider.notifier).setExtensions(extensions);
  }

  void setScanDepth(String depth) {
    state = state.copyWith(scanDepth: depth);
    ScanBackgroundService.instance.updateConfig(scanDepth: depth);
    ref.read(scanConfigProvider.notifier).setScanDepth(depth);
  }

  void setMaxFileSizeForExtension(String ext, int mb) {
    final updated = Map<String, int>.from(state.maxFileSizeByExtension)..[ext] = mb;
    state = state.copyWith(maxFileSizeByExtension: updated);
    ScanBackgroundService.instance.updateConfig(maxFileSizeByExtension: updated);
    ref.read(scanConfigProvider.notifier).setMaxFileSizeForExtension(ext, mb);
  }

  // ---------------------------------------------------------------------------
  // Scan Execution (delegated to background service)
  // ---------------------------------------------------------------------------

  void startScan() {
    ScanBackgroundService.instance.startScan(
      paths: state.paths,
      extensions: state.extensions,
      scanDepth: state.scanDepth,
      maxFileSizeByExtension: state.maxFileSizeByExtension,
    );
  }

  void cancelScan() {
    ScanBackgroundService.instance.cancelScan();
  }

  // ---------------------------------------------------------------------------
  // Preview / Import
  // ---------------------------------------------------------------------------

  Future<void> prepareImport() async {
    final importService = ScanImportService(
      ref.read(unifiedObjectProvider.notifier),
      ref.read(unifiedObjectProvider).objects,
    );

    final allCandidates = <ImportCandidate>[];
    for (final result in state.scanResults) {
      final candidates = importService.mapScanResult(result);
      allCandidates.addAll(candidates);
    }

    // Detect conflicts
    final conflicts = importService.detectConflicts(allCandidates);

    state = state.copyWith(
      importCandidates: allCandidates,
      importConflicts: conflicts,
    );
  }

  void setCandidateSelected(int index, bool selected) {
    final candidates = [...state.importCandidates];
    if (index < 0 || index >= candidates.length) return;
    candidates[index] = ImportCandidate(
      source: candidates[index].source,
      existingObjectId: candidates[index].existingObjectId,
      fields: candidates[index].fields,
      isSelected: selected,
    );
    state = state.copyWith(importCandidates: candidates);
  }

  void setFieldAction(int candidateIndex, int fieldIndex, ImportAction action) {
    final candidates = [...state.importCandidates];
    if (candidateIndex < 0 || candidateIndex >= candidates.length) return;
    final fields = [...candidates[candidateIndex].fields];
    if (fieldIndex < 0 || fieldIndex >= fields.length) return;

    final old = fields[fieldIndex];
    fields[fieldIndex] = ImportFieldCandidate(
      source: old.source,
      targetPropertyId: old.targetPropertyId,
      suggestedAction: old.suggestedAction,
      userAction: action,
      mappingSource: old.mappingSource,
      mappingConfidence: old.mappingConfidence,
    );

    candidates[candidateIndex] = ImportCandidate(
      source: candidates[candidateIndex].source,
      existingObjectId: candidates[candidateIndex].existingObjectId,
      fields: fields,
      isSelected: candidates[candidateIndex].isSelected,
    );

    state = state.copyWith(importCandidates: candidates);
  }

  // ---------------------------------------------------------------------------
  // AI-assisted field mapping
  // ---------------------------------------------------------------------------

  Future<void> performAiMapping() async {
    if (state.scanResults.isEmpty) return;

    state = state.copyWith(
      aiMappingStatus: AiMappingStatus.loading,
      aiMappingError: '',
    );

    // 自动加载模型：若未加载则尝试根据配置初始化
    final modelState = ref.read(llmModelProvider);
    if (!modelState.hasValue || modelState.value != LlmModelState.loaded) {
      try {
        await ref.read(llmModelProvider.notifier).loadFromConfig();
      } on LlmException catch (e) {
        final errorMsg = switch (e.code) {
          LlmErrorCode.unauthorized => 'API Key 无效或权限不足，请先配置 LLM',
          LlmErrorCode.modelNotFound => '模型未加载，请先配置 LLM',
          LlmErrorCode.network => '网络连接失败，请检查网络后重试',
          _ => '模型加载失败: ${e.message}',
        };
        state = state.copyWith(
          aiMappingStatus: AiMappingStatus.error,
          aiMappingError: errorMsg,
        );
        return;
      }
    }

    final importService = ScanImportService(
      ref.read(unifiedObjectProvider.notifier),
      ref.read(unifiedObjectProvider).objects,
    );

    // 保存用户当前的选择状态，错误回退时恢复
    final previousSelection = <String, bool>{};
    for (final c in state.importCandidates) {
      previousSelection[c.source.section] = c.isSelected;
    }

    try {
      final modelNotifier = ref.read(llmModelProvider.notifier);
      final allCandidates = <ImportCandidate>[];

      for (final result in state.scanResults) {
        // 隐私过滤：云端模式下 critical 字段禁止外发
        final service = modelNotifier.service;
        final isCloud = service is LlmCloudService;

        var hasCritical = false;
        for (final section in result.sections) {
          for (final field in section.fields) {
            if (field.sensitivity == SensitivityLevel.critical) {
              hasCritical = true;
              break;
            }
          }
          if (hasCritical) break;
        }

        if (isCloud && hasCritical) {
          // critical 数据不回传云端，回退到规则引擎映射
          final candidates = importService.mapScanResult(result);
          allCandidates.addAll(candidates);
          continue;
        }

        // 构建文件内容预览和 schema 描述（sensitive 字段脱敏）
        final fileName = result.meta.sourceFile.split('/').last;
        final contentPreview = result.sections
            .map((s) => s.fields.map((f) {
              final displayValue = f.sensitivity == SensitivityLevel.sensitive
                  ? '[REDACTED_SENSITIVE]'
                  : f.value;
              return '${f.key}: $displayValue';
            }).join('\n'))
            .join('\n---\n');

        // 简化 schema：列出所有出现的 section 和字段
        final schemaBuffer = StringBuffer();
        for (final section in result.sections) {
          schemaBuffer.writeln('Section: ${section.section}');
          for (final field in section.fields) {
            schemaBuffer.writeln('  - ${field.key}');
          }
        }

        final prompt = LlmPromptTemplates.fieldMapping(
          fileName: fileName,
          contentPreview: contentPreview.isEmpty
              ? '(文件内容为空或无法预览)'
              : contentPreview.substring(
                  0,
                  contentPreview.length > 2000 ? 2000 : contentPreview.length,
                ),
          schemaJson: schemaBuffer.isEmpty
              ? '(无可用字段)'
              : schemaBuffer.toString(),
        );

        final llmResponse = await modelNotifier.infer(prompt, maxTokens: 1024);

        // 解析 LLM 返回的 JSON
        final llmResult = LlmFieldMappingParser.parse(
          llmResponse,
          source: 'local',
        );

        final candidates = importService.mapScanResultWithLlm(
          result,
          llmResult,
        );
        allCandidates.addAll(candidates);
      }

      // 重新检测冲突
      final conflicts = importService.detectConflicts(allCandidates);

      state = state.copyWith(
        importCandidates: allCandidates,
        importConflicts: conflicts,
        aiMappingStatus: AiMappingStatus.success,
      );
    } on LlmException catch (e) {
      final errorMsg = switch (e.code) {
        LlmErrorCode.timeout => '模型响应超时，已回退到规则引擎',
        LlmErrorCode.modelNotFound => '模型未加载，请先配置 LLM',
        LlmErrorCode.network => '网络连接失败，已回退到规则引擎',
        LlmErrorCode.unauthorized => 'API Key 无效或权限不足，已回退到规则引擎',
        LlmErrorCode.rateLimited => '请求频率超限，已回退到规则引擎',
        LlmErrorCode.privacyBlocked => '隐私策略阻止了请求，已回退到规则引擎',
        _ => 'AI 映射失败: ${e.message}',
      };

      _fallbackToRuleEngine(importService, previousSelection, errorMsg);
    } on FormatException catch (_) {
      _fallbackToRuleEngine(
        importService,
        previousSelection,
        '模型返回格式错误，已回退到规则引擎',
      );
    } on Exception catch (e) {
      _fallbackToRuleEngine(
        importService,
        previousSelection,
        'AI 映射失败: $e',
      );
    }
  }

  void _fallbackToRuleEngine(
    ScanImportService importService,
    Map<String, bool> previousSelection,
    String errorMsg,
  ) {
    final allCandidates = <ImportCandidate>[];
    for (final result in state.scanResults) {
      final candidates = importService.mapScanResult(result);
      // 恢复用户之前的选择状态
      for (final c in candidates) {
        final wasSelected = previousSelection[c.source.section];
        if (wasSelected != null) {
          c.isSelected = wasSelected;
        }
      }
      allCandidates.addAll(candidates);
    }
    final conflicts = importService.detectConflicts(allCandidates);

    state = state.copyWith(
      importCandidates: allCandidates,
      importConflicts: conflicts,
      aiMappingStatus: AiMappingStatus.error,
      aiMappingError: errorMsg,
    );
  }

  Future<ScanImportResult> executeImport() async {
    final importService = ScanImportService(
      ref.read(unifiedObjectProvider.notifier),
      ref.read(unifiedObjectProvider).objects,
    );

    final confirmed = state.importCandidates
        .where((c) => c.isSelected)
        .toList();

    final result = await importService.executeImport(confirmed);
    state = state.copyWith(importResult: result);
    return result;
  }

  void reset() {
    ScanBackgroundService.instance.reset();
    _initialized = false;
  }
}
