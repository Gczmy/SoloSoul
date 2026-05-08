import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/constants/sensitivity_enums.dart';
import 'package:solosoul_flutter/core/models/scan/scan_result_model.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_provider.dart';
import 'package:solosoul_flutter/presentation/providers/scan/local_search_state.dart';
import 'package:solosoul_flutter/core/services/scan/scan_import_service.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitivity_tag.dart';
import 'package:solosoul_flutter/presentation/widgets/sensitive_value_widget.dart';
import 'package:solosoul_flutter/core/models/sensitivity_models.dart';

// =============================================================================
// Scan Preview Page
// =============================================================================

class ScanPreviewPage extends ConsumerStatefulWidget {
  const ScanPreviewPage({super.key});

  @override
  ConsumerState<ScanPreviewPage> createState() => _ScanPreviewPageState();
}

class _ScanPreviewPageState extends ConsumerState<ScanPreviewPage> {
  bool _isImporting = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(localSearchProvider.notifier).prepareImport();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(localSearchProvider);
    final theme = Theme.of(context);

    final totalCandidates = state.importCandidates.length;
    final selectedCount = state.importCandidates.where((c) => c.isSelected).length;
    final hasConflicts = state.importConflicts.isNotEmpty;

    return Scaffold(
      appBar: SoloGlassAppBar(
        backRoute: AppRoutes.localSearch,
        title: Text(AppLocalizations.of(context).scanPreviewTitle),
        centerTitle: true,
        actions: [
          // AI 智能映射按钮
          if (totalCandidates > 0)
            _AiMappingButton(
              isLoading: state.aiMappingStatus.isLoading,
              onPressed: () => _performAiMapping(),
            ),
          if (totalCandidates > 0)
            Padding(
              padding: const EdgeInsets.only(right: 16),
              child: Center(
                child: Chip(
                  avatar: Icon(
                    Icons.check_circle,
                    size: 18,
                    color: theme.colorScheme.primary,
                  ),
                  label: Text('$selectedCount / $totalCandidates'),
                ),
              ),
            ),
        ],
      ),
      body: state.importCandidates.isEmpty
          ? _ScanPreviewEmptyState(theme: theme)
          : _ScanPreviewList(state: state, theme: theme),
      bottomNavigationBar: state.importCandidates.isEmpty
          ? null
          : _ScanPreviewBottomBar(
              state: state,
              theme: theme,
              hasConflicts: hasConflicts,
              isImporting: _isImporting,
              onImport: _doImport,
              onToggleSelectAll: _toggleSelectAll,
            ),
    );
  }

  void _toggleSelectAll() {
    final state = ref.read(localSearchProvider);
    final allSelected = state.importCandidates.every((c) => c.isSelected);
    final notifier = ref.read(localSearchProvider.notifier);
    for (var i = 0; i < state.importCandidates.length; i++) {
      notifier.setCandidateSelected(i, !allSelected);
    }
  }

  Future<void> _performAiMapping() async {
    await ref.read(localSearchProvider.notifier).performAiMapping();

    if (!mounted) return;
    final state = ref.read(localSearchProvider);
    if (state.aiMappingStatus.isError && state.aiMappingError != null) {
      final error = state.aiMappingError!;
      final isConfigError = error.contains('configure') ||
          error.contains('API Key') ||
          error.contains('permission');
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error),
          backgroundColor: Theme.of(context).colorScheme.error,
          action: isConfigError
              ? SnackBarAction(
                  label: AppLocalizations.of(context).scanGoToConfig,
                  onPressed: () => context.push(AppRoutes.llmConfig),
                )
              : null,
        ),
      );
    } else if (state.aiMappingStatus.isSuccess) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).scanAiMappingComplete)),
      );
    }
  }

  Future<void> _doImport() async {
    setState(() => _isImporting = true);
    try {
      await ref.read(localSearchProvider.notifier).executeImport();
      if (mounted) {
        context.pushReplacement(AppRoutes.scanImportResult);
      }
    } finally {
      if (mounted) setState(() => _isImporting = false);
    }
  }
}

// =============================================================================
// AI Mapping Button
// =============================================================================

