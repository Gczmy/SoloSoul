// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get commonCancel => '取消';

  @override
  String get commonConfirm => '确认';

  @override
  String get commonSave => '保存';

  @override
  String get commonImport => '导入';

  @override
  String get commonError => '错误';

  @override
  String get commonRetry => '重试';

  @override
  String get commonClose => '关闭';

  @override
  String get commonLoading => '加载中...';

  @override
  String get commonSuccess => '成功';

  @override
  String get commonDelete => '删除';

  @override
  String get commonEdit => '编辑';

  @override
  String get settingsLanguage => '语言';

  @override
  String get settingsLanguageSubtitle => '选择您偏好的语言';

  @override
  String get settingsLanguageEnglish => 'English';

  @override
  String get settingsLanguageChinese => '中文 (Chinese)';

  @override
  String get settingsAiChat => 'AI 对话';

  @override
  String get settingsAiChatSubtitle => '与本地或云端模型聊天';

  @override
  String get settingsDeleteAccountWarning => '删除账户后，该账号的所有数据都会被清空，确定要删除吗？';

  @override
  String get mainAppTitle => 'SoloSoul';

  @override
  String get mainSplashTagline => '独奏生命数据，重塑数字原点';

  @override
  String get mainLaunchFailed => '启动失败';

  @override
  String get sidebarAiChat => 'AI 对话';

  @override
  String get llmConfigTitle => 'AI 助手设置';

  @override
  String get llmConfigNotLoaded => '配置未加载';

  @override
  String get llmConfigOllamaNotRunning => 'Ollama 服务未运行\n请确认已安装并启动 Ollama';

  @override
  String llmConfigOllamaModelNotInstalled(String model, String models) {
    return 'Ollama 运行中，但模型 $model 未安装\n已安装模型: $models';
  }

  @override
  String get llmConfigLocalSuccess => '本地模型连接成功！';

  @override
  String llmConfigConnectionFailed(String message) {
    return '连接失败: $message';
  }

  @override
  String llmConfigUnknownError(String message) {
    return '未知错误: $message';
  }

  @override
  String llmConfigSaveFailed(String message) {
    return '保存失败: $message';
  }

  @override
  String get llmConfigDeleteTitle => '删除配置';

  @override
  String llmConfigDeleteConfirm(String name) {
    return '确认删除 \"$name\" 吗？此操作不可撤销。';
  }

  @override
  String get llmConfigExperimental => '实验性功能';

  @override
  String llmConfigLoadFailed(String message) {
    return '加载失败: $message';
  }

  @override
  String get llmConfigInferenceBackend => '推理后端';

  @override
  String get llmConfigModelName => '模型名称';

  @override
  String get llmConfigInstructions => '使用说明';

  @override
  String get llmConfigInstructionsOllama =>
      '1. 安装 Ollama: https://ollama.com\n2. 拉取模型: ollama pull qwen2.5:1.5b\n3. 保持 Ollama 在后台运行';

  @override
  String get llmConfigCloudConfig => '云端配置';

  @override
  String get llmConfigAddProfile => '新增配置';

  @override
  String get llmConfigCloudConsent => '同意云端处理';

  @override
  String get llmConfigCloudConsentDesc =>
      '我确认当前批次不含 critical 级别字段，并同意将数据发送至指定的企业/私有 API 端点。';

  @override
  String get llmConfigStatsSubtitle => '查看 Token 消耗、对话次数等';

  @override
  String get llmConfigTesting => '测试中...';

  @override
  String get llmConfigTestConnection => '测试连接';

  @override
  String llmConfigModelInfo(String model) {
    return '模型: $model';
  }

  @override
  String llmConfigEndpointInfo(String endpoint) {
    return '端点: $endpoint';
  }

  @override
  String get llmConfigNoProfiles => '暂无云端配置';

  @override
  String get llmConfigNoProfilesHint => '点击下方按钮创建第一个云端 API 配置';

  @override
  String get llmConfigNameRequired => '请输入配置名称';

  @override
  String get llmConfigApiKeyRequired => '新增配置时必须填写 API Key';

  @override
  String get llmConfigEndpointModelRequired => 'Endpoint 和 Model 不能为空';

  @override
  String get llmConfigEditProfile => '编辑配置';

  @override
  String get llmConfigProfileName => '配置名称';

  @override
  String get llmConfigProfileNameHint => '例如：OpenAI 生产环境';

  @override
  String get llmConfigApiKeySet => 'API Key（已配置）';

  @override
  String get llmConfigApiKeyNew => 'API Key *';

  @override
  String get llmConfigApiKeyHintNew => '输入新值以替换现有密钥';

  @override
  String get llmConfigApiKeyHintKeep => '留空将保持现有密钥不变';

  @override
  String get llmConfigSave => '保存修改';

  @override
  String get llmConfigCreate => '创建配置';

  @override
  String get llmConfigBackendLocal => '本地模型';

  @override
  String get llmConfigBackendCloud => '云端 API';

  @override
  String get llmStatsTitle => '使用统计';

  @override
  String get llmStatsCurrentModel => '当前模型';

  @override
  String get llmStatsSessionStats => '本次会话统计';

  @override
  String get llmStatsAccountStats => '账户累计统计';

  @override
  String get llmStatsTokenBreakdown => 'Token 构成';

  @override
  String get llmStatsDailyTrend => '每日 Token 趋势（最近 14 天）';

  @override
  String get llmStatsModelUsage => '模型使用占比';

  @override
  String get llmStatsReset => '重置统计';

  @override
  String get llmStatsResetConfirm => '确认重置所有使用统计吗？此操作不可撤销。';

  @override
  String get llmStatsResetSuccess => '统计已重置';

  @override
  String get llmStatsUnknown => '未知';

  @override
  String get llmStatsNotLoaded => '未加载';

  @override
  String get llmStatsLocalModelOllama => '本地模型 (Ollama)';

  @override
  String get llmStatsModelLabel => '模型';

  @override
  String get llmStatsProviderLabel => '提供商';

  @override
  String get llmStatsConversationCount => '对话次数';

  @override
  String get llmStatsTokenConsumption => 'Token 消耗';

  @override
  String get llmStatsLastLoaded => '最后加载';

  @override
  String get llmStatsLastUsed => '最后使用';

  @override
  String get llmStatsTotalConversations => '累计对话';

  @override
  String get llmStatsTotalTokens => '累计 Token';

  @override
  String get llmStatsSession => '本次会话';

  @override
  String get llmStatsAccountTotal => '账户累计';

  @override
  String get llmStatsAllModels => '全部模型';

  @override
  String get llmChatTitle => 'AI 对话';

  @override
  String get llmChatBackendCloud => '云端';

  @override
  String get llmChatBackendLocal => '本地';

  @override
  String get llmChatModelNotConfigured => '未配置';

  @override
  String get llmChatModelNotLoaded => '模型尚未加载，请先配置 LLM';

  @override
  String get llmChatClearSession => '清除会话';

  @override
  String get llmChatThinking => '正在思考…';

  @override
  String get llmChatNoResponse => '（未收到回复）';

  @override
  String get llmChatInputHintReady => '输入消息...';

  @override
  String get llmChatInputHintNotReady => '模型未就绪';

  @override
  String get llmChatStatusReady => '就绪';

  @override
  String get llmChatStatusNotReady => '未就绪';

  @override
  String get llmChatLoadingConfig => '正在加载模型配置…';

  @override
  String get llmChatStartConversation => '开始与 AI 对话';

  @override
  String get llmChatConnectCloudModel => '连接云端模型';

  @override
  String get llmChatStartLocalModel => '启动本地模型';

  @override
  String get llmChatGoToConfig => '前往 LLM 配置';

  @override
  String get llmErrorConfigNotLoaded => '配置未加载';

  @override
  String get llmErrorCloudConfigIncomplete => '云端配置不完整：请检查 API Key 和隐私同意';

  @override
  String get llmErrorNoActiveCloudProfile => '没有激活的云端配置';

  @override
  String get llmErrorApiKeyEmpty => 'API Key 为空';

  @override
  String get llmCopy => '复制';

  @override
  String get llmCopied => '已复制';

  @override
  String get llmInferenceError => '推理出错';

  @override
  String get ocrScanDocument => '扫描文档';

  @override
  String get ocrTakePhoto => '拍照';

  @override
  String get ocrSelectDocument => '选择文档';

  @override
  String get ocrLlmAssist => '使用 LLM 协助提取字段';

  @override
  String get ocrLlmAssistSubtitle => '提高字段识别准确率';

  @override
  String get ocrNoModelAvailable => '未配置可用 LLM 模型';

  @override
  String get ocrGoToConfig => '前往配置';

  @override
  String get ocrLlmConfig => 'LLM 配置';

  @override
  String get ocrModelSelectorLabel => '选择模型';

  @override
  String get ocrPrivacyNotice => '所有识别均在您的设备本地完成。图片不会上传到任何服务器。旅行证件和身份证将自动检测。';

  @override
  String get ocrTip => '提示：为获得最佳效果，请确保文字清晰可见且光线充足。';

  @override
  String get ocrRecognizing => '正在识别文字...';

  @override
  String get ocrRecognitionFailed => '识别失败';

  @override
  String get ocrTryAgain => '重试';

  @override
  String get ocrTravelDocumentDetected => '检测到旅行证件';

  @override
  String get ocrRescan => '重新扫描';

  @override
  String get scanGoToConfig => '去配置';

  @override
  String get scanAiMappingComplete => 'AI 智能映射完成';

  @override
  String get scanAiMapping => 'AI 智能映射';

  @override
  String llmStatsTotalFormatted(String total) {
    return '共 $total';
  }

  @override
  String llmStatsModelSummary(int count, String total) {
    return '共 $count 个模型 · 累计 $total tokens';
  }

  @override
  String llmStatsModelDetail(String provider, String total, int count) {
    return '$provider · $total tokens · $count 次调用';
  }
}
