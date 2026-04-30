import 'package:flutter/material.dart';

/// Pure UI shell - only handles layout, no business logic.
/// Actions are injected via slots for maximum flexibility.
class UniversalEntryCard extends StatelessWidget {
  final Widget title;
  final Widget? subtitle;
  final Widget? leading;
  final List<Widget> actions;
  final List<Widget> children;
  final List<Widget> bottomActions;

  const UniversalEntryCard({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    this.actions = const [],
    this.children = const [],
    this.bottomActions = const [],
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Stack(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Left icon
                  if (leading != null) ...[
                    Padding(padding: const EdgeInsets.only(top: 2), child: leading),
                    const SizedBox(width: 12),
                  ],
                  // Content: title, subtitle, children (full width)
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        title,
                        if (subtitle != null) subtitle!,
                        if (children.isNotEmpty) ...[
                          const SizedBox(height: 4),
                          ...children,
                        ],
                      ],
                    ),
                  ),
                ],
              ),
            ),
            // Action buttons float in top-right, freeing content to use full width
            if (actions.isNotEmpty)
              Positioned(
                top: 8,
                right: 0,
                child: Row(mainAxisSize: MainAxisSize.min, children: actions),
              ),
          ],
        ),
        // Bottom actions (e.g., history button)
        if (bottomActions.isNotEmpty) ...bottomActions,
      ],
    );
  }
}

/// A simpler variant that uses standard icon + text layout
class UniversalEntryTile extends StatelessWidget {
  final Widget title;
  final Widget? subtitle;
  final Widget? leading;
  final List<Widget> actions;
  final List<Widget> children;

  const UniversalEntryTile({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    this.actions = const [],
    this.children = const [],
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (leading != null) ...[
            Padding(padding: const EdgeInsets.only(top: 2), child: leading),
            const SizedBox(width: 12),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                title,
                if (subtitle != null) subtitle!,
                if (children.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  ...children,
                ],
              ],
            ),
          ),
          if (actions.isNotEmpty)
            Row(mainAxisSize: MainAxisSize.min, children: actions),
        ],
      ),
    );
  }
}
