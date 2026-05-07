import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';
import 'package:solosoul_flutter/core/services/mrz_vault_service.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/presentation/widgets/mrz_scanner_sheet.dart';

/// 通用文档扫描按钮。
///
/// 点击后打开 MRZ 扫描器，自动识别护照/身份证并保存到对应 Vault 分区。
class ScanDocumentButton extends ConsumerWidget {
  const ScanDocumentButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16),
      child: InkWell(
        onTap: () => _showMrzScanner(context, ref),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  Icons.document_scanner_outlined,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Scan Document',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Extract data from passport or ID card',
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
    ).animate().fadeIn(duration: 300.ms);
  }

  Future<void> _showMrzScanner(BuildContext context, WidgetRef ref) async {
    final result = await showModalBottomSheet<MrzScanResult?>(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) => const MrzScannerSheet(),
    );

    if (result != null && context.mounted) {
      final outcome = await MrzVaultService.saveMrzToVault(
        ref,
        mrzData: result.mrzData,
        imageBytes: result.imageBytes,
        saveImage: result.saveImage,
      );
      if (context.mounted) {
        showOverlaySnackBar(
          context,
          content: outcome.message,
          type: outcome.success ? SnackBarType.success : SnackBarType.error,
        );
      }
    }
  }
}
