import 'dart:convert' show jsonDecode, jsonEncode;
import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:version/version.dart';

import 'package:solosoul_flutter/frb/api.dart' as frb;
import 'package:solosoul_flutter/frb/plugin/manager.dart' as frb_plugin;
import 'package:solosoul_flutter/frb/plugin/manifest.dart' as frb_manifest;
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/providers/plugin_provider.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';
import 'package:solosoul_flutter/core/models/plugin_models.dart' show PluginRegistryEntry, resolvePluginI18n;
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_access_review_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_consent_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_detail_dialog.dart';
import 'package:solosoul_flutter/presentation/widgets/plugin_radio_list_dialog.dart';
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard/plugin_result_dialog.dart';
import 'package:solosoul_flutter/presentation/pages/plugin_dashboard_page.dart'
    show PluginRunSession, PluginResultData, getPluginManifest;

// ============================================================================
// Plugin Card
// ============================================================================

class PluginCard extends ConsumerWidget {
  final String pluginId;
  final PluginDashboardData data;

  const PluginCard({super.key, required this.pluginId, required this.data});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final manifest = getPluginManifest(data, pluginId);
    final registryEntry = data.registry.plugins[pluginId];
    final locale = Localizations.localeOf(context).toString();
    final name = resolvePluginI18n(
      registryEntry?.i18n, 'name', locale, manifest?.name ?? pluginId,
    );
    final version = manifest?.version ?? '';
    final publisher = manifest?.publisher ?? '';

    final isInstalled = data.isInstalled(pluginId);
    final isRunning = data.isRunning(pluginId);
    final hasUpdate = data.hasUpdate(pluginId);
    final installingMap = ref.watch(pluginInstallingProvider);
    final isUpdating = installingMap[pluginId] ?? false;

