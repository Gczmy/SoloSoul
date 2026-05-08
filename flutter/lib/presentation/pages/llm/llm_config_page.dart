import 'package:flutter/material.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';
import 'package:solosoul_flutter/presentation/theme/glass_adapters.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/core/router/app_router.dart';
import 'package:solosoul_flutter/core/services/llm/llm_config_models.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_model_provider.dart';

// =============================================================================
// LLM Config Page
// =============================================================================

/// Settings page for LLM backend selection and cloud API configuration.
class LlmConfigPage extends ConsumerStatefulWidget {
  const LlmConfigPage({super.key});

  @override
  ConsumerState<LlmConfigPage> createState() => _LlmConfigPageState();
}

class _LlmConfigPageState extends ConsumerState<LlmConfigPage> {
  bool _isTesting = false;
  String? _testResult;
  bool _testSuccess = false;

  Future<void> _testConnection() async {
    setState(() {
      _isTesting = true;
      _testResult = null;
    });

    try {
      final config = ref.read(llmConfigProvider).value;
      if (config == null) {
        setState(() {
          _testResult = AppLocalizations.of(context).llmConfigNotLoaded;
          _testSuccess = false;
        });
        return;
      }

      if (config.backendType == LlmBackendType.cloud) {
        final result = await ref.read(llmModelProvider.notifier).testActiveCloudConnection();
        setState(() {
          _testResult = result;
          _testSuccess = true;
        });
      } else {
        final service = LlmLocalService(
          modelName: config.localModelPath ?? 'qwen2.5:1.5b',
        );
        final status = await service.checkStatus();
        if (!status.serviceRunning) {
          setState(() {
            _testResult = AppLocalizations.of(context).llmConfigOllamaNotRunning;
            _testSuccess = false;
          });
        } else if (!status.modelAvailable) {
          setState(() {
            _testResult = AppLocalizations.of(context).llmConfigOllamaModelNotInstalled(
                config.localModelPath ?? 'qwen2.5:1.5b',
                status.installedModels.join(', '),
              );
            _testSuccess = false;
          });
        } else {
          await service.testConnection();
          setState(() {
            _testResult = AppLocalizations.of(context).llmConfigLocalSuccess;
            _testSuccess = true;
          });
        }
      }
    } on LlmException catch (e) {
      final l10n = AppLocalizations.of(context);
      final errorMsg = switch (e.code) {
        LlmErrorCode.configNotLoaded => l10n.llmErrorConfigNotLoaded,
        LlmErrorCode.cloudConfigIncomplete => l10n.llmErrorCloudConfigIncomplete,
        LlmErrorCode.noActiveProfile => l10n.llmErrorNoActiveCloudProfile,
        LlmErrorCode.apiKeyMissing => l10n.llmErrorApiKeyEmpty,
        LlmErrorCode.unauthorized => l10n.llmErrorApiKeyEmpty,
        _ => e.message,
      };
      setState(() {
        _testResult = errorMsg;
        _testSuccess = false;
      });
    } on Exception catch (e) {
      setState(() {
        _testResult = 'Unknown error: $e';
        _testSuccess = false;
      });
    } finally {
      setState(() => _isTesting = false);
    }
  }

