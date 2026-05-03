import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

class AddPageInput extends StatelessWidget {
  final TextEditingController controller;
  final String iconName;
  final VoidCallback onIconTap;
  final VoidCallback onConfirm;

  const AddPageInput({
    super.key,
    required this.controller,
    required this.iconName,
    required this.onIconTap,
    required this.onConfirm,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 0, vertical: 2),
      child: Container(
        height: 40,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        decoration: BoxDecoration(
          color: theme.colorScheme.primary.withValues(alpha: 0.05),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.primary.withValues(alpha: 0.3),
          ),
        ),
        child: Row(
          children: [
            InkWell(
              onTap: onIconTap,
              borderRadius: BorderRadius.circular(6),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(
                  UnifiedObjectService.getIconFromName(iconName),
                  size: 20,
                  color: theme.colorScheme.primary,
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: controller,
                autofocus: true,
                decoration: const InputDecoration(
                  border: InputBorder.none,
                  isDense: true,
                  contentPadding: EdgeInsets.zero,
                ),
                style: theme.textTheme.bodyMedium,
                onSubmitted: (_) => onConfirm(),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.check, size: 18),
              onPressed: onConfirm,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
            ),
          ],
        ),
      ),
    );
  }
}