    // 确定状态标签
    String statusLabel;
    Color statusColor;
    if (isRunning) {
      statusLabel = l10n.pluginStatusRunning;
      statusColor = Colors.purple;
    } else if (hasUpdate) {
      statusLabel = l10n.pluginStatusUpdateAvailable;
      statusColor = Colors.orange;
    } else if (isInstalled) {
      statusLabel = l10n.pluginStatusInstalled;
      statusColor = Colors.green;
    } else if (registryEntry != null) {
      statusLabel = l10n.pluginStatusNotInstalled;
      statusColor = Colors.grey;
    } else {
      statusLabel = l10n.pluginStatusIncompatible;
      statusColor = Colors.red;
    }

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        name,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 16,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Row(
                        children: [
                          Text(
                            '$publisher · v$version',
                            style: TextStyle(
                              fontSize: 13,
                              color: Colors.grey.shade600,
                            ),
                          ),
                          if (registryEntry != null)
                            GestureDetector(
                              onTap: () => _showVersionHistory(context, ref),
                              child: Padding(
                                padding: const EdgeInsets.only(left: 6),
                                child: Icon(
                                  Icons.history,
                                  size: 14,
                                  color: Theme.of(context).colorScheme.primary,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: statusColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Text(
                    statusLabel,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                      color: statusColor,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              children: _buildActionButtons(context, ref, l10n, isInstalled, isRunning, hasUpdate, isUpdating),
            ),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildActionButtons(
    BuildContext context,
    WidgetRef ref,
    AppLocalizations l10n,
    bool isInstalled,
    bool isRunning,
    bool hasUpdate,
    bool isUpdating,
  ) {
    final buttons = <Widget>[];

    if (!isInstalled) {
      // 未安装：显示安装按钮（安装中时显示 loading）
      final dash = ref.read(pluginDashboardProvider).asData?.value;
      if (dash != null && dash.registry.plugins.containsKey(pluginId)) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: isUpdating ? null : () => _onInstall(context, ref),
            icon: isUpdating
                ? const SizedBox(
                    width: 14,
                    height: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.download_rounded, size: 16),
            label: Text(isUpdating ? '安装中' : l10n.pluginActionInstall),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      }
    } else {
      // 已安装：显示运行/停止按钮
      if (isRunning) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: Platform.isIOS ? null : () => _onStop(context, ref),
            icon: const Icon(Icons.stop_rounded, size: 16),
            label: Text(l10n.pluginActionStop),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      } else {
        buttons.push(
          OutlinedButton.icon(
            onPressed: Platform.isIOS ? null : () => _onRun(context, ref),
            icon: const Icon(Icons.play_arrow_rounded, size: 16),
            label: Text(l10n.pluginActionRun),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      }

      // 有更新：显示更新按钮（更新中时显示 loading）
      if (hasUpdate) {
        buttons.push(
          OutlinedButton.icon(
            onPressed: isUpdating ? null : () => _onUpdate(context, ref),
            icon: isUpdating
                ? const SizedBox(
                    width: 14,
                    height: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.update_rounded, size: 16),
            label: Text(isUpdating ? '更新中' : l10n.pluginActionUpdate),
            style: OutlinedButton.styleFrom(
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              visualDensity: VisualDensity.compact,
              textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
            ),
          ),
        );
      }

      // 卸载按钮
      buttons.push(
        TextButton(
          onPressed: () => _onUninstall(context, ref),
          style: TextButton.styleFrom(foregroundColor: AppTheme.errorColor),
          child: Text(l10n.pluginActionUninstall),
        ),
      );
    }

    // 详细信息按钮（始终显示在最右侧）
    buttons.push(
      TextButton(
        onPressed: () => _showPluginDetail(context, ref),
        child: Text(l10n.pluginActionDetail),
      ),
    );

    return buttons;
  }

  Future<void> _showPluginDetail(BuildContext context, WidgetRef ref) async {
    final installer = await ref.read(initializedPluginInstallerProvider.future);
    final installedInfo = await installer.getInstalledInfo(pluginId);

    frb_manifest.PluginManifest? installedManifest;
    if (data.isInstalled(pluginId)) {
      for (final m in data.installed) {
        if (m.pluginId == pluginId) {
          installedManifest = m;
          break;
        }
      }
    }

    if (context.mounted) {
      await showDialog<void>(
        context: context,
        builder: (ctx) => PluginDetailDialog(
          pluginId: pluginId,
          registryEntry: data.registry.plugins[pluginId],
          installedManifest: installedManifest,
          installedInfo: installedInfo,
          isInstalled: data.isInstalled(pluginId),
        ),
      );
    }
  }

  Future<void> _onInstall(BuildContext context, WidgetRef ref) async {
    await _performInstallOrUpdate(context, ref, isUpdate: false);
  }

  Future<void> _onUpdate(BuildContext context, WidgetRef ref) async {
    await _performInstallOrUpdate(context, ref, isUpdate: true);
  }

  Future<void> _performInstallOrUpdate(
    BuildContext context,
    WidgetRef ref, {
    required bool isUpdate,
    String? targetVersion,
  }) async {
    final l10n = AppLocalizations.of(context);
    final dashboard = ref.read(pluginDashboardProvider).asData?.value;
    if (dashboard == null) return;

    final entry = dashboard.registry.plugins[pluginId];
    if (entry == null) return;

    // 标记安装/更新中
    ref.read(pluginInstallingProvider.notifier).setLoading(pluginId, true);

    try {
      final packageInfo = await PackageInfo.fromPlatform();
      final versionKey = targetVersion ?? entry.latestVersion;
      final versionInfo = entry.versions[versionKey];
      final appVersion = packageInfo.version;
      final pluginApiVersion = versionInfo?.pluginApiVersion ?? '1.0';

      // 1. 下载插件工件（wasm + manifest）
      final artifacts = await downloadPluginArtifacts(
        ref,
        pluginId,
        entry,
        appVersion,
        pluginApiVersion,
        targetVersion: targetVersion,
      );
      if (!context.mounted) return;

      // 2. 解析 field_access 并进行安装前审查
      final fieldAccess = artifacts.parseFieldAccess();
      if (fieldAccess != null &&
          fieldAccess.isNotEmpty &&
          context.mounted) {
        final shouldContinue = await _showAccessReview(
          context,
          ref,
          entry,
          fieldAccess,
        );
        if (!context.mounted) return;
        if (!shouldContinue) return;
      }

      // 3. 执行安装
      final installer = await ref.read(initializedPluginInstallerProvider.future);
      if (!context.mounted) return;
      await installer.installFromArtifacts(artifacts);
      if (!context.mounted) return;

      // 4. 局部更新安装状态，避免全页刷新
      final manifest = artifacts.toManifest();
      if (manifest != null) {
        ref.read(pluginDashboardProvider.notifier).addInstalledPlugin(manifest);
      }

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              isUpdate ? l10n.pluginUpdateSuccess : l10n.pluginInstallSuccess,
            ),
          ),
        );
      }
    } on Exception catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${l10n.commonError}: $e')),
        );
      }
    } finally {
      // 清除安装/更新中状态
      ref.read(pluginInstallingProvider.notifier).clear(pluginId);
    }
  }

  /// 显示插件字段访问审查弹窗。
  /// 返回 `true` 表示用户选择继续安装，`false` 表示取消。
  Future<bool> _showAccessReview(
    BuildContext context,
    WidgetRef ref,
    PluginRegistryEntry entry,
    List<Map<String, dynamic>> fieldAccess,
  ) async {
    final languageCode = Localizations.localeOf(context).languageCode;
    final pluginName = resolvePluginI18n(
      entry.i18n, 'name', languageCode, entry.name,
    );

    // 构建 FieldAccessStatus 列表（基于 manifest 声明，不扫描实际数据）
    final fieldStatuses = fieldAccess.map((access) {
      final semanticType = access['semantic_type'] as String?;
      final key = access['key'] as String?;
      final requiredSensitivityStr = access['required_sensitivity'] as String?;

      final requiredSensitivity = _parseSensitivity(requiredSensitivityStr);

      // 尝试从语义类型注册表获取标签
      String? fieldLabel;
      if (semanticType != null) {
        final type = SemanticTypeRegistry.getType(semanticType);
        fieldLabel = type?.getLabel(languageCode) ?? semanticType;
      }

      return FieldAccessStatus(
        fieldKey: key,
        fieldLabel: fieldLabel ?? key ?? semanticType ?? 'Unknown',
        semanticType: semanticType,
        sectionName: null,
        actualSensitivity: null,
        requiredSensitivity: requiredSensitivity,
        status: AccessStatus.ok,
      );
    }).toList();

    if (!context.mounted) return false;

    final result = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => PluginAccessReviewDialog(
        pluginName: pluginName,
        fieldStatuses: fieldStatuses,
        onModifySensitivity: () {
          Navigator.of(ctx).pop(false);
        },
        onCreateMissingFields: () {
          Navigator.of(ctx).pop(false);
        },
        onContinueInstall: () => Navigator.of(ctx).pop(true),
        onCancel: () => Navigator.of(ctx).pop(false),
      ),
    );

    return result == true;
  }

  SensitivityLevel? _parseSensitivity(String? value) {
    return switch (value?.toLowerCase()) {
      'public' => SensitivityLevel.public,
      'internal' => SensitivityLevel.internal,
      'private' => SensitivityLevel.internal,
      'sensitive' => SensitivityLevel.sensitive,
      'restricted' => SensitivityLevel.sensitive,
      'critical' => SensitivityLevel.critical,
      _ => null,
    };
  }

  Future<void> _showVersionHistory(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final registryEntry = data.registry.plugins[pluginId];
    if (registryEntry == null) return;

    final installedVersion = data.installedVersion(pluginId);
    final versions = registryEntry.versions.entries.toList()
      ..sort((a, b) {
        try {
          return Version.parse(b.key).compareTo(Version.parse(a.key));
        } on Exception {
          return b.key.compareTo(a.key);
        }
      });

    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) {
        return DraggableScrollableSheet(
          initialChildSize: 0.6,
          minChildSize: 0.3,
          maxChildSize: 0.9,
          expand: false,
          builder: (_, scrollController) {
            return Column(
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          l10n.pluginVersionHistoryTitle,
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close),
                        onPressed: () => Navigator.of(ctx).pop(),
                        visualDensity: VisualDensity.compact,
                      ),
                    ],
                  ),
                ),
                const Divider(height: 1),
                Expanded(
                  child: ListView.builder(
                    controller: scrollController,
                    itemCount: versions.length,
                    itemBuilder: (context, index) {
                      final ver = versions[index].key;
                      final info = versions[index].value;
                      final isCurrent = ver == installedVersion;

                      return ListTile(
                        leading: Container(
                          width: 56,
                          alignment: Alignment.center,
                          child: isCurrent
                              ? Icon(
                                  Icons.check_circle,
                                  color: Theme.of(context).colorScheme.primary,
                                )
                              : Text(
                                  'v$ver',
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w600,
                                    fontSize: 12,
                                  ),
                                ),
                        ),
                        title: Text(
                          isCurrent ? l10n.pluginVersionCurrentLabel(ver) : 'v$ver',
                          style: TextStyle(
                            fontWeight: isCurrent ? FontWeight.bold : FontWeight.normal,
                            color: isCurrent
                                ? Theme.of(context).colorScheme.primary
                                : null,
                          ),
                        ),
                        subtitle: Builder(
                          builder: (context) {
                            final changelog = info.changelog;
                            return Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  info.releasedAt.toLocal().toString().split(' ').first,
                                  style: TextStyle(
                                    fontSize: 12,
                                    color: Colors.grey.shade600,
                                  ),
                                ),
                                if (changelog != null && changelog.isNotEmpty)
                                  Padding(
                                    padding: const EdgeInsets.only(top: 4),
                                    child: Text(
                                      changelog,
                                      style: Theme.of(context).textTheme.bodySmall,
                                    ),
                                  ),
                              ],
                            );
                          },
                        ),
                        trailing: isCurrent
                            ? Chip(
                                label: Text(l10n.pluginDetailCurrent),
                                visualDensity: VisualDensity.compact,
                                backgroundColor: Colors.transparent,
                                side: BorderSide(
                                  color: Theme.of(context)
                                      .colorScheme
                                      .primary
                                      .withValues(alpha: 0.5),
                                ),
                              )
                            : TextButton(
                                onPressed: () async {
                                  Navigator.of(ctx).pop();
                                  await _performInstallOrUpdate(
                                    context,
                                    ref,
                                    isUpdate: data.isInstalled(pluginId),
                                    targetVersion: ver,
                                  );
                                },
                                child: Text(l10n.pluginActionInstall),
                              ),
                      );
                    },
                  ),
                ),
              ],
            );
          },
        );
      },
    );
  }

  /// 为特定插件准备初始场景参数。
  /// 用户取消时返回 null，调用方应终止流程。
  Future<Map<String, dynamic>?> _prepareInitialParams(BuildContext context) async {
    if (pluginId == 'com.solosoul.official.doc-checklist') {
      final scenarioResult = await _showDocChecklistScenarioDialog(context);
      if (scenarioResult == null) return null;
      return {
        'scenario_id': scenarioResult['id'],
        'fields': scenarioResult['fields'],
      };
    }
    if (pluginId == 'com.solosoul.official.form-prefiller') {
      if (!context.mounted) return null;
      final scenarioResult = await _showFormPrefillerScenarioDialog(context);
      if (scenarioResult == null) return null;
      return {
        'scenario_id': scenarioResult['id'],
      };
    }
    return null;
  }

  Future<void> _onRun(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);

    final initialParams = await _prepareInitialParams(context);
    if (initialParams == null && (
      pluginId == 'com.solosoul.official.doc-checklist' ||
      pluginId == 'com.solosoul.official.form-prefiller'
    )) {
      return;
    }

    final stream = runPlugin(ref, pluginId, params: initialParams);
    final session = PluginRunSession();

    try {
      await for (final event in stream) {
        if (!context.mounted) break;
        switch (event) {
          case frb_plugin.PluginEvent_ConsentRequest(
              requestId: final reqId,
              field: '__dialog__',
            ):
            await _handleDialogConsent(context, reqId, session, initialParams);
          case frb_plugin.PluginEvent_ConsentRequest(
              requestId: final reqId,
              field: final field,
              sensitivity: final sensitivityStr,
              pluginName: final pname,
            ):
            await _handleFieldConsent(context, reqId, field, sensitivityStr, pname, session);
          case frb_plugin.PluginEvent_Result(jsonData: final jsonData):
            _handlePluginResult(jsonData, session);
          case frb_plugin.PluginEvent_Log(level: final level, message: final message):
            await _handlePluginLog(context, level, message, session);
          case frb_plugin.PluginEvent_Completed(exitCode: final exitCode):
            await _handlePluginCompleted(exitCode, ref, session);
          case frb_plugin.PluginEvent_Error(message: final message):
            _handlePluginError(context, message, l10n);
          default:
            break;
        }
      }

      if (session.hasCompleted && context.mounted) {
        await _showRunResult(context, session);
      }
    } on Exception catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${l10n.commonError}: $e')),
        );
      }
    }
  }

  Future<void> _handleDialogConsent(
    BuildContext context,
    String reqId,
    PluginRunSession session,
    Map<String, dynamic>? initialParams,
  ) async {
    debugPrint('[plugin_dialog] received __dialog__ ConsentRequest reqId=$reqId');

    if (initialParams != null && initialParams['scenario_id'] != null) {
      debugPrint('[plugin_dialog] skipping dialog, using pre-selected scenario: ${initialParams['scenario_id']}');
      await frb.frbPluginConsentResponse(
        requestId: reqId,
        approved: true,
        value: jsonEncode({'selected': initialParams['scenario_id']}),
      );
      return;
    }

    var configJson = session.dialogConfigs.remove(reqId);
    for (var retry = 0; retry < 10 && configJson == null; retry++) {
      await Future.delayed(const Duration(milliseconds: 100));
      configJson = session.dialogConfigs.remove(reqId);
    }
    if (configJson == null) {
      debugPrint('[plugin_dialog] config NOT FOUND for reqId=$reqId');
    }
    if (configJson == null || !context.mounted) {
      await frb.frbPluginConsentResponse(requestId: reqId, approved: false);
      return;
    }

    try {
      final config = jsonDecode(configJson) as Map<String, dynamic>;
      final type = config['type'] as String?;
      if (type != 'radio_list') {
        await frb.frbPluginConsentResponse(requestId: reqId, approved: false);
        return;
      }

      final locale = Localizations.localeOf(context);
      String resolveL10n(dynamic raw) {
        return switch (raw) {
          Map<String, dynamic> map =>
            map[locale.toString()] ??
            map[locale.languageCode] ??
            map['en'] ??
            map.values.first as String,
          String s => s,
          _ => '',
        };
      }

      final items = (config['items'] as List).map((e) {
        final map = e as Map<String, dynamic>;
        return PluginRadioItem(
          id: map['id'] as String,
          label: resolveL10n(map['label']),
        );
      }).toList();

      final selected = await showDialog<String>(
        context: context,
        builder: (_) => PluginRadioListDialog(
          title: resolveL10n(config['title']),
          description: config['description'] != null
              ? resolveL10n(config['description'])
              : null,
          items: items,
        ),
      );

      await frb.frbPluginConsentResponse(
        requestId: reqId,
        approved: selected != null,
        value: selected != null ? jsonEncode({'selected': selected}) : null,
      );
    } on Exception catch (_) {
      await frb.frbPluginConsentResponse(requestId: reqId, approved: false);
    }
  }

  Future<void> _handleFieldConsent(
    BuildContext context,
    String reqId,
    String field,
    String sensitivityStr,
    String pname,
    PluginRunSession session,
  ) async {
    if (session.batchPreConsentPhase) {
      session.batchRequests.add(
        frb_plugin.PluginEvent_ConsentRequest(
          requestId: reqId,
          pluginId: pluginId,
          pluginName: pname,
          field: field,
          sensitivity: sensitivityStr,
        ),
      );
      if (session.batchPluginName == null) {
        final entry = data.registry.plugins[pluginId];
        if (!context.mounted) return;
        final languageCode = Localizations.localeOf(context).languageCode;
        session.batchPluginName = resolvePluginI18n(
          entry?.i18n, 'name', languageCode, pname,
        );
      }
      return;
    }

    debugPrint('[plugin_consent] runtime single consent for field=$field');
    if (!context.mounted) {
      await frb.frbPluginConsentResponse(requestId: reqId, approved: false);
      return;
    }
    final approved = await showPluginConsentDialog(
      context: context,
      pluginId: pluginId,
      pluginName: session.batchPluginName ?? pname,
      fieldId: field,
      requestId: reqId,
      sensitivity: _parseSensitivity(sensitivityStr) ?? SensitivityLevel.sensitive,
    );
    await frb.frbPluginConsentResponse(
      requestId: reqId,
      approved: approved == true,
    );
  }

  void _handlePluginResult(String jsonData, PluginRunSession session) {
    session.batchPreConsentPhase = false;
    try {
      session.pluginResults.add(PluginResultData.fromJson(jsonData));
    } on Exception catch (e) {
      session.pluginResults.add(PluginResultData(
        type: 'text',
        data: {'content': '结果解析失败: $e\n\n原始数据:\n$jsonData'},
      ));
      session.pluginLogs.add('[结果解析错误] $e');
    }
  }

  Future<void> _handlePluginLog(
    BuildContext context,
    String level,
    String message,
    PluginRunSession session,
  ) async {
    if (level == 'dialog_config') {
      final idx = message.indexOf('|');
      if (idx > 0) {
        final reqId = message.substring(0, idx);
        final config = message.substring(idx + 1);
        session.dialogConfigs[reqId] = config;
        debugPrint('[plugin_dialog] cached config for reqId=$reqId');
      }
      return;
    }

    if (level == 'batch_end') {
      session.batchPreConsentPhase = false;
      if (session.batchRequests.isNotEmpty) {
        if (!context.mounted) return;
        final approved = await showDialog<bool>(
          context: context,
          barrierDismissible: false,
          builder: (ctx) => PluginBatchConsentDialog(
            pluginId: pluginId,
            pluginName: session.batchPluginName ?? pluginId,
            requests: session.batchRequests.map((r) => BatchConsentRequest(
              requestId: r.requestId,
              field: r.field,
              sensitivity: r.sensitivity,
            )).toList(),
          ),
        );
        for (final req in session.batchRequests) {
          try {
            await frb.frbPluginConsentResponse(
              requestId: req.requestId,
              approved: approved == true,
              value: null,
            );
          } on Exception catch (_) {}
        }
        session.batchRequests.clear();
      }
    }

    if (level == 'info' || level == 'error') {
      session.batchPreConsentPhase = false;
    }
    if (level == 'info' && message.isNotEmpty && !message.startsWith('pre-consent|')) {
      session.pluginLogs.add(message);
    }
    if (level == 'error') {
      session.errorMessages.add(message);
    }
  }

  Future<void> _handlePluginCompleted(
    int exitCode,
    WidgetRef ref,
    PluginRunSession session,
  ) async {
    session.batchPreConsentPhase = false;
    final installer = await ref.read(initializedPluginInstallerProvider.future);
    await installer.recordLastUsed(pluginId);
    session.completedExitCode = exitCode;
    session.hasCompleted = true;
  }

  void _handlePluginError(
    BuildContext context,
    String message,
    AppLocalizations l10n,
  ) {
    if (message.contains('User denied or timed out field access')) return;
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('${l10n.commonError}: $message')),
      );
    }
  }

  Future<void> _showRunResult(
    BuildContext context,
    PluginRunSession session,
  ) async {
    final registryEntry = data.registry.plugins[pluginId];
    final languageCode = Localizations.localeOf(context).languageCode;
    final pluginName = resolvePluginI18n(
      registryEntry?.i18n, 'name', languageCode, getPluginManifest(data, pluginId)?.name ?? pluginId,
    );
    await _showExecutionResult(
      context: context,
      pluginName: pluginName,
      pluginLogs: session.pluginLogs,
      pluginResults: session.pluginResults,
      errorMessages: session.errorMessages,
      exitCode: session.completedExitCode!,
    );
  }

  /// Stream 结束后统一展示执行结果对话框。
  Future<void> _showExecutionResult({
    required BuildContext context,
    required String pluginName,
    required List<String> pluginLogs,
    required List<PluginResultData> pluginResults,
    required List<String> errorMessages,
    required int exitCode,
  }) async {
    final l10n = AppLocalizations.of(context);

    if (pluginLogs.isNotEmpty || pluginResults.isNotEmpty) {
      // 有日志或结构化结果：弹出结果展示对话框
      await showDialog<void>(
        context: context,
        builder: (ctx) => PluginResultDialog(
          pluginName: pluginName,
          logs: pluginLogs,
          results: pluginResults,
          exitCode: exitCode,
          hasErrors: exitCode != 0 && errorMessages.isNotEmpty,
        ),
      );
    } else if (exitCode == 0) {
      // 无日志但执行成功：弹出执行完成确认对话框
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          insetPadding: EdgeInsets.symmetric(
            horizontal: MediaQuery.of(ctx).size.width * 0.25,
            vertical: 24,
          ),
          title: Row(
            children: [
              Icon(Icons.check_circle, color: Colors.green.shade600),
              const SizedBox(width: 8),
              Expanded(child: Text(pluginName)),
            ],
          ),
          content: Text(l10n.pluginRunSuccess),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(l10n.commonClose),
            ),
          ],
        ),
      );
    } else {
      // 执行失败：弹出错误对话框
      final errorMsg = errorMessages.isNotEmpty
          ? errorMessages.join('\n')
          : '插件执行失败 (exit: $exitCode)';
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          insetPadding: EdgeInsets.symmetric(
            horizontal: MediaQuery.of(ctx).size.width * 0.2,
            vertical: 24,
          ),
          title: Row(
            children: [
              const Icon(Icons.error_outline, color: AppTheme.errorColor),
              const SizedBox(width: 8),
              Expanded(child: Text(pluginName)),
            ],
          ),
          content: SelectableText(errorMsg),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(l10n.commonClose),
            ),
          ],
        ),
      );
    }
  }

  Future<void> _onStop(BuildContext context, WidgetRef ref) async {
    final service = ref.read(pluginServiceProvider);
    await service.initialize();
    await service.forceUnload(pluginId);
    ref.invalidate(activeSessionsProvider);
  }

  Future<void> _onUninstall(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.pluginUninstallConfirmTitle),
        content: Text(l10n.pluginUninstallConfirmMessage),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(backgroundColor: AppTheme.errorColor),
            child: Text(l10n.pluginActionUninstall),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      try {
        final installer = await ref.read(initializedPluginInstallerProvider.future);
        if (!context.mounted) return;
        await installer.uninstall(pluginId);
        if (!context.mounted) return;

        // 清除 Riverpod 缓存，确保刷新时不再显示已卸载的插件
        ref.invalidate(installedPluginsProvider);

        // 局部更新安装状态，避免全页刷新
        ref.read(pluginDashboardProvider.notifier).removeInstalledPlugin(pluginId);

        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(l10n.pluginUninstallSuccess)),
          );
        }
      } on Exception catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('${l10n.commonError}: $e')),
          );
        }
      }
    }
  }

  /// 材料清单插件场景选择对话框
  /// 返回 {id, fields} 或 null（用户取消）
  Future<Map<String, dynamic>?> _showDocChecklistScenarioDialog(BuildContext context) async {
    final locale = Localizations.localeOf(context);

    // 场景定义（与 Rust 插件源码及 scenarios.json 保持一致）
    final scenarios = [
      {
        'id': 'japan-visa',
        'label': {'zh': '日本签证', 'en': 'Japan Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'employment.company', 'financial.bankStatement', 'travel.itinerary', 'travel.hotelBooking'],
      },
      {
        'id': 'us-visa',
        'label': {'zh': '美国签证 (B1/B2)', 'en': 'US Visa (B1/B2)'},
        'fields': ['passport.number', 'identity.idPhoto', 'visa.ds160Confirmation', 'visa.interviewAppointment', 'financial.bankStatement', 'employment.company'],
      },
      {
        'id': 'schengen-visa',
        'label': {'zh': '申根签证', 'en': 'Schengen Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'insurance.travel', 'travel.itinerary', 'travel.hotelBooking', 'financial.bankStatement', 'employment.company'],
      },
      {
        'id': 'uk-visa',
        'label': {'zh': '英国签证', 'en': 'UK Visa'},
        'fields': ['passport.number', 'identity.idPhoto', 'medical.tbTest', 'visa.casLetter', 'financial.bankStatement', 'travel.hotelBooking'],
      },
      {
        'id': 'bank-account',
        'label': {'zh': '银行开户', 'en': 'Bank Account'},
        'fields': ['passport.number', 'address.street', 'employment.company'],
      },
      {
        'id': 'hotel-checkin',
        'label': {'zh': '酒店入住', 'en': 'Hotel Check-in'},
        'fields': ['passport.number', 'travel.hotelBooking', 'card.number'],
      },
    ];

    String resolveL10n(Map<String, String> map) {
      return map[locale.toString()] ??
          map[locale.languageCode] ??
          map['en'] ??
          map.values.first;
    }

    final items = scenarios.map((s) {
      return PluginRadioItem(
        id: s['id'] as String,
        label: resolveL10n(s['label'] as Map<String, String>),
      );
    }).toList();

    final selected = await showDialog<String>(
      context: context,
      builder: (_) => PluginRadioListDialog(
        title: '选择签证/业务类型',
        description: '选择场景后，插件将请求访问相关字段，请继续授权。',
        items: items,
      ),
    );

    if (selected == null) return null;

    final scenario = scenarios.firstWhere((s) => s['id'] == selected);
    return {
      'id': selected,
      'fields': scenario['fields'],
    };
  }

  /// 表单预填插件场景选择对话框
  /// 返回 {id} 或 null（用户取消）
  Future<Map<String, dynamic>?> _showFormPrefillerScenarioDialog(BuildContext context) async {
    final locale = Localizations.localeOf(context);

    final scenarios = [
      {
        'id': 'visa-application',
        'label': {'zh': '签证申请表', 'en': 'Visa Application'},
      },
      {
        'id': 'hotel-checkin',
        'label': {'zh': '酒店入住', 'en': 'Hotel Check-in'},
      },
      {
        'id': 'bank-account',
        'label': {'zh': '银行开户', 'en': 'Bank Account'},
      },
      {
        'id': 'airline-checkin',
        'label': {'zh': '航空值机', 'en': 'Airline Check-in'},
      },
    ];

    String resolveL10n(Map<String, String> map) {
      return map[locale.toString()] ??
          map[locale.languageCode] ??
          map['en'] ??
          map.values.first;
    }

    final items = scenarios.map((s) {
      return PluginRadioItem(
        id: s['id'] as String,
        label: resolveL10n(s['label'] as Map<String, String>),
      );
    }).toList();

    final selected = await showDialog<String>(
      context: context,
      builder: (_) => PluginRadioListDialog(
        title: '选择表单场景',
        description: '选择场景后，插件将生成 Vault 字段到表单字段的映射表。',
        items: items,
      ),
    );

    if (selected == null) return null;
    return {'id': selected};
  }
}

// Dart 3 的 List.push 扩展
extension ListPush<T> on List<T> {
  void push(T item) => add(item);
}
