import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/core/services/document_field_extractor.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_service.dart';
import 'package:solosoul_flutter/core/services/llm/llm_prompt_templates.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/core/services/mrz_vault_service.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/services/pdf_render_service.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_utils.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/extracted_fields_preview.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_preview_card.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_action_button.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_llm_option.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_llm_section.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_scanner_result_card.dart';

/// 通用 OCR 扫描底部 Sheet（智能 MRZ 版）
///
/// 提供相机/相册选图 → 通用 OCR 识别 → 智能判断是否为护照/ID卡 →
/// 若检测到 MRZ 则展示结构化结果，否则展示通用文本。
///
/// 用户无感获得 MRZ 结构化体验。
/// OCR 扫描器核心内容（无 sheet 包装）。
/// 可直接嵌入页面（如 TabView），也可被 [OcrScannerSheet] 包裹作为底部弹窗。
class OcrScannerBody extends ConsumerStatefulWidget {
  final VoidCallback? onClose;
  final ValueChanged<SmartOcrResult>? onResult;

  const OcrScannerBody({super.key, this.onClose, this.onResult});

  @override
  ConsumerState<OcrScannerBody> createState() => _OcrScannerBodyState();
}

class _OcrScannerBodyState extends ConsumerState<OcrScannerBody> {
  bool _isLoading = false;
  String? _errorMessage;
  SmartOcrResult? _result;
  Set<String> _selectedFieldKeys = {};

  // Original image data for attachment saving
  Uint8List? _originalImageBytes;
  bool _saveAttachment = true;
  bool _isSaving = false; // guard against double-tap
  String? _targetSectionId; // user-selected import section

  // LLM Assist 状态
  bool _useLlmAssist = false;
  String? _selectedModelId;
  List<OcrScannerLlmOption> _modelOptions = const [];
  bool _isCheckingModels = false;

  @override
  void initState() {
    super.initState();
    _loadModelOptions();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // 标题 + 可选关闭按钮
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20),
          child: Row(
            children: [
              Text(
                AppLocalizations.of(context).ocrScanDocument,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const Spacer(),
              if (widget.onClose != null)
                IconButton(
                  onPressed: widget.onClose,
                  icon: const Icon(Icons.close),
                ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        // 内容区
        Expanded(
          child: SingleChildScrollView(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: _buildContent(),
          ),
        ),
        const SizedBox(height: 16),
      ],
    );
  }

  Widget _buildContent() {
    final l10n = AppLocalizations.of(context);
    if (_isLoading) {
      return Padding(
        padding: const EdgeInsets.all(40),
        child: Center(
          child: Column(
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 16),
              Text(l10n.ocrRecognizing),
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
        const _OcrPrivacyNotice(),
        const SizedBox(height: 16),
        // LLM 协助区域
        OcrScannerLlmSection(
          useLlmAssist: _useLlmAssist,
          modelOptions: _modelOptions,
          isCheckingModels: _isCheckingModels,
          selectedModelId: _selectedModelId,
          onLlmAssistChanged: (v) {
            setState(() => _useLlmAssist = v ?? false);
            if (_useLlmAssist && _modelOptions.isEmpty && !_isCheckingModels) {
              _loadModelOptions();
            }
          },
          onModelChanged: (value) => setState(() => _selectedModelId = value),
        ),
        const SizedBox(height: 8),
        // 操作按钮
        if (defaultTargetPlatform == TargetPlatform.iOS ||
            defaultTargetPlatform == TargetPlatform.android) ...[
          OcrScannerActionButton(
            icon: Icons.camera_alt_outlined,
            label: AppLocalizations.of(context).ocrTakePhoto,
            description: AppLocalizations.of(context).ocrUseCamera,
            onTap: () => _pickImage(ImageSource.camera),
          ),
          const SizedBox(height: 12),
        ],
        OcrScannerActionButton(
          icon: Icons.folder_open_outlined,
          label: AppLocalizations.of(context).ocrSelectDocument,
          description: AppLocalizations.of(context).ocrPhotoOrPdf,
          onTap: _pickDocument,
        ),
        const SizedBox(height: 24),
        // 提示
        Text(
          AppLocalizations.of(context).ocrTip,
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
    final l10n = AppLocalizations.of(context);
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
            AppLocalizations.of(context).ocrRecognitionFailed,
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
            label: Text(l10n.ocrTryAgain),
          ),
        ],
      ),
    );
  }

  Widget _buildResultState() {
    final l10n = AppLocalizations.of(context);
    final result = _result!;

    return Column(
      children: [
        if (result is SmartOcrMrzResult) const _MrzDetectedBadge(),

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

        // Section selector + attachment checkbox (MRZ result only)
        if (result is SmartOcrMrzResult) ...[
          const SizedBox(height: 16),
          Row(
            children: [
              const Icon(Icons.folder_outlined, size: 16),
              const SizedBox(width: 8),
              Text(
                'Import to: ${ocrScannerSectionLabel(_targetSectionId ?? ocrScannerDetectedSectionId(result.mrzData))}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const Spacer(),
              TextButton(
                onPressed: () => showOcrScannerSectionPicker(
                  context,
                  l10n,
                  result.mrzData,
                  _targetSectionId,
                  (v) => setState(() => _targetSectionId = v),
                ),
                child: Text(l10n.commonEdit),
              ),
            ],
          ),
          if (_originalImageBytes != null) ...[
            const SizedBox(height: 12),
            CheckboxListTile(
              dense: true,
              value: _saveAttachment,
              onChanged: (v) { if (v != null) setState(() => _saveAttachment = v); },
              title: Text(
                l10n.scanAttachFile,
                style: Theme.of(context).textTheme.bodySmall,
              ),
              contentPadding: EdgeInsets.zero,
              controlAffinity: ListTileControlAffinity.leading,
            ),
          ],
        ],

        const SizedBox(height: 20),

        _buildResultActions(context, l10n, result),
        const SizedBox(height: 16),
      ],
    );
  }

  Widget _buildResultActions(BuildContext context, AppLocalizations l10n, dynamic result) {
    return Row(
      children: [
        Expanded(
          child: OutlinedButton.icon(
            onPressed: () => setState(() => _result = null),
            icon: const Icon(Icons.refresh),
            label: Text(l10n.ocrRescan),
          ),
        ),
        const SizedBox(width: 12),
        if (result is SmartOcrMrzResult)
          Expanded(
            child: FilledButton.icon(
              onPressed: () => _saveMrzToVault(result.mrzData),
              icon: const Icon(Icons.save),
              label: Text(l10n.commonSave),
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
                  final onResult = widget.onResult;
                  if (onResult != null) {
                    onResult(filtered);
                  } else {
                    _resetState();
                  }
                } else {
                  final onResult = widget.onResult;
                  final result = _result;
                  if (onResult != null && result != null) {
                    onResult(result);
                  } else {
                    _resetState();
                  }
                }
              },
              icon: const Icon(Icons.download),
              label: Text(l10n.commonImport),
            ),
          ),
      ],
    );
  }