  void _showProfileEditor({LlmCloudProfile? profile}) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => _ProfileEditorSheet(
        profile: profile,
        onSave: (name, providerType, apiKey, endpoint, model, anthropicVersion) async {
          final notifier = ref.read(llmConfigProvider.notifier);
          try {
            if (profile == null) {
              await notifier.addCloudProfile(
                name: name,
                providerType: providerType,
                apiKey: apiKey,
                endpoint: endpoint,
                model: model,
                anthropicVersion: anthropicVersion,
              );
            } else {
              await notifier.updateCloudProfile(
                profileId: profile.id,
                name: name,
                providerType: providerType,
                apiKey: apiKey.isEmpty ? null : apiKey,
                endpoint: endpoint,
                model: model,
                anthropicVersion: anthropicVersion,
              );
            }
            if (ctx.mounted) Navigator.pop(ctx);
          } on Exception catch (e) {
            if (ctx.mounted) {
              ScaffoldMessenger.of(ctx).showSnackBar(
                SnackBar(content: Text(AppLocalizations.of(ctx).llmConfigSaveFailed(e.toString()))),
              );
            }
          }
        },
      ),
    );
  }

  Future<void> _deleteProfile(LlmCloudProfile profile) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(AppLocalizations.of(context).llmConfigDeleteTitle),
        content: Text(AppLocalizations.of(ctx).llmConfigDeleteConfirm(profile.name)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(AppLocalizations.of(context).commonCancel)),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(backgroundColor: Theme.of(ctx).colorScheme.error),
            child: Text(AppLocalizations.of(context).commonDelete),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref.read(llmConfigProvider.notifier).deleteCloudProfile(profile.id);
    }
  }

  @override
  Widget build(BuildContext context) {
    final asyncConfig = ref.watch(llmConfigProvider);
    final notifier = ref.read(llmConfigProvider.notifier);

    return Scaffold(
      appBar: SoloGlassAppBar(
        title: Text(AppLocalizations.of(context).llmConfigTitle),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Chip(
              label: Text(AppLocalizations.of(context).llmConfigExperimental, style: const TextStyle(fontSize: 12)),
              backgroundColor: Theme.of(context).colorScheme.secondaryContainer,
              side: BorderSide.none,
            ),
          ),
        ],
      ),
      body: asyncConfig.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(AppLocalizations.of(context).llmConfigLoadFailed(e.toString()))),
        data: (config) {
          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              // Backend selection
              _SectionTitle(title: AppLocalizations.of(context).llmConfigInferenceBackend),
              _BackendSelector(
                current: config.backendType,
                onChanged: notifier.setBackendType,
              ),
              const SizedBox(height: 24),

              // Local model settings
              if (config.backendType == LlmBackendType.local) ...[
                _SectionTitle(title: AppLocalizations.of(context).llmStatsLocalModelOllama),
                _TextFieldCard(
                  controller: TextEditingController(text: config.localModelPath ?? 'qwen2.5:1.5b'),
                  label: AppLocalizations.of(context).llmConfigModelName,
                  hint: 'qwen2.5:1.5b, llama3.2, deepseek-r1:1.5b...',
                  onChanged: (value) {
                    notifier.setLocalModelPath(value);
                  },
                ),
                const SizedBox(height: 8),
                Card(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          AppLocalizations.of(context).llmConfigInstructions,
                          style: const TextStyle(fontWeight: FontWeight.w600),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          AppLocalizations.of(context).llmConfigInstructionsOllama,
                          style: const TextStyle(fontSize: 13),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 24),
              ],

              // Cloud API profiles
              if (config.backendType == LlmBackendType.cloud) ...[
                _SectionTitle(title: AppLocalizations.of(context).llmConfigCloudConfig),
                if (config.cloudProfiles.isEmpty)
                  const _EmptyProfilesState()
                else
                  ...config.cloudProfiles.map((profile) {
                    final isActive = profile.id == config.activeCloudProfileId;
                    return _ProfileCard(
                      profile: profile,
                      isActive: isActive,
                      onActivate: () => notifier.setActiveCloudProfile(profile.id),
                      onEdit: () => _showProfileEditor(profile: profile),
                      onDelete: () => _deleteProfile(profile),
                    );
                  }),
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: () => _showProfileEditor(),
                  icon: const Icon(Icons.add),
                  label: Text(AppLocalizations.of(context).llmConfigAddProfile),
                ),
                const SizedBox(height: 16),

                // Privacy consent
                Card(
                  child: CheckboxListTile(
                    title: Text(AppLocalizations.of(context).llmConfigCloudConsent),
                    subtitle: Text(
                      AppLocalizations.of(context).llmConfigCloudConsentDesc,
                    ),
                    value: config.cloudConsent,
                    onChanged: (v) => notifier.setCloudConsent(v ?? false),
                  ),
                ),
                const SizedBox(height: 24),
              ],

              // Stats entry
              const SizedBox(height: 16),
              Card(
                clipBehavior: Clip.antiAlias,
                child: ListTile(
                  leading: Icon(Icons.bar_chart, color: Theme.of(context).colorScheme.primary),
                  title: Text(AppLocalizations.of(context).llmStatsTitle),
                  subtitle: Text(AppLocalizations.of(context).llmConfigStatsSubtitle),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => context.push(AppRoutes.llmStats),
                ),
              ),
              const SizedBox(height: 24),

              // Test button
              FilledButton.icon(
                onPressed: _isTesting ? null : _testConnection,
                icon: _isTesting
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.play_arrow),
                label: Text(_isTesting ? AppLocalizations.of(context).llmConfigTesting : AppLocalizations.of(context).llmConfigTestConnection),
              ),

              // Test result
              if (_testResult != null) ...[
                const SizedBox(height: 12),
                Card(
                  color: _testSuccess
                      ? Theme.of(context).colorScheme.primaryContainer
                      : Theme.of(context).colorScheme.errorContainer,
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Row(
                      children: [
                        Icon(
                          _testSuccess ? Icons.check_circle : Icons.error,
                          color: _testSuccess
                              ? Theme.of(context).colorScheme.primary
                              : Theme.of(context).colorScheme.error,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _testResult!,
                            style: TextStyle(
                              color: _testSuccess
                                  ? Theme.of(context).colorScheme.onPrimaryContainer
                                  : Theme.of(context).colorScheme.onErrorContainer,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

// =============================================================================
// Profile Card
// =============================================================================

class _ProfileCard extends StatelessWidget {
  final LlmCloudProfile profile;
  final bool isActive;
  final VoidCallback onActivate;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _ProfileCard({
    required this.profile,
    required this.isActive,
    required this.onActivate,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onActivate,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Radio<String>(
                    value: profile.id,
                    groupValue: isActive ? profile.id : '',
                    onChanged: (_) => onActivate(),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      profile.providerType.label,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onPrimaryContainer,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      profile.name,
                      style: theme.textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.edit, size: 18),
                    tooltip: AppLocalizations.of(context).commonEdit,
                    onPressed: onEdit,
                  ),
                  IconButton(
                    icon: Icon(Icons.delete_outline, size: 18, color: theme.colorScheme.error),
                    tooltip: AppLocalizations.of(context).commonDelete,
                    onPressed: onDelete,
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.only(left: 48),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      AppLocalizations.of(context).llmConfigModelInfo(profile.model),
                      style: theme.textTheme.bodySmall,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      AppLocalizations.of(context).llmConfigEndpointInfo(profile.endpoint),
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// =============================================================================
// Empty State
// =============================================================================

class _EmptyProfilesState extends StatelessWidget {
  const _EmptyProfilesState();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      color: theme.colorScheme.surfaceContainerHighest,
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            Icon(
              Icons.cloud_off,
              size: 48,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(height: 12),
            Text(
              AppLocalizations.of(context).llmConfigNoProfiles,
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              AppLocalizations.of(context).llmConfigNoProfilesHint,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Profile Editor BottomSheet
// =============================================================================

class _ProfileEditorSheet extends StatefulWidget {
  final LlmCloudProfile? profile;
  final Future<void> Function(
    String name,
    LlmCloudProviderType providerType,
    String apiKey,
    String endpoint,
    String model,
    String? anthropicVersion,
  ) onSave;

  const _ProfileEditorSheet({this.profile, required this.onSave});

  @override
  State<_ProfileEditorSheet> createState() => _ProfileEditorSheetState();
}

class _ProfileEditorSheetState extends State<_ProfileEditorSheet> {
  final _nameController = TextEditingController();
  final _apiKeyController = TextEditingController();
  final _endpointController = TextEditingController();
  final _modelController = TextEditingController();
  final _versionController = TextEditingController();

  LlmCloudProviderType _provider = LlmCloudProviderType.openai;
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    final p = widget.profile;
    if (p != null) {
      _nameController.text = p.name;
      _provider = p.providerType;
      _endpointController.text = p.endpoint;
      _modelController.text = p.model;
      _versionController.text = p.anthropicVersion ?? '2023-06-01';
    } else {
      _endpointController.text = 'https://api.openai.com/v1';
      _modelController.text = 'gpt-4o-mini';
      _versionController.text = '2023-06-01';
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _apiKeyController.dispose();
    _endpointController.dispose();
    _modelController.dispose();
    _versionController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final name = _nameController.text.trim();
    final apiKey = _apiKeyController.text.trim();
    final endpoint = _endpointController.text.trim();
    final model = _modelController.text.trim();
    final version = _versionController.text.trim();

    if (name.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).llmConfigNameRequired)),
      );
      return;
    }
    if (widget.profile == null && apiKey.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).llmConfigApiKeyRequired)),
      );
      return;
    }
    if (endpoint.isEmpty || model.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(AppLocalizations.of(context).llmConfigEndpointModelRequired)),
      );
      return;
    }

    setState(() => _isSaving = true);
    try {
      await widget.onSave(
        name,
        _provider,
        apiKey,
        endpoint,
        model,
        _provider == LlmCloudProviderType.anthropic ? version : null,
      );
    } finally {
      if (mounted) setState(() => _isSaving = false);
    }
  }

  void _onProviderChanged(LlmCloudProviderType type) {
    setState(() {
      _provider = type;
      if (type == LlmCloudProviderType.anthropic) {
        _endpointController.text = 'https://api.anthropic.com';
        _modelController.text = 'claude-3-5-sonnet-20241022';
      } else {
        _endpointController.text = 'https://api.openai.com/v1';
        _modelController.text = 'gpt-4o-mini';
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final isEditing = widget.profile != null;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
        left: 16,
        right: 16,
        top: 16,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text(
                isEditing ? AppLocalizations.of(context).llmConfigEditProfile : AppLocalizations.of(context).llmConfigAddProfile,
                style: theme.textTheme.titleLarge,
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _nameController,
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).llmConfigProfileName,
              hintText: AppLocalizations.of(context).llmConfigProfileNameHint,
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          _CloudProviderSelector(
            current: _provider,
            onChanged: _onProviderChanged,
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _apiKeyController,
            decoration: InputDecoration(
              labelText: isEditing ? AppLocalizations.of(context).llmConfigApiKeySet : AppLocalizations.of(context).llmConfigApiKeyNew,
              hintText: isEditing
                  ? AppLocalizations.of(context).llmConfigApiKeyHintNew
                  : (_provider == LlmCloudProviderType.anthropic
                      ? 'sk-ant-api03-...'
                      : 'sk-...'),
              helperText: isEditing ? AppLocalizations.of(context).llmConfigApiKeyHintKeep : null,
              border: const OutlineInputBorder(),
            ),
            obscureText: true,
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _endpointController,
            decoration: InputDecoration(
              labelText: l10n.llmApiEndpoint,
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _modelController,
            decoration: InputDecoration(
              labelText: l10n.llmModel,
              border: const OutlineInputBorder(),
            ),
          ),
          if (_provider == LlmCloudProviderType.anthropic) ...[
            const SizedBox(height: 12),
            TextField(
              controller: _versionController,
              decoration: InputDecoration(
                labelText: l10n.llmAnthropicVersion,
                hintText: '2023-06-01',
                border: const OutlineInputBorder(),
              ),
            ),
          ],
          const SizedBox(height: 24),
          FilledButton(
            onPressed: _isSaving ? null : _save,
            child: _isSaving
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                  )
                : Text(isEditing ? AppLocalizations.of(context).llmConfigSave : AppLocalizations.of(context).llmConfigCreate),
          ),
          const SizedBox(height: 8),
        ],
      ),
    );
  }
}

// =============================================================================
// Reusable Widgets
// =============================================================================

class _SectionTitle extends StatelessWidget {
  final String title;
  const _SectionTitle({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: Theme.of(context).colorScheme.primary,
              fontWeight: FontWeight.w600,
            ),
      ),
    );
  }
}

