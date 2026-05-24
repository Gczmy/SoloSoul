import 'package:flutter/material.dart';
import 'package:version/version.dart';

import 'package:solosoul_flutter/core/models/plugin_models.dart';
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 插件详细信息对话框
/// 支持三个 Tab 页面：[插件介绍][更新日志][插件信息]
class PluginDetailDialog extends StatefulWidget {
  final String pluginId;
  final PluginRegistryEntry? registryEntry;
  final frb_manifest.PluginManifest? installedManifest;
  final InstalledPluginInfo? installedInfo;
  final bool isInstalled;

  const PluginDetailDialog({
    super.key,
    required this.pluginId,
    this.registryEntry,
    this.installedManifest,
    this.installedInfo,
    required this.isInstalled,
  });

  @override
  State<PluginDetailDialog> createState() => _PluginDetailDialogState();
}

class _PluginDetailDialogState extends State<PluginDetailDialog>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final name = _resolvePluginName(context);

    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.8,
        ),
        child: ClipRRect(
        borderRadius: BorderRadius.circular(16),
        child: Scaffold(
          appBar: AppBar(
            automaticallyImplyLeading: false,
            title: Text(name),
            actions: [
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.of(context).pop(),
              ),
            ],
            bottom: TabBar(
              controller: _tabController,
              tabs: [
                Tab(text: l10n.pluginDetailTitleIntro),
                Tab(text: l10n.pluginDetailTitleChangelog),
                Tab(text: l10n.pluginDetailTitleInfo),
              ],
            ),
          ),
          body: TabBarView(
            controller: _tabController,
            children: [
              _buildIntroductionTab(theme),
              _buildChangelogTab(theme),
              _buildInfoTab(theme),
            ],
          ),
        ),
        ),
      ),
    );
  }

  String _resolvePluginName(BuildContext context) {
    final locale = Localizations.localeOf(context).toString();
    final i18n = widget.registryEntry?.i18n ?? widget.installedManifest?.i18N;
    final fallback = widget.registryEntry?.name ?? widget.installedManifest?.name ?? widget.pluginId;
    return resolvePluginI18n(i18n, 'name', locale, fallback);
  }

  String? _resolvePluginDescription(BuildContext context) {
    final locale = Localizations.localeOf(context).toString();
    final i18n = widget.registryEntry?.i18n ?? widget.installedManifest?.i18N;
    final fallback = widget.registryEntry?.description ?? widget.installedManifest?.description;
    return resolvePluginI18n(i18n, 'description', locale, fallback ?? '');
  }

  Widget _buildIntroductionTab(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    final description = _resolvePluginDescription(context);
    final requiredFields = widget.installedManifest?.requiredFields ?? [];
    final optionalFields = widget.installedManifest?.optionalFields ?? [];

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (description != null && description.isNotEmpty) ...[
            Text(
              l10n.pluginDetailFeatureIntro,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              description,
              style: theme.textTheme.bodyMedium,
            ),
            const SizedBox(height: 24),
          ],
          if (requiredFields.isNotEmpty || optionalFields.isNotEmpty) ...[
            Text(
              l10n.pluginDetailRequiredFields,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            if (requiredFields.isNotEmpty) ...[
              _buildFieldChipGroup(l10n.pluginDetailRequired, requiredFields, theme.colorScheme.primary),
              const SizedBox(height: 8),
            ],
            if (optionalFields.isNotEmpty)
              _buildFieldChipGroup(l10n.pluginDetailOptional, optionalFields, theme.colorScheme.outline),
            const SizedBox(height: 24),
          ],
          if (widget.registryEntry != null) ...[
            Text(
              l10n.pluginDetailVersionCompat,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            _buildInfoRow(l10n.pluginDetailMinAppVersion, widget.registryEntry!.versions.values.first.minAppVersion),
            _buildInfoRow(l10n.pluginDetailMaxAppVersion, widget.registryEntry!.versions.values.first.maxAppVersion),
            _buildInfoRow(l10n.pluginDetailPluginApiVersion, widget.registryEntry!.versions.values.first.pluginApiVersion),
          ],
        ],
      ),
    );
  }

  Widget _buildFieldChipGroup(String label, List<String> fields, Color borderColor) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            fontSize: 12,
            color: Colors.grey.shade600,
          ),
        ),
        const SizedBox(height: 4),
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: fields.map((f) {
            return Chip(
              label: Text(
                f,
                style: const TextStyle(fontSize: 11),
              ),
              visualDensity: VisualDensity.compact,
              backgroundColor: Colors.transparent,
              side: BorderSide(color: borderColor.withValues(alpha: 0.5)),
              padding: EdgeInsets.zero,
              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
            );
          }).toList(),
        ),
      ],
    );
  }

  Widget _buildChangelogTab(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    final versions = widget.registryEntry?.versions.entries.toList();
    if (versions == null || versions.isEmpty) {
      return Center(child: Text(l10n.pluginDetailNoVersions));
    }

    versions.sort((a, b) {
      try {
        return Version.parse(b.key).compareTo(Version.parse(a.key));
      } on Exception {
        return b.key.compareTo(a.key);
      }
    });

    return ListView.separated(
      padding: const EdgeInsets.all(16),
      itemCount: versions.length,
      separatorBuilder: (_, __) => const Divider(height: 32),
      itemBuilder: (context, index) {
        final ver = versions[index].key;
        final info = versions[index].value;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: index == 0
                          ? theme.colorScheme.primary.withValues(alpha: 0.5)
                          : theme.colorScheme.outline.withValues(alpha: 0.5),
                    ),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'v$ver',
                    style: TextStyle(
                      fontWeight: FontWeight.bold,
                      fontSize: 13,
                      color: index == 0
                          ? theme.colorScheme.primary
                          : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  info.releasedAt.toLocal().toString().split(' ').first,
                  style: TextStyle(
                    fontSize: 12,
                    color: Colors.grey.shade600,
                  ),
                ),
                if (widget.isInstalled &&
                    ver == (widget.installedManifest?.version ?? widget.installedInfo?.version))
                  Padding(
                    padding: const EdgeInsets.only(left: 8),
                    child: Chip(
                      label: Text(l10n.pluginDetailCurrent, style: const TextStyle(fontSize: 10)),
                      visualDensity: VisualDensity.compact,
                      backgroundColor: Colors.transparent,
                      side: BorderSide(color: theme.colorScheme.primary.withValues(alpha: 0.5)),
                      padding: EdgeInsets.zero,
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            if (info.changelog != null && info.changelog!.isNotEmpty)
              Text(
                info.changelog!,
                style: theme.textTheme.bodyMedium,
              )
            else
              Text(
                l10n.pluginDetailNoChangelog,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: Colors.grey.shade500,
                  fontStyle: FontStyle.italic,
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _buildInfoTab(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _buildInfoSection(
            theme,
            title: l10n.pluginDetailBasicInfo,
            rows: {
              l10n.pluginDetailPluginId: widget.pluginId,
              l10n.pluginDetailPluginName: _resolvePluginName(context),
              l10n.pluginDetailPublisher: widget.registryEntry?.publisher ?? widget.installedManifest?.publisher ?? '-',
              if (widget.registryEntry?.homepage != null)
                l10n.pluginDetailHomepage: widget.registryEntry!.homepage!,
            },
          ),
          const SizedBox(height: 24),
          _buildInfoSection(
            theme,
            title: l10n.pluginDetailInstallInfo,
            rows: {
              l10n.pluginDetailStatus: widget.isInstalled ? l10n.pluginDetailStatusInstalled : l10n.pluginDetailStatusNotInstalled,
              if (widget.isInstalled) ...{
                l10n.pluginDetailInstalledVersion: widget.installedManifest?.version ??
                    widget.installedInfo?.version ??
                    '-',
                l10n.pluginDetailLatestVersion: widget.registryEntry?.latestVersion ?? '-',
                if (widget.installedInfo?.installedAt != null)
                  l10n.pluginDetailInstallTime: _formatDateTime(widget.installedInfo!.installedAt!),
                if (widget.installedInfo?.lastUsedAt != null)
                  l10n.pluginDetailLastUsed: _formatDateTime(widget.installedInfo!.lastUsedAt!)
                else if (widget.isInstalled)
                  l10n.pluginDetailLastUsed: l10n.pluginDetailNeverUsed,
              },
            },
          ),
        ],
      ),
    );
  }

  Widget _buildInfoSection(
    ThemeData theme, {
    required String title,
    required Map<String, String> rows,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: theme.textTheme.titleSmall?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 12),
        ...rows.entries.map((e) => _buildInfoRow(e.key, e.value)),
      ],
    );
  }

  Widget _buildInfoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              softWrap: false,
              style: TextStyle(
                fontSize: 13,
                color: Colors.grey.shade600,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(fontSize: 13),
            ),
          ),
        ],
      ),
    );
  }

  String _formatDateTime(DateTime dt) {
    final local = dt.toLocal();
    return '${local.year}-${local.month.toString().padLeft(2, '0')}-${local.day.toString().padLeft(2, '0')} '
        '${local.hour.toString().padLeft(2, '0')}:${local.minute.toString().padLeft(2, '0')}';
  }
}