  Future<void> _pickImage(ImageSource source) async {
    final l10n = AppLocalizations.of(context);
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
      // Store for potential attachment saving
      _originalImageBytes = bytes;

      // Step 1: 通用 OCR 识别
      final ocrResult = await OcrService.recognizeText(Uint8List.fromList(bytes));
      SoloLog.d('OcrScannerSheet',
          'General OCR: ${ocrResult.blocks.length} blocks, confidence=${ocrResult.confidence}');

      // Step 2: 从 OCR 结果中智能提取 MRZ 候选行
      final mrzCandidates = OcrService.extractMrzLinesFromResult(ocrResult);
      SoloLog.d('OcrScannerSheet',
          'MRZ candidate lines: ${mrzCandidates.length}, candidates=$mrzCandidates');

      // Step 3: 从候选行中精确筛选并尝试解析 MRZ
      final mrzData = OcrScannerUtils.parseMrzFromCandidates(mrzCandidates);
      if (mrzData != null) {
        SoloLog.d('OcrScannerSheet',
            'MRZ parsed: docType=${mrzData.documentType}, docNo=${mrzData.documentNumber}');
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
              l10n: l10n,
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
          _errorMessage = l10n.ocrNoTextDetectedImage;
        });
      }
    } on OcrTimeoutException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = l10n.ocrRecognitionTimeoutImage;
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
    final l10n = AppLocalizations.of(context);
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
              _errorMessage = l10n.ocrPdfRenderFailed;
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
      final mrzData = OcrScannerUtils.parseMrzFromCandidates(mrzCandidates);
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
              l10n: l10n,
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
              ? l10n.ocrNoTextDetectedPdf
              : l10n.ocrNoTextDetectedImage;
        });
      }
    } on OcrTimeoutException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = isPdf
              ? l10n.ocrRecognitionTimeoutPdf
              : l10n.ocrRecognitionTimeoutImage;
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
    if (_isSaving) return; // guard double-tap
    setState(() => _isSaving = true);
    final result = await MrzVaultService.saveMrzToVault(
      ref,
      mrzData: mrzData,
      imageBytes: _originalImageBytes,
      saveImage: _saveAttachment && _originalImageBytes != null,
      targetSectionId: _targetSectionId,
    );

    if (mounted) {
      _isSaving = false;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(result.message),
          backgroundColor: result.success ? null : Colors.red,
          duration: const Duration(seconds: 2),
        ),
      );

      if (result.success) {
        final onResult = widget.onResult;
        final scanResult = _result;
        if (onResult != null && scanResult != null) {
          onResult(scanResult);
        } else {
          _resetState();
        }
      }
    }
  }

  void _resetState() {
    setState(() {
      _result = null;
      _errorMessage = null;
      _originalImageBytes = null;
      _selectedFieldKeys = {};
      _isSaving = false;
    });
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
      final options = <OcrScannerLlmOption>[];

      // Local models
      options.addAll(await _buildLocalModelOptions(config));

      // Cloud profiles
      options.addAll(await _buildCloudModelOptions(config));

      if (mounted) {
        setState(() {
          _modelOptions = options;
          _selectedModelId = _autoSelectModel(options, _selectedModelId);
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

  Future<List<OcrScannerLlmOption>> _buildLocalModelOptions(LlmConfigState config) async {
    final options = <OcrScannerLlmOption>[];
    final localModelName = config.localModelPath ?? LlmLocalService.defaultModelName;
    final localService = LlmLocalService(modelName: localModelName);
    final status = await localService.checkStatus();
    if (status.installedModels.isNotEmpty) {
      for (final model in status.installedModels) {
        final isAvailable = status.serviceRunning && status.installedModels.contains(model);
        options.add(OcrScannerLlmOption(
          id: 'local://$model',
          displayName: model,
          isLocal: true,
          isAvailable: isAvailable,
        ));
      }
    } else {
      options.add(OcrScannerLlmOption(
        id: 'local://$localModelName',
        displayName: localModelName,
        isLocal: true,
        isAvailable: false,
      ));
    }
    return options;
  }

  Future<List<OcrScannerLlmOption>> _buildCloudModelOptions(LlmConfigState config) async {
    final options = <OcrScannerLlmOption>[];
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
      options.add(OcrScannerLlmOption(
        id: 'cloud://${profile.id}',
        displayName: '${profile.name} · ${profile.model}',
        isLocal: false,
        isAvailable: isAvailable,
      ));
    }
    return options;
  }

  String? _autoSelectModel(List<OcrScannerLlmOption> options, String? currentSelection) {
    if (currentSelection != null || options.isEmpty) return currentSelection;
    final available = options.where((o) => o.isAvailable).toList();
    return available.isNotEmpty ? available.first.id : options.first.id;
  }

  Future<ExtractionResult?> _performLlmExtraction({
    required String rawText,
    required List<OcrBlock> blocks,
    required String modelId,
    required AppLocalizations l10n,
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
      final fieldSchema = '''
{
  "fields": [
    {"id": "name", "label": "${l10n.ocrFieldName}", "type": "text"},
    {"id": "phone", "label": "${l10n.ocrFieldPhone}", "type": "text"},
    {"id": "email", "label": "${l10n.ocrFieldEmail}", "type": "text"},
    {"id": "address", "label": "${l10n.ocrFieldAddress}", "type": "text"},
    {"id": "company", "label": "${l10n.ocrFieldCompany}", "type": "text"},
    {"id": "title", "label": "${l10n.ocrFieldTitle}", "type": "text"},
    {"id": "date", "label": "${l10n.ocrFieldDate}", "type": "text"},
    {"id": "amount", "label": "${l10n.ocrFieldAmount}", "type": "text"},
    {"id": "invoice_number", "label": "${l10n.ocrFieldInvoiceNumber}", "type": "text"},
    {"id": "website", "label": "${l10n.ocrFieldWebsite}", "type": "text"},
    {"id": "id_number", "label": "${l10n.ocrFieldIdNumber}", "type": "text"}
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
    final group1 = match?.group(1);
    if (group1 != null) {
      return group1.trim();
    }
    return text.trim();
  }
}

class _OcrPrivacyNotice extends StatelessWidget {
  const _OcrPrivacyNotice();

  @override
  Widget build(BuildContext context) {
    return Container(
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
              AppLocalizations.of(context).ocrPrivacyNotice,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onPrimaryContainer,
                  ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 底部 Sheet 包装的 OCR 扫描器。
/// 内部使用 [OcrScannerBody]，添加拖拽指示器、关闭按钮与顶部圆角。
class OcrScannerSheet extends StatelessWidget {
  const OcrScannerSheet({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: SafeArea(
        child: Padding(
          padding: EdgeInsets.only(bottom: MediaQuery.of(context).padding.bottom),
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
              Expanded(
                child: OcrScannerBody(
                  onClose: () => Navigator.of(context).pop(),
                  onResult: (result) => Navigator.of(context).pop(result),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MrzDetectedBadge extends StatelessWidget {
  const _MrzDetectedBadge();

  @override
  Widget build(BuildContext context) {
    return Container(
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
            AppLocalizations.of(context).ocrTravelDocumentDetected,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                  fontWeight: FontWeight.w600,
                ),
          ),
        ],
      ),
    );
  }
}
