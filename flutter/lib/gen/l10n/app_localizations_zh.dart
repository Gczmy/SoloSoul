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
  String get genericFilterClearAll => '清除';

  @override
  String get commonLoading => '加载中...';

  @override
  String get commonSuccess => '成功';

  @override
  String get commonDelete => '删除';

  @override
  String get commonEdit => '编辑';

  @override
  String get commonUndo => '撤销';

  @override
  String get settingsLanguage => '语言';

  @override
  String get settingsLanguageSubtitle => '选择您偏好的语言';

  @override
  String get settingsLanguageEnglish => 'English';

  @override
  String get settingsLanguageChinese => '中文 (Chinese)';

  @override
  String get loginDataRecoveryTitle => '数据恢复';

  @override
  String loginDataRecoveryMessage(String time) {
    return '您的保险库为空，但存在 $time 的备份。是否要恢复？';
  }

  @override
  String get loginSkip => '跳过';

  @override
  String get loginRestoreBackup => '恢复备份';

  @override
  String get loginRestoreSuccess => '恢复成功。您的数据已可用。';

  @override
  String get loginRestoreFailed => '恢复失败';

  @override
  String get loginBiometricGeneric => '生物识别';

  @override
  String get loginBiometricFaceId => '面容 ID';

  @override
  String get loginBiometricTouchId => '触控 ID';

  @override
  String get loginBiometricIris => '虹膜';

  @override
  String loginUnlockReason(String biometricType) {
    return '使用 $biometricType';
  }

  @override
  String get loginBiometricFailed => '生物识别失败或已取消';

  @override
  String get loginUnlockFailedUsePassword => '解锁失败。请使用主密码。';

  @override
  String get loginPasswordMinLength => '密码至少需要 8 个字符';

  @override
  String get loginInvalidPassword => '主密码无效';

  @override
  String loginUnlockFailed(String message) {
    return '解锁失败: $message';
  }

  @override
  String get loginAccountNameRequired => '账户名称必填';

  @override
  String get loginPasswordsDoNotMatch => '两次输入的密码不一致';

  @override
  String get loginCreateAccountFailed => '创建账户失败';

  @override
  String get loginUnlockVaultFailed => '解锁失败。请重试。';

  @override
  String loginPasswordHint(String hint) {
    return '密码提示: $hint';
  }

  @override
  String get loginNever => '从未';

  @override
  String get loginToday => '今天';

  @override
  String get loginYesterday => '昨天';

  @override
  String loginDaysAgo(int count) {
    return '$count 天前';
  }

  @override
  String get loginBackToAccountList => '返回账户列表';

  @override
  String get loginAccountName => '账户名称';

  @override
  String get loginAccountNameHint => '例如: 个人, 工作';

  @override
  String get loginMasterPassword => '主密码';

  @override
  String get loginEnterPassword => '输入您的密码';

  @override
  String get loginCreateStrongPassword => '创建一个强密码';

  @override
  String get loginConfirmPassword => '确认密码';

  @override
  String get loginReenterPassword => '重新输入您的密码';

  @override
  String get loginPasswordHintOptional => '密码提示（可选）';

  @override
  String get loginPasswordHintHelp => '帮助您记住密码的提示';

  @override
  String get loginShowPasswordHint => '显示密码提示';

  @override
  String get loginNoAccounts => '未找到账户';

  @override
  String get loginCreateAccount => '创建账户';

  @override
  String loginLastAccessed(String time) {
    return '最后访问: $time';
  }

  @override
  String get loginAccountListEmpty => '账户列表为空';

  @override
  String get loginCreateFirstAccount => '创建您的第一个账户以开始使用';

  @override
  String get loginSelectAccountToUnlock => '选择账户以解锁';

  @override
  String get loginShowLess => '收起';

  @override
  String loginShowAllAccounts(int count) {
    return '显示全部 $count 个账户';
  }

  @override
  String get loginNoAccountsYet => '暂无账户';

  @override
  String get loginRecent => '最近';

  @override
  String get workspaceObjects => '对象';

  @override
  String get workspaceNoItems => '暂无项目';

  @override
  String get workspaceNoObjects => '暂无对象';

  @override
  String get workspaceAddFirstItem => '添加第一个项目';

  @override
  String get workspaceCreateFirstObject => '创建您的第一个对象以开始使用';

  @override
  String get workspaceDeletePage => '删除页面';

  @override
  String get workspaceDeleteSection => '删除分区';

  @override
  String workspaceDeleteSectionConfirm(String name) {
    return '确定要删除 \"$name\" 吗？';
  }

  @override
  String workspaceDeletePageConfirm(String name, int count) {
    return '确定要删除 \"$name\" 吗？此页面中的 $count 个项目也将移至回收站。';
  }

  @override
  String get workspaceSectionDeleted => '分区已删除';

  @override
  String workspaceMovedToTrash(String name) {
    return '\"$name\" 已移至回收站';
  }

  @override
  String get workspaceAddSubPage => '添加子页面';

  @override
  String get workspaceAddSection => '添加分区';

  @override
  String get workspaceAddSectionDialog => '添加分区';

  @override
  String get workspaceSectionName => '名称';

  @override
  String get workspaceEnterSectionName => '输入分区名称';

  @override
  String get workspaceIcon => '图标';

  @override
  String get objectEditorEditSection => '编辑分区';

  @override
  String get objectEditorNewSection => '新建分区';

  @override
  String get objectEditorType => '类型';

  @override
  String get objectEditorNameRequired => '名称为必填项';

  @override
  String objectEditorDuplicateProperties(String names) {
    return '重复的属性名称: $names';
  }

  @override
  String objectEditorSaveFailed(String message) {
    return '保存失败: $message';
  }

  @override
  String get objectEditorIcon => '图标';

  @override
  String get objectEditorName => '名称';

  @override
  String get objectEditorEnterSectionName => '输入分区名称';

  @override
  String get objectEditorSelectType => '选择类型';

  @override
  String get objectEditorNoParent => '无父级（根）';

  @override
  String get objectEditorItemProperties => '项目属性';

  @override
  String get objectEditorAddProperty => '添加属性';

  @override
  String get objectEditorKeyName => '键名';

  @override
  String get objectEditorPropertyTypeText => '文本';

  @override
  String get objectEditorPropertyTypeDate => '日期';

  @override
  String get objectEditorPropertyTypeNumber => '数字';

  @override
  String get objectEditorPropertyTypeCheckbox => '复选框';

  @override
  String get objectEditorPropertyTypeSelect => '单选';

  @override
  String get objectEditorPropertyTypeMultiSelect => '多选';

  @override
  String get objectEditorPropertyTypeUrl => '链接';

  @override
  String get objectEditorSensitivity => '敏感度';

  @override
  String get objectEditorDeletePropertyTitle => '删除属性';

  @override
  String get pageEditorNameRequired => '名称为必填项';

  @override
  String get pageEditorEditPage => '编辑页面';

  @override
  String get pageEditorNewPage => '新建页面';

  @override
  String get pageEditorName => '名称';

  @override
  String get pageEditorEnterPageName => '输入页面名称';

  @override
  String get pageEditorIcon => '图标';

  @override
  String get pageEditorParent => '父级';

  @override
  String get homeScan => '扫描';

  @override
  String get homeQuickActions => '快捷操作';

  @override
  String get homeEditQuickActions => '编辑快捷操作';

  @override
  String get homeEditQuickActionsDone => '完成';

  @override
  String get homeSecurityStatus => '安全状态';

  @override
  String get searchTitle => '搜索';

  @override
  String get searchHint => '搜索字段...';

  @override
  String get profileType => '类型';

  @override
  String get profileTypeEmail => '邮箱';

  @override
  String get profileTypePhone => '电话';

  @override
  String get settingsTitle => '设置';

  @override
  String get settingsDebugModeEnabled => '调试模式已启用';

  @override
  String get settingsInvalidPassword => '密码无效';

  @override
  String get settingsPasswordChangedSuccess => '主密码修改成功';

  @override
  String get settingsPasswordHintChangedSuccess => '密码提示修改成功';

  @override
  String get settingsOk => '确定';

  @override
  String get settingsEnableDebugMode => '启用调试模式';

  @override
  String get settingsEnableDebugModeDesc => '输入主密码以启用调试日志。';

  @override
  String settingsUseBiometric(String biometricType) {
    return '使用 $biometricType';
  }

  @override
  String get settingsOr => '或';

  @override
  String get settingsMasterPassword => '主密码';

  @override
  String get settingsShowPasswordHint => '显示密码提示';

  @override
  String get settingsEnable => '启用';

  @override
  String get securitySettingsTitle => '安全设置';

  @override
  String get securitySettingsBiometricFailed => '生物识别失败或已取消';

  @override
  String get securitySettingsBiometricEnabled => '生物识别解锁已启用';

  @override
  String get securitySettingsResetToDefaults => '恢复默认';

  @override
  String get securitySettingsResetTitle => '重置安全设置';

  @override
  String get securitySettingsResetConfirm => '这将把所有安全设置恢复为默认值。确定吗？';

  @override
  String get securitySettingsReset => '重置';

  @override
  String get securitySettingsNotImplemented => '功能尚未实现';

  @override
  String get sensitivitySettingsTitle => '敏感度设置';

  @override
  String get sensitivitySettingsVerify => '验证';

  @override
  String get sensitivitySettingsConfirmDowngrade => '确认降级';

  @override
  String get sensitivitySettingsChangeLevel => '更改敏感度级别';

  @override
  String get sensitivitySettingsSearchHint => '搜索字段...';

  @override
  String get sensitivitySettingsClearSearch => '清除搜索';

  @override
  String get trashTitle => '回收站';

  @override
  String get trashVerify => '验证';

  @override
  String get trashEmptyTrash => '清空回收站';

  @override
  String get trashConfirmRestore => '确认恢复';

  @override
  String trashRestoreConfirm(String name) {
    return '恢复 \"$name\"？';
  }

  @override
  String get trashConfirmPermanentDelete => '确认永久删除';

  @override
  String get trashSearchHint => '搜索回收站...';

  @override
  String get syncTitle => '设备同步';

  @override
  String get syncNoActiveAccount => '没有用于同步的活跃账户';

  @override
  String get syncEnterAddressAndKey => '输入地址和配对密钥';

  @override
  String get syncInvalidPairingKey => '配对密钥无效';

  @override
  String get syncPairingKeyCopied => '配对密钥已复制到剪贴板';

  @override
  String get syncRemoteAddress => '远程地址';

  @override
  String get syncRemoteAddressHint => '192.168.1.5:9900';

  @override
  String get syncPairingKey => '配对密钥';

  @override
  String get syncPairingKeyHint => '生成共享配对密钥以建立设备间的安全连接。两台设备必须使用相同的密钥。';

  @override
  String get syncGenerateAndCopyKey => '生成并复制密钥';

  @override
  String syncWithDevice(String name) {
    return '与 $name 同步';
  }

  @override
  String get syncButton => '同步';

  @override
  String get dataManagementTitle => '数据管理';

  @override
  String get dataManagementBackupNow => '立即备份';

  @override
  String get dataManagementSpecialBackupLimit => '特殊备份数量已达上限';

  @override
  String get dataManagementNameBackup => '命名特殊备份';

  @override
  String get dataManagementBackupNameHint => '例如: 大更新前';

  @override
  String get dataManagementBackupNameLabel => '备份名称';

  @override
  String get dataManagementCreate => '创建';

  @override
  String get dataManagementRenameBackup => '重命名特殊备份';

  @override
  String get dataManagementNewName => '新名称';

  @override
  String get dataManagementRename => '重命名';

  @override
  String get dataManagementRestoreBackupTitle => '恢复特殊备份？';

  @override
  String dataManagementRestoreBackupConfirm(String name) {
    return '恢复特殊备份 \"$name\"？';
  }

  @override
  String get dataManagementDeleteBackupTitle => '删除特殊备份？';

  @override
  String dataManagementDeleteBackupConfirm(String name) {
    return '删除特殊备份 \"$name\"？';
  }

  @override
  String get operationLogTitle => '操作日志';

  @override
  String get operationLogVerify => '验证';

  @override
  String get operationLogClearLogTitle => '清空日志';

  @override
  String get operationLogClear => '清空';

  @override
  String get operationLogClearLog => '清空日志';

  @override
  String get operationLogSearchHint => '搜索日志...';

  @override
  String objectEditorDeletePropertyConfirm(String name) {
    return '确定要删除 \"$name\" 吗？';
  }

  @override
  String get workspaceAddSectionButton => '添加分区';

  @override
  String get workspaceEditPage => '编辑页面';

  @override
  String get workspaceDone => '完成';

  @override
  String get workspaceReorder => '重新排序';

  @override
  String get workspaceAdd => '添加';

  @override
  String get loginCreateNewAccount => '创建新账户';

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
  String get llmChatStatusLoading => '加载中';

  @override
  String get llmChatStatusError => '错误';

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

  @override
  String get ocrFieldName => '姓名/名称';

  @override
  String get ocrFieldPhone => '电话';

  @override
  String get ocrFieldEmail => '邮箱';

  @override
  String get ocrFieldAddress => '地址';

  @override
  String get ocrFieldCompany => '公司/机构';

  @override
  String get ocrFieldTitle => '职位/头衔';

  @override
  String get ocrFieldDate => '日期';

  @override
  String get ocrFieldAmount => '金额';

  @override
  String get ocrFieldInvoiceNumber => '发票/单据号码';

  @override
  String get ocrFieldWebsite => '网站/URL';

  @override
  String get ocrFieldIdNumber => '证件号码';

  @override
  String get llmChatEmptyResponse => '模型未返回任何内容，请检查配置或重试';

  @override
  String llmChatInferenceFailed(String error) {
    return '推理失败：$error';
  }

  @override
  String get sidebarHome => '首页';

  @override
  String get sidebarSearch => '搜索';

  @override
  String get sidebarLocalImport => '本地导入';

  @override
  String get sidebarProfile => '个人资料';

  @override
  String get sidebarTravel => '旅行';

  @override
  String get sidebarFinancial => '财务';

  @override
  String get sidebarProfessional => '职业';

  @override
  String get sidebarAddPage => '添加页面';

  @override
  String get sidebarLockVault => '锁定保险库';

  @override
  String get sidebarTrash => '回收站';

  @override
  String get sidebarSync => '同步';

  @override
  String get sidebarSettings => '设置';

  @override
  String get sidebarPlugin => '插件';

  @override
  String get sidebarSecurity => '安全';

  @override
  String get sidebarOperationLog => '操作日志';

  @override
  String get sidebarSensitivity => '敏感度';

  @override
  String get sidebarCollapse => '收起';

  @override
  String get sidebarExpand => '展开';

  @override
  String get sidebarPages => '页面';

  @override
  String get sidebarDropToMakeRootPage => '拖拽至此处设为根页面';

  @override
  String get commonBack => '返回';

  @override
  String get homeTitle => '首页';

  @override
  String get homeEndToEndEncrypted => '端到端加密';

  @override
  String get homeEncryptionDesc => 'AES-256-GCM + Argon2id';

  @override
  String get homeLocalStorage => '本地存储';

  @override
  String get homeLocalStorageDesc => '数据加密并存储在本地';

  @override
  String get homeZeroKnowledge => '零知识架构';

  @override
  String get homeZeroKnowledgeDesc => '主密码永不存储';

  @override
  String get profileTitle => '个人资料';

  @override
  String get profileIdentity => '身份信息';

  @override
  String get profileContactInfo => '联系信息';

  @override
  String get profileIdentityDocuments => '身份证件';

  @override
  String get profileAddresses => '地址';

  @override
  String get profileTitleLabel => '标题';

  @override
  String get profileTypeLabel => '类型';

  @override
  String get profileValueLabel => '值';

  @override
  String get travelTitle => '旅行';

  @override
  String get travelPassports => '护照';

  @override
  String get travelVisas => '签证';

  @override
  String get travelHistory => '旅行记录';

  @override
  String get financialTitle => '财务';

  @override
  String get financialBankAccounts => '银行账户';

  @override
  String get financialCards => '卡片';

  @override
  String get financialTaxIdentification => '税务识别号';

  @override
  String get professionalTitle => '职业';

  @override
  String get professionalEducation => '教育';

  @override
  String get professionalEmployment => '就业';

  @override
  String get professionalAwards => '奖项';

  @override
  String get professionalSkills => '技能';

  @override
  String get professionalLanguages => '语言';

  @override
  String get professionalArticles => '文章';

  @override
  String get localSearchTitle => '本地搜索导入';

  @override
  String get localSearchPaths => '搜索路径';

  @override
  String get localSearchFileTypes => '文件类型';

  @override
  String get localSearchScanDepth => '扫描深度';

  @override
  String get localSearchFilenameOnly => '仅文件名';

  @override
  String get localSearchFilenameOnlyDesc => '最快 — 仅检查文件名';

  @override
  String get localSearchFingerprint => '文件名 + 内容指纹';

  @override
  String get localSearchFingerprintDesc => '平衡 — 正则匹配内容';

  @override
  String get localSearchFullText => '全文解析';

  @override
  String get localSearchFullTextDesc => '最慢 — 深度内容分析';

  @override
  String get localSearchDefaultPaths => '使用默认路径';

  @override
  String get localSearchDefaultPathsDesc => '文档、桌面、下载';

  @override
  String get localSearchCustomPaths => '自定义路径';

  @override
  String get localSearchCustomPathsDesc => '选择特定文件夹';

  @override
  String get localSearchAddFolder => '添加文件夹';

  @override
  String get localSearchStartScan => '开始扫描';

  @override
  String get localSearchScanning => '扫描中...';

  @override
  String get localSearchScanned => '已扫描';

  @override
  String get localSearchFound => '发现';

  @override
  String get localSearchSkipped => '已跳过';

  @override
  String get localSearchCancelScan => '取消扫描';

  @override
  String get localSearchGoBack => '返回';

  @override
  String get localSearchScanAgain => '重新扫描';

  @override
  String get localSearchNoResults => '未找到结果';

  @override
  String get scanImportComplete => '导入完成';

  @override
  String get scanImportGoHome => '回到首页';

  @override
  String get scanImportCreated => '已创建';

  @override
  String get scanImportUpdated => '已更新';

  @override
  String get scanImportFields => '字段';

  @override
  String get scanImportSkipped => '已跳过';

  @override
  String get scanPreviewTitle => '预览并确认';

  @override
  String get scanPreviewNew => '新建';

  @override
  String get scanPreviewUpdate => '更新';

  @override
  String get scanPreviewImportAction => '导入操作';

  @override
  String get settingsAccount => '账户';

  @override
  String get settingsCurrentAccount => '当前账户';

  @override
  String get settingsAllAccounts => '所有账户';

  @override
  String get settingsDataManagement => '数据管理';

  @override
  String get settingsErrorLoadingAccounts => '加载账户出错';

  @override
  String get settingsPleaseRestart => '请重启应用';

  @override
  String get settingsAccess => '访问';

  @override
  String get settingsLockVault => '锁定保险库';

  @override
  String get settingsLockVaultDesc => '立即锁定并需要密码';

  @override
  String get settingsChangePassword => '更改主密码或密码提示';

  @override
  String get settingsChangePasswordDesc => '更新您的保险库密码或密码提示';

  @override
  String get settingsSecurity => '安全';

  @override
  String get settingsAutoLockPrivacy => '自动锁定与隐私';

  @override
  String get settingsAutoLockPrivacyDesc => '配置超时和隐私设置';

  @override
  String get settingsVerifyPassword => '请输入主密码以访问安全设置。';

  @override
  String get settingsSensitivity => '敏感度设置';

  @override
  String get settingsSensitivityDesc => '配置字段敏感度';

  @override
  String get settingsOperationLog => '操作日志';

  @override
  String get settingsOperationLogDesc => '查看活动历史';

  @override
  String get settingsSync => '同步';

  @override
  String get settingsCloudSync => '云同步';

  @override
  String get settingsNotConfigured => '未配置';

  @override
  String get settingsOfflineMode => '离线模式';

  @override
  String get settingsOfflineModeDesc => '仅本地数据';

  @override
  String get settingsAiAssistant => 'AI 助手';

  @override
  String get settingsLlmConfig => 'LLM 配置';

  @override
  String get settingsLlmConfigDesc => '本地模型或云端 API';

  @override
  String get settingsAbout => '关于';

  @override
  String get settingsVersion => '版本';

  @override
  String get settingsDebugLog => '调试日志';

  @override
  String get settingsDebugLogDesc => '查看调试日志';

  @override
  String get settingsPrivacyPolicy => '隐私政策';

  @override
  String get settingsPrivacyPolicyDesc => '查看我们的隐私政策';

  @override
  String get settingsTermsOfService => '服务条款';

  @override
  String get settingsTermsOfServiceDesc => '查看服务条款';

  @override
  String get settingsLocal => '本地';

  @override
  String get settingsPrivate => '私有';

  @override
  String get settingsUniversal => '通用';

  @override
  String get securityVaultSecurity => '保险库安全';

  @override
  String get securityAutoLockDelay => '自动锁定延迟';

  @override
  String get securityAutoLockDesc => '不活动后锁定保险库';

  @override
  String get securityBiometricUnlock => '生物识别解锁';

  @override
  String get securityPrivacy => '隐私';

  @override
  String get securityAppPrivacyScreen => '应用隐私屏幕';

  @override
  String get securityAppPrivacyDesc => '在应用切换器中隐藏内容';

  @override
  String get securityLockOnBlur => '窗口失焦时锁定';

  @override
  String get securityLockOnBlurDesc => '切换应用时锁定';

  @override
  String get securityClipboard => '剪贴板';

  @override
  String get securityAutoClearDelay => '自动清除延迟';

  @override
  String get securityAutoClearDesc => '复制敏感数据后清除剪贴板';

  @override
  String get sensitivityVerifyPassword => '请输入主密码以访问敏感度设置。';

  @override
  String get sensitivityCritical => '受限';

  @override
  String get sensitivityCriticalDesc => '最高敏感度 - 始终遮罩，需要验证';

  @override
  String get sensitivitySensitive => '敏感';

  @override
  String get sensitivitySensitiveDesc => '需要保护的个人信​​息';

  @override
  String get sensitivityInternal => '内部';

  @override
  String get sensitivityInternalDesc => '仅供内部使用 - 可被显示设置隐藏';

  @override
  String get sensitivityPublic => '公开';

  @override
  String get sensitivityPublicDesc => '最低敏感度 - 始终可见';

  @override
  String get syncSynchronizing => '同步中...';

  @override
  String get syncDeviceDiscovery => '设备发现';

  @override
  String get syncManualConnection => '手动连接';

  @override
  String get syncLastSync => '上次同步';

  @override
  String get syncStatus => '状态';

  @override
  String get syncDirection => '方向';

  @override
  String get syncData => '数据';

  @override
  String get syncError => '错误';

  @override
  String get trashVerifyPassword => '请输入主密码以查看回收站。';

  @override
  String get trashRestored => '已恢复 ';

  @override
  String get trashPermanentlyDeleted => '已永久删除 ';

  @override
  String get operationLogVerifyPassword => '请输入主密码以查看操作日志。';

  @override
  String get dataMgmtRestoreBackup => '恢复备份？';

  @override
  String get dataMgmtDeleteBackup => '删除备份？';

  @override
  String get dataMgmtConfirmDeletion => '请输入主密码以确认删除备份。';

  @override
  String get llmApiEndpoint => 'API 端点';

  @override
  String get llmModel => '模型';

  @override
  String get llmAnthropicVersion => 'Anthropic API 版本';

  @override
  String get llmOpenAI => 'OpenAI';

  @override
  String get llmAnthropic => 'Anthropic';

  @override
  String get searchUnlock => '解锁';

  @override
  String get searchDeleted => '已删除';

  @override
  String get searchReveal => '显示';

  @override
  String get searchRestrictedHint => '受限 - 需要密码查看';

  @override
  String get searchPrivateHint => '私密 - 点击显示';

  @override
  String get sensitivityRestricted => '受限';

  @override
  String get commonAdd => '添加';

  @override
  String get commonCopy => '复制';

  @override
  String get dialogCurrentPassword => '当前密码';

  @override
  String get dialogNewPassword => '新密码';

  @override
  String get dialogConfirmNewPassword => '确认新密码';

  @override
  String get dialogChange => '修改';

  @override
  String get dialogLock => '锁定';

  @override
  String get dialogVerifyIdentity => '验证身份';

  @override
  String get dialogDeleteItem => '删除项目';

  @override
  String get dialogDeleteSection => '删除分区？';

  @override
  String get dialogDeleteSectionConfirm => '该分区及其项目将被移至回收站。';

  @override
  String get biometricTestTouchId => '测试 Touch ID';

  @override
  String get biometricTestFaceId => '测试 Face ID';

  @override
  String get dialogAddQuickAction => '添加快捷操作';

  @override
  String get homePageEditorSections => '分区';

  @override
  String get homePageEditorIcon => '图标';

  @override
  String get homePageEditorSectionTitle => '分区标题';

  @override
  String get settingsDeleteAccount => '删除账户';

  @override
  String get settingsDebugLogCopyTitle => '复制日志到剪贴板';

  @override
  String get settingsDebugLogCopied => '已复制脱敏日志到剪贴板';

  @override
  String get settingsDebugLogTitle => '调试日志';

  @override
  String get dialogSelectFolder => '选择此文件夹';

  @override
  String get iconPickerTitle => '选择图标';

  @override
  String get iconCategoryWork => '工作与学习';

  @override
  String get iconCategoryPeople => '人物与身份';

  @override
  String get iconCategoryTravel => '旅行与出行';

  @override
  String get iconCategoryFinance => '财务与商业';

  @override
  String get iconCategoryLife => '生活与健康';

  @override
  String get iconCategoryTech => '科技与设备';

  @override
  String get iconCategoryCreative => '创意与艺术';

  @override
  String get iconCategoryGeneral => '通用';

  @override
  String get operationDetails => '操作详情';

  @override
  String get trashHistory => '历史';

  @override
  String dialogUseBiometric(String biometricType) {
    return '使用$biometricType';
  }

  @override
  String dialogDeleteItemConfirm(String name) {
    return '确定要删除\"$name\"吗？';
  }

  @override
  String entryHistoryCount(int count) {
    return '历史($count)';
  }

  @override
  String biometricPasswordHint(String hint) {
    return '密码提示：$hint';
  }

  @override
  String get settingsUnknown => '未知';

  @override
  String get settingsActive => '活跃';

  @override
  String get settingsCloudSyncSetup => '云同步设置';

  @override
  String get settingsComingSoon => '此功能将在未来更新中提供。';

  @override
  String get settingsTagline => '您的本地数字孪生。隐私优先的通用身份。';

  @override
  String get settingsVerifyIdentityDebug => '验证身份以启用调试模式';

  @override
  String get loginNoPasswordHint => '无密码提示';

  @override
  String get commonVerify => '验证';

  @override
  String get commonRefresh => '刷新';

  @override
  String get commonShowLess => '收起';

  @override
  String get debugLogCopyToClipboard => '复制到剪贴板';

  @override
  String get debugLogDisable => '禁用调试模式';

  @override
  String get debugLogEmpty => '暂无调试日志';

  @override
  String get deleteAccountEnterPassword => '输入密码以确认';

  @override
  String get deleteAccountPasswordRequired => '密码为必填项';

  @override
  String get deleteAccountInvalidPassword => '密码无效';

  @override
  String get pageEditorPageTitleHint => '页面标题';

  @override
  String get pageEditorSaveFirst => '请先保存页面以添加分区';

  @override
  String get pageEditorNoSections => '暂无分区';

  @override
  String get pageNoSections => '暂无分区';

  @override
  String get pageRestoreDefaults => '恢复默认分区';

  @override
  String get pageEditorEnterSectionTitle => '输入分区标题';

  @override
  String get pageEditorEditSectionTitle => '编辑分区';

  @override
  String get folderPickerGoUp => '上一级';

  @override
  String get headerLockSensitivity => '锁定敏感信息访问';

  @override
  String get datePickerClear => '清除日期';

  @override
  String get entryCopyAll => '复制全部';

  @override
  String get entryNoHistory => '暂无历史记录';

  @override
  String get operationViewDetails => '查看详情';

  @override
  String get scanStopScan => '停止扫描';

  @override
  String get settingsNoHintAvailable => '无可用提示';

  @override
  String get sensitiveRestrictedMessage => '受限字段。请输入主密码查看。';

  @override
  String get syncUnknownError => '未知错误';

  @override
  String get syncScanning => '扫描中...';

  @override
  String get syncScan => '扫描';

  @override
  String get syncSyncing => '同步中...';

  @override
  String get syncConnectSync => '连接并同步';

  @override
  String get scanMappingBoth => 'AI+规则';

  @override
  String get scanMappingAi => 'AI';

  @override
  String get mrzDocumentType => '证件类型';

  @override
  String get mrzDocumentNumber => '证件号码';

  @override
  String get mrzSurname => '姓氏';

  @override
  String get mrzGivenNames => '名字';

  @override
  String get mrzNationality => '国籍';

  @override
  String get mrzDateOfBirth => '出生日期';

  @override
  String get mrzSex => '性别';

  @override
  String get mrzExpiryDate => '有效期';

  @override
  String get changePasswordMinLength => '最少8个字符';

  @override
  String get operationActionCreate => '创建';

  @override
  String get operationActionUpdate => '更新';

  @override
  String get operationActionDelete => '删除';

  @override
  String get operationActionRestore => '恢复';

  @override
  String get operationActionPurge => '彻底删除';

  @override
  String get operationPlatformAndroid => '安卓';

  @override
  String get operationPlatformWeb => '网页';

  @override
  String get operationLabelTimestamp => '时间戳';

  @override
  String get operationLabelAction => '操作';

  @override
  String get operationLabelSection => '分区';

  @override
  String get operationLabelFieldPath => '字段路径';

  @override
  String get operationLabelDescription => '描述';

  @override
  String get operationLabelDevice => '设备';

  @override
  String get versionCurrentVersion => '当前版本';

  @override
  String get versionLatestVersion => '最新版本';

  @override
  String get versionUpdateStatus => '更新状态';

  @override
  String get versionPlatform => '平台';

  @override
  String get accountCreated => '创建时间';

  @override
  String get accountLastLogin => '最后登录';

  @override
  String get accountLastOperation => '最后操作';

  @override
  String get accountLoginDevices => '登录设备';

  @override
  String get homeDefaultPages => '默认页面';

  @override
  String get homeCustomizedPages => '自定义页面';

  @override
  String get trashDetailLabel => '详情';

  @override
  String get trashRestoreLabel => '恢复';

  @override
  String get trashPurgeLabel => '彻底删除';

  @override
  String get commonTitle => '标题';

  @override
  String predefinedUnknownType(String type) {
    return '未知类型：$type';
  }

  @override
  String get commonShowPassword => '显示密码';

  @override
  String get commonHidePassword => '隐藏密码';

  @override
  String get objectCardAddItem => '添加项目';

  @override
  String get dataManagementRestoreBackupTooltip => '恢复';

  @override
  String get dataManagementSpecialBackupTooltip => '保存为特殊备份';

  @override
  String get dataManagementSpecialBackupsTitle => '特别备份';

  @override
  String get dataManagementNoSpecialBackups => '暂无特别备份。创建一个以保留特定版本。';

  @override
  String dataManagementSpecialBackupsCount(int count, int max) {
    return '$count / $max 个特别备份';
  }

  @override
  String dataManagementBackupsSummary(int count, String totalSize) {
    return '$count 个常规备份 · 共 $totalSize';
  }

  @override
  String get passwordVerificationRestricted => '受限字段。请输入主密码以继续。';

  @override
  String get passwordVerificationInvalid => '密码无效';

  @override
  String passwordVerificationBackoff(Object seconds) {
    return '密码错误次数过多，请等待 $seconds 秒后重试。';
  }

  @override
  String passwordVerificationLockedOut(Object minutes) {
    return '账户已锁定，请等待 $minutes 分钟。';
  }

  @override
  String get trashPasswordRequired => '需要密码';

  @override
  String trashEmptyConfirm(int count) {
    return '确定要永久删除回收站中的全部 $count 个项目吗？';
  }

  @override
  String get trashEmptyWarning => '此操作不可撤销。所有项目将被永久删除。';

  @override
  String trashEmptyComplete(int count) {
    return '已永久删除全部 $count 个项目';
  }

  @override
  String trashRestoreConfirmBody(String name) {
    return '确定要恢复\"$name\"吗？';
  }

  @override
  String trashRestoredItem(String name) {
    return '已恢复\"$name\"';
  }

  @override
  String trashPermanentDeleteConfirm(String name) {
    return '确定要永久删除\"$name\"吗？';
  }

  @override
  String get trashPermanentDeleteWarning => '此操作不可撤销。该项目将被永久删除。';

  @override
  String trashPermanentDeletedItem(String name) {
    return '已永久删除\"$name\"';
  }

  @override
  String get trashEmpty => '回收站为空';

  @override
  String get trashNoMatching => '无匹配项目';

  @override
  String get trashDeletedAppear => '已删除的项目将显示在此处';

  @override
  String get trashAdjustSearch => '尝试调整搜索条件';

  @override
  String trashFoundResults(int count) {
    return '找到 $count 条结果';
  }

  @override
  String get trashNoResults => '未找到结果';

  @override
  String trashTotalItems(int count) {
    return '回收站中共 $count 个项目';
  }

  @override
  String get trashSectionTitle => '页面与对象';

  @override
  String get trashAutoPurgeNotice => '回收站中的项目在30天后将被永久删除';

  @override
  String get trashEmptyTrashButton => '清空回收站';

  @override
  String get operationLogPasswordRequired => '需要密码';

  @override
  String get operationLogClearConfirm => '确定要清除所有操作历史吗？';

  @override
  String operationLogFoundResults(int count) {
    return '找到 $count 条结果';
  }

  @override
  String get operationLogNoMatching => '无匹配条目';

  @override
  String get operationLogTryDifferent => '尝试其他搜索词';

  @override
  String get operationLogAdjustFilters => '尝试调整筛选条件';

  @override
  String get operationLogFilters => '筛选条件';

  @override
  String get sensitivityPasswordRequired => '需要密码';

  @override
  String sensitivityDowngradeWarning(String name) {
    return '您正在将\"$name\"降级到更低的敏感度级别。';
  }

  @override
  String get sensitivityDowngradeConfirm => '此字段将以更少的保护可见。继续？';

  @override
  String sensitivityMovedToPrivate(String name) {
    return '\"$name\"已移至私有';
  }

  @override
  String get sensitivityNoFields => '此分区无字段';

  @override
  String get sensitivityKeepHighest => '保持最高';

  @override
  String get sensitivityMoveHigher => '移至更高';

  @override
  String get sensitivityKeepLowest => '保持最低';

  @override
  String get sensitivityMoveLower => '移至更低';

  @override
  String sensitivityMovedHigher(String name) {
    return '\"$name\"已移至更高敏感度';
  }

  @override
  String sensitivityFoundResults(int count) {
    return '找到 $count 条结果';
  }

  @override
  String get sensitivityNoResults => '未找到结果';

  @override
  String get sensitivityAdjustHint => '调整每个字段的敏感度级别。受限字段需要额外验证才能查看。';

  @override
  String sensitivityNoFieldsMatch(String query) {
    return '无字段匹配\"$query\"';
  }

  @override
  String sensitivityFieldsConfigured(int count) {
    return '已配置 $count 个字段';
  }

  @override
  String sensitivityTotalFields(int count) {
    return '共 $count 个字段';
  }

  @override
  String get commonNA => '不适用';

  @override
  String accountIdLabel(String id) {
    return '账户 ID：$id';
  }

  @override
  String get accountNoRecentOps => '无最近操作';

  @override
  String get accountNoDevices => '无设备记录';

  @override
  String get accountRecentDevices => '最近设备';

  @override
  String get settingsAllAccountsTitle => '全部账户';

  @override
  String accountLastLoginLabel(String time) {
    return '最后登录：$time';
  }

  @override
  String get versionUnavailable => '不可用';

  @override
  String get versionUpToDate => '已是最新';

  @override
  String get versionUpdateAvailable => '有可用更新';

  @override
  String settingsAccountCount(int count) {
    return '$count 个账户';
  }

  @override
  String get debugLogSanitizeWarning => '日志将在复制前脱敏，但剪贴板内容对其他应用可见。使用后应清除剪贴板。';

  @override
  String get debugLogActiveNotice => '调试模式已激活。正在记录日志。';

  @override
  String get homeVaultUnlocked => '保险库已解锁';

  @override
  String get homeOnline => '在线';

  @override
  String get homeOffline => '离线';

  @override
  String get searchEnterMinChars => '输入至少2个字符进行搜索';

  @override
  String get searchNoResultsBody => '未找到结果';

  @override
  String get searchAdjustFilters => '尝试调整筛选条件或搜索词';

  @override
  String get syncComplete => '同步完成';

  @override
  String syncFailed(String error) {
    return '同步失败：$error';
  }

  @override
  String get syncDirectionPushed => '已推送本地更改';

  @override
  String get syncDirectionPulled => '已拉取远程更改';

  @override
  String get syncDirectionMerged => '已合并双方更改';

  @override
  String get syncDirectionNoChange => '已同步';

  @override
  String get syncDiscoveryHint => '扫描本地网络中的 SoloSoul 设备';

  @override
  String syncFoundDevices(int count) {
    return '发现 $count 台设备';
  }

  @override
  String get syncTestFailed => '失败';

  @override
  String get syncDirectionPush => '推送';

  @override
  String get syncDirectionPull => '拉取';

  @override
  String get syncDirectionMergedShort => '已合并';

  @override
  String get syncDirectionNoChangeShort => '无更改';

  @override
  String get localSearchScanLocalFiles => '扫描本地文件';

  @override
  String get localSearchDescription => '在本地文件中搜索个人信息并导入到您的保险库中。';

  @override
  String get localSearchSelectHint => '点击选择，长按调整大小限制。';

  @override
  String get localSearchPrivacyNotice => '所有扫描均在本地进行。数据不会离开您的设备。导入前您将预览所有结果。';

  @override
  String localSearchSkipLargerThan(String label) {
    return '跳过大于以下大小的 $label 文件：';
  }

  @override
  String get localSearchScanningFiles => '正在扫描文件...';

  @override
  String get localSearchScanCanceled => '扫描已取消';

  @override
  String get localSearchScanComplete => '扫描完成';

  @override
  String get localSearchNoResultsBody =>
      '扫描文件中未发现个人信息。请尝试使用\"全文解析\"模式或添加更多文件夹。';

  @override
  String get localSearchNoFiles => '无文件';

  @override
  String get scanDeselectAll => '取消全选';

  @override
  String get ocrScanDescription => '扫描护照、身份证或任何文档';

  @override
  String get ocrNoTextDetected => '未检测到文本';

  @override
  String get ocrBusinessCardSaved => '名片已保存';

  @override
  String get ocrInvoiceSaved => '发票已保存';

  @override
  String get ocrDocumentSavedAsNote => '文档已保存为笔记';

  @override
  String get ocrBusinessCard => '名片';

  @override
  String get ocrInvoice => '发票';

  @override
  String get ocrResume => '简历';

  @override
  String get ocrNoResumeSections => '未检测到简历分区';

  @override
  String get ocrResumeSaved => '简历已保存';

  @override
  String ocrResumeSavedSections(int count) {
    return '简历已保存，共 $count 个分区';
  }

  @override
  String ocrSavedSectionsFailed(int success, int fail) {
    return '已保存 $success 个分区，$fail 个失败';
  }

  @override
  String get ocrScannedDocument => '扫描文档';

  @override
  String get ocrUseCamera => '使用相机拍摄文档';

  @override
  String get ocrPhotoOrPdf => '照片或 PDF 文件';

  @override
  String commonShowMore(int count) {
    return '显示另外 $count 项';
  }

  @override
  String get commonCopiedToClipboard => '已复制到剪贴板';

  @override
  String get commonUntitled => '无标题';

  @override
  String predefinedDeletedItem(String title, String name) {
    return '已删除 $title：$name';
  }

  @override
  String commonErrorWithMessage(String message) {
    return '错误：$message';
  }

  @override
  String get commonObject => '对象';

  @override
  String get fieldHistoryLatest => '最新';

  @override
  String get commonEmpty => '(空)';

  @override
  String get lockVaultMessage => '锁定保险库后需要主密码才能解锁。';

  @override
  String get changePasswordWarning => '修改密码将使用新密钥重新加密您的所有数据，您也可以选择仅修改密码提示。';

  @override
  String get changePasswordCurrentRequired => '请输入当前密码';

  @override
  String get changePasswordNewRequired => '请输入新密码';

  @override
  String get changePasswordMustDiffer => '新密码必须与当前密码不同';

  @override
  String get changePasswordFailed => '密码修改失败';

  @override
  String get errorInvalidCurrentPassword => '当前密码不正确';

  @override
  String entryAttachments(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个附件',
      one: '1 个附件',
    );
    return '$_temp0';
  }

  @override
  String get profileEncryptionDesc => '您的数据使用 AES-256-GCM 加密';

  @override
  String get financialEncryptionDesc => '您的财务数据使用 AES-256-GCM 加密';

  @override
  String llmStatsPrompt(String tokens) {
    return '提示词 $tokens';
  }

  @override
  String llmStatsCompletion(String tokens) {
    return '补全 $tokens';
  }

  @override
  String get llmProviderOllama => 'Ollama';

  @override
  String get homeNoMorePages => '没有更多页面可添加';

  @override
  String get homeDefaultAccountName => '账户';

  @override
  String get syncEnterPairingKey => '输入从另一台设备共享的配对密钥。';

  @override
  String get ocrNoTextDetectedImage => '图像中未检测到文本。请使用更清晰的文档照片重试。';

  @override
  String get ocrNoTextDetectedPdf => 'PDF 中未检测到文本。请使用更清晰的扫描文档重试。';

  @override
  String get ocrRecognitionTimeoutImage => '识别超时。请使用更清晰的图像重试。';

  @override
  String get ocrRecognitionTimeoutPdf => '识别超时。请使用更清晰的 PDF 重试。';

  @override
  String get ocrPdfRenderFailed => '无法渲染 PDF 页面。文件可能已损坏或受密码保护。';

  @override
  String get commonAddItem => '添加项目';

  @override
  String get profileAddContact => '添加联系人';

  @override
  String get profileEditContact => '编辑联系人';

  @override
  String get commonAddButton => '添加';

  @override
  String get profileIdCard => '身份证';

  @override
  String get profileAddress => '地址';

  @override
  String get syncScanHint => '扫描本地网络中的 SoloSoul 设备。';

  @override
  String get syncPairingHint => '生成共享配对密钥以建立设备间的安全连接。两台设备必须使用相同的密钥。';

  @override
  String get syncTestSuccess => '成功';

  @override
  String get trashEmptyMessage => '回收站为空';

  @override
  String accountDeviceCount(int count) {
    return '$count 台设备';
  }

  @override
  String get profileEncryptionTitle => '端到端加密';

  @override
  String get profileIdCardSection => '身份证';

  @override
  String profileFormatIdentity(String data) {
    return '身份\n$data';
  }

  @override
  String profileFormatIdCard(String data) {
    return '身份证\n$data';
  }

  @override
  String travelFormatPassport(String data) {
    return '护照\n$data';
  }

  @override
  String travelFormatVisa(String data) {
    return '签证\n$data';
  }

  @override
  String travelFormatHistory(String data) {
    return '旅行历史\n$data';
  }

  @override
  String financialFormatBankAccount(String data) {
    return '银行账户\n$data';
  }

  @override
  String financialFormatCard(String data) {
    return '银行卡\n$data';
  }

  @override
  String financialFormatTaxId(String data) {
    return '税号\n$data';
  }

  @override
  String professionalFormatEducation(String data) {
    return '教育\n$data';
  }

  @override
  String professionalFormatEmployment(String data) {
    return '工作\n$data';
  }

  @override
  String professionalFormatAward(String data) {
    return '奖项\n$data';
  }

  @override
  String professionalFormatSkill(String data) {
    return '技能\n$data';
  }

  @override
  String professionalFormatLanguage(String data) {
    return '语言\n$data';
  }

  @override
  String professionalFormatArticle(String data) {
    return '文章\n$data';
  }

  @override
  String get fieldFullName => '全名';

  @override
  String get fieldGivenName => '名字';

  @override
  String get fieldFamilyName => '姓氏';

  @override
  String get fieldDateOfBirth => '出生日期';

  @override
  String get fieldDate => '日期';

  @override
  String get fieldGender => '性别';

  @override
  String get fieldNationality => '国籍';

  @override
  String get fieldTitle => '标题';

  @override
  String get fieldType => '类型';

  @override
  String get fieldValue => '值';

  @override
  String get fieldNumber => '号码';

  @override
  String get fieldIdCardNumber => '身份证号码';

  @override
  String get fieldIssueDate => '签发日期';

  @override
  String get fieldExpiryDate => '过期日期';

  @override
  String get fieldHolderName => '持有人姓名';

  @override
  String get fieldCountry => '国家';

  @override
  String get fieldStreet => '街道';

  @override
  String get fieldCity => '城市';

  @override
  String get fieldState => '州/省';

  @override
  String get fieldPostalCode => '邮政编码';

  @override
  String get fieldPassportNumber => '护照号码';

  @override
  String get fieldIssuingCountry => '签发国';

  @override
  String get fieldVisaNumber => '签证号码';

  @override
  String get fieldEntryDate => '入境日期';

  @override
  String get fieldExitDate => '离境日期';

  @override
  String get fieldSwiftCode => 'SWIFT 代码';

  @override
  String get fieldIban => 'IBAN';

  @override
  String get fieldCardNumber => '卡号';

  @override
  String get fieldCardholderName => '持卡人姓名';

  @override
  String get fieldCvv => 'CVV';

  @override
  String get fieldTaxIdNumber => '税号';

  @override
  String get fieldInstitution => '机构';

  @override
  String get fieldDegree => '学位';

  @override
  String get fieldFieldOfStudy => '专业领域';

  @override
  String get fieldStartDate => '开始日期';

  @override
  String get fieldEndDate => '结束日期';

  @override
  String get fieldCompany => '公司';

  @override
  String get fieldPosition => '职位';

  @override
  String get fieldCategory => '类别';

  @override
  String get fieldLevel => '等级';

  @override
  String get fieldLanguage => '语言';

  @override
  String get fieldProficiency => '熟练度';

  @override
  String get fieldOrganization => '组织';

  @override
  String get fieldPhone => '电话';

  @override
  String get fieldEmail => '邮箱';

  @override
  String get fieldContent => '内容';

  @override
  String get fieldDone => '已完成';

  @override
  String get fieldDueDate => '截止日期';

  @override
  String get commonYes => '是';

  @override
  String get commonNo => '否';

  @override
  String get fieldCountryCode => '国家代码';

  @override
  String get fieldPlaceOfIssue => '签发地点';

  @override
  String get fieldPlaceOfBirth => '出生地点';

  @override
  String get fieldSex => '性别';

  @override
  String get fieldAuthority => '签发机关';

  @override
  String get fieldVisaType => '签证类型';

  @override
  String get fieldDestination => '目的地';

  @override
  String get fieldTravelType => '旅行类型';

  @override
  String get fieldDepartureCity => '出发城市';

  @override
  String get fieldDepartureTime => '出发时间';

  @override
  String get fieldArrivalTime => '到达时间';

  @override
  String get fieldFlightNumber => '航班号';

  @override
  String get fieldTicketPrice => '票价';

  @override
  String get fieldAirline => '航空公司';

  @override
  String get fieldCurrency => '货币';

  @override
  String get fieldSwiftBic => 'SWIFT/BIC';

  @override
  String get fieldSortCode => '排序码';

  @override
  String get fieldCardType => '卡片类型';

  @override
  String get fieldTaxIdType => '税务识别号类型';

  @override
  String get fieldIssuingAuthority => '签发机关';

  @override
  String get fieldDegreeCustom => '自定义学位信息';

  @override
  String get fieldField => '专业领域';

  @override
  String get fieldResponsibilities => '工作职能';

  @override
  String get fieldIssuer => '颁奖方';

  @override
  String get fieldDescription => '详细信息';

  @override
  String get fieldName => '名称';

  @override
  String get fieldAuthors => '作者';

  @override
  String get fieldContactInfo => '联系方式';

  @override
  String get fieldAbstract => '摘要';

  @override
  String get fieldDoi => 'DOI';

  @override
  String get fieldUrl => '网址';

  @override
  String get fieldVenue => '发表场所';

  @override
  String get fieldYear => '年份';

  @override
  String get fieldCitation => '引用格式';

  @override
  String get datePickerSelectDate => '选择日期';

  @override
  String get headerSensitiveAccessLocked => '敏感访问已锁定';

  @override
  String get operationLogPropertySnapshot => '属性快照';

  @override
  String get dataMgmtVaultDataSize => '保险库数据大小';

  @override
  String get dataMgmtAppVersion => '应用版本';

  @override
  String dataMgmtRestoreOverwrite(String time) {
    return '这将用 $time 的备份覆盖您当前的数据。系统将首先创建当前状态的安全备份。';
  }

  @override
  String get dataMgmtRestoreSuccess => '恢复成功。请重启应用。';

  @override
  String get dataMgmtRestoreFailed => '恢复失败';

  @override
  String dataMgmtDeleteBackupConfirm(String time) {
    return '删除 $time 的备份？';
  }

  @override
  String get dataMgmtBackupCreated => '备份创建成功';

  @override
  String get dataMgmtBackupFailed => '备份失败';

  @override
  String dataMgmtBackupError(String error) {
    return '备份错误：$error';
  }

  @override
  String get dataMgmtBackupDeleted => '备份已删除';

  @override
  String get dataMgmtOperationCreatedBackup => '创建了备份';

  @override
  String get dataMgmtOperationRestoredBackup => '恢复了备份';

  @override
  String get dataMgmtOperationDeletedBackup => '删除了备份';

  @override
  String get dataMgmtOperationPromotedBackup => '将备份升级为特殊备份';

  @override
  String get dataMgmtOperationCreatedSpecial => '创建了特殊备份';

  @override
  String get dataMgmtOperationRenamedSpecial => '重命名了特殊备份';

  @override
  String get dataMgmtOperationRestoredSpecial => '恢复了特殊备份';

  @override
  String dataMgmtSpecialBackupSaved(String name) {
    return '已保存为特殊备份 \"$name\"';
  }

  @override
  String get dataMgmtSpecialBackupFailed => '保存为特殊备份失败';

  @override
  String dataMgmtSpecialBackupCreated(String name) {
    return '特殊备份 \"$name\" 已创建';
  }

  @override
  String get dataMgmtSpecialBackupCreateFailed => '特殊备份创建失败';

  @override
  String dataMgmtRenamedTo(String name) {
    return '已重命名为 \"$name\"';
  }

  @override
  String dataMgmtSpecialBackupLimit(int max) {
    return '最多可保留 $max 个特殊备份。请先删除一个现有备份后再创建新的特殊备份。';
  }

  @override
  String dataMgmtSpecialBackupPromoteLimit(int max) {
    return '最多可保留 $max 个特殊备份。请先删除一个现有备份后再升级。';
  }

  @override
  String get dataMgmtSafetyBackupNotice => '系统将首先创建当前状态的安全备份。';

  @override
  String get dataMgmtSpecialBackupRestored => '特殊备份已恢复。请重启应用。';

  @override
  String get dataMgmtOperationDeletedSpecial => '删除了特殊备份';

  @override
  String get operationCreatedAccount => '创建了账户';

  @override
  String get operationDeletedAccount => '删除了账户';

  @override
  String get operationChangedPassword => '修改了密码';

  @override
  String get dataMgmtVaultSize => '保险库大小：';

  @override
  String get dataMgmtBackupEncryptionDesc => '备份使用您的保险库密钥加密。每次解锁时自动运行备份。';

  @override
  String get dataMgmtRegularBackups => '常规备份';

  @override
  String get dataMgmtNoBackups => '暂无备份';

  @override
  String get loginDataYourControl => '您的数据，由您掌控';

  @override
  String get loginEnterMasterPassword => '输入主密码';

  @override
  String get loginUnlockYourVault => '解锁您的保险库';

  @override
  String get loginUnlockButton => '解锁';

  @override
  String get loginNoPasswordRecovery => '没有密码恢复机制。如果您忘记了主密码，将无法访问您的数据。';

  @override
  String get loginPleaseEnterPassword => '请输入您的密码';

  @override
  String get securityBiometricUnlockSubtitle => '使用面容 ID / 触控 ID 解锁';

  @override
  String get securityBiometricNotAvailable => '此设备不支持生物识别';

  @override
  String get scanAttachFile => '保存原始文件作为附件';

  @override
  String trashDaysAgo(Object count) {
    return '$count 天前';
  }

  @override
  String trashHoursAgo(Object count) {
    return '$count 小时前';
  }

  @override
  String trashMinutesAgo(Object count) {
    return '$count 分钟前';
  }

  @override
  String get trashJustNow => '刚刚';

  @override
  String get trashDeletedRecently => '最近删除';

  @override
  String trashDeletedAgo(Object time) {
    return '$time删除';
  }

  @override
  String get trashShowSections => '显示分区';

  @override
  String get trashShowItems => '显示项目';

  @override
  String get typeCollection => '分区';

  @override
  String get typePage => '页面';

  @override
  String get typeItem => '项目';

  @override
  String get typeUnknown => '项目';

  @override
  String get operationPlatformMacos => 'macOS';

  @override
  String get operationPlatformIos => 'iOS';

  @override
  String get operationPlatformWindows => 'Windows';

  @override
  String get operationPlatformLinux => 'Linux';

  @override
  String get logSectionIdentity => '身份信息';

  @override
  String get logSectionContactInfo => '联系方式';

  @override
  String get logSectionAddress => '地址';

  @override
  String get logSectionIdCard => '身份证';

  @override
  String get logSectionPassport => '护照';

  @override
  String get logSectionVisa => '签证';

  @override
  String get logSectionTravelHistory => '出行记录';

  @override
  String get logSectionBankAccount => '银行账户';

  @override
  String get logSectionCard => '卡片';

  @override
  String get logSectionEducation => '教育';

  @override
  String get logSectionEmployment => '工作';

  @override
  String get logSectionSkill => '技能';

  @override
  String get logSectionLanguage => '语言';

  @override
  String get logSectionTravel => '出行';

  @override
  String get logSectionFinancial => '财务';

  @override
  String get logSectionProfessional => '职业';

  @override
  String get logSectionSensitivity => '敏感设置';

  @override
  String get logSectionCustom => '自定义';

  @override
  String get logSectionDefault => '分区';

  @override
  String get operationFilterLabel => '筛选';

  @override
  String get trashFilterLabel => '筛选：';

  @override
  String get trashTimeFilterLabel => '时间：';

  @override
  String get trashTimeFilterAll => '全部';

  @override
  String get trashTimeFilter10Days => '10 天内';

  @override
  String get trashTimeFilter1Day => '1 天内';

  @override
  String get trashTimeFilter6Hours => '6 小时内';

  @override
  String get trashTimeFilter1Hour => '1 小时内';

  @override
  String get trashTypeFilterLabel => '类型：';

  @override
  String get sectionTemplateTitle => '分区模板';

  @override
  String get sectionTemplateFilterAll => '全部';

  @override
  String get sectionTemplatePageTagBank => '银行';

  @override
  String sectionTemplateSelected(int count) {
    return '已选择 $count 个';
  }

  @override
  String get sectionTemplateSelectButton => '选择模板';

  @override
  String get sectionTemplateEmpty => '暂无可用模板';

  @override
  String get sectionTemplateEmptyHint => '配置后将显示模板';

  @override
  String sectionTemplateApplied(String name) {
    return '已应用模板 \"$name\"';
  }

  @override
  String get templateChinaBankAccountName => '中国银行账户';

  @override
  String get templateChinaBankAccountDesc => '包含中国各大银行账户信息';

  @override
  String get templateUkBankAccountName => '英国银行账户';

  @override
  String get templateUkBankAccountDesc =>
      '包含英国银行账户信息（Sort Code + Account Number）';

  @override
  String get templateUsBankAccountName => '美国银行账户';

  @override
  String get templateUsBankAccountDesc =>
      '包含美国银行账户信息（Routing Number + Account Number）';

  @override
  String sectionTemplateFieldCount(int count) {
    return '$count 个字段';
  }

  @override
  String get objectEditorDefaultFieldTitle => '标题';

  @override
  String get objectEditorDefaultFieldItemName => '项目名称';

  @override
  String objectEditorMaxLength(int count) {
    return '$count';
  }

  @override
  String get fieldBankName => '银行名称';

  @override
  String get fieldAccountNumber => '账号';

  @override
  String get fieldAccountHolder => '持有人';

  @override
  String get fieldBranchName => '支行名称';

  @override
  String get fieldRoutingNumber => '路由号码';

  @override
  String get fieldAccountType => '账户类型';

  @override
  String get fieldChecking => '支票账户';

  @override
  String get fieldSavings => '储蓄账户';

  @override
  String get templateProfileIdentityName => '身份信息';

  @override
  String get templateProfileIdentityDesc => '个人身份信息，包括姓名、出生日期、性别和国籍';

  @override
  String get templateProfileContactName => '联系方式';

  @override
  String get templateProfileContactDesc => '联系方式，如电话号码和电子邮件地址';

  @override
  String get templateProfileIdCardName => '身份证件';

  @override
  String get templateProfileIdCardDesc => '身份证件，包括身份证、驾驶证和护照';

  @override
  String get templateProfileAddressName => '地址';

  @override
  String get templateProfileAddressDesc => '物理地址，包括街道、城市、省份和邮政编码';

  @override
  String get templateFinancialBankAccountName => '银行账户';

  @override
  String get templateFinancialBankAccountDesc => '包含银行账户信息，包括账号和 SWIFT 代码';

  @override
  String get templateFinancialCardName => '卡片';

  @override
  String get templateFinancialCardDesc => '包含支付卡片信息，包括卡号和 CVV';

  @override
  String get templateFinancialTaxIdName => '税务识别号';

  @override
  String get templateFinancialTaxIdDesc => '包含税务识别号信息';

  @override
  String get templateProfessionalEducationName => '教育';

  @override
  String get templateProfessionalEducationDesc => '正规教育记录，包括学位、学校和专业';

  @override
  String get templateProfessionalEmploymentName => '就业';

  @override
  String get templateProfessionalEmploymentDesc => '工作经历，包括公司、职位、工作职责和任期';

  @override
  String get templateProfessionalSkillName => '技能';

  @override
  String get templateProfessionalSkillDesc => '专业技能和熟练程度';

  @override
  String get templateProfessionalLanguageName => '语言';

  @override
  String get templateProfessionalLanguageDesc => '语言和熟练程度';

  @override
  String get templateProfessionalAwardName => '奖项';

  @override
  String get templateProfessionalAwardDesc => '专业奖项、荣誉和认可';

  @override
  String get templateProfessionalArticleName => '文章';

  @override
  String get templateProfessionalArticleDesc =>
      '学术文章与出版物，包括标题、作者、DOI、发表场所及引用格式';

  @override
  String get templateTravelPassportName => '护照';

  @override
  String get templateTravelPassportDesc => '包含护照信息，包括号码、签发日期、有效期和持有人详情';

  @override
  String get templateTravelVisaName => '签证';

  @override
  String get templateTravelVisaDesc => '包含签证信息，包括类型、号码、签发日期和有效期';

  @override
  String get templateTravelHistoryName => '出行记录';

  @override
  String get templateTravelHistoryDesc => '出行记录，包括目的地、日期、航班和出行详情';

  @override
  String get objectEditorSchemaUpdated => '属性定义已更新，自动同步中';

  @override
  String objectEditorShowDeprecated(int count) {
    return '$count 个已废弃';
  }

  @override
  String get objectEditorHideDeprecated => '隐藏废弃';

  @override
  String get objectEditorDeprecatedProperties => '废弃属性';

  @override
  String get objectEditorDeprecatedBadge => '已废弃';

  @override
  String get objectEditorRestoreProperty => '恢复属性';

  @override
  String operationLogCreatedItem(String name) {
    return '创建项目\"$name\"';
  }

  @override
  String operationLogUpdatedItem(String name) {
    return '更新项目\"$name\"';
  }

  @override
  String operationLogDeletedItem(String name) {
    return '删除项目\"$name\"';
  }

  @override
  String operationLogRestoredItem(String name) {
    return '恢复\"$name\"';
  }

  @override
  String operationNotifCreated(String name) {
    return '添加了\"$name\"';
  }

  @override
  String operationNotifUpdated(String name) {
    return '更新了\"$name\"';
  }

  @override
  String operationNotifDeleted(String name) {
    return '删除了\"$name\"';
  }

  @override
  String operationNotifRestored(String name) {
    return '恢复了\"$name\"';
  }

  @override
  String operationNotifPurged(String name) {
    return '彻底删除了\"$name\"';
  }

  @override
  String get operationNotifUndo => '撤销';

  @override
  String get operationNotifDismiss => '关闭';

  @override
  String get predefinedCopySuccess => '已复制到剪贴板';

  @override
  String predefinedDeleteFailed(String section) {
    return '删除$section失败';
  }

  @override
  String operationLogSensitivitySet(String field, String level) {
    return '已将\"$field\"敏感度设置为$level';
  }

  @override
  String operationLogSensitivityChanged(
    String field,
    String oldLevel,
    String newLevel,
  ) {
    return '已将\"$field\"敏感度从$oldLevel更改为$newLevel';
  }

  @override
  String operationLogSensitivityReverted(String field, String oldLevel) {
    return '已将\"$field\"敏感度恢复为默认值（原为$oldLevel）';
  }

  @override
  String get operationCreatedItem => '创建项目';

  @override
  String get operationUpdatedItem => '更新项目';

  @override
  String get operationDeletedItem => '删除项目';

  @override
  String get operationRestoredItem => '恢复项目';

  @override
  String get operationPurgedItem => '永久删除项目';

  @override
  String get operationChangedSensitivity => '更改敏感度';

  @override
  String get operationUpgradedSensitivity => '提升敏感度';

  @override
  String get operationDowngradedSensitivity => '降低敏感度';

  @override
  String get operationRevertedSensitivity => '恢复敏感度';

  @override
  String get pluginManagement => '插件管理';

  @override
  String get pluginManagementSubtitle => '管理已安装插件并浏览插件市场';

  @override
  String get pluginManagementSubtitleIOS => 'iOS 暂不支持运行插件';

  @override
  String get pluginIOSUnsupportedBanner => '由于平台限制，iOS 暂不支持运行插件。';

  @override
  String get pluginDashboardTitle => '插件管理';

  @override
  String get pluginTabAll => '全部';

  @override
  String get pluginTabInstalled => '已安装';

  @override
  String get pluginTabAvailable => '可安装';

  @override
  String get pluginSearchHint => '搜索插件...';

  @override
  String get pluginEmptyStateTitle => '暂无可用的插件';

  @override
  String get pluginEmptyStateSubtitle => '连接网络以浏览插件市场';

  @override
  String get pluginOfflineBanner => '离线模式，无法获取新插件';

  @override
  String get pluginStatusInstalled => '已安装';

  @override
  String get pluginStatusNotInstalled => '未安装';

  @override
  String get pluginStatusUpdateAvailable => '有更新';

  @override
  String get pluginStatusIncompatible => '不兼容';

  @override
  String get pluginStatusRunning => '运行中';

  @override
  String get pluginActionInstall => '安装';

  @override
  String get pluginActionUpdate => '更新';

  @override
  String get pluginActionRun => '运行';

  @override
  String get pluginActionStop => '停止';

  @override
  String get pluginActionUninstall => '卸载';

  @override
  String get pluginConsentDialogTitle => '插件数据授权';

  @override
  String pluginConsentDialogSubtitle(String pluginName) {
    return '插件 \"$pluginName\" 请求访问以下数据：';
  }

  @override
  String get pluginConsentDialogDataLifetime => '数据仅在本次会话期间可用，到期后自动销毁。';

  @override
  String get pluginConsentButtonDeny => '拒绝';

  @override
  String get pluginConsentButtonAuthorize => '授权访问';

  @override
  String get pluginSensitivityPublic => '公开';

  @override
  String get pluginSensitivityInternal => '内部';

  @override
  String get pluginSensitivitySensitive => '敏感';

  @override
  String get pluginSensitivityCritical => '关键';

  @override
  String get pluginUninstallConfirmTitle => '确定卸载插件？';

  @override
  String get pluginUninstallConfirmMessage => '此操作将删除插件文件并撤销所有数据授权，审计日志将保留。';

  @override
  String get pluginInstallSuccess => '插件安装成功';

  @override
  String get pluginUninstallSuccess => '插件卸载成功';

  @override
  String get pluginRunSuccess => '插件执行成功';

  @override
  String get pluginErrorNotFound => '插件未找到';

  @override
  String get pluginErrorIncompatible => '插件与当前应用版本不兼容';

  @override
  String get pluginErrorSecurity => '安全校验失败';

  @override
  String get pluginErrorNetwork => '网络错误，请检查网络连接';
}
