import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/mrz_vault_service.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/services/pdf_render_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/extracted_fields_preview.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_preview_card.dart';

/// 通用 OCR 扫描底部 Sheet（智能 MRZ 版）
///
/// 提供相机/相册选图 → 通用 OCR 识别 → 智能判断是否为护照/ID卡 →
/// 若检测到 MRZ 则展示结构化结果，否则展示通用文本。
///
/// 用户无感获得 MRZ 结构化体验。
class OcrScannerSheet extends ConsumerStatefulWidget {
  const OcrScannerSheet({super.key});

  @override
  ConsumerState<OcrScannerSheet> createState() => _OcrScannerSheetState();
}

class _OcrScannerSheetState extends ConsumerState<OcrScannerSheet> {
  bool _isLoading = false;
  String? _errorMessage;
  SmartOcrResult? _result;
  Set<String> _selectedFieldKeys = {};

  // LLM Assist 状态
  bool _useLlmAssist = false;
  String? _selectedModelId;
  List<_LlmModelOption> _modelOptions = const [];
  bool _isCheckingModels = false;

  @override
  void initState() {
    super.initState();
    _loadModelOptions();
  }

  @override
  Widget build(BuildContext context) {
    final bottomPadding = MediaQuery.of(context).padding.bottom;

    return Container(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: SafeArea(
        child: Padding(
          padding: EdgeInsets.only(bottom: bottomPadding),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // 拖拽指示器
              Container(
                margin: const EdgeInsets.only(top: 12, bottom: 8),
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.outlineVariant,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              // 标题
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: Row(
                  children: [
                    Text(
                      'Scan Document',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    const Spacer(),
                    IconButton(
                      onPressed: () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              // 内容区
              Flexible(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.symmetric(horizontal: 20),
                  child: _buildContent(),
                ),
              ),
              const SizedBox(height: 16),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildContent() {
    if (_isLoading) {
      return const Padding(
        padding: EdgeInsets.all(40),
        child: Center(
          child: Column(
            children: [
              CircularProgressIndicator(),
              SizedBox(height: 16),
              Text('Recognizing text...'),
            ],
          ),
        ),
      );
    }

    if (_errorMessage != null) {
      return _buildErrorState();
    }

    if (_result != null) {
      return _buildResultState();
    }

    return _buildInitialState();
  }

  Widget _buildInitialState() {
    return Column(
      children: [
        const SizedBox(height: 16),
        // 隐私提示
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Theme.of(context)
                .colorScheme
                .primaryContainer
                .withValues(alpha: 0.5),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(
            children: [
              Icon(
                Icons.security,
                size: 20,
                color: Theme.of(context).colorScheme.onPrimaryContainer,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'All recognition is done locally on your device. '
                  'Images are never uploaded to any server. '
                  'Travel documents and ID cards will be automatically detected.',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onPrimaryContainer,
                      ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        // LLM 协助区域
        _buildLlmAssistSection(),
        const SizedBox(height: 8),
        // 操作按钮
        if (defaultTargetPlatform == TargetPlatform.iOS ||
            defaultTargetPlatform == TargetPlatform.android) ...[
          _ActionButton(
            icon: Icons.camera_alt_outlined,
            label: 'Take Photo',
            description: 'Use camera to capture document',
            onTap: () => _pickImage(ImageSource.camera),
          ),
          const SizedBox(height: 12),
        ],
        _ActionButton(
          icon: Icons.folder_open_outlined,
          label: 'Select Document',
          description: 'Photo or PDF file',
          onTap: _pickDocument,
        ),
        const SizedBox(height: 24),
        // 提示
        Text(
          'Tip: For best results, ensure the text is clearly visible '
          'and the image is well-lit.',
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        const SizedBox(height: 16),
      ],
    );
  }

  Widget _buildErrorState() {
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        children: [
          Icon(
            Icons.error_outline,
            size: 48,
            color: Theme.of(context).colorScheme.error,
          ),
          const SizedBox(height: 16),
          Text(
            'Recognition Failed',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          Text(
            _errorMessage!,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
          const SizedBox(height: 24),
          FilledButton.icon(
            onPressed: () => setState(() => _errorMessage = null),
            icon: const Icon(Icons.refresh),
            label: const Text('Try Again'),
          ),
        ],
      ),
    );
  }

  Widget _buildResultState() {
    final result = _result!;

    return Column(
      children: [
        // 智能检测提示徽章
        if (result is SmartOcrMrzResult)
          Container(
            margin: const EdgeInsets.only(bottom: 12),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.primaryContainer,
              borderRadius: BorderRadius.circular(20),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.verified_user_outlined,
                  size: 16,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
                const SizedBox(width: 6),
                Text(
                  'Travel document detected',
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: Theme.of(context).colorScheme.onPrimaryContainer,
                        fontWeight: FontWeight.w600,
                      ),
                ),
              ],
            ),
          ),

        // 结果展示
        if (result is SmartOcrMrzResult)
          MrzPreviewCard(mrzData: result.mrzData)
        else if (result is SmartOcrTextResult)
          ExtractedFieldsPreview(
            result: result.extraction,
            selectedKeys: _selectedFieldKeys,
            onToggle: (key) {
              setState(() {
                if (_selectedFieldKeys.contains(key)) {
                  _selectedFieldKeys.remove(key);
                } else {
                  _selectedFieldKeys.add(key);
                }
              });
            },
          ),

        const SizedBox(height: 20),

        // 操作按钮
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                onPressed: () => setState(() => _result = null),
                icon: const Icon(Icons.refresh),
                label: const Text('Rescan'),
              ),
            ),
            const SizedBox(width: 12),
            if (result is SmartOcrMrzResult)
              Expanded(
                child: FilledButton.icon(
                  onPressed: () => _saveMrzToVault(result.mrzData),
                  icon: const Icon(Icons.save),
                  label: const Text('Save'),
                ),
              )
            else
              Expanded(
                child: FilledButton.icon(
                  onPressed: () {
                    if (_result is SmartOcrTextResult) {
                      final textResult = _result as SmartOcrTextResult;
                      final filteredFields = <String, ExtractedField>{};
                      for (final key in _selectedFieldKeys) {
                        if (textResult.extraction.fields.containsKey(key)) {
                          filteredFields[key] = textResult.extraction.fields[key]!;
                        }
                      }
                      final filtered = SmartOcrTextResult(
                        textResult.ocrResult,
                        ExtractionResult(
                          documentType: textResult.extraction.documentType,
                          fields: filteredFields,
                          rawText: textResult.extraction.rawText,
                        ),
                      );
                      Navigator.of(context).pop(filtered);
                    } else {
                      Navigator.of(context).pop(_result);
                    }
                  },
                  icon: const Icon(Icons.download),
                  label: const Text('Import'),
                ),
              ),
          ],
        ),
        const SizedBox(height: 16),
      ],
    );
  }

  Future<void> _pickImage(ImageSource source) async {
    final picker = ImagePicker();
    final picked = await picker.pickImage(
      source: source,
      maxWidth: 2048,
      maxHeight: 2048,
      imageQuality: 90,
    );

    if (picked == null) return;

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final bytes = await picked.readAsBytes();

      // Step 1: 通用 OCR 识别
      final ocrResult = await OcrService.recognizeText(Uint8List.fromList(bytes));
      SoloLog.d('OcrScannerSheet',
          'General OCR: ${ocrResult.blocks.length} blocks, confidence=${ocrResult.confidence}');

      // Step 2: 从 OCR 结果中智能提取 MRZ 候选行
      final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);
      SoloLog.d('OcrScannerSheet',
          'MRZ candidate lines: ${mrzCandidates.length}, candidates=$mrzCandidates');

      // Step 3: 从候选行中精确筛选并尝试解析 MRZ
      MrzData? mrzData;
      if (mrzCandidates.isNotEmpty) {
        // 优先尝试 TD3 护照（2 行 × 44 字符）— 取最后 2 个 44 字符行
        final td3Lines = mrzCandidates.where((l) => l.length == 44).toList();
        if (td3Lines.length >= 2) {
          final lastTwo = td3Lines.sublist(td3Lines.length - 2);
          mrzData = MrzParser.parse(lastTwo);
          SoloLog.d('OcrScannerSheet',
              'Trying TD3 with ${lastTwo.length} lines: $lastTwo');
        }

        // 尝试 TD1 身份证（3 行 × 30 字符）— 取最后 3 个 30 字符行
        if (mrzData == null) {
          final td1Lines = mrzCandidates.where((l) => l.length == 30).toList();
          if (td1Lines.length >= 3) {
            final lastThree = td1Lines.sublist(td1Lines.length - 3);
            mrzData = MrzParser.parse(lastThree);
            SoloLog.d('OcrScannerSheet',
                'Trying TD1 with ${lastThree.length} lines: $lastThree');
          }
        }

        // 尝试 TD2（2 行 × 36 字符）— 取最后 2 个 36 字符行
        if (mrzData == null) {
          final td2Lines = mrzCandidates.where((l) => l.length == 36).toList();
          if (td2Lines.length >= 2) {
            final lastTwo = td2Lines.sublist(td2Lines.length - 2);
            mrzData = MrzParser.parse(lastTwo);
            SoloLog.d('OcrScannerSheet',
                'Trying TD2 with ${lastTwo.length} lines: $lastTwo');
          }
        }

        if (mrzData != null) {
          SoloLog.d('OcrScannerSheet',
              'MRZ parsed: docType=${mrzData.documentType}, docNo=${mrzData.documentNumber}');
        }
      }

      final finalMrzData = mrzData;
      if (finalMrzData != null) {
        if (mounted) {
          setState(() {
            _isLoading = false;
            _result = SmartOcrMrzResult(
              mrzData: finalMrzData,
              rawOcrResult: ocrResult,
            );
          });
        }
      } else {
        var extraction = FieldExtractorPipeline.extract(ocrResult.rawText, ocrResult.blocks);

        // LLM 协助提取
        if (_useLlmAssist && _selectedModelId != null) {
          try {
            final llmExtraction = await _performLlmExtraction(
              rawText: ocrResult.rawText,
              blocks: ocrResult.blocks,
              modelId: _selectedModelId!,
            );
            if (llmExtraction != null) {
              extraction = llmExtraction;
            }
          } on Exception catch (e) {
            SoloLog.w('OcrScannerSheet',
                'LLM extraction failed, fallback to rule engine: $e');
          }
        }

        if (mounted) {
          setState(() {
            _isLoading = false;
            _result = SmartOcrTextResult(ocrResult, extraction);
            _selectedFieldKeys = Set<String>.from(extraction.fields.keys);
          });
        }
      }
    } on OcrTextNotDetectedException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'No text detected in the image. '
              'Please try again with a clearer photo of the document.';
        });
      }
    } on OcrTimeoutException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'Recognition timed out. Please try again with a clearer image.';
        });
      }
    } on OcrException catch (e) {
      SoloLog.w('OcrScannerSheet', 'OCR error: $e');
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = e.toString();
        });
      }
    }
  }

  Future<void> _pickDocument() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['jpg', 'jpeg', 'png', 'pdf'],
      withData: false,
    );

    if (result == null || result.files.isEmpty) return;

    final file = result.files.first;
    final path = file.path;
    if (path == null) return;

    final ext = file.extension?.toLowerCase() ?? '';
    final isPdf = ext == 'pdf';

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final Uint8List bytes;
      if (isPdf) {
        final rendered = await PdfRenderService().renderPage(
          path,
          pageNumber: 1,
          dpi: 300,
        );
        if (rendered == null) {
          if (mounted) {
            setState(() {
              _isLoading = false;
              _errorMessage = 'Failed to render PDF page. The file may be corrupted or password-protected.';
            });
          }
          return;
        }
        bytes = rendered;
      } else {
        bytes = await File(path).readAsBytes();
      }

      final ocrResult = await OcrService.recognizeText(bytes);
      final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);

      MrzData? mrzData;
      if (mrzCandidates.isNotEmpty) {
        final td3Lines = mrzCandidates.where((l) => l.length == 44).toList();
        if (td3Lines.length >= 2) {
          final lastTwo = td3Lines.sublist(td3Lines.length - 2);
          mrzData = MrzParser.parse(lastTwo);
        }
        if (mrzData == null) {
          final td1Lines = mrzCandidates.where((l) => l.length == 30).toList();
          if (td1Lines.length >= 3) {
            final lastThree = td1Lines.sublist(td1Lines.length - 3);
            mrzData = MrzParser.parse(lastThree);
          }
        }
        if (mrzData == null) {
          final td2Lines = mrzCandidates.where((l) => l.length == 36).toList();
          if (td2Lines.length >= 2) {
            final lastTwo = td2Lines.sublist(td2Lines.length - 2);
            mrzData = MrzParser.parse(lastTwo);
          }
        }
      }

      final finalMrzData = mrzData;
      if (finalMrzData != null) {
        if (mounted) {
          setState(() {
            _isLoading = false;
            _result = SmartOcrMrzResult(
              mrzData: finalMrzData,
              rawOcrResult: ocrResult,
            );
          });
        }
      } else {
        var extraction = FieldExtractorPipeline.extract(ocrResult.rawText, ocrResult.blocks);

        // LLM 协助提取
        if (_useLlmAssist && _selectedModelId != null) {
          try {
            final llmExtraction = await _performLlmExtraction(
              rawText: ocrResult.rawText,
              blocks: ocrResult.blocks,
              modelId: _selectedModelId!,
            );
            if (llmExtraction != null) {
              extraction = llmExtraction;
            }
          } on Exception catch (e) {
            SoloLog.w('OcrScannerSheet',
                'LLM extraction failed, fallback to rule engine: $e');
          }
        }

        if (mounted) {
          setState(() {
            _isLoading = false;
            _result = SmartOcrTextResult(ocrResult, extraction);
            _selectedFieldKeys = Set<String>.from(extraction.fields.keys);
          });
        }
      }
    } on OcrTextNotDetectedException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = isPdf
              ? 'No text detected in the PDF. Please try again with a clearer scanned document.'
              : 'No text detected in the image. Please try again with a clearer photo of the document.';
        });
      }
    } on OcrTimeoutException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = isPdf
              ? 'Recognition timed out. Please try again with a clearer PDF.'
              : 'Recognition timed out. Please try again with a clearer image.';
        });
      }
    } on OcrException catch (e) {
      SoloLog.w('OcrScannerSheet', 'Document OCR error: $e');
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = e.toString();
        });
      }
    }
  }

  Future<void> _saveMrzToVault(MrzData mrzData) async {
    final result = await MrzVaultService.saveMrzToVault(ref, mrzData: mrzData);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(result.message),
          backgroundColor: result.success ? null : Colors.red,
          duration: const Duration(seconds: 2),
        ),
      );

      if (result.success) {
        Navigator.of(context).pop(_result);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // LLM Assist Section
  // ---------------------------------------------------------------------------

  Widget _buildLlmAssistSection() {
    final hasModels = _modelOptions.isNotEmpty;

    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              controlAffinity: ListTileControlAffinity.leading,
              title: Text(AppLocalizations.of(context).ocrLlmAssist),
              subtitle: Text(AppLocalizations.of(context).ocrLlmAssistSubtitle),
              value: _useLlmAssist,
              onChanged: (v) {
                setState(() => _useLlmAssist = v ?? false);
                if (_useLlmAssist && _modelOptions.isEmpty && !_isCheckingModels) {
                  _loadModelOptions();
                }
              },
            ),
            if (_useLlmAssist) ...[
              const Divider(height: 1),
              const SizedBox(height: 8),
              if (_isCheckingModels)
                const Padding(
                  padding: EdgeInsets.all(8),
                  child: Center(
                    child: SizedBox(
                      height: 24,
                      width: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
                )
              else if (!hasModels)
                _buildNoModelState()
              else
                _buildModelSelector(),
            ],
            // 常驻配置按钮
            Align(
              alignment: Alignment.centerRight,
              child: TextButton.icon(
                onPressed: () => context.push(AppRoutes.llmConfig),
                icon: const Icon(Icons.settings, size: 16),
                label: Text(AppLocalizations.of(context).ocrLlmConfig),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNoModelState() {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                Icons.error_outline,
                size: 16,
                color: Theme.of(context).colorScheme.error,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  AppLocalizations.of(context).ocrNoModelAvailable,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.error,
                    fontSize: 13,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          FilledButton.tonalIcon(
            onPressed: () => context.push(AppRoutes.llmConfig),
            icon: const Icon(Icons.arrow_forward, size: 16),
            label: Text(AppLocalizations.of(context).ocrGoToConfig),
          ),
        ],
      ),
    );
  }

  Widget _buildModelSelector() {
    return DropdownButtonFormField<String>(
      // ignore: deprecated_member_use
      value: _selectedModelId,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: AppLocalizations.of(context).ocrModelSelectorLabel,
        border: OutlineInputBorder(),
        contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      items: _modelOptions.map((option) {
        return DropdownMenuItem<String>(
          value: option.id,
          child: Row(
            children: [
              Icon(
                Icons.circle,
                size: 8,
                color: option.isAvailable ? Colors.green : Colors.red,
              ),
              const SizedBox(width: 8),
              Icon(
                option.isLocal ? Icons.computer : Icons.cloud,
                size: 16,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  option.displayName,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: option.isAvailable
                        ? Theme.of(context).colorScheme.onSurface
                        : Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        );
      }).toList(),
      onChanged: (value) => setState(() => _selectedModelId = value),
    );
  }

  Future<void> _loadModelOptions() async {
    setState(() => _isCheckingModels = true);
    try {
      final configAsync = ref.read(llmConfigProvider);
      if (!configAsync.hasValue) {
        if (mounted) {
          setState(() {
            _modelOptions = const [];
            _isCheckingModels = false;
          });
        }
        return;
      }
      final config = configAsync.value!;
      final options = <_LlmModelOption>[];

      // Local models
      final localModelName = config.localModelPath ?? LlmLocalService.defaultModelName;
      final localService = LlmLocalService(modelName: localModelName);
      final status = await localService.checkStatus();
      if (status.installedModels.isNotEmpty) {
        for (final model in status.installedModels) {
          final isAvailable = status.serviceRunning && status.installedModels.contains(model);
          options.add(_LlmModelOption(
            id: 'local://$model',
            displayName: model,
            isLocal: true,
            isAvailable: isAvailable,
          ));
        }
      } else {
        // Fallback: show configured model even if Ollama not running
        options.add(_LlmModelOption(
          id: 'local://$localModelName',
          displayName: localModelName,
          isLocal: true,
          isAvailable: false,
        ));
      }

      // Cloud profiles
      for (final profile in config.cloudProfiles) {
        String? apiKey;
        try {
          apiKey = await LlmConfigService.instance.getApiKeyByRef(profile.apiKeyRef);
        } on Exception catch (_) {
          apiKey = null;
        }
        final isAvailable = profile.endpoint.isNotEmpty &&
            profile.model.isNotEmpty &&
            (apiKey != null && apiKey.isNotEmpty);
        options.add(_LlmModelOption(
          id: 'cloud://${profile.id}',
          displayName: '${profile.name} · ${profile.model}',
          isLocal: false,
          isAvailable: isAvailable,
        ));
      }

      if (mounted) {
        setState(() {
          _modelOptions = options;
          if (_selectedModelId == null && options.isNotEmpty) {
            // Auto-select first available model, fallback to first option
            final available = options.where((o) => o.isAvailable).toList();
            _selectedModelId = available.isNotEmpty ? available.first.id : options.first.id;
          }
          _isCheckingModels = false;
        });
      }
    } on Exception catch (e) {
      SoloLog.w('OcrScannerSheet', 'Failed to load model options: $e');
      if (mounted) {
        setState(() {
          _modelOptions = const [];
          _isCheckingModels = false;
        });
      }
    }
  }

  Future<ExtractionResult?> _performLlmExtraction({
    required String rawText,
    required List<OcrBlock> blocks,
    required String modelId,
  }) async {
    // Save current global config for restoration
    final configAsync = ref.read(llmConfigProvider);
    if (!configAsync.hasValue) return null;
    final previousConfig = configAsync.value!;

    final configNotifier = ref.read(llmConfigProvider.notifier);
    final modelNotifier = ref.read(llmModelProvider.notifier);

    try {
      // Activate selected model
      if (modelId.startsWith('local://')) {
        final modelName = modelId.substring(8);
        await configNotifier.setBackendType(LlmBackendType.local);
        await configNotifier.setLocalModelPath(modelName);
      } else if (modelId.startsWith('cloud://')) {
        final profileId = modelId.substring(8);
        await configNotifier.setBackendType(LlmBackendType.cloud);
        await configNotifier.setActiveCloudProfile(profileId);
        // Ensure cloud consent; if not, fail gracefully
        if (!previousConfig.cloudConsent) {
          SoloLog.w('OcrScannerSheet', 'Cloud consent not granted, skipping LLM extraction');
          return null;
        }
      }

      // Load model
      await modelNotifier.loadFromConfig();

      // Build prompt
      const fieldSchema = '''
{
  "fields": [
    {"id": "name", "label": "姓名/名称", "type": "text"},
    {"id": "phone", "label": "电话", "type": "text"},
    {"id": "email", "label": "邮箱", "type": "text"},
    {"id": "address", "label": "地址", "type": "text"},
    {"id": "company", "label": "公司/机构", "type": "text"},
    {"id": "title", "label": "职位/头衔", "type": "text"},
    {"id": "date", "label": "日期", "type": "text"},
    {"id": "amount", "label": "金额", "type": "text"},
    {"id": "invoice_number", "label": "发票/单据号码", "type": "text"},
    {"id": "website", "label": "网站/URL", "type": "text"},
    {"id": "id_number", "label": "证件号码", "type": "text"}
  ]
}''';

      final prompt = LlmPromptTemplates.structuredExtraction(
        sourceText: rawText.length > 3000 ? rawText.substring(0, 3000) : rawText,
        fieldSchemaJson: fieldSchema,
      );

      final response = await modelNotifier.infer(prompt, maxTokens: 1024);

      // Parse JSON
      final jsonText = _extractJson(response);
      final json = jsonDecode(jsonText) as Map<String, dynamic>;
      final extractedFieldsList = json['extracted_fields'] as List<dynamic>? ?? [];

      final fields = <String, ExtractedField>{};
      for (final item in extractedFieldsList) {
        if (item is! Map<String, dynamic>) continue;
        final propertyId = item['property_id']?.toString() ?? '';
        final value = item['value']?.toString() ?? '';
        if (propertyId.isEmpty || value.isEmpty) continue;

        // Try to find matching OCR block for bbox
        BoundingBox bbox = const BoundingBox(x: 0, y: 0, width: 0, height: 0);
        OcrBlock? matchedBlock;
        for (final block in blocks) {
          if (block.text.contains(value) || value.contains(block.text)) {
            matchedBlock = block;
            break;
          }
        }
        if (matchedBlock != null) {
          bbox = matchedBlock.bbox;
        }

        fields[propertyId] = ExtractedField(value: value, bbox: bbox);
      }

      return ExtractionResult(
        documentType: json['document_type']?.toString() ?? 'generic',
        fields: fields,
        rawText: rawText,
      );
    } on LlmException catch (e) {
      SoloLog.w('OcrScannerSheet', 'LLM inference failed: ${e.message}');
      return null;
    } on FormatException catch (e) {
      SoloLog.w('OcrScannerSheet', 'LLM response parse failed: $e');
      return null;
    } on Exception catch (e) {
      SoloLog.w('OcrScannerSheet', 'LLM extraction unexpected error: $e');
      return null;
    } finally {
      // Restore previous global config
      try {
        if (previousConfig.backendType == LlmBackendType.local) {
          await configNotifier.setBackendType(LlmBackendType.local);
          final localPath = previousConfig.localModelPath;
          if (localPath != null && localPath.isNotEmpty) {
            await configNotifier.setLocalModelPath(localPath);
          }
        } else {
          await configNotifier.setBackendType(LlmBackendType.cloud);
          final activeId = previousConfig.activeCloudProfileId;
          if (activeId != null && activeId.isNotEmpty) {
            await configNotifier.setActiveCloudProfile(activeId);
          }
        }
        await modelNotifier.loadFromConfig();
      } on Exception catch (e) {
        SoloLog.w('OcrScannerSheet', 'Failed to restore LLM config: $e');
      }
    }
  }

  String _extractJson(String text) {
    final codeBlockRe = RegExp(r'```(?:json)?\s*([\s\S]*?)\s*```');
    final match = codeBlockRe.firstMatch(text);
    if (match != null) {
      return match.group(1)!.trim();
    }
    return text.trim();
  }
}

// =============================================================================
// LLM Model Option
// =============================================================================

class _LlmModelOption {
  final String id;
  final String displayName;
  final bool isLocal;
  final bool isAvailable;

  const _LlmModelOption({
    required this.id,
    required this.displayName,
    required this.isLocal,
    required this.isAvailable,
  });
}

// =============================================================================
// Action Button
// =============================================================================

class _ActionButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final String description;
  final VoidCallback onTap;

  const _ActionButton({
    required this.icon,
    required this.label,
    required this.description,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Icon(
                icon,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      label,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      description,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context).colorScheme.onSurfaceVariant,
                          ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