class _BackendSelector extends StatelessWidget {
  final LlmBackendType current;
  final ValueChanged<LlmBackendType> onChanged;

  const _BackendSelector({required this.current, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<LlmBackendType>(
      segments: [
        ButtonSegment(
          value: LlmBackendType.local,
          label: Text(AppLocalizations.of(context).llmConfigBackendLocal),
          icon: const Icon(Icons.computer),
        ),
        ButtonSegment(
          value: LlmBackendType.cloud,
          label: Text(AppLocalizations.of(context).llmConfigBackendCloud),
          icon: const Icon(Icons.cloud),
        ),
      ],
      selected: {current},
      onSelectionChanged: (set) {
        if (set.isNotEmpty) onChanged(set.first);
      },
    );
  }
}

class _CloudProviderSelector extends StatelessWidget {
  final LlmCloudProviderType current;
  final ValueChanged<LlmCloudProviderType> onChanged;

  const _CloudProviderSelector({
    required this.current,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return SegmentedButton<LlmCloudProviderType>(
      segments: [
        ButtonSegment(
          value: LlmCloudProviderType.openai,
          label: Text(l10n.llmOpenAI),
          icon: const Icon(Icons.cloud_queue),
        ),
        ButtonSegment(
          value: LlmCloudProviderType.anthropic,
          label: Text(l10n.llmAnthropic),
          icon: const Icon(Icons.smart_toy),
        ),
      ],
      selected: {current},
      onSelectionChanged: (set) {
        if (set.isNotEmpty) onChanged(set.first);
      },
    );
  }
}

class _TextFieldCard extends StatelessWidget {
  final TextEditingController? controller;
  final String label;
  final String hint;
  final bool obscureText;
  final ValueChanged<String> onChanged;

  const _TextFieldCard({
    this.controller,
    required this.label,
    required this.hint,
    // ignore: unused_element_parameter
    this.obscureText = false,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: TextField(
          controller: controller,
          decoration: InputDecoration(
            labelText: label,
            hintText: hint,
            border: InputBorder.none,
          ),
          obscureText: obscureText,
          onChanged: onChanged,
        ),
      ),
    );
  }
}
