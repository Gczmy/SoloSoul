import 'package:flutter/material.dart';

/// A 90×90 placeholder widget with a dashed rounded border.
///
/// Used as a drop target / drag placeholder or empty slot indicator.
class DashedPlaceholder extends StatelessWidget {
  final Widget? child;
  final Color? color;

  const DashedPlaceholder({super.key, this.child, this.color});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 90,
      height: 90,
      child: Padding(
        padding: const EdgeInsets.all(4),
        child: CustomPaint(
          painter: DashedBorderPainter(
            color: color ?? Theme.of(context).colorScheme.primary.withValues(alpha: 0.4),
          ),
          child: child ?? const SizedBox.expand(),
        ),
      ),
    );
  }
}

/// [CustomPainter] that draws a rounded-rectangle dashed border.
class DashedBorderPainter extends CustomPainter {
  final Color color;

  const DashedBorderPainter({required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    final rrect = RRect.fromRectAndRadius(
      Rect.fromLTWH(1, 1, size.width - 2, size.height - 2),
      const Radius.circular(10),
    );

    final path = Path()..addRRect(rrect);
    final dashedPath = _createDashedPath(path, 6, 4);
    canvas.drawPath(dashedPath, paint);
  }

  Path _createDashedPath(Path source, double dashLength, double dashGap) {
    final dashed = Path();
    for (final metric in source.computeMetrics()) {
      var distance = 0.0;
      while (distance < metric.length) {
        final start = distance;
        final end = (distance + dashLength).clamp(0.0, metric.length);
        dashed.addPath(metric.extractPath(start, end), Offset.zero);
        distance += dashLength + dashGap;
      }
    }
    return dashed;
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
