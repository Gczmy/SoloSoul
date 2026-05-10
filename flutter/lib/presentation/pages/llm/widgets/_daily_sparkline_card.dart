import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/core/services/llm/llm_usage_stats.dart';

// =============================================================================
// Daily Line Chart Card
// =============================================================================

class LLMDailySparklineCard extends StatelessWidget {
  final List<LlmDailyUsage> daily;
  final ThemeData theme;

  const LLMDailySparklineCard({
    super.key,
    required this.daily,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    final sorted = List<LlmDailyUsage>.from(daily)
      ..sort((a, b) => a.date.compareTo(b.date));
    final last14 = sorted.length > 14
        ? sorted.sublist(sorted.length - 14)
        : sorted;
    if (last14.isEmpty) return const SizedBox.shrink();

    // Build data series: prioritize per-model lines, otherwise draw total tokens
    final series = <_Series>[];
    final allModels =
        last14.expand((d) => d.perModelTokens.keys).toSet().toList()..sort();
    if (allModels.isNotEmpty) {
      for (final model in allModels) {
        series.add(
          _Series(
            name: model.split('/').last,
            values: last14
                .map((d) => (d.perModelTokens[model] ?? 0).toDouble())
                .toList(),
          ),
        );
      }
    } else {
      series.add(
        _Series(
          name: AppLocalizations.of(context).llmStatsAllModels,
          values: last14.map((d) => d.totalTokens.toDouble()).toList(),
        ),
      );
    }

    final colors = _chartColors(theme);
    final allValues = series.expand((s) => s.values);
    final rawMax = allValues.isEmpty
        ? 1.0
        : allValues.reduce((a, b) => a > b ? a : b);
    final yMax = _niceMax(rawMax);

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            CustomPaint(
              size: const Size(double.infinity, 200),
              painter: _LineChartPainter(
                theme: theme,
                dates: last14.map((d) => d.date).toList(),
                series: series,
                colors: colors,
                yMax: yMax,
              ),
            ),
            const SizedBox(height: 12),
            // Legend
            Wrap(
              spacing: 16,
              runSpacing: 8,
              children: List.generate(series.length, (i) {
                return Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 10,
                      height: 3,
                      decoration: BoxDecoration(
                        color: colors[i % colors.length],
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      series[i].name,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                );
              }),
            ),
          ],
        ),
      ),
    );
  }

  static List<Color> _chartColors(ThemeData theme) => [
    theme.colorScheme.primary,
    theme.colorScheme.secondary,
    theme.colorScheme.tertiary,
    Colors.orange,
    Colors.purple,
    Colors.teal,
    Colors.pink,
    Colors.indigo,
  ];

  static double _niceMax(double max) {
    if (max <= 0) return 1;
    final exponent = (math.log(max) / math.ln10).floor();
    final fraction = max / math.pow(10, exponent);
    double nice;
    if (fraction <= 1) {
      nice = 1;
    } else if (fraction <= 2) {
      nice = 2;
    } else if (fraction <= 5) {
      nice = 5;
    } else {
      nice = 10;
    }
    return nice * math.pow(10, exponent);
  }
}

// =============================================================================
// Series Data Class
// =============================================================================

class _Series {
  final String name;
  final List<double> values;
  _Series({required this.name, required this.values});
}

// =============================================================================
// Line Chart Painter
// =============================================================================

class _LineChartPainter extends CustomPainter {
  final ThemeData theme;
  final List<DateTime> dates;
  final List<_Series> series;
  final List<Color> colors;
  final double yMax;

  _LineChartPainter({
    required this.theme,
    required this.dates,
    required this.series,
    required this.colors,
    required this.yMax,
  });

  @override
  void paint(Canvas canvas, Size size) {
    const plotLeft = 56.0;
    final plotRight = size.width - 12;
    const plotTop = 8.0;
    final plotBottom = size.height - 28;
    final plotWidth = plotRight - plotLeft;
    final plotHeight = plotBottom - plotTop;

    final n = dates.length;
    if (n < 1 || plotWidth <= 0 || plotHeight <= 0) return;

    double xForIndex(int i) {
      if (n <= 1) return plotLeft + plotWidth / 2;
      return plotLeft + i * (plotWidth / (n - 1));
    }

    final gridPaint = Paint()
      ..color = theme.colorScheme.outlineVariant.withValues(alpha: 0.3)
      ..strokeWidth = 0.5;

    final labelStyle = TextStyle(
      color: theme.colorScheme.onSurfaceVariant,
      fontSize: 10,
    );

    // Y-axis: 5 ticks + grid lines
    const yTicks = 5;
    for (int i = 0; i <= yTicks; i++) {
      final value = (i / yTicks) * yMax;
      final y = plotBottom - (i / yTicks) * plotHeight;
      canvas.drawLine(Offset(plotLeft, y), Offset(plotRight, y), gridPaint);
      final tp = TextPainter(
        text: TextSpan(text: _formatY(value), style: labelStyle),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.right,
      )..layout(maxWidth: plotLeft - 4);
      tp.paint(canvas, Offset(plotLeft - 4 - tp.width, y - tp.height / 2));
    }

    // X-axis labels (show max 4-5 to avoid crowding)
    final xLabelStyle = TextStyle(
      color: theme.colorScheme.onSurfaceVariant,
      fontSize: 10,
    );
    final xStep = math.max(1, (n / 4).ceil());
    for (int i = 0; i < n; i += xStep) {
      final x = xForIndex(i);
      final label = '${dates[i].month}/${dates[i].day}';
      final tp = TextPainter(
        text: TextSpan(text: label, style: xLabelStyle),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.center,
      )..layout();
      tp.paint(canvas, Offset(x - tp.width / 2, plotBottom + 6));
    }

    // Draw each line
    for (int s = 0; s < series.length; s++) {
      final color = colors[s % colors.length];
      final linePaint = Paint()
        ..color = color
        ..strokeWidth = 2
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round;

      final pointPaint = Paint()
        ..color = color
        ..style = PaintingStyle.fill;

      final path = Path();
      for (int i = 0; i < n; i++) {
        final x = xForIndex(i);
        final y =
            plotBottom -
            (series[s].values[i] / yMax).clamp(0, yMax) * plotHeight;
        if (i == 0) {
          path.moveTo(x, y);
        } else {
          path.lineTo(x, y);
        }
        canvas.drawCircle(Offset(x, y), 2.5, pointPaint);
      }
      canvas.drawPath(path, linePaint);
    }
  }

  static String _formatY(double value) {
    if (value >= 1000000) return '${(value / 1000000).toStringAsFixed(1)}M';
    if (value >= 1000) return '${(value / 1000).toStringAsFixed(1)}K';
    return value.toStringAsFixed(0);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}
