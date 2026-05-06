import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/ocr_service.dart';
import 'package:solosoul_flutter/core/utils/solo_log.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_preview_card.dart';

/// MRZ 扫描底部 Sheet
///
/// 提供相机/相册选图 → OCR 识别 → 预览确认的完整流程。
class MrzScannerSheet extends StatefulWidget {
  const MrzScannerSheet({super.key});

  @override
  State<MrzScannerSheet> createState() => _MrzScannerSheetState();
}

class _MrzScannerSheetState extends State<MrzScannerSheet> {
  bool _isLoading = false;
  String? _errorMessage;
  MrzData? _mrzResult;

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
                      'Scan Passport',
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
              Text('Recognizing passport...'),
            ],
          ),
        ),
      );
    }

    if (_errorMessage != null) {
      return _buildErrorState();
    }

    if (_mrzResult != null) {
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
            color: Theme.of(context).colorScheme.primaryContainer.withValues(alpha: 0.5),
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
                  'Images are never uploaded to any server.',
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
          description: 'Use camera to capture passport',
          onTap: () => _pickImage(ImageSource.camera),
        ),
        const SizedBox(height: 12),
        _ActionButton(
          icon: Icons.photo_library_outlined,
          label: 'Choose from Gallery',
          description: 'Select an existing photo',
          onTap: () => _pickImage(ImageSource.gallery),
        ),
        const SizedBox(height: 24),
        // 提示
        Text(
          'Tip: Place the passport on a flat surface with good lighting '
          'for best recognition results.',
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
    return Column(
      children: [
        MrzPreviewCard(mrzData: _mrzResult!),
        const SizedBox(height: 20),
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                onPressed: () => setState(() => _mrzResult = null),
                icon: const Icon(Icons.refresh),
                label: const Text('Rescan'),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: FilledButton.icon(
                onPressed: () => Navigator.of(context).pop(_mrzResult),
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
      final mrz = await OcrService.extractMrz(Uint8List.fromList(bytes));

      if (mounted) {
        setState(() {
          _isLoading = false;
          _mrzResult = mrz;
        });
      }
    } on OcrMrzNotFoundException {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'Could not find the MRZ area on the passport. '
              'Please make sure the entire passport page is visible and well-lit.';
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
      SoloLog.w('MrzScannerSheet', 'OCR error: $e');
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = e.toString();
        });
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
