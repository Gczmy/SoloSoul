import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/ocr_result.dart';

/// MRZ 结果预览卡片
///
/// 以结构化方式展示解析后的护照信息，供用户确认。
class MrzPreviewCard extends StatelessWidget {
  final MrzData mrzData;

  const MrzPreviewCard({
    super.key,
    required this.mrzData,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 标题行
            Row(
              children: [
                Icon(
                  Icons.verified_user_outlined,
                  size: 20,
                  color: colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Recognized Information',
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                ),
                const Spacer(),
                // 置信度徽章
                _ConfidenceBadge(confidence: mrzData.confidence),
              ],
            ),
            const Divider(height: 24),
            // 信息字段
            _InfoRow(
              label: 'Document Type',
              value: _formatDocType(mrzData.documentType),
            ),
            _InfoRow(
              label: 'Document Number',
              value: mrzData.documentNumber,
              isSensitive: true,
            ),
            _InfoRow(
              label: 'Surname',
              value: mrzData.surname,
            ),
            _InfoRow(
              label: 'Given Names',
              value: mrzData.givenNames,
            ),
            _InfoRow(
              label: 'Nationality',
              value: mrzData.nationality,
            ),
            _InfoRow(
              label: 'Date of Birth',
              value: _formatDate(mrzData.dateOfBirth),
            ),
            _InfoRow(
              label: 'Sex',
              value: _formatSex(mrzData.sex),
            ),
            _InfoRow(
              label: 'Expiry Date',
              value: _formatDate(mrzData.expiryDate),
            ),
            // 原始行（可折叠）
            const SizedBox(height: 8),
            _RawLinesSection(rawLines: mrzData.rawLines),
          ],
        ),
      ),
    );
  }

  String _formatDocType(String type) {
    return switch (type) {
      'P' || 'P<' => 'Passport',
      'I' || 'I<' || 'C' || 'C<' => 'ID Card',
      'V' || 'V<' => 'Visa',
      _ => type,
    };
  }

  String _formatDate(String mrzDate) {
    if (mrzDate.length != 6) return mrzDate;
    try {
      final year = int.parse(mrzDate.substring(0, 2));
      final month = int.parse(mrzDate.substring(2, 4));
      final day = int.parse(mrzDate.substring(4, 6));
      final fullYear = year >= 50 ? 1900 + year : 2000 + year;
      return '$fullYear-${month.toString().padLeft(2, '0')}-${day.toString().padLeft(2, '0')}';
    } on FormatException {
      return mrzDate;
    }
  }

  String _formatSex(String sex) {
    return switch (sex) {
      'M' => 'Male',
      'F' => 'Female',
      'X' || '<' => 'Unspecified',
      _ => sex,
    };
  }
}

/// 信息行组件
class _InfoRow extends StatelessWidget {
  final String label;
  final String value;
  final bool isSensitive;

  const _InfoRow({
    required this.label,
    required this.value,
    this.isSensitive = false,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
            ),
          ),
          Expanded(
            child: Text(
              value.isEmpty ? '-' : value,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w500,
                    letterSpacing: isSensitive ? 0.5 : null,
                  ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 置信度徽章
class _ConfidenceBadge extends StatelessWidget {
  final double confidence;

  const _ConfidenceBadge({required this.confidence});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final percentage = (confidence * 100).round();

    Color badgeColor;
    if (confidence >= 0.95) {
      badgeColor = Colors.green;
    } else if (confidence >= 0.85) {
      badgeColor = Colors.orange;
    } else {
      badgeColor = colorScheme.error;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: badgeColor.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(
        '$percentage%',
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: badgeColor,
              fontWeight: FontWeight.w600,
            ),
      ),
    );
  }
}

/// 原始 MRZ 行折叠区
class _RawLinesSection extends StatefulWidget {
  final List<String> rawLines;

  const _RawLinesSection({required this.rawLines});

  @override
  State<_RawLinesSection> createState() => _RawLinesSectionState();
}

class _RawLinesSectionState extends State<_RawLinesSection> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        InkWell(
          onTap: () => setState(() => _expanded = !_expanded),
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Row(
              children: [
                Icon(
                  _expanded ? Icons.expand_less : Icons.expand_more,
                  size: 18,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Text(
                  'Raw MRZ Lines',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                ),
              ],
            ),
          ),
        ),
        if (_expanded)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: widget.rawLines.map((line) {
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 2),
                  child: Text(
                    line,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                          letterSpacing: 1,
                          color: colorScheme.onSurfaceVariant,
                        ),
                  ),
                );
              }).toList(),
            ),
          ),
      ],
    );
  }
}
