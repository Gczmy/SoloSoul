import 'package:flutter/material.dart';
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
          _testResult = '配置未加载';
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
            _testResult = 'Ollama 服务未运行\n请确认已安装并启动 Ollama';
            _testSuccess = false;
          });
        } else if (!status.modelAvailable) {
          setState(() {
            _testResult = 'Ollama 运行中，但模型 ${config.localModelPath ?? 'qwen2.5:1.5b'} 未安装\n'
                '已安装模型: ${status.installedModels.join(', ')}';
            _testSuccess = false;
          });
        } else {
          await service.testConnection();
          setState(() {
            _testResult = '本地模型连接成功！';
            _testSuccess = true;
          });
        }
      }
    } on LlmException catch (e) {
      setState(() {
        _testResult = '连接失败: ${e.message}';
        _testSuccess = false;
      });
    } on Exception catch (e) {
      setState(() {
        _testResult = '未知错误: $e';
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
                SnackBar(content: Text('保存失败: $e')),
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
        title: const Text('删除配置'),
        content: Text('确认删除 "${profile.name}" 吗？此操作不可撤销。'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            style: FilledButton.styleFrom(backgroundColor: Theme.of(ctx).colorScheme.error),
            child: const Text('删除'),
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
      appBar: AppBar(
        title: const Text('LLM 设置'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Chip(
              label: const Text('实验性功能', style: TextStyle(fontSize: 12)),
              backgroundColor: Theme.of(context).colorScheme.secondaryContainer,
              side: BorderSide.none,
            ),
          ),
        ],
      ),
      body: asyncConfig.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('加载失败: $e')),
        data: (config) {
          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              // Backend selection
              const _SectionTitle(title: '推理后端'),
              _BackendSelector(
                current: config.backendType,
                onChanged: notifier.setBackendType,
              ),
              const SizedBox(height: 24),

              // Local model settings
              if (config.backendType == LlmBackendType.local) ...[
                const _SectionTitle(title: '本地模型 (Ollama)'),
                _TextFieldCard(
                  controller: TextEditingController(text: config.localModelPath ?? 'qwen2.5:1.5b'),
                  label: '模型名称',
                  hint: 'qwen2.5:1.5b, llama3.2, deepseek-r1:1.5b...',
                  onChanged: (value) {
                    notifier.setLocalModelPath(value);
                  },
                ),
                const SizedBox(height: 8),
                Card(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  child: const Padding(
                    padding: EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          '使用说明',
                          style: TextStyle(fontWeight: FontWeight.w600),
                        ),
                        SizedBox(height: 4),
                        Text(
                          '1. 安装 Ollama: https://ollama.com\n'
                          '2. 拉取模型: ollama pull qwen2.5:1.5b\n'
                          '3. 保持 Ollama 在后台运行',
                          style: TextStyle(fontSize: 13),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 24),
              ],

              // Cloud API profiles
              if (config.backendType == LlmBackendType.cloud) ...[
                const _SectionTitle(title: '云端配置'),
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
                  label: const Text('新增配置'),
                ),
                const SizedBox(height: 16),

                // Privacy consent
                Card(
                  child: CheckboxListTile(
                    title: const Text('同意云端处理'),
                    subtitle: const Text(
                      '我确认当前批次不含 critical 级别字段，'
                      '并同意将数据发送至指定的企业/私有 API 端点。',
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
                  title: const Text('使用统计'),
                  subtitle: const Text('查看 Token 消耗、对话次数等'),
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
                label: Text(_isTesting ? '测试中...' : '测试连接'),
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
                    tooltip: '编辑',
                    onPressed: onEdit,
                  ),
                  IconButton(
                    icon: Icon(Icons.delete_outline, size: 18, color: theme.colorScheme.error),
                    tooltip: '删除',
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
                      '模型: ${profile.model}',
                      style: theme.textTheme.bodySmall,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '端点: ${profile.endpoint}',
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
              '暂无云端配置',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              '点击下方按钮创建第一个云端 API 配置',
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
        const SnackBar(content: Text('请输入配置名称')),
      );
      return;
    }
    if (widget.profile == null && apiKey.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('新增配置时必须填写 API Key')),
      );
      return;
    }
    if (endpoint.isEmpty || model.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Endpoint 和 Model 不能为空')),
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
                isEditing ? '编辑配置' : '新增配置',
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
            decoration: const InputDecoration(
              labelText: '配置名称',
              hintText: '例如：OpenAI 生产环境',
              border: OutlineInputBorder(),
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
              labelText: isEditing ? 'API Key（已配置）' : 'API Key *',
              hintText: isEditing
                  ? '输入新值以替换现有密钥'
                  : (_provider == LlmCloudProviderType.anthropic
                      ? 'sk-ant-api03-...'
                      : 'sk-...'),
              helperText: isEditing ? '留空将保持现有密钥不变' : null,
              border: const OutlineInputBorder(),
            ),
            obscureText: true,
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _endpointController,
            decoration: const InputDecoration(
              labelText: 'API Endpoint',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _modelController,
            decoration: const InputDecoration(
              labelText: 'Model',
              border: OutlineInputBorder(),
            ),
          ),
          if (_provider == LlmCloudProviderType.anthropic) ...[
            const SizedBox(height: 12),
            TextField(
              controller: _versionController,
              decoration: const InputDecoration(
                labelText: 'Anthropic API Version',
                hintText: '2023-06-01',
                border: OutlineInputBorder(),
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
                : Text(isEditing ? '保存修改' : '创建配置'),
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
      segments: const [
        ButtonSegment(
          value: LlmBackendType.local,
          label: Text('本地模型'),
          icon: Icon(Icons.computer),
        ),
        ButtonSegment(
          value: LlmBackendType.cloud,
          label: Text('云端 API'),
          icon: Icon(Icons.cloud),
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
    return SegmentedButton<LlmCloudProviderType>(
      segments: const [
        ButtonSegment(
          value: LlmCloudProviderType.openai,
          label: Text('OpenAI'),
          icon: Icon(Icons.cloud_queue),
        ),
        ButtonSegment(
          value: LlmCloudProviderType.anthropic,
          label: Text('Anthropic'),
          icon: Icon(Icons.smart_toy),
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
