import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/widgets/home/dashed_placeholder.dart';

class AddButton extends StatefulWidget {
  final VoidCallback onTap;

  const AddButton({super.key, required this.onTap});

  @override
  State<AddButton> createState() => _AddButtonState();
}

class _AddButtonState extends State<AddButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final borderColor = _isHovered
        ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.4)
        : theme.colorScheme.primary.withValues(alpha: 0.4);
    final iconColor = _isHovered
        ? theme.colorScheme.onSurfaceVariant
        : theme.colorScheme.primary;

    return SizedBox(
      width: 90,
      height: 90,
      child: MouseRegion(
        onEnter: (_) => setState(() => _isHovered = true),
        onExit: (_) => setState(() => _isHovered = false),
        child: GestureDetector(
          onTap: widget.onTap,
          behavior: HitTestBehavior.opaque,
          child: DashedPlaceholder(
            color: borderColor,
            child: Container(
              color: _isHovered
                  ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.12)
                  : null,
              child: Center(
                child: Icon(
                  Icons.add,
                  color: iconColor,
                  size: 28,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