class _AiMappingButton extends StatelessWidget {
  final bool isLoading;
  final VoidCallback onPressed;

  const _AiMappingButton({
    required this.isLoading,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (isLoading) {
      return Padding(
        padding: const EdgeInsets.only(right: 8),
        child: SizedBox(
          width: 24,
          height: 24,
          child: CircularProgressIndicator(
            strokeWidth: 2,
            color: theme.colorScheme.primary,
          ),
        ),
      );
    }

    return Tooltip(
      message: AppLocalizations.of(context).scanAiMapping,
      child: IconButton(
        icon: Icon(Icons.auto_fix_high, color: theme.colorScheme.primary),
        onPressed: onPressed,
      ),
    );
  }
}

// =============================================================================
// Candidate Card
// =============================================================================

class _CandidateCard extends ConsumerStatefulWidget {
  final ImportCandidate candidate;
  final int candidateIndex;
  final List<ImportConflict> conflicts;

  const _CandidateCard({
    required this.candidate,
    required this.candidateIndex,
    required this.conflicts,
  });

  @override
  ConsumerState<_CandidateCard> createState() => _CandidateCardState();
}

class _CandidateCardState extends ConsumerState<_CandidateCard> {
  bool _expanded = true;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final candidate = widget.candidate;
    final isNew = candidate.existingObjectId == null;

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Column(
        children: [
          // Header
          InkWell(
            onTap: () => setState(() => _expanded = !_expanded),
            borderRadius: const BorderRadius.vertical(top: Radius.circular(12)),
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Checkbox(
                    value: candidate.isSelected,
                    onChanged: (v) {
                      ref.read(localSearchProvider.notifier)
                          .setCandidateSelected(widget.candidateIndex, v!);
                    },
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(
                              candidate.source.display,
                              style: theme.textTheme.titleMedium?.copyWith(
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const SizedBox(width: 8),
                            if (isNew)
                              _Badge(
                                label: AppLocalizations.of(context).scanPreviewNew,
                                color: theme.colorScheme.primary,
                              )
                            else
                              _Badge(
                                label: AppLocalizations.of(context).scanPreviewUpdate,
                                color: theme.colorScheme.secondary,
                              ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Text(
                          '${candidate.fields.length} field(s)',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  Icon(
                    _expanded ? Icons.expand_less : Icons.expand_more,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),

          // Fields
          if (_expanded)
            Column(
              children: [
                // Attach original file toggle
                CheckboxListTile(
                  dense: true,
                  value: candidate.attachOriginalFile,
                  onChanged: candidate.sourceFilePath != null
                      ? (v) {
                          ref.read(localSearchProvider.notifier)
                              .setAttachFile(widget.candidateIndex, v!);
                        }
                      : null,
                  title: Text(
                    AppLocalizations.of(context).scanAttachFile,
                    style: theme.textTheme.bodySmall,
                  ),
                  contentPadding: EdgeInsets.zero,
                  controlAffinity: ListTileControlAffinity.leading,
                ),
                const Divider(),
                ...candidate.fields.asMap().entries.map((entry) {
                final fieldIndex = entry.key;
                final field = entry.value;
                final isConflict = widget.conflicts.any(
                  (c) => c.field.source.key == field.source.key,
                );

                return _FieldRow(
                  field: field,
                  candidateIndex: widget.candidateIndex,
                  isConflict: isConflict,
                  section: candidate.source.section,
                  onActionChanged: (action) {
                    ref.read(localSearchProvider.notifier)
                        .setFieldAction(widget.candidateIndex, fieldIndex, action);
                  },
                );
              }).toList(),
              ],
            ),
        ],
      ),
    );
  }
}

// =============================================================================
// Field Row
// =============================================================================

class _FieldRow extends StatelessWidget {
  final ImportFieldCandidate field;
  final int candidateIndex;
  final bool isConflict;
  final String? section;
  final ValueChanged<ImportAction> onActionChanged;

  const _FieldRow({
    required this.field,
    required this.candidateIndex,
    required this.isConflict,
    this.section,
    required this.onActionChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final source = field.source;

    Color? rowColor;
    if (isConflict) {
      rowColor = theme.colorScheme.errorContainer.withValues(alpha: 0.3);
    } else if (field.userAction == ImportAction.autoFill) {
      rowColor = theme.colorScheme.primaryContainer.withValues(alpha: 0.2);
    } else if (field.userAction == ImportAction.skip) {
      rowColor = theme.colorScheme.surfaceContainerHighest;
    }

    return Container(
      color: rowColor,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Conflict indicator
          if (isConflict)
            Padding(
              padding: const EdgeInsets.only(right: 8, top: 2),
              child: Icon(Icons.warning_amber, color: theme.colorScheme.error, size: 18),
            ),

          // Field info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      section != null
                          ? FieldRegistry.displayNameForField(section!, source.key)
                          : source.key,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(width: 8),
                    SensitivityTag(level: source.sensitivity),
                    if (source.confidence != null) ...[
                      const SizedBox(width: 8),
                      Text(
                        '${(source.confidence! * 100).toStringAsFixed(0)}%',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                    // AI 映射来源徽章
                    if (field.mappingSource == 'llm' || field.mappingSource == 'both') ...[
                      const SizedBox(width: 8),
                      _Badge(
                        label: field.mappingSource == 'both' ? AppLocalizations.of(context).scanMappingBoth : AppLocalizations.of(context).scanMappingAi,
                        color: theme.colorScheme.tertiary,
                      ),
                      if (field.mappingConfidence < 1.0) ...[
                        const SizedBox(width: 4),
                        Text(
                          '${(field.mappingConfidence * 100).toStringAsFixed(0)}%',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.tertiary,
                            fontSize: 10,
                          ),
                        ),
                      ],
                    ],
                  ],
                ),
                const SizedBox(height: 4),
                // Value with masking for critical/sensitive
                _ValueDisplayWidget(
                  source: source,
                  theme: theme,
                  candidateIndex: candidateIndex,
                ),
              ],
            ),
          ),

          // Action dropdown
          _ActionDropdown(
            currentAction: field.userAction,
            onChanged: onActionChanged,
          ),
        ],
      ),
    );
  }


}

