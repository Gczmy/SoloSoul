import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/models/smart_ocr_result.dart';
import 'package:solosoul_flutter/core/services/mrz_vault_service.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/services/pdf_render_service.dart';
import 'package:solosoul_flutter/core/utils/mrz_parser.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_preview_card.dart';
import 'package:solosoul_flutter/presentation/widgets/ocr_result_preview.dart';

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
                  'Passports and ID cards will be automatically detected.',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onPrimaryContainer,
                      ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 24),
        // 操作按钮
        _ActionButton(
          icon: Icons.camera_alt_outlined,
          label: 'Take Photo',
          description: 'Use camera to capture document',
          onTap: () => _pickImage(ImageSource.camera),
        ),
        const SizedBox(height: 12),
        _ActionButton(
          icon: Icons.photo_library_outlined,
          label: 'Choose from Gallery',
          description: 'Select an existing photo',
          onTap: () => _pickImage(ImageSource.gallery),
        ),
        const SizedBox(height: 12),
        _ActionButton(
          icon: Icons.picture_as_pdf_outlined,
          label: 'Select PDF',
          description: 'Import a scanned document from PDF',
          onTap: _pickPdf,
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
          OcrResultPreview(result: result.ocrResult),

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
                  onPressed: () => Navigator.of(context).pop(_result),
                  icon: const Icon(Icons.check),
                  label: const Text('Confirm'),
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
      // MrzParser.parse 要求传入的 lines 恰好匹配标准格式：
      // - TD3 护照: 2 行 × 44 字符
      // - TD1 身份证: 3 行 × 30 字符
      // - TD2: 2 行 × 36 字符
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

      if (mounted) {
        setState(() {
          _isLoading = false;
          if (mrzData != null) {
            _result = SmartOcrMrzResult(
              mrzData: mrzData,
              rawOcrResult: ocrResult,
            );
          } else {
            _result = SmartOcrTextResult(ocrResult);
          }
        });
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

  Future<void> _pickPdf() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['pdf'],
      withData: false,
    );

    if (result == null || result.files.isEmpty) return;

    final path = result.files.first.path;
    if (path == null) return;

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final bytes = await PdfRenderService().renderPage(
        path,
        pageNumber: 1,
        dpi: 300,
      );

      if (bytes == null) {
        if (mounted) {
          setState(() {
            _isLoading = false;
            _errorMessage = 'Failed to render PDF page. The file may be corrupted or password-protected.';
          });
        }
        return;
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

      if (mounted) {
        setState(() {
          _isLoading = false;
          if (mrzData != null) {
            _result = SmartOcrMrzResult(
              mrzData: mrzData,
              rawOcrResult: ocrResult,
            );
          } else {
            _result = SmartOcrTextResult(ocrResult);
          }
        });
      }
    } on OcrTextNotDetectedException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'No text detected in the PDF. '
              'Please try again with a clearer scanned document.';
        });
      }
    } on OcrTimeoutException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'Recognition timed out. Please try again with a clearer PDF.';
        });
      }
    } on OcrException catch (e) {
      SoloLog.w('OcrScannerSheet', 'PDF OCR error: $e');
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
}

/// 操作按钮组件
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
