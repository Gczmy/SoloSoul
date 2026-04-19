import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/profile_storage_service.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/providers/sensitivity_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart' hide SensitivityLevel;
import 'package:solosoul_flutter/presentation/widgets/password_verification_dialog.dart';

/// Widget to display and animate field history
class FieldHistoryView extends ConsumerStatefulWidget {
  final String fieldName;
  final FieldHistory history;
  final bool initiallyExpanded;

  const FieldHistoryView({
    super.key,
    required this.fieldName,
    required this.history,
    this.initiallyExpanded = false,
  });

  @override
  ConsumerState<FieldHistoryView> createState() => _FieldHistoryViewState();
}

class _FieldHistoryViewState extends ConsumerState<FieldHistoryView> {
  bool _userToggledExpanded = false;

  bool get _isRestrictedField {
    final settings = ref.read(sensitivitySettingsProvider);
    final level = settings.getFieldLevel(widget.fieldName);
    return level == SensitivityLevel.restricted;
  }

  /// Whether sensitivity access is currently unlocked (recently verified)
  bool get _hasSensitiveAccess {
    final sensitiveAccess = ref.watch(sensitivePageAccessProvider);
    final oneMinuteAgo = DateTime.now().subtract(const Duration(minutes: 1));
    return sensitiveAccess.lastVerified != null &&
        sensitiveAccess.lastVerified!.isAfter(oneMinuteAgo);
  }

  /// Derive expanded state from field level and access status
  bool get _isExpanded {
    if (_isRestrictedField && !_hasSensitiveAccess) {
      // Always collapse restricted fields when access is locked
      return false;
    }
    return _userToggledExpanded || widget.initiallyExpanded;
  }

  Future<void> _toggleExpanded() async {
    if (_isRestrictedField && !_hasSensitiveAccess) {
      // Show password dialog for restricted fields
      final authNotifier = ref.read(authNotifierProvider.notifier);
      final selectedAccount = authNotifier.selectedAccount;
      if (selectedAccount == null) return;

      final password = await showPasswordVerificationDialog(
        context: context,
        ref: ref,
        passwordHint: selectedAccount.passwordHint,
        onVerify: authNotifier.verifyPasswordForSensitiveData,
      );
      if (password == null) return;

      // Mark as verified
      ref.read(sensitivePageAccessProvider.notifier).markVerified();
    }

    setState(() => _userToggledExpanded = !_userToggledExpanded);
  }

  String _formatTimestamp(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);

    if (diff.inDays > 365) {
      return '${(diff.inDays / 365).floor()} year(s) ago';
    } else if (diff.inDays > 30) {
      return '${(diff.inDays / 30).floor()} month(s) ago';
    } else if (diff.inDays > 0) {
      return '${diff.inDays} day(s) ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours} hour(s) ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes} minute(s) ago';
    } else {
      return 'Just now';
    }
  }

  String _formatFullTimestamp(DateTime timestamp) {
    return '${timestamp.year}-${timestamp.month.toString().padLeft(2, '0')}-${timestamp.day.toString().padLeft(2, '0')} '
        '${timestamp.hour.toString().padLeft(2, '0')}:${timestamp.minute.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final entries = widget.history.entries;
    final latestEntry = entries.isNotEmpty ? entries.last : null;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Toggle button
        InkWell(
          onTap: _toggleExpanded,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(
                  Icons.history,
                  size: 16,
                  color: AppTheme.primaryColor,
                ),
                const SizedBox(width: 4),
                Text(
                  'History',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: AppTheme.primaryColor,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                const SizedBox(width: 4),
                Text(
                  '(${entries.length})',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: AppTheme.primaryColor.withValues(alpha: 0.7),
                  ),
                ),
                Icon(
                  _isExpanded ? Icons.expand_less : Icons.expand_more,
                  size: 16,
                  color: AppTheme.primaryColor,
                ),
              ],
            ),
          ),
        ),

        // Expanded history list with animation
        if (_isExpanded) ...[
          const SizedBox(height: 8),
          Container(
            constraints: const BoxConstraints(maxHeight: 200),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(
                color: theme.colorScheme.outline.withValues(alpha: 0.2),
              ),
            ),
            child: ListView.builder(
              shrinkWrap: true,
              padding: const EdgeInsets.symmetric(vertical: 4),
              itemCount: entries.length,
              itemBuilder: (context, index) {
                final entry = entries[entries.length - 1 - index]; // Most recent first
                final isLatest = entry == latestEntry;

                return _HistoryEntryTile(
                  entry: entry,
                  isLatest: isLatest,
                  fieldName: widget.fieldName,
                  formatTimestamp: _formatTimestamp,
                  formatFullTimestamp: _formatFullTimestamp,
                  index: index,
                );
              },
            ),
          ).animate().fadeIn(duration: 200.ms).slideY(begin: -0.1, end: 0),
        ],
      ],
    );
  }
}

class _HistoryEntryTile extends StatelessWidget {
  final FieldHistoryEntry entry;
  final bool isLatest;
  final String fieldName;
  final String Function(DateTime) formatTimestamp;
  final String Function(DateTime) formatFullTimestamp;
  final int index;

  const _HistoryEntryTile({
    required this.entry,
    required this.isLatest,
    required this.fieldName,
    required this.formatTimestamp,
    required this.formatFullTimestamp,
    required this.index,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Timeline indicator
          Column(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: isLatest
                      ? AppTheme.primaryColor
                      : AppTheme.primaryColor.withValues(alpha: 0.4),
                ),
              ),
              if (!isLatest)
                Container(
                  width: 1,
                  height: 24,
                  color: AppTheme.primaryColor.withValues(alpha: 0.2),
                ),
            ],
          ),
          const SizedBox(width: 12),
          // Content
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    if (isLatest)
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: AppTheme.primaryColor.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          'Latest',
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: AppTheme.primaryColor,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    if (isLatest) const SizedBox(width: 8),
                    Text(
                      formatTimestamp(entry.timestamp),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const Spacer(),
                    Tooltip(
                      message: formatFullTimestamp(entry.timestamp),
                      child: Text(
                        formatFullTimestamp(entry.timestamp),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
                          fontSize: 11,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 2),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: entry.values.entries.map<Widget>((e) {
                    final value = e.value;
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 2),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          SizedBox(
                            width: 80,
                            child: Text(
                              e.key,
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ),
                          Expanded(
                            child: Text(
                              value.isNotEmpty ? value : '(empty)',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                fontWeight: isLatest ? FontWeight.w500 : FontWeight.normal,
                                fontStyle: value.isEmpty ? FontStyle.italic : null,
                                color: value.isEmpty
                                    ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6)
                                    : null,
                              ),
                            ),
                          ),
                        ],
                      ),
                    );
                  }).toList(),
                ),
              ],
            ),
          ),
        ],
      ),
    ).animate().fadeIn(delay: (index * 50).ms, duration: 200.ms);
  }
}

/// Shows a timestamp for when a field was last modified
class FieldLastModified extends StatelessWidget {
  final DateTime timestamp;
  final String fieldName;

  const FieldLastModified({
    super.key,
    required this.timestamp,
    required this.fieldName,
  });

  String _formatTimestamp(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);

    if (diff.inDays > 365) {
      return '${(diff.inDays / 365).floor()}y ago';
    } else if (diff.inDays > 30) {
      return '${(diff.inDays / 30).floor()}mo ago';
    } else if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'Just now';
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.access_time,
          size: 12,
          color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
        ),
        const SizedBox(width: 2),
        Text(
          _formatTimestamp(timestamp),
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
            fontSize: 11,
          ),
        ),
      ],
    );
  }
}