// =============================================================================
// Action Dropdown
// =============================================================================

class _ActionDropdown extends StatelessWidget {
  final ImportAction currentAction;
  final ValueChanged<ImportAction> onChanged;

  const _ActionDropdown({
    required this.currentAction,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    Color iconColor;
    IconData iconData;
    switch (currentAction) {
      case ImportAction.autoFill:
        iconColor = theme.colorScheme.primary;
        iconData = Icons.auto_fix_high;
      case ImportAction.skip:
        iconColor = theme.colorScheme.outline;
        iconData = Icons.skip_next;
      case ImportAction.overwrite:
        iconColor = theme.colorScheme.error;
        iconData = Icons.edit;
      case ImportAction.createNew:
        iconColor = theme.colorScheme.tertiary;
        iconData = Icons.add;
    }

    return PopupMenuButton<ImportAction>(
      initialValue: currentAction,
      onSelected: onChanged,
      icon: Icon(iconData, color: iconColor, size: 20),
      tooltip: AppLocalizations.of(context).scanPreviewImportAction,
      itemBuilder: (context) => [
        _buildMenuItem(ImportAction.autoFill, 'Auto-fill', Icons.auto_fix_high, theme),
        _buildMenuItem(ImportAction.createNew, 'Create new', Icons.add, theme),
        _buildMenuItem(ImportAction.overwrite, 'Overwrite', Icons.edit, theme),
        _buildMenuItem(ImportAction.skip, 'Skip', Icons.skip_next, theme),
      ],
    );
  }

  PopupMenuItem<ImportAction> _buildMenuItem(
    ImportAction action,
    String label,
    IconData icon,
    ThemeData theme,
  ) {
    final isSelected = action == currentAction;
    return PopupMenuItem(
      value: action,
      child: Row(
        children: [
          Icon(
            icon,
            size: 18,
            color: isSelected ? theme.colorScheme.primary : null,
          ),
          const SizedBox(width: 8),
          Text(
            label,
            style: TextStyle(
              color: isSelected ? theme.colorScheme.primary : null,
              fontWeight: isSelected ? FontWeight.w600 : null,
            ),
          ),
          if (isSelected) ...[
            const Spacer(),
            Icon(Icons.check, size: 18, color: theme.colorScheme.primary),
          ],
        ],
      ),
    );
  }
}

// =============================================================================
// Badge
// =============================================================================

class _ScanPreviewEmptyState extends StatelessWidget {
  final ThemeData theme;

