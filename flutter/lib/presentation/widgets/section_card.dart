import 'package:flutter/material.dart';

class CollapsibleSectionCard extends StatefulWidget {
  final String title;
  final IconData icon;
  final List<Widget> children;
  final IconData? actionIcon;
  final VoidCallback? onAction;
  final int maxVisibleItems;
  final Widget? footer;
  /// Builder for empty state content. If not provided, children are shown
  /// even when the list is empty.
  final Widget Function(ThemeData theme)? emptyContentBuilder;

  const CollapsibleSectionCard({
    super.key,
    required this.title,
    required this.icon,
    required this.children,
    this.actionIcon,
    this.onAction,
    this.maxVisibleItems = 3,
    this.footer,
    this.emptyContentBuilder,
  });

  @override
  State<CollapsibleSectionCard> createState() => _CollapsibleSectionCardState();
}

class _CollapsibleSectionCardState extends State<CollapsibleSectionCard> {
  bool _isExpanded = false;

  @override
  void didUpdateWidget(CollapsibleSectionCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Reset expanded state if item count dropped below maxVisibleItems
    if (widget.children.length <= widget.maxVisibleItems) {
      _isExpanded = false;
    }
  }

  bool get _shouldCollapse => widget.children.length > widget.maxVisibleItems;
  int get _visibleCount => _shouldCollapse && !_isExpanded
      ? widget.maxVisibleItems
      : widget.children.length;
  int get _hiddenCount => widget.children.length - widget.maxVisibleItems;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(widget.icon, size: 20, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.title,
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                if (widget.actionIcon != null && widget.onAction != null)
                  IconButton(
                    icon: Icon(widget.actionIcon, size: 20),
                    onPressed: widget.onAction,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    tooltip: 'Add',
                  ),
              ],
            ),
            const SizedBox(height: 8),
            // Content
            if (widget.children.isEmpty && widget.emptyContentBuilder != null)
              widget.emptyContentBuilder!(theme)
            else
              ...widget.children.take(_visibleCount),
            // Expand/Collapse button
            if (_shouldCollapse) ...[
              const SizedBox(height: 8),
              InkWell(
                onTap: () => setState(() => _isExpanded = !_isExpanded),
                borderRadius: BorderRadius.circular(8),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        _isExpanded
                            ? Icons.keyboard_arrow_up
                            : Icons.keyboard_arrow_down,
                        size: 20,
                        color: theme.colorScheme.primary,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        _isExpanded
                            ? 'Show less'
                            : 'Show $_hiddenCount more',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.primary,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
            // Footer (form or other content inside the card)
            if (widget.footer != null) ...[
              const SizedBox(height: 16),
              const Divider(height: 1),
              const SizedBox(height: 16),
              widget.footer!,
            ],
          ],
        ),
      ),
    );
  }
}

class SectionCard extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<Widget> children;
  final IconData? actionIcon;
  final VoidCallback? onAction;
  final Color? titleColor;

  const SectionCard({
    super.key,
    required this.title,
    required this.icon,
    required this.children,
    this.actionIcon,
    this.onAction,
    this.titleColor,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(
                  icon,
                  size: 20,
                  color: titleColor ?? theme.colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    title,
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: titleColor,
                    ),
                  ),
                ),
                if (actionIcon != null && onAction != null)
                  IconButton(
                    icon: Icon(actionIcon, size: 20),
                    onPressed: onAction,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    tooltip: 'Add',
                  ),
              ],
            ),
            const SizedBox(height: 8),
            // Content
            ...children,
          ],
        ),
      ),
    );
  }
}
