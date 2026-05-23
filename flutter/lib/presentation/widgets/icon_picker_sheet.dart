import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/widgets/categorized_icon_grid.dart';

/// Bottom sheet for picking an icon from a categorized grid.
class IconPickerSheet extends StatelessWidget {
  final String currentIcon;

  const IconPickerSheet({super.key, required this.currentIcon});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l10n.iconPickerTitle, style: theme.textTheme.titleLarge),
            const SizedBox(height: 8),
            Flexible(
              child: CategorizedIconGrid(
                currentIcon: currentIcon,
                onSelected: (name) => Navigator.pop(context, name),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