  const _ScanPreviewEmptyState({required this.theme});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.inbox_outlined,
            size: 64,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 16),
          Text(
            'No importable items found',
            style: theme.textTheme.titleMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Try scanning with a different depth or more folders.',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: () => context.pop(),
            child: Text(AppLocalizations.of(context).commonBack),
          ),
        ],
      ),
    );
  }
}

class _ScanPreviewList extends StatelessWidget {
  final LocalSearchState state;
  final ThemeData theme;

  const _ScanPreviewList({required this.state, required this.theme});

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 100),
      itemCount: state.importCandidates.length,
      itemBuilder: (context, candidateIndex) {
        final candidate = state.importCandidates[candidateIndex];
        return _CandidateCard(
          candidate: candidate,
          candidateIndex: candidateIndex,
          conflicts: state.importConflicts
              .where((c) => c.candidate.source.section == candidate.source.section)
              .toList(),
        );
      },
    );
  }
}

class _Badge extends StatelessWidget {
  final String label;
  final Color color;

  const _Badge({required this.label, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.3)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _ScanPreviewBottomBar extends StatelessWidget {
  final LocalSearchState state;
  final ThemeData theme;
  final bool hasConflicts;
  final bool isImporting;
  final VoidCallback onImport;
  final VoidCallback onToggleSelectAll;

  const _ScanPreviewBottomBar({
    required this.state,
    required this.theme,
    required this.hasConflicts,
    required this.isImporting,
    required this.onImport,
    required this.onToggleSelectAll,
  });

  @override
  Widget build(BuildContext context) {
    final selectedCount = state.importCandidates.where((c) => c.isSelected).length;

    return SafeArea(
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          border: Border(
            top: BorderSide(color: theme.colorScheme.outlineVariant),
          ),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (hasConflicts)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                margin: const EdgeInsets.only(bottom: 12),
                decoration: BoxDecoration(
                  color: theme.colorScheme.errorContainer.withValues(alpha: 0.5),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Icon(Icons.warning_amber, color: theme.colorScheme.error, size: 18),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        '${state.importConflicts.length} conflict(s) detected. Review before importing.',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onErrorContainer,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            Row(
              children: [
                // Select all / none
                TextButton.icon(
                  onPressed: onToggleSelectAll,
                  icon: Icon(
                    state.importCandidates.every((c) => c.isSelected)
                        ? Icons.deselect
                        : Icons.select_all,
                  ),
                  label: Text(
                    state.importCandidates.every((c) => c.isSelected)
                        ? AppLocalizations.of(context).scanDeselectAll
                        : 'Select All',
                  ),
                ),
                const Spacer(),
                // Import button
                FilledButton.icon(
                  onPressed: selectedCount == 0 || isImporting ? null : onImport,
                  icon: isImporting
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.download),
                  label: Text('${AppLocalizations.of(context).commonImport} ($selectedCount)'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _ValueDisplayWidget extends StatelessWidget {
  final ScanField source;
  final ThemeData theme;
  final int candidateIndex;

  const _ValueDisplayWidget({
    required this.source,
    required this.theme,
    required this.candidateIndex,
  });

  @override
  Widget build(BuildContext context) {
    final isSensitive = source.sensitivity == SensitivityLevel.critical ||
        source.sensitivity == SensitivityLevel.sensitive;

    if (isSensitive) {
      return SensitiveValueWidget(
        fieldId: '${candidateIndex}_${source.key}',
        value: source.value,
        sensitivityLevel: source.sensitivity,
        requireVerification: false,
      );
    }

    return Text(
      source.value,
      style: theme.textTheme.bodyMedium,
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
    );
  }
}
