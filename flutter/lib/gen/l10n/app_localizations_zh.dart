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
    return '使用 $biometricType 解锁 SoloSoul';
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
  String get syncPairingKey => '配对密钥 (hex)';

  @override
  String get syncPairingKeyHint => '输入共享配对密钥';

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
}
