import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart'
    show DefaultPageIds, DefaultSectionIds;
import 'package:solosoul_flutter/presentation/widgets/object_category_page.dart';
import 'package:solosoul_flutter/presentation/widgets/scan_document_button.dart';

class FinancialPage extends ConsumerWidget {
  const FinancialPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    return ObjectCategoryPage(
      title: l10n.financialTitle,
      pageId: DefaultPageIds.financial,
      headerWidgets: const [
        ScanDocumentButton(parentId: DefaultSectionIds.taxId),
        SizedBox(height: 16),
      ],
    );
  }
}
