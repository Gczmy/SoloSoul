import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';
import 'package:solosoul_flutter/presentation/widgets/categorized_icon_grid.dart';

/// Inline icon picker widget.
///
/// Shows a 48×48 trigger button that opens a categorized bottom sheet
/// of ~100 predefined Material-style icons.
class IconPicker extends StatelessWidget {
  final String iconName;
  final ValueChanged<String> onChanged;

  const IconPicker({super.key, required this.iconName, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return InkWell(
      onTap: () async {
        final result = await showModalBottomSheet<String>(
          context: context,
          isScrollControlled: true,
          builder: (ctx) => DraggableScrollableSheet(
            initialChildSize: 0.6,
            minChildSize: 0.4,
            maxChildSize: 0.85,
            expand: false,
            builder: (ctx, scrollController) => Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Flexible(
                    child: CategorizedIconGrid(
                      currentIcon: iconName,
                      onSelected: (name) => Navigator.pop(ctx, name),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
        if (result != null) onChanged(result);
      },
      borderRadius: BorderRadius.circular(12),
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          color: theme.colorScheme.primary.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Icon(
          UnifiedObjectService.getIconFromName(iconName),
          color: theme.colorScheme.primary,
        ),
      ),
    );
  }
}
