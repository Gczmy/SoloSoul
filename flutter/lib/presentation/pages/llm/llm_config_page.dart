import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:solosoul_flutter/core/services/llm/llm_service.dart';
import 'package:solosoul_flutter/presentation/providers/llm/llm_config_provider.dart';

// =============================================================================
// LLM Config Page
// =============================================================================

/// Settings page for LLM backend selection and cloud API configuration.
///
/// UI layout placeholders — actual local model file picker and connection
/// tests will be wired in P1 implementation.
class LlmConfigPage extends ConsumerStatefulWidget {
  const LlmConfigPage({super.key});

  @override
  ConsumerState<LlmConfigPage> createState() => _LlmConfigPageState();
}

class _LlmConfigPageState extends ConsumerState<LlmConfigPage> {
  final _apiKeyController = TextEditingController();
  final _endpointController = TextEditingController();
  final _modelController = TextEditingController();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final asyncValue = ref.read(llmConfigProvider);
      final config = asyncValue.value;
      if (config != null) {
        _apiKeyController.text = config.cloudApiKey;
        _endpointController.text = config.cloudEndpoint;
        _modelController.text = config.cloudModel;
      }
    });
  }

  @override
  void dispose() {
    _apiKeyController.text = '';
    _apiKeyController.dispose();
    _endpointController.dispose();
    _modelController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final asyncConfig = ref.watch(llmConfigProvider);
    final notifier = ref.read(llmConfigProvider.notifier);

    return Scaffold(
      appBar: AppBar(title: const Text('LLM 设置')),
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

              // Local model path (placeholder)
              if (config.backendType == LlmBackendType.local) ...[
                const _SectionTitle(title: '本地模型'),
                Card(
                  child: ListTile(
                    leading: const Icon(Icons.folder_open),
                    title: Text(
                      config.localModelPath ?? '选择模型文件（.gguf / .bin）',
                      style: TextStyle(
                        color: config.localModelPath != null
                            ? null
                            : Theme.of(context).hintColor,
                      ),
                    ),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () {
                      // TODO(P1): File picker integration
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(
                            content: Text('文件选择器占位 — P1 实现')),
                      );
                    },
                  ),
                ),
                const SizedBox(height: 24),
              ],

              // Cloud API settings
              if (config.backendType == LlmBackendType.cloud) ...[
                const _SectionTitle(title: '云端 API'),
                _TextFieldCard(
                  controller: _apiKeyController,
                  label: 'API Key',
                  hint: 'sk-...',
                  obscureText: true,
                  onChanged: notifier.setCloudApiKey,
                ),
                const SizedBox(height: 12),
                _TextFieldCard(
                  controller: _endpointController,
                  label: 'API Endpoint',
                  hint: 'https://api.openai.com/v1',
                  onChanged: notifier.setCloudEndpoint,
                ),
                const SizedBox(height: 12),
                _TextFieldCard(
                  controller: _modelController,
                  label: 'Model',
                  hint: 'gpt-4o-mini',
                  onChanged: notifier.setCloudModel,
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

              // Test button (stub)
              FilledButton.icon(
                onPressed: () async {
                  // TODO(P1): Connection test
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('连接测试占位 — P1 实现')),
                  );
                },
                icon: const Icon(Icons.play_arrow),
                label: const Text('测试连接'),
              ),
            ],
          );
        },
      ),
    );
  }
}

// =============================================================================
// Widgets
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

class _TextFieldCard extends StatelessWidget {
  final TextEditingController controller;
  final String label;
  final String hint;
  final bool obscureText;
  final ValueChanged<String> onChanged;

  const _TextFieldCard({
    required this.controller,
    required this.label,
    required this.hint,
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
