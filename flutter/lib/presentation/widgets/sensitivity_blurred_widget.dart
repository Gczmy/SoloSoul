import 'dart:ui';
import 'package:flutter/material.dart';

/// Text widget with gaussian blur effect for masking sensitive content.
class BlurredText extends StatelessWidget {
  final String text;
  final double blurRadius;
  final TextStyle? style;
  final bool isBlurred;

  const BlurredText({
    super.key,
    required this.text,
    this.blurRadius = 8.0,
    this.style,
    this.isBlurred = true,
  });

  @override
  Widget build(BuildContext context) {
    if (!isBlurred) {
      return Text(text, style: style);
    }

    return ClipRect(
      child: Stack(
        children: [
          // Blurred text layer
          ImageFiltered(
            imageFilter: ImageFilter.blur(
              sigmaX: blurRadius,
              sigmaY: blurRadius,
            ),
            child: Text(
              text,
              style: style?.copyWith(
                color: Colors.transparent,
                shadows: [
                  Shadow(
                    color: (style?.color ?? Theme.of(context).colorScheme.onSurface)
                        .withValues(alpha: 0.3),
                    blurRadius: blurRadius,
                  ),
                ],
              ),
            ),
          ),
          // Visible blurred text
          Text(text, style: style),
        ],
      ),
    );
  }
}

/// Blurred value widget with animated reveal/hide.
class BlurredValue extends StatefulWidget {
  final String value;
  final bool isBlurred;
  final VoidCallback? onTap;
  final double blurRadius;
  final TextStyle? style;

  const BlurredValue({
    super.key,
    required this.value,
    required this.isBlurred,
    this.onTap,
    this.blurRadius = 10.0,
    this.style,
  });

  @override
  State<BlurredValue> createState() => _BlurredValueState();
}

class _BlurredValueState extends State<BlurredValue>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;
  late Animation<double> _blurAnimation;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 300),
      vsync: this,
    );
    _blurAnimation = Tween<double>(
      begin: widget.blurRadius,
      end: 0.0,
    ).animate(CurvedAnimation(
      parent: _controller,
      curve: Curves.easeOut,
    ));

    if (!widget.isBlurred) {
      _controller.value = 1.0;
    }
  }

  @override
  void didUpdateWidget(BlurredValue oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.isBlurred != oldWidget.isBlurred) {
      if (widget.isBlurred) {
        _controller.reverse();
      } else {
        _controller.forward();
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final effectiveStyle = widget.style ?? theme.textTheme.bodyMedium;

    return GestureDetector(
      onTap: widget.onTap,
      child: AnimatedBuilder(
        animation: _blurAnimation,
        builder: (context, child) {
          return ClipRect(
            child: ImageFiltered(
              imageFilter: ImageFilter.blur(
                sigmaX: _blurAnimation.value,
                sigmaY: _blurAnimation.value,
              ),
              child: Text(
                widget.value,
                style: effectiveStyle,
              ),
            ),
          );
        },
      ),
    );
  }
}

/// Masked text using dots pattern (for critical level).
class MaskedText extends StatelessWidget {
  final String value;
  final bool isMasked;
  final TextStyle? style;

  const MaskedText({
    super.key,
    required this.value,
    required this.isMasked,
    this.style,
  });

  String _maskedValue(String val) {
    // Short values (dates, short IDs, phone numbers ≤ 12 chars) are fully masked
    if (val.length <= 12) {
      return '••••••••';
    }
    return '${val.substring(0, 4)}••••••••${val.substring(val.length - 4)}';
  }

  @override
  Widget build(BuildContext context) {
    final displayText = isMasked ? _maskedValue(value) : value;
    final theme = Theme.of(context);
    final effectiveStyle = (style ?? theme.textTheme.bodyMedium)?.copyWith(
      fontFamily: isMasked ? 'monospace' : null,
      letterSpacing: isMasked ? 2 : null,
    );

    return Text(displayText, style: effectiveStyle);
  }
}