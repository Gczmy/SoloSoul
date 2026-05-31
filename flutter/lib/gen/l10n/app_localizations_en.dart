// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get commonCancel => 'Cancel';

  @override
  String get commonConfirm => 'Confirm';

  @override
  String get commonSave => 'Save';

  @override
  String get commonImport => 'Import';

  @override
  String get commonError => 'Error';

  @override
  String get commonRetry => 'Retry';

  @override
  String get commonClose => 'Close';

  @override
  String get genericFilterClearAll => 'Clear';

  @override
  String get commonLoading => 'Loading...';

  @override
  String get commonSuccess => 'Success';

  @override
  String get commonDelete => 'Delete';

  @override
  String get commonEdit => 'Edit';

  @override
  String get commonUndo => 'Undo';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsLanguageSubtitle => 'Select your preferred language';

  @override
  String get settingsLanguageEnglish => 'English';

  @override
  String get settingsLanguageChinese => '中文 (Chinese)';

  @override
  String get loginDataRecoveryTitle => 'Data Recovery';

  @override
  String loginDataRecoveryMessage(String time) {
    return 'Your vault appears to be empty, but a backup exists from $time. Would you like to restore from this backup?';
  }

  @override
  String get loginSkip => 'Skip';

  @override
  String get loginRestoreBackup => 'Restore Backup';

  @override
  String get loginRestoreSuccess =>
      'Restore successful. Your data is now available.';

  @override
  String get loginRestoreFailed => 'Restore failed';

  @override
  String get loginBiometricGeneric => 'Biometric';

  @override
  String get loginBiometricFaceId => 'Face ID';

  @override
  String get loginBiometricTouchId => 'Touch ID';

  @override
  String get loginBiometricIris => 'Iris';

  @override
  String loginUnlockReason(String biometricType) {
    return 'Unlock SoloSoul with $biometricType';
  }

  @override
  String get loginBiometricFailed =>
      'Biometric authentication failed or was cancelled';

  @override
  String get loginUnlockFailedUsePassword =>
      'Failed to unlock vault. Please use your master password.';

  @override
  String get loginPasswordMinLength => 'Password must be at least 8 characters';

  @override
  String get loginInvalidPassword => 'Invalid master password';

  @override
  String loginUnlockFailed(String message) {
    return 'Unlock failed: $message';
  }

  @override
  String get loginAccountNameRequired => 'Account name is required';

  @override
  String get loginPasswordsDoNotMatch => 'Passwords do not match';

  @override
  String get loginCreateAccountFailed => 'Failed to create account';

  @override
  String get loginUnlockVaultFailed =>
      'Failed to unlock vault. Please try again.';

  @override
  String loginPasswordHint(String hint) {
    return 'Password Hint: $hint';
  }

  @override
  String get loginNever => 'Never';

  @override
  String get loginToday => 'Today';

  @override
  String get loginYesterday => 'Yesterday';

  @override
  String loginDaysAgo(int count) {
    return '$count days ago';
  }

  @override
  String get loginBackToAccountList => 'Back to Account List';

  @override
  String get loginAccountName => 'Account Name';

  @override
  String get loginAccountNameHint => 'e.g., Personal, Work';

  @override
  String get loginMasterPassword => 'Master Password';

  @override
  String get loginEnterPassword => 'Enter your password';

  @override
  String get loginCreateStrongPassword => 'Create a strong password';

  @override
  String get loginConfirmPassword => 'Confirm Password';

  @override
  String get loginReenterPassword => 'Re-enter your password';

  @override
  String get loginPasswordHintOptional => 'Password Hint (Optional)';

  @override
  String get loginPasswordHintHelp => 'A hint to help you remember';

  @override
  String get loginShowPasswordHint => 'Show password hint';

  @override
  String get loginNoAccounts => 'No accounts found';

  @override
  String get loginCreateAccount => 'Create Account';

  @override
  String loginLastAccessed(String time) {
    return 'Last accessed: $time';
  }

  @override
  String get loginAccountListEmpty => 'Account list empty';

  @override
  String get loginCreateFirstAccount =>
      'Create your first account to get started';

  @override
  String get loginSelectAccountToUnlock => 'Select an account to unlock';

  @override
  String get loginShowLess => 'Show less';

  @override
  String loginShowAllAccounts(int count) {
    return 'Show all $count accounts';
  }

  @override
  String get loginNoAccountsYet => 'No accounts yet';

  @override
  String get loginRecent => 'Recent';

  @override
  String get workspaceObjects => 'Objects';

  @override
  String get workspaceNoItems => 'No items yet';

  @override
  String get workspaceNoObjects => 'No objects yet';

  @override
  String get workspaceAddFirstItem => 'Add your first item';

  @override
  String get workspaceCreateFirstObject =>
      'Create your first object to get started';

  @override
  String get workspaceDeletePage => 'Delete Page';

  @override
  String get workspaceDeleteSection => 'Delete Section';

  @override
  String workspaceDeleteSectionConfirm(String name) {
    return 'Are you sure you want to delete \"$name\"?';
  }

  @override
  String workspaceDeletePageConfirm(String name, int count) {
    return 'Are you sure you want to delete \"$name\"? All $count item(s) inside this page will also be moved to trash.';
  }

  @override
  String get workspaceSectionDeleted => 'Section deleted';

  @override
  String workspaceMovedToTrash(String name) {
    return '\"$name\" moved to trash';
  }

  @override
  String get workspaceAddSubPage => 'Add Sub-Page';

  @override
  String get workspaceAddSection => 'Add Section';

  @override
  String get workspaceAddSectionDialog => 'Add Section';

  @override
  String get workspaceSectionName => 'Name';

  @override
  String get workspaceEnterSectionName => 'Enter section name';

  @override
  String get workspaceIcon => 'Icon';

  @override
  String get objectEditorEditSection => 'Edit Section';

  @override
  String get objectEditorNewSection => 'New Section';

  @override
  String get objectEditorType => 'Type';

  @override
  String get objectEditorNameRequired => 'Name is required';

  @override
  String objectEditorDuplicateProperties(String names) {
    return 'Duplicate property names: $names';
  }

  @override
  String objectEditorSaveFailed(String message) {
    return 'Failed to save: $message';
  }

  @override
  String get objectEditorIcon => 'Icon';

  @override
  String get objectEditorName => 'Name';

  @override
  String get objectEditorEnterSectionName => 'Enter section name';

  @override
  String get objectEditorSelectType => 'Select type';

  @override
  String get objectEditorNoParent => 'No parent (root)';

  @override
  String get objectEditorParentPage => 'Parent Page';

  @override
  String get objectEditorItemProperties => 'Item Properties';

  @override
  String get objectEditorAddProperty => 'Add Property';

  @override
  String get objectEditorKeyName => 'Key name';

  @override
  String get objectEditorPropertyTypeText => 'Text';

  @override
  String get objectEditorPropertyTypeDate => 'Date';

  @override
  String get objectEditorPropertyTypeNumber => 'Number';

  @override
  String get objectEditorPropertyTypeCheckbox => 'Checkbox';

  @override
  String get objectEditorPropertyTypeSelect => 'Select';

  @override
  String get objectEditorPropertyTypeMultiSelect => 'Multi-Select';

  @override
  String get objectEditorPropertyTypeUrl => 'URL';

  @override
  String get objectEditorSensitivity => 'Sensitivity';

  @override
  String get objectEditorDeletePropertyTitle => 'Delete Property';

  @override
  String get pageEditorNameRequired => 'Name is required';

  @override
  String get pageEditorEditPage => 'Edit Page';

  @override
  String get pageEditorNewPage => 'New Page';

  @override
  String get pageEditorName => 'Name';

  @override
  String get pageEditorEnterPageName => 'Enter page name';

  @override
  String get pageEditorIcon => 'Icon';

  @override
  String get pageEditorParent => 'Parent';

  @override
  String get homeScan => 'Scan';

  @override
  String get homeQuickActions => 'Quick Actions';

  @override
  String get homeEditQuickActions => 'Edit quick actions';

  @override
  String get homeEditQuickActionsDone => 'Done';

  @override
  String get homeSecurityStatus => 'Security Status';

  @override
  String get searchTitle => 'Search';

  @override
  String get searchHint => 'Search fields...';

  @override
  String get profileType => 'Type';

  @override
  String get profileTypeEmail => 'email';

  @override
  String get profileTypePhone => 'phone';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get settingsDebugModeEnabled => 'Debug mode enabled';

  @override
  String get settingsInvalidPassword => 'Invalid password';

  @override
  String get settingsPasswordChangedSuccess =>
      'Master password changed successfully';

  @override
  String get settingsPasswordHintChangedSuccess =>
      'Password hint changed successfully';

  @override
  String get settingsOk => 'OK';

  @override
  String get settingsEnableDebugMode => 'Enable Debug Mode';

  @override
  String get settingsEnableDebugModeDesc =>
      'Enter your master password to enable Debug Log.';

  @override
  String settingsUseBiometric(String biometricType) {
    return 'Use $biometricType';
  }

  @override
  String get settingsOr => 'or';

  @override
  String get settingsMasterPassword => 'Master Password';

  @override
  String get settingsShowPasswordHint => 'Show password hint';

  @override
  String get settingsEnable => 'Enable';

  @override
  String get securitySettingsTitle => 'Security Settings';

  @override
  String get securitySettingsBiometricFailed =>
      'Biometric authentication failed or was cancelled';

  @override
  String get securitySettingsBiometricEnabled => 'Biometric unlock enabled';

  @override
  String get securitySettingsResetToDefaults => 'Reset to Defaults';

  @override
  String get securitySettingsResetTitle => 'Reset Security Settings';

  @override
  String get securitySettingsResetConfirm =>
      'This will reset all security settings to their default values. Are you sure?';

  @override
  String get securitySettingsReset => 'Reset';

  @override
  String get securitySettingsNotImplemented => 'Feature not yet implemented';

  @override
  String get sensitivitySettingsTitle => 'Sensitivity Settings';

  @override
  String get sensitivitySettingsVerify => 'Verify';

  @override
  String get sensitivitySettingsConfirmDowngrade => 'Confirm Downgrade';

  @override
  String get sensitivitySettingsChangeLevel => 'Change sensitivity level';

  @override
  String get sensitivitySettingsSearchHint => 'Search fields...';

  @override
  String get sensitivitySettingsClearSearch => 'Clear search';

  @override
  String get trashTitle => 'Trash';

  @override
  String get trashVerify => 'Verify';

  @override
  String get trashEmptyTrash => 'Empty Trash';

  @override
  String get trashConfirmRestore => 'Confirm Restore';

  @override
  String trashRestoreConfirm(String name) {
    return 'Restore \"$name\"?';
  }

  @override
  String get trashConfirmPermanentDelete => 'Confirm Permanent Delete';

  @override
  String get trashSearchHint => 'Search trash...';

  @override
  String get syncTitle => 'Device Sync';

  @override
  String get syncNoActiveAccount => 'No active account for sync';

  @override
  String get syncEnterAddressAndKey => 'Enter address and pairing key';

  @override
  String get syncInvalidPairingKey => 'Invalid pairing key hex';

  @override
  String get syncPairingKeyCopied => 'Pairing key copied to clipboard';

  @override
  String get syncRemoteAddress => 'Remote Address';

  @override
  String get syncRemoteAddressHint => '192.168.1.5:9900';

  @override
  String get syncPairingKey => 'Pairing Key';

  @override
  String get syncPairingKeyHint =>
      'Generate a shared pairing key to establish a secure connection between devices. Both devices must use the same key.';

  @override
  String get syncGenerateAndCopyKey => 'Generate & Copy Key';

  @override
  String syncWithDevice(String name) {
    return 'Sync with $name';
  }

  @override
  String get syncButton => 'Sync';

  @override
  String get dataManagementTitle => 'Data Management';

  @override
  String get dataManagementBackupNow => 'Backup Now';

  @override
  String get dataManagementSpecialBackupLimit => 'Special Backup Limit Reached';

  @override
  String get dataManagementNameBackup => 'Name Special Backup';

  @override
  String get dataManagementBackupNameHint => 'e.g. Before Major Update';

  @override
  String get dataManagementBackupNameLabel => 'Backup name';

  @override
  String get dataManagementCreate => 'Create';

  @override
  String get dataManagementRenameBackup => 'Rename Special Backup';

  @override
  String get dataManagementNewName => 'New name';

  @override
  String get dataManagementRename => 'Rename';

  @override
  String get dataManagementRestoreBackupTitle => 'Restore Special Backup?';

  @override
  String dataManagementRestoreBackupConfirm(String name) {
    return 'Restore special backup \"$name\"?';
  }

  @override
  String get dataManagementDeleteBackupTitle => 'Delete Special Backup?';

  @override
  String dataManagementDeleteBackupConfirm(String name) {
    return 'Delete special backup \"$name\"?';
  }

  @override
  String get operationLogTitle => 'Operation Log';

  @override
  String get operationLogVerify => 'Verify';

  @override
  String get operationLogClearLogTitle => 'Clear Log';

  @override
  String get operationLogClear => 'Clear';

  @override
  String get operationLogClearLog => 'Clear log';

  @override
  String get operationLogSearchHint => 'Search logs...';

  @override
  String objectEditorDeletePropertyConfirm(String name) {
    return 'Are you sure you want to delete \"$name\"?';
  }

  @override
  String get workspaceAddSectionButton => 'Add Section';

  @override
  String get workspaceEditPage => 'Edit Page';

  @override
  String get workspaceDone => 'Done';

  @override
  String get workspaceReorder => 'Reorder';

  @override
  String get workspaceAdd => 'Add';

  @override
  String get loginCreateNewAccount => 'Create New Account';

  @override
  String get settingsAiChat => 'AI Chat';

  @override
  String get settingsAiChatSubtitle => 'Chat with local or cloud models';

  @override
  String get settingsDeleteAccountWarning =>
      'After deleting the account, all data will be cleared. Are you sure you want to delete it?';

  @override
  String get mainAppTitle => 'SoloSoul';

  @override
  String get mainSplashTagline =>
      'Orchestrate your life data, reshape your digital origin';

  @override
  String get mainLaunchFailed => 'Launch failed';

  @override
  String get sidebarAiChat => 'AI Chat';

  @override
  String get llmConfigTitle => 'AI Assistant Settings';

  @override
  String get llmConfigNotLoaded => 'Config not loaded';

  @override
  String get llmConfigOllamaNotRunning =>
      'Ollama service is not running\nPlease make sure Ollama is installed and running';

  @override
  String llmConfigOllamaModelNotInstalled(String model, String models) {
    return 'Ollama is running, but model $model is not installed\nInstalled models: $models';
  }

  @override
  String get llmConfigLocalSuccess => 'Local model connected successfully!';

  @override
  String llmConfigConnectionFailed(String message) {
    return 'Connection failed: $message';
  }

  @override
  String llmConfigUnknownError(String message) {
    return 'Unknown error: $message';
  }

  @override
  String llmConfigSaveFailed(String message) {
    return 'Save failed: $message';
  }

  @override
  String get llmConfigDeleteTitle => 'Delete Configuration';

  @override
  String llmConfigDeleteConfirm(String name) {
    return 'Are you sure you want to delete \"$name\"? This action cannot be undone.';
  }

  @override
  String get llmConfigExperimental => 'Experimental';

  @override
  String llmConfigLoadFailed(String message) {
    return 'Load failed: $message';
  }

  @override
  String get llmConfigInferenceBackend => 'Inference Backend';

  @override
  String get llmConfigModelName => 'Model Name';

  @override
  String get llmConfigInstructions => 'Instructions';

  @override
  String get llmConfigInstructionsOllama =>
      '1. Install Ollama: https://ollama.com\n2. Pull model: ollama pull qwen2.5:1.5b\n3. Keep Ollama running in the background';

  @override
  String get llmConfigCloudConfig => 'Cloud Configuration';

  @override
  String get llmConfigAddProfile => 'Add Configuration';

  @override
  String get llmConfigCloudConsent => 'Consent to Cloud Processing';

  @override
  String get llmConfigCloudConsentDesc =>
      'I confirm that the current batch does not contain critical-level fields, and agree to send data to the specified enterprise/private API endpoint.';

  @override
  String get llmConfigStatsSubtitle =>
      'View token consumption, conversation count, etc.';

  @override
  String get llmConfigTesting => 'Testing...';

  @override
  String get llmConfigTestConnection => 'Test Connection';

  @override
  String llmConfigModelInfo(String model) {
    return 'Model: $model';
  }

  @override
  String llmConfigEndpointInfo(String endpoint) {
    return 'Endpoint: $endpoint';
  }

  @override
  String get llmConfigNoProfiles => 'No cloud configurations yet';

  @override
  String get llmConfigNoProfilesHint =>
      'Tap the button below to create your first cloud API configuration';

  @override
  String get llmConfigNameRequired => 'Please enter a configuration name';

  @override
  String get llmConfigApiKeyRequired =>
      'API Key is required for new configurations';

  @override
  String get llmConfigEndpointModelRequired =>
      'Endpoint and Model cannot be empty';

  @override
  String get llmConfigEditProfile => 'Edit Configuration';

  @override
  String get llmConfigProfileName => 'Configuration Name';

  @override
  String get llmConfigProfileNameHint => 'e.g. OpenAI Production';

  @override
  String get llmConfigApiKeySet => 'API Key (configured)';

  @override
  String get llmConfigApiKeyNew => 'API Key *';

  @override
  String get llmConfigApiKeyHintNew =>
      'Enter a new value to replace the existing key';

  @override
  String get llmConfigApiKeyHintKeep => 'Leave blank to keep the existing key';

  @override
  String get llmConfigSave => 'Save Changes';

  @override
  String get llmConfigCreate => 'Create Configuration';

  @override
  String get llmConfigBackendLocal => 'Local Model';

  @override
  String get llmConfigBackendCloud => 'Cloud API';

  @override
  String get llmStatsTitle => 'Usage Statistics';

  @override
  String get llmStatsCurrentModel => 'Current Model';

  @override
  String get llmStatsSessionStats => 'Session Statistics';

  @override
  String get llmStatsAccountStats => 'Account Statistics';

  @override
  String get llmStatsTokenBreakdown => 'Token Breakdown';

  @override
  String get llmStatsDailyTrend => 'Daily Token Trend (Last 14 Days)';

  @override
  String get llmStatsModelUsage => 'Model Usage';

  @override
  String get llmStatsReset => 'Reset Statistics';

  @override
  String get llmStatsResetConfirm =>
      'Are you sure you want to reset all usage statistics? This action cannot be undone.';

  @override
  String get llmStatsResetSuccess => 'Statistics have been reset';

  @override
  String get llmStatsUnknown => 'Unknown';

  @override
  String get llmStatsNotLoaded => 'Not Loaded';

  @override
  String get llmStatsLocalModelOllama => 'Local Model (Ollama)';

  @override
  String get llmStatsModelLabel => 'Model';

  @override
  String get llmStatsProviderLabel => 'Provider';

  @override
  String get llmStatsConversationCount => 'Conversations';

  @override
  String get llmStatsTokenConsumption => 'Token Consumption';

  @override
  String get llmStatsLastLoaded => 'Last Loaded';

  @override
  String get llmStatsLastUsed => 'Last Used';

  @override
  String get llmStatsTotalConversations => 'Total Conversations';

  @override
  String get llmStatsTotalTokens => 'Total Tokens';

  @override
  String get llmStatsSession => 'Session';

  @override
  String get llmStatsAccountTotal => 'Account Total';

  @override
  String get llmStatsAllModels => 'All Models';

  @override
  String get llmChatTitle => 'AI Chat';

  @override
  String get llmChatBackendCloud => 'Cloud';

  @override
  String get llmChatBackendLocal => 'Local';

  @override
  String get llmChatModelNotConfigured => 'Not configured';

  @override
  String get llmChatModelNotLoaded =>
      'Model not loaded. Please configure LLM first.';

  @override
  String get llmChatClearSession => 'Clear session';

  @override
  String get llmChatThinking => 'Thinking...';

  @override
  String get llmChatNoResponse => 'No response received';

  @override
  String get llmChatInputHintReady => 'Type a message...';

  @override
  String get llmChatInputHintNotReady => 'Model not ready';

  @override
  String get llmChatStatusReady => 'Ready';

  @override
  String get llmChatStatusLoading => 'Loading';

  @override
  String get llmChatStatusError => 'Error';

  @override
  String get llmChatStatusNotReady => 'Not ready';

  @override
  String get llmChatLoadingConfig => 'Loading model configuration...';

  @override
  String get llmChatStartConversation => 'Start chatting with AI';

  @override
  String get llmChatConnectCloudModel => 'Connect cloud model';

  @override
  String get llmChatStartLocalModel => 'Start local model';

  @override
  String get llmChatGoToConfig => 'Go to LLM Config';

  @override
  String get llmErrorConfigNotLoaded => 'Configuration not loaded';

  @override
  String get llmErrorCloudConfigIncomplete =>
      'Cloud configuration incomplete: please check API Key and privacy consent';

  @override
  String get llmErrorNoActiveCloudProfile => 'No active cloud configuration';

  @override
  String get llmErrorApiKeyEmpty => 'API Key is empty';

  @override
  String get llmCopy => 'Copy';

  @override
  String get llmCopied => 'Copied';

  @override
  String get llmInferenceError => 'Inference Error';

  @override
  String get ocrScanDocument => 'Scan Document';

  @override
  String get ocrTakePhoto => 'Take Photo';

  @override
  String get ocrSelectDocument => 'Select Document';

  @override
  String get ocrLlmAssist => 'Use LLM to assist extraction';

  @override
  String get ocrLlmAssistSubtitle => 'Improve field recognition accuracy';

  @override
  String get ocrNoModelAvailable => 'No LLM model available';

  @override
  String get ocrGoToConfig => 'Go to Config';

  @override
  String get ocrLlmConfig => 'LLM Config';

  @override
  String get ocrModelSelectorLabel => 'Select Model';

  @override
  String get ocrPrivacyNotice =>
      'All recognition is done locally on your device. Images are never uploaded to any server. Travel documents and ID cards will be automatically detected.';

  @override
  String get ocrTip =>
      'Tip: For best results, ensure the text is clearly visible and the image is well-lit.';

  @override
  String get ocrRecognizing => 'Recognizing text...';

  @override
  String get ocrRecognitionFailed => 'Recognition Failed';

  @override
  String get ocrTryAgain => 'Try Again';

  @override
  String get ocrTravelDocumentDetected => 'Travel document detected';

  @override
  String get ocrRescan => 'Rescan';

  @override
  String get scanGoToConfig => 'Go to Config';

  @override
  String get scanAiMappingComplete => 'AI mapping completed';

  @override
  String get scanAiMapping => 'AI Smart Mapping';

  @override
  String llmStatsTotalFormatted(String total) {
    return 'Total: $total';
  }

  @override
  String llmStatsModelSummary(int count, String total) {
    return '$count models · Total $total tokens';
  }

  @override
  String llmStatsModelDetail(String provider, String total, int count) {
    return '$provider · $total tokens · $count calls';
  }

  @override
  String get ocrFieldName => 'Name';

  @override
  String get ocrFieldPhone => 'Phone';

  @override
  String get ocrFieldEmail => 'Email';

  @override
  String get ocrFieldAddress => 'Address';

  @override
  String get ocrFieldCompany => 'Company/Organization';

  @override
  String get ocrFieldTitle => 'Title/Position';

  @override
  String get ocrFieldDate => 'Date';

  @override
  String get ocrFieldAmount => 'Amount';

  @override
  String get ocrFieldInvoiceNumber => 'Invoice/Document Number';

  @override
  String get ocrFieldWebsite => 'Website/URL';

  @override
  String get ocrFieldIdNumber => 'ID Number';

  @override
  String get llmChatEmptyResponse =>
      'The model returned no content. Please check the configuration or try again.';

  @override
  String llmChatInferenceFailed(String error) {
    return 'Inference failed: $error';
  }

  @override
  String get sidebarHome => 'Home';

  @override
  String get sidebarSearch => 'Search';

  @override
  String get sidebarLocalImport => 'Local Import';

  @override
  String get sidebarProfile => 'Profile';

  @override
  String get sidebarTravel => 'Travel';

  @override
  String get sidebarFinancial => 'Financial';

  @override
  String get sidebarProfessional => 'Professional';

  @override
  String get sidebarAddPage => 'Add Page';

  @override
  String get sidebarLockVault => 'Lock Vault';

  @override
  String get sidebarTrash => 'Trash';

  @override
  String get sidebarSync => 'Sync';

  @override
  String get sidebarSettings => 'Settings';

  @override
  String get sidebarPlugin => 'Plugins';

  @override
  String get sidebarSecurity => 'Security';

  @override
  String get sidebarOperationLog => 'Operation Log';

  @override
  String get sidebarSensitivity => 'Sensitivity';

  @override
  String get sidebarCollapse => 'Collapse';

  @override
  String get sidebarExpand => 'Expand';

  @override
  String get sidebarPages => 'PAGES';

  @override
  String get sidebarDropToMakeRootPage => 'Drop to make root page';

  @override
  String get commonBack => 'Back';

  @override
  String get homeTitle => 'Home';

  @override
  String get homeEndToEndEncrypted => 'End-to-End Encrypted';

  @override
  String get homeEncryptionDesc => 'AES-256-GCM + Argon2id';

  @override
  String get homeLocalStorage => 'Local Storage';

  @override
  String get homeLocalStorageDesc => 'Data encrypted and stored locally';

  @override
  String get homeZeroKnowledge => 'Zero Knowledge';

  @override
  String get homeZeroKnowledgeDesc => 'Master password never stored';

  @override
  String get profileTitle => 'Profile';

  @override
  String get profileIdentity => 'Identity';

  @override
  String get profileContactInfo => 'Contact Information';

  @override
  String get profileIdentityDocuments => 'Identity Documents';

  @override
  String get profileAddresses => 'Addresses';

  @override
  String get profileTitleLabel => 'Title';

  @override
  String get profileTypeLabel => 'Type';

  @override
  String get profileValueLabel => 'Value';

  @override
  String get travelTitle => 'Travel';

  @override
  String get travelPassports => 'Passports';

  @override
  String get travelVisas => 'Visas';

  @override
  String get travelHistory => 'Travel History';

  @override
  String get financialTitle => 'Financial';

  @override
  String get financialBankAccounts => 'Bank Accounts';

  @override
  String get financialCards => 'Cards';

  @override
  String get financialTaxIdentification => 'Tax Identification';

  @override
  String get professionalTitle => 'Professional';

  @override
  String get professionalEducation => 'Education';

  @override
  String get professionalEmployment => 'Employment';

  @override
  String get professionalAwards => 'Awards';

  @override
  String get professionalSkills => 'Skills';

  @override
  String get professionalLanguages => 'Languages';

  @override
  String get professionalArticles => 'Articles';

  @override
  String get builtinSectionIdentityTitle => 'Identity';

  @override
  String get builtinSectionContactTitle => 'Contact Information';

  @override
  String get builtinSectionIdentityDocumentTitle => 'Identity Documents';

  @override
  String get builtinSectionAddressTitle => 'Addresses';

  @override
  String get builtinSectionPassportTitle => 'Passports';

  @override
  String get builtinSectionVisaTitle => 'Visas';

  @override
  String get builtinSectionTravelHistoryTitle => 'Travel History';

  @override
  String get builtinSectionBankAccountTitle => 'Bank Accounts';

  @override
  String get builtinSectionPaymentCardTitle => 'Payment Cards';

  @override
  String get builtinSectionTaxIdTitle => 'Tax Identification';

  @override
  String get builtinSectionEducationTitle => 'Education';

  @override
  String get builtinSectionEmploymentTitle => 'Employment';

  @override
  String get builtinSectionSkillTitle => 'Skills';

  @override
  String get builtinSectionLanguageTitle => 'Languages';

  @override
  String get builtinSectionAwardTitle => 'Awards';

  @override
  String get builtinSectionArticleTitle => 'Articles';

  @override
  String get localSearchTitle => 'Local Search Import';

  @override
  String get localSearchPaths => 'Search Paths';

  @override
  String get localSearchFileTypes => 'File Types';

  @override
  String get localSearchScanDepth => 'Scan Depth';

  @override
  String get localSearchFilenameOnly => 'Filename only';

  @override
  String get localSearchFilenameOnlyDesc => 'Fastest — only check filenames';

  @override
  String get localSearchFingerprint => 'Filename + Content fingerprint';

  @override
  String get localSearchFingerprintDesc => 'Balanced — regex match on content';

  @override
  String get localSearchFullText => 'Full text parsing';

  @override
  String get localSearchFullTextDesc => 'Slowest — deep content analysis';

  @override
  String get localSearchDefaultPaths => 'Use default paths';

  @override
  String get localSearchDefaultPathsDesc => 'Documents, Desktop, Downloads';

  @override
  String get localSearchCustomPaths => 'Custom paths';

  @override
  String get localSearchCustomPathsDesc => 'Select specific folders';

  @override
  String get localSearchAddFolder => 'Add folder';

  @override
  String get localSearchStartScan => 'Start Scan';

  @override
  String get localSearchScanning => 'Scanning...';

  @override
  String get localSearchScanned => 'Scanned';

  @override
  String get localSearchFound => 'Found';

  @override
  String get localSearchSkipped => 'Skipped';

  @override
  String get localSearchCancelScan => 'Cancel Scan';

  @override
  String get localSearchGoBack => 'Go Back';

  @override
  String get localSearchScanAgain => 'Scan Again';

  @override
  String get localSearchNoResults => 'No Results Found';

  @override
  String get scanImportComplete => 'Import Complete';

  @override
  String get scanImportGoHome => 'Go Home';

  @override
  String get scanImportCreated => 'Created';

  @override
  String get scanImportUpdated => 'Updated';

  @override
  String get scanImportFields => 'Fields';

  @override
  String get scanImportSkipped => 'Skipped';

  @override
  String get scanPreviewTitle => 'Preview & Confirm';

  @override
  String get scanPreviewNew => 'New';

  @override
  String get scanPreviewUpdate => 'Update';

  @override
  String get scanPreviewImportAction => 'Import action';

  @override
  String get settingsAccount => 'Account';

  @override
  String get settingsCurrentAccount => 'Current Account';

  @override
  String get settingsAllAccounts => 'All Accounts';

  @override
  String get settingsDataManagement => 'Data Management';

  @override
  String get settingsErrorLoadingAccounts => 'Error loading accounts';

  @override
  String get settingsPleaseRestart => 'Please restart the app';

  @override
  String get settingsAccess => 'Access';

  @override
  String get settingsLockVault => 'Lock Vault';

  @override
  String get settingsLockVaultDesc => 'Lock now and require password';

  @override
  String get settingsChangePassword =>
      'Change Master Password or Password Hint';

  @override
  String get settingsChangePasswordDesc =>
      'Update your vault password or password hint';

  @override
  String get settingsSecurity => 'Security';

  @override
  String get settingsAutoLockPrivacy => 'Auto-Lock & Privacy';

  @override
  String get settingsAutoLockPrivacyDesc =>
      'Configure timeout and privacy settings';

  @override
  String get settingsVerifyPassword =>
      'Enter your master password to access security settings.';

  @override
  String get settingsSensitivity => 'Sensitivity Level Settings';

  @override
  String get settingsSensitivityDesc => 'Configure field sensitivity';

  @override
  String get settingsOperationLog => 'Operation Log';

  @override
  String get settingsOperationLogDesc => 'View activity history';

  @override
  String get settingsSync => 'Sync';

  @override
  String get settingsCloudSync => 'Cloud Sync';

  @override
  String get settingsNotConfigured => 'Not configured';

  @override
  String get settingsOfflineMode => 'Offline Mode';

  @override
  String get settingsOfflineModeDesc => 'Local data only';

  @override
  String get settingsAiAssistant => 'AI Assistant';

  @override
  String get settingsLlmConfig => 'LLM Configuration';

  @override
  String get settingsLlmConfigDesc => 'Local model or cloud API';

  @override
  String get settingsAbout => 'About';

  @override
  String get settingsVersion => 'Version';

  @override
  String get settingsDebugLog => 'Debug Log';

  @override
  String get settingsDebugLogDesc => 'View debug log';

  @override
  String get settingsPrivacyPolicy => 'Privacy Policy';

  @override
  String get settingsPrivacyPolicyDesc => 'View our privacy policy';

  @override
  String get settingsTermsOfService => 'Terms of Service';

  @override
  String get settingsTermsOfServiceDesc => 'View terms of service';

  @override
  String get settingsLocal => 'Local';

  @override
  String get settingsPrivate => 'Private';

  @override
  String get settingsUniversal => 'Universal';

  @override
  String get securityVaultSecurity => 'Vault Security';

  @override
  String get securityAutoLockDelay => 'Auto-Lock Delay';

  @override
  String get securityAutoLockDesc => 'Lock vault after inactivity';

  @override
  String get securityBiometricUnlock => 'Biometric Unlock';

  @override
  String get securityPrivacy => 'Privacy';

  @override
  String get securityAppPrivacyScreen => 'App Privacy Screen';

  @override
  String get securityAppPrivacyDesc => 'Hide content in app switcher';

  @override
  String get securityLockOnBlur => 'Lock on Window Blur';

  @override
  String get securityLockOnBlurDesc => 'Lock when switching apps';

  @override
  String get securityClipboard => 'Clipboard';

  @override
  String get securityAutoClearDelay => 'Auto-Clear Delay';

  @override
  String get securityAutoClearDesc =>
      'Clear clipboard after copying sensitive data';

  @override
  String get sensitivityVerifyPassword =>
      'Enter your master password to access sensitivity settings.';

  @override
  String get sensitivityCritical => 'Critical';

  @override
  String get sensitivityCriticalDesc =>
      'Maximum sensitivity - always masked, requires verification';

  @override
  String get sensitivitySensitive => 'Sensitive';

  @override
  String get sensitivitySensitiveDesc =>
      'Personal information requiring protection';

  @override
  String get sensitivityInternal => 'Internal';

  @override
  String get sensitivityInternalDesc =>
      'Internal use only - can be hidden by display settings';

  @override
  String get sensitivityPublic => 'Public';

  @override
  String get sensitivityPublicDesc => 'Lowest sensitivity - always visible';

  @override
  String get syncSynchronizing => 'Synchronizing...';

  @override
  String get syncDeviceDiscovery => 'Device Discovery';

  @override
  String get syncManualConnection => 'Manual Connection';

  @override
  String get syncLastSync => 'Last Sync';

  @override
  String get syncStatus => 'Status';

  @override
  String get syncDirection => 'Direction';

  @override
  String get syncData => 'Data';

  @override
  String get syncError => 'Error';

  @override
  String get trashVerifyPassword =>
      'Enter your master password to view the trash.';

  @override
  String get trashRestored => 'Restored ';

  @override
  String get trashPermanentlyDeleted => 'Permanently deleted ';

  @override
  String get operationLogVerifyPassword =>
      'Enter your master password to view the operation log.';

  @override
  String get dataMgmtRestoreBackup => 'Restore Backup?';

  @override
  String get dataMgmtDeleteBackup => 'Delete Backup?';

  @override
  String get dataMgmtConfirmDeletion =>
      'Enter your master password to confirm backup deletion.';

  @override
  String get llmApiEndpoint => 'API Endpoint';

  @override
  String get llmModel => 'Model';

  @override
  String get llmAnthropicVersion => 'Anthropic API Version';

  @override
  String get llmOpenAI => 'OpenAI';

  @override
  String get llmAnthropic => 'Anthropic';

  @override
  String get searchUnlock => 'Unlock';

  @override
  String get searchDeleted => 'Deleted';

  @override
  String get searchReveal => 'Reveal';

  @override
  String get searchCriticalHint => 'Critical - password required to view';

  @override
  String get searchPrivateHint => 'Private - reveal to view';

  @override
  String get sensitivityCriticalOnly => 'Critical only';

  @override
  String get commonAdd => 'Add';

  @override
  String get commonCopy => 'Copy';

  @override
  String get dialogCurrentPassword => 'Current Password';

  @override
  String get dialogNewPassword => 'New Password';

  @override
  String get dialogConfirmNewPassword => 'Confirm New Password';

  @override
  String get dialogChange => 'Change';

  @override
  String get dialogLock => 'Lock';

  @override
  String get dialogVerifyIdentity => 'Verify Identity';

  @override
  String get dialogDeleteItem => 'Delete Item';

  @override
  String get dialogDeleteSection => 'Delete Section?';

  @override
  String get dialogDeleteSectionConfirm =>
      'This section and its items will be moved to trash.';

  @override
  String get biometricTestTouchId => 'Test Touch ID';

  @override
  String get biometricTestFaceId => 'Test Face ID';

  @override
  String get dialogAddQuickAction => 'Add Quick Action';

  @override
  String get homePageEditorSections => 'Sections';

  @override
  String get homePageEditorIcon => 'Icon';

  @override
  String get homePageEditorSectionTitle => 'Section Title';

  @override
  String get settingsDeleteAccount => 'Delete Account';

  @override
  String get settingsDebugLogCopyTitle => 'Copy Logs to Clipboard';

  @override
  String get settingsDebugLogCopied => 'Sanitized logs copied to clipboard';

  @override
  String get settingsDebugLogTitle => 'Debug Log';

  @override
  String get dialogSelectFolder => 'Select This Folder';

  @override
  String get iconPickerTitle => 'Choose Icon';

  @override
  String get iconCategoryWork => 'Work & Study';

  @override
  String get iconCategoryPeople => 'People & Identity';

  @override
  String get iconCategoryTravel => 'Travel & Transport';

  @override
  String get iconCategoryFinance => 'Finance & Business';

  @override
  String get iconCategoryLife => 'Life & Health';

  @override
  String get iconCategoryTech => 'Tech & Devices';

  @override
  String get iconCategoryCreative => 'Creative & Art';

  @override
  String get iconCategoryGeneral => 'General';

  @override
  String get operationDetails => 'Operation Details';

  @override
  String get trashHistory => 'History';

  @override
  String dialogUseBiometric(String biometricType) {
    return 'Use $biometricType';
  }

  @override
  String dialogDeleteItemConfirm(String name) {
    return 'Are you sure you want to delete \"$name\"?';
  }

  @override
  String entryHistoryCount(int count) {
    return 'History($count)';
  }

  @override
  String biometricPasswordHint(String hint) {
    return 'Password Hint: $hint';
  }

  @override
  String get settingsUnknown => 'Unknown';

  @override
  String get settingsActive => 'Active';

  @override
  String get settingsCloudSyncSetup => 'Cloud sync setup';

  @override
  String get settingsComingSoon =>
      'This feature will be available in a future update.';

  @override
  String get settingsTagline =>
      'Your Local Digital Twin. Privacy-First Universal Identity.';

  @override
  String get settingsVerifyIdentityDebug =>
      'Verify your identity to enable debug mode';

  @override
  String get loginNoPasswordHint => 'No password hint available';

  @override
  String get commonVerify => 'Verify';

  @override
  String get commonRefresh => 'Refresh';

  @override
  String get commonShowLess => 'Show less';

  @override
  String get debugLogCopyToClipboard => 'Copy to clipboard';

  @override
  String get debugLogDisable => 'Disable debug mode';

  @override
  String get debugLogEmpty => 'No debug logs available.';

  @override
  String get deleteAccountEnterPassword => 'Enter password to confirm';

  @override
  String get deleteAccountPasswordRequired => 'Password is required';

  @override
  String get deleteAccountInvalidPassword => 'Invalid password';

  @override
  String get pageEditorPageTitleHint => 'Page title';

  @override
  String get pageEditorSaveFirst => 'Save the page first to add sections';

  @override
  String get pageEditorNoSections => 'No sections yet';

  @override
  String get pageNoSections => 'No sections yet';

  @override
  String get pageRestoreDefaults => 'Restore Defaults';

  @override
  String get pageEditorEnterSectionTitle => 'Enter section title';

  @override
  String get pageEditorEditSectionTitle => 'Edit Section';

  @override
  String get folderPickerGoUp => 'Go up';

  @override
  String get headerLockSensitivity => 'Lock Sensitivity Access';

  @override
  String get datePickerClear => 'Clear date';

  @override
  String get entryCopyAll => 'Copy All';

  @override
  String get entryNoHistory => 'No history yet';

  @override
  String get operationViewDetails => 'View details';

  @override
  String get scanStopScan => 'Stop scan';

  @override
  String get settingsNoHintAvailable => 'No hint available';

  @override
  String get sensitiveCriticalMessage =>
      'Critical field. Enter your master password to view.';

  @override
  String get syncUnknownError => 'Unknown error';

  @override
  String get syncScanning => 'Scanning...';

  @override
  String get syncScan => 'Scan';

  @override
  String get syncSyncing => 'Syncing...';

  @override
  String get syncConnectSync => 'Connect & Sync';

  @override
  String get scanMappingBoth => 'AI+Rule';

  @override
  String get scanMappingAi => 'AI';

  @override
  String get mrzDocumentType => 'Document Type';

  @override
  String get mrzDocumentNumber => 'Document Number';

  @override
  String get mrzSurname => 'Surname';

  @override
  String get mrzGivenNames => 'Given Names';

  @override
  String get mrzNationality => 'Nationality';

  @override
  String get mrzDateOfBirth => 'Date of Birth';

  @override
  String get mrzSex => 'Sex';

  @override
  String get mrzExpiryDate => 'Expiry Date';

  @override
  String get changePasswordMinLength => 'Minimum 8 characters';

  @override
  String get operationActionCreate => 'Create';

  @override
  String get operationActionUpdate => 'Update';

  @override
  String get operationActionDelete => 'Delete';

  @override
  String get operationActionRestore => 'Restore';

  @override
  String get operationActionPurge => 'Purge';

  @override
  String get operationPlatformAndroid => 'Android';

  @override
  String get operationPlatformWeb => 'Web';

  @override
  String get operationLabelTimestamp => 'Timestamp';

  @override
  String get operationLabelAction => 'Action';

  @override
  String get operationLabelSection => 'Section';

  @override
  String get operationLabelFieldPath => 'Field Path';

  @override
  String get operationLabelDescription => 'Description';

  @override
  String get operationLabelDevice => 'Device';

  @override
  String get versionCurrentVersion => 'Current Version';

  @override
  String get versionLatestVersion => 'Latest Version';

  @override
  String get versionUpdateStatus => 'Update Status';

  @override
  String get versionPlatform => 'Platform';

  @override
  String get accountCreated => 'Created';

  @override
  String get accountLastLogin => 'Last Login';

  @override
  String get accountLastOperation => 'Last Operation';

  @override
  String get accountLoginDevices => 'Login Devices';

  @override
  String get homeDefaultPages => 'Default Pages';

  @override
  String get homeCustomizedPages => 'Customized Pages';

  @override
  String get trashDetailLabel => 'Details';

  @override
  String get trashRestoreLabel => 'Restore';

  @override
  String get trashPurgeLabel => 'Purge';

  @override
  String get commonTitle => 'Title';

  @override
  String predefinedUnknownType(String type) {
    return 'Unknown type: $type';
  }

  @override
  String get commonShowPassword => 'Show password';

  @override
  String get commonHidePassword => 'Hide password';

  @override
  String get objectCardAddItem => 'Add Item';

  @override
  String get dataManagementRestoreBackupTooltip => 'Restore';

  @override
  String get dataManagementSpecialBackupTooltip => 'Save as special backup';

  @override
  String get dataManagementSpecialBackupsTitle => 'Special Backups';

  @override
  String get dataManagementNoSpecialBackups =>
      'No special backups yet. Create one to preserve a specific version.';

  @override
  String dataManagementSpecialBackupsCount(int count, int max) {
    return '$count / $max special backups';
  }

  @override
  String dataManagementBackupsSummary(int count, String totalSize) {
    return '$count regular backup(s) · total $totalSize';
  }

  @override
  String get passwordVerificationCritical =>
      'Critical field. Enter your master password to proceed.';

  @override
  String get passwordVerificationInvalid => 'Invalid password';

  @override
  String passwordVerificationBackoff(Object seconds) {
    return 'Too many failed attempts. Please wait ${seconds}s before trying again.';
  }

  @override
  String passwordVerificationLockedOut(Object minutes) {
    return 'Account locked. Please wait $minutes minutes.';
  }

  @override
  String get trashPasswordRequired => 'Password Required';

  @override
  String trashEmptyConfirm(int count) {
    return 'Are you sure you want to permanently delete all $count items in trash?';
  }

  @override
  String get trashEmptyWarning =>
      'This action cannot be undone. All items will be permanently removed.';

  @override
  String trashEmptyComplete(int count) {
    return 'All $count items permanently deleted';
  }

  @override
  String trashRestoreConfirmBody(String name) {
    return 'Are you sure you want to restore \"$name\"?';
  }

  @override
  String trashRestoredItem(String name) {
    return 'Restored \"$name\"';
  }

  @override
  String trashPermanentDeleteConfirm(String name) {
    return 'Are you sure you want to permanently delete \"$name\"?';
  }

  @override
  String get trashPermanentDeleteWarning =>
      'This action cannot be undone. The item will be permanently removed.';

  @override
  String trashPermanentDeletedItem(String name) {
    return 'Permanently deleted \"$name\"';
  }

  @override
  String get trashEmpty => 'Trash is empty';

  @override
  String get trashNoMatching => 'No matching items';

  @override
  String get trashDeletedAppear => 'Deleted items will appear here';

  @override
  String get trashAdjustSearch => 'Try adjusting your search';

  @override
  String trashFoundResults(int count) {
    return 'Found $count result(s)';
  }

  @override
  String get trashNoResults => 'No results found';

  @override
  String trashTotalItems(int count) {
    return '$count total items in trash';
  }

  @override
  String get trashSectionTitle => 'Pages & Objects';

  @override
  String get trashAutoPurgeNotice =>
      'Items in trash are permanently deleted after 30 days';

  @override
  String get trashEmptyTrashButton => 'Empty Trash';

  @override
  String get operationLogPasswordRequired => 'Password Required';

  @override
  String get operationLogClearConfirm =>
      'Are you sure you want to clear all operation history?';

  @override
  String operationLogFoundResults(int count) {
    return 'Found $count result(s)';
  }

  @override
  String get operationLogNoMatching => 'No matching entries';

  @override
  String get operationLogTryDifferent => 'Try a different search term';

  @override
  String get operationLogAdjustFilters => 'Try adjusting your filters';

  @override
  String get operationLogFilters => 'Filters';

  @override
  String get sensitivityPasswordRequired => 'Password Required';

  @override
  String sensitivityDowngradeWarning(String name) {
    return 'You are about to downgrade \"$name\" to a lower sensitivity level.';
  }

  @override
  String get sensitivityDowngradeConfirm =>
      'This field will be visible with fewer protections. Continue?';

  @override
  String sensitivityMovedToPrivate(String name) {
    return '\"$name\" moved to Private';
  }

  @override
  String get sensitivityNoFields => 'No fields in this section';

  @override
  String get sensitivityKeepHighest => 'Keep at Highest';

  @override
  String get sensitivityMoveHigher => 'Move to Higher';

  @override
  String get sensitivityKeepLowest => 'Keep at Lowest';

  @override
  String get sensitivityMoveLower => 'Move to Lower';

  @override
  String sensitivityMovedHigher(String name) {
    return '\"$name\" moved to higher sensitivity';
  }

  @override
  String sensitivityFoundResults(int count) {
    return 'Found $count result(s)';
  }

  @override
  String get sensitivityNoResults => 'No results found';

  @override
  String get sensitivityAdjustHint =>
      'Adjust the sensitivity level for each field. Critical fields require additional verification to view.';

  @override
  String sensitivityNoFieldsMatch(String query) {
    return 'No fields match \"$query\"';
  }

  @override
  String sensitivityFieldsConfigured(int count) {
    return '$count fields configured';
  }

  @override
  String sensitivityTotalFields(int count) {
    return '$count total fields';
  }

  @override
  String get commonNA => 'N/A';

  @override
  String accountIdLabel(String id) {
    return 'Account ID: $id';
  }

  @override
  String get accountNoRecentOps => 'No recent operations';

  @override
  String get accountNoDevices => 'No devices recorded';

  @override
  String get accountRecentDevices => 'Recent Devices';

  @override
  String get settingsAllAccountsTitle => 'All Accounts';

  @override
  String accountLastLoginLabel(String time) {
    return 'Last login: $time';
  }

  @override
  String get versionUnavailable => 'Unavailable';

  @override
  String get versionUpToDate => 'Up to date';

  @override
  String get versionUpdateAvailable => 'Update available';

  @override
  String settingsAccountCount(int count) {
    return '$count account(s)';
  }

  @override
  String get debugLogSanitizeWarning =>
      'Logs will be sanitized before copying, but clipboard content is visible to other apps. The clipboard should be cleared after use.';

  @override
  String get debugLogActiveNotice =>
      'Debug mode is active. Logs are being recorded.';

  @override
  String get homeVaultUnlocked => 'Vault Unlocked';

  @override
  String get homeOnline => 'Online';

  @override
  String get homeOffline => 'Offline';

  @override
  String get searchEnterMinChars => 'Enter at least 2 characters to search';

  @override
  String get searchNoResultsBody => 'No results found';

  @override
  String get searchAdjustFilters =>
      'Try adjusting your filters or search terms';

  @override
  String get syncComplete => 'Sync complete';

  @override
  String syncFailed(String error) {
    return 'Sync failed: $error';
  }

  @override
  String get syncDirectionPushed => 'Pushed local changes';

  @override
  String get syncDirectionPulled => 'Pulled remote changes';

  @override
  String get syncDirectionMerged => 'Merged changes from both devices';

  @override
  String get syncDirectionNoChange => 'Already in sync';

  @override
  String get syncDiscoveryHint =>
      'Scan for nearby SoloSoul devices on your local network.';

  @override
  String syncFoundDevices(int count) {
    return 'Found $count device(s)';
  }

  @override
  String get syncTestFailed => 'Failed';

  @override
  String get syncDirectionPush => 'Push';

  @override
  String get syncDirectionPull => 'Pull';

  @override
  String get syncDirectionMergedShort => 'Merged';

  @override
  String get syncDirectionNoChangeShort => 'No Change';

  @override
  String get localSearchScanLocalFiles => 'Scan Local Files';

  @override
  String get localSearchDescription =>
      'Search your local files for personal information and import them into your Vault.';

  @override
  String get localSearchSelectHint =>
      'Tap to select. Long press to adjust size limit.';

  @override
  String get localSearchPrivacyNotice =>
      'All scanning is done locally. No data leaves your device. You will preview all results before importing.';

  @override
  String localSearchSkipLargerThan(String label) {
    return 'Skip $label files larger than:';
  }

  @override
  String get localSearchScanningFiles => 'Scanning files...';

  @override
  String get localSearchScanCanceled => 'Scan canceled';

  @override
  String get localSearchScanComplete => 'Scan complete';

  @override
  String get localSearchNoResultsBody =>
      'No personal information was found in the scanned files. Try using \"Full text parsing\" mode or adding more folders.';

  @override
  String get localSearchNoFiles => 'No files';

  @override
  String get scanDeselectAll => 'Deselect All';

  @override
  String get ocrScanDescription => 'Scan passport, ID card, or any document';

  @override
  String get ocrNoTextDetected => 'No text detected';

  @override
  String get ocrBusinessCardSaved => 'Business card saved';

  @override
  String get ocrInvoiceSaved => 'Invoice saved';

  @override
  String get ocrDocumentSavedAsNote => 'Document saved as a note';

  @override
  String get ocrBusinessCard => 'Business Card';

  @override
  String get ocrInvoice => 'Invoice';

  @override
  String get ocrResume => 'Resume';

  @override
  String get ocrNoResumeSections => 'No resume sections detected';

  @override
  String get ocrResumeSaved => 'Resume saved';

  @override
  String ocrResumeSavedSections(int count) {
    return 'Resume saved with $count sections';
  }

  @override
  String ocrSavedSectionsFailed(int success, int fail) {
    return 'Saved $success sections, $fail failed';
  }

  @override
  String get ocrScannedDocument => 'Scanned Document';

  @override
  String get ocrUseCamera => 'Use camera to capture document';

  @override
  String get ocrPhotoOrPdf => 'Photo or PDF file';

  @override
  String commonShowMore(int count) {
    return 'Show $count more';
  }

  @override
  String get commonCopiedToClipboard => 'Copied to clipboard';

  @override
  String get commonUntitled => 'Untitled';

  @override
  String predefinedDeletedItem(String title, String name) {
    return 'Deleted $title: $name';
  }

  @override
  String commonErrorWithMessage(String message) {
    return 'Error: $message';
  }

  @override
  String get commonObject => 'Object';

  @override
  String get fieldHistoryLatest => 'Latest';

  @override
  String get commonEmpty => '(empty)';

  @override
  String get lockVaultMessage =>
      'Locking the vault will require your master password to unlock again.';

  @override
  String get changePasswordWarning =>
      'Changing your password will re-encrypt all your data with the new key. You may also update only the password hint.';

  @override
  String get changePasswordCurrentRequired => 'Current password is required';

  @override
  String get changePasswordNewRequired => 'New password is required';

  @override
  String get changePasswordMustDiffer => 'New password must be different';

  @override
  String get changePasswordFailed => 'Failed to change password';

  @override
  String get errorInvalidCurrentPassword => 'Invalid current password';

  @override
  String entryAttachments(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count attachments',
      one: '1 attachment',
    );
    return '$_temp0';
  }

  @override
  String get profileEncryptionDesc => 'Your data is encrypted with AES-256-GCM';

  @override
  String get financialEncryptionDesc =>
      'Your financial data is encrypted with AES-256-GCM';

  @override
  String llmStatsPrompt(String tokens) {
    return 'Prompt $tokens';
  }

  @override
  String llmStatsCompletion(String tokens) {
    return 'Completion $tokens';
  }

  @override
  String get llmProviderOllama => 'Ollama';

  @override
  String get homeNoMorePages => 'No more pages to add';

  @override
  String get homeDefaultAccountName => 'Account';

  @override
  String get syncEnterPairingKey =>
      'Enter the pairing key shared from the other device.';

  @override
  String get ocrNoTextDetectedImage =>
      'No text detected in the image. Please try again with a clearer photo of the document.';

  @override
  String get ocrNoTextDetectedPdf =>
      'No text detected in the PDF. Please try again with a clearer scanned document.';

  @override
  String get ocrRecognitionTimeoutImage =>
      'Recognition timed out. Please try again with a clearer image.';

  @override
  String get ocrRecognitionTimeoutPdf =>
      'Recognition timed out. Please try again with a clearer PDF.';

  @override
  String get ocrPdfRenderFailed =>
      'Failed to render PDF page. The file may be corrupted or password-protected.';

  @override
  String get commonAddItem => 'Add Item';

  @override
  String get profileAddContact => 'Add Contact';

  @override
  String get profileEditContact => 'Edit Contact';

  @override
  String get commonAddButton => 'Add';

  @override
  String get profileIdCard => 'ID Card';

  @override
  String get profileAddress => 'Address';

  @override
  String get syncScanHint =>
      'Scan for nearby SoloSoul devices on your local network.';

  @override
  String get syncPairingHint =>
      'Generate a shared pairing key to establish a secure connection between devices. Both devices must use the same key.';

  @override
  String get syncTestSuccess => 'Success';

  @override
  String get trashEmptyMessage => 'Trash is empty';

  @override
  String accountDeviceCount(int count) {
    return '$count device(s)';
  }

  @override
  String get profileEncryptionTitle => 'End-to-End Encrypted';

  @override
  String get profileIdCardSection => 'ID card';

  @override
  String profileFormatIdentity(String data) {
    return 'Identity\n$data';
  }

  @override
  String profileFormatIdCard(String data) {
    return 'ID Card\n$data';
  }

  @override
  String travelFormatPassport(String data) {
    return 'Passport\n$data';
  }

  @override
  String travelFormatVisa(String data) {
    return 'Visa\n$data';
  }

  @override
  String travelFormatHistory(String data) {
    return 'Travel History\n$data';
  }

  @override
  String financialFormatBankAccount(String data) {
    return 'Bank Account\n$data';
  }

  @override
  String financialFormatCard(String data) {
    return 'Card\n$data';
  }

  @override
  String financialFormatTaxId(String data) {
    return 'Tax ID\n$data';
  }

  @override
  String professionalFormatEducation(String data) {
    return 'Education\n$data';
  }

  @override
  String professionalFormatEmployment(String data) {
    return 'Employment\n$data';
  }

  @override
  String professionalFormatAward(String data) {
    return 'Award\n$data';
  }

  @override
  String professionalFormatSkill(String data) {
    return 'Skill\n$data';
  }

  @override
  String professionalFormatLanguage(String data) {
    return 'Language\n$data';
  }

  @override
  String professionalFormatArticle(String data) {
    return 'Article\n$data';
  }

  @override
  String get fieldFullName => 'Full Name';

  @override
  String get fieldGivenName => 'Given Name';

  @override
  String get fieldFamilyName => 'Family Name';

  @override
  String get fieldDateOfBirth => 'Date of Birth';

  @override
  String get fieldDate => 'Date';

  @override
  String get fieldGender => 'Gender';

  @override
  String get fieldNationality => 'Nationality';

  @override
  String get fieldTitle => 'Title';

  @override
  String get fieldType => 'Type';

  @override
  String get fieldValue => 'Value';

  @override
  String get fieldNumber => 'Number';

  @override
  String get fieldIdCardNumber => 'ID Card Number';

  @override
  String get fieldIssueDate => 'Issue Date';

  @override
  String get fieldExpiryDate => 'Expiry Date';

  @override
  String get fieldHolderName => 'Holder Name';

  @override
  String get fieldCountry => 'Country';

  @override
  String get fieldStreet => 'Street';

  @override
  String get fieldCity => 'City';

  @override
  String get fieldDistrict => 'District';

  @override
  String get fieldState => 'State';

  @override
  String get fieldPostalCode => 'Postal Code';

  @override
  String get fieldPassportNumber => 'Passport Number';

  @override
  String get fieldIssuingCountry => 'Issuing Country';

  @override
  String get fieldVisaNumber => 'Visa Number';

  @override
  String get fieldEntryDate => 'Entry Date';

  @override
  String get fieldExitDate => 'Exit Date';

  @override
  String get fieldSwiftCode => 'SWIFT Code';

  @override
  String get fieldIban => 'IBAN';

  @override
  String get fieldCardNumber => 'Card Number';

  @override
  String get fieldCardholderName => 'Cardholder Name';

  @override
  String get fieldCvv => 'CVV';

  @override
  String get fieldTaxIdNumber => 'Tax ID Number';

  @override
  String get fieldInstitution => 'Institution';

  @override
  String get fieldDegree => 'Degree';

  @override
  String get fieldFieldOfStudy => 'Field of Study';

  @override
  String get fieldStartDate => 'Start Date';

  @override
  String get fieldEndDate => 'End Date';

  @override
  String get fieldCompany => 'Company';

  @override
  String get fieldPosition => 'Position';

  @override
  String get fieldCategory => 'Category';

  @override
  String get fieldLevel => 'Level';

  @override
  String get fieldLanguage => 'Language';

  @override
  String get fieldProficiency => 'Proficiency';

  @override
  String get fieldOrganization => 'Organization';

  @override
  String get fieldPhone => 'Phone';

  @override
  String get fieldEmail => 'Email';

  @override
  String get fieldContent => 'Content';

  @override
  String get fieldDone => 'Done';

  @override
  String get fieldDueDate => 'Due Date';

  @override
  String get commonYes => 'Yes';

  @override
  String get commonNo => 'No';

  @override
  String get fieldCountryCode => 'Country Code';

  @override
  String get fieldPlaceOfIssue => 'Place of Issue';

  @override
  String get fieldPlaceOfBirth => 'Place of Birth';

  @override
  String get fieldSex => 'Sex';

  @override
  String get fieldAuthority => 'Authority';

  @override
  String get fieldVisaType => 'Visa Type';

  @override
  String get fieldDestination => 'Destination';

  @override
  String get fieldTravelType => 'Travel Type';

  @override
  String get fieldDepartureCity => 'Departure City';

  @override
  String get fieldDepartureTime => 'Departure Time';

  @override
  String get fieldArrivalTime => 'Arrival Time';

  @override
  String get fieldFlightNumber => 'Flight Number';

  @override
  String get fieldTicketPrice => 'Ticket Price';

  @override
  String get fieldAirline => 'Airline';

  @override
  String get fieldCurrency => 'Currency';

  @override
  String get fieldSwiftBic => 'SWIFT/BIC';

  @override
  String get fieldSortCode => 'Sort Code';

  @override
  String get fieldCardType => 'Card Type';

  @override
  String get fieldTaxIdType => 'Tax ID Type';

  @override
  String get fieldIssuingAuthority => 'Issuing Authority';

  @override
  String get fieldDegreeCustom => 'Custom Degree';

  @override
  String get fieldField => 'Field of Study';

  @override
  String get fieldResponsibilities => 'Responsibilities';

  @override
  String get fieldIssuer => 'Issuer';

  @override
  String get fieldDescription => 'Description';

  @override
  String get fieldName => 'Name';

  @override
  String get fieldAuthors => 'Authors';

  @override
  String get fieldContactInfo => 'Contact';

  @override
  String get fieldAbstract => 'Abstract';

  @override
  String get fieldDoi => 'DOI';

  @override
  String get fieldUrl => 'URL';

  @override
  String get fieldVenue => 'Venue';

  @override
  String get fieldYear => 'Year';

  @override
  String get fieldCitation => 'Citation';

  @override
  String get datePickerSelectDate => 'Select date';

  @override
  String get headerSensitiveAccessLocked => 'Sensitive access locked';

  @override
  String get operationLogPropertySnapshot => 'Property Snapshot';

  @override
  String get dataMgmtVaultDataSize => 'Vault data size';

  @override
  String get dataMgmtAppVersion => 'App version';

  @override
  String dataMgmtRestoreOverwrite(String time) {
    return 'This will overwrite your current data with the backup from $time. A safety backup of the current state will be created first.';
  }

  @override
  String get dataMgmtRestoreSuccess =>
      'Restore successful. Please restart the app.';

  @override
  String get dataMgmtRestoreFailed => 'Restore failed';

  @override
  String dataMgmtDeleteBackupConfirm(String time) {
    return 'Delete backup from $time?';
  }

  @override
  String get dataMgmtBackupCreated => 'Backup created successfully';

  @override
  String get dataMgmtBackupFailed => 'Backup failed';

  @override
  String dataMgmtBackupError(String error) {
    return 'Backup error: $error';
  }

  @override
  String get dataMgmtBackupDeleted => 'Backup deleted';

  @override
  String get dataMgmtOperationCreatedBackup => 'Created backup';

  @override
  String get dataMgmtOperationRestoredBackup => 'Restored backup';

  @override
  String get dataMgmtOperationDeletedBackup => 'Deleted backup';

  @override
  String get dataMgmtOperationPromotedBackup => 'Promoted backup to special';

  @override
  String get dataMgmtOperationCreatedSpecial => 'Created special backup';

  @override
  String get dataMgmtOperationRenamedSpecial => 'Renamed special backup';

  @override
  String get dataMgmtOperationRestoredSpecial => 'Restored special backup';

  @override
  String dataMgmtSpecialBackupSaved(String name) {
    return 'Saved as special backup \"$name\"';
  }

  @override
  String get dataMgmtSpecialBackupFailed => 'Failed to save as special backup';

  @override
  String dataMgmtSpecialBackupCreated(String name) {
    return 'Special backup \"$name\" created';
  }

  @override
  String get dataMgmtSpecialBackupCreateFailed => 'Special backup failed';

  @override
  String dataMgmtRenamedTo(String name) {
    return 'Renamed to \"$name\"';
  }

  @override
  String dataMgmtSpecialBackupLimit(int max) {
    return 'You can keep up to $max special backups. Please delete an existing one before creating a new special backup.';
  }

  @override
  String dataMgmtSpecialBackupPromoteLimit(int max) {
    return 'You can keep up to $max special backups. Please delete an existing one before promoting.';
  }

  @override
  String get dataMgmtSafetyBackupNotice =>
      'A safety backup of the current state will be created first.';

  @override
  String get dataMgmtSpecialBackupRestored =>
      'Special backup restored. Please restart the app.';

  @override
  String get dataMgmtOperationDeletedSpecial => 'Deleted special backup';

  @override
  String get operationCreatedAccount => 'Created account';

  @override
  String get operationDeletedAccount => 'Deleted account';

  @override
  String get operationChangedPassword => 'Changed password';

  @override
  String get dataMgmtVaultSize => 'Vault size: ';

  @override
  String get dataMgmtBackupEncryptionDesc =>
      'Backups are encrypted with your vault key. Auto-backup runs on every unlock.';

  @override
  String get dataMgmtRegularBackups => 'Regular Backups';

  @override
  String get dataMgmtNoBackups => 'No backups yet';

  @override
  String get loginDataYourControl => 'Your data, your control';

  @override
  String get loginEnterMasterPassword => 'Enter Master Password';

  @override
  String get loginUnlockYourVault => 'Unlock your vault';

  @override
  String get loginUnlockButton => 'Unlock';

  @override
  String get loginNoPasswordRecovery =>
      'There is no password recovery. If you forget your master password, your data cannot be accessed.';

  @override
  String get loginPleaseEnterPassword => 'Please enter your password';

  @override
  String get securityBiometricUnlockSubtitle =>
      'Use Face ID / Touch ID to unlock';

  @override
  String get securityBiometricNotAvailable =>
      'Biometrics not available on this device';

  @override
  String get scanAttachFile => 'Attach original file';

  @override
  String trashDaysAgo(Object count) {
    return '${count}d ago';
  }

  @override
  String trashHoursAgo(Object count) {
    return '${count}h ago';
  }

  @override
  String trashMinutesAgo(Object count) {
    return '${count}m ago';
  }

  @override
  String get trashJustNow => 'Just now';

  @override
  String get trashDeletedRecently => 'Deleted recently';

  @override
  String trashDeletedAgo(Object time) {
    return 'Deleted $time ago';
  }

  @override
  String get trashShowSections => 'Show sections';

  @override
  String get trashShowItems => 'Show items';

  @override
  String get typeCollection => 'Section';

  @override
  String get typePage => 'Page';

  @override
  String get typeItem => 'Item';

  @override
  String get typeUnknown => 'Item';

  @override
  String get operationPlatformMacos => 'macOS';

  @override
  String get operationPlatformIos => 'iOS';

  @override
  String get operationPlatformWindows => 'Windows';

  @override
  String get operationPlatformLinux => 'Linux';

  @override
  String get logSectionIdentity => 'Identity';

  @override
  String get logSectionContactInfo => 'Contact';

  @override
  String get logSectionAddress => 'Address';

  @override
  String get logSectionIdCard => 'ID Card';

  @override
  String get logSectionPassport => 'Passport';

  @override
  String get logSectionVisa => 'Visa';

  @override
  String get logSectionTravelHistory => 'Travel History';

  @override
  String get logSectionBankAccount => 'Bank Account';

  @override
  String get logSectionCard => 'Card';

  @override
  String get logSectionEducation => 'Education';

  @override
  String get logSectionEmployment => 'Employment';

  @override
  String get logSectionSkill => 'Skill';

  @override
  String get logSectionLanguage => 'Language';

  @override
  String get logSectionTravel => 'Travel';

  @override
  String get logSectionFinancial => 'Financial';

  @override
  String get logSectionProfessional => 'Professional';

  @override
  String get logSectionSensitivity => 'Sensitivity';

  @override
  String get logSectionCustom => 'Custom';

  @override
  String get logSectionDefault => 'Section';

  @override
  String get operationFilterLabel => 'Filter';

  @override
  String get trashFilterLabel => 'Filter:';

  @override
  String get trashTimeFilterLabel => 'Time:';

  @override
  String get trashTimeFilterAll => 'All';

  @override
  String get trashTimeFilter10Days => 'Within 10 days';

  @override
  String get trashTimeFilter1Day => 'Within 1 day';

  @override
  String get trashTimeFilter6Hours => 'Within 6 hours';

  @override
  String get trashTimeFilter1Hour => 'Within 1 hour';

  @override
  String get trashTypeFilterLabel => 'Type:';

  @override
  String get sectionTemplateTitle => 'Section Template';

  @override
  String get sectionTemplateFilterAll => 'All';

  @override
  String get sectionTemplatePageTagBank => 'Bank';

  @override
  String sectionTemplateSelected(int count) {
    return '$count selected';
  }

  @override
  String get sectionTemplateSelectButton => 'Select Template';

  @override
  String get sectionTemplateEmpty => 'No templates available';

  @override
  String get sectionTemplateEmptyHint =>
      'Templates will appear here once configured';

  @override
  String sectionTemplateApplied(String name) {
    return 'Template \"$name\" applied';
  }

  @override
  String get templateChinaBankAccountName => 'China Bank Account';

  @override
  String get templateChinaBankAccountDesc =>
      'Contains Chinese bank account information';

  @override
  String get templateUkBankAccountName => 'UK Bank Account';

  @override
  String get templateUkBankAccountDesc =>
      'Contains UK bank account information (Sort Code + Account Number)';

  @override
  String get templateUsBankAccountName => 'US Bank Account';

  @override
  String get templateUsBankAccountDesc =>
      'Contains US bank account information (Routing Number + Account Number)';

  @override
  String sectionTemplateFieldCount(int count) {
    return '$count fields';
  }

  @override
  String get objectEditorDefaultFieldTitle => 'Title';

  @override
  String get objectEditorDefaultFieldItemName => 'Item Name';

  @override
  String objectEditorMaxLength(int count) {
    return '$count';
  }

  @override
  String get fieldBankName => 'Bank Name';

  @override
  String get fieldAccountNumber => 'Account Number';

  @override
  String get fieldAccountHolder => 'Account Holder';

  @override
  String get fieldBranchName => 'Branch Name';

  @override
  String get fieldRoutingNumber => 'Routing Number';

  @override
  String get fieldAccountType => 'Account Type';

  @override
  String get fieldChecking => 'Checking';

  @override
  String get fieldSavings => 'Savings';

  @override
  String get sectionContact => 'Contact';

  @override
  String get sectionFinancial => 'Financial';

  @override
  String get sectionMedical => 'Medical';

  @override
  String get sectionSecurity => 'Security';

  @override
  String get sectionDigitalAccounts => 'Digital Accounts';

  @override
  String get sectionInsurance => 'Insurance';

  @override
  String get templateProfileIdentityName => 'Identity';

  @override
  String get templateProfileIdentityDesc =>
      'Personal identity information including name, date of birth, gender, and nationality';

  @override
  String get templateProfileContactName => 'Contact Information';

  @override
  String get templateProfileContactDesc =>
      'Contact details such as phone number and email address';

  @override
  String get templateProfileIdCardName => 'Identity Documents';

  @override
  String get templateProfileIdCardDesc =>
      'Identity documents including ID cards, driver\'s license, and passport';

  @override
  String get templateProfileAddressName => 'Addresses';

  @override
  String get templateProfileAddressDesc =>
      'Physical addresses including street, city, state, and postal code';

  @override
  String get templateFinancialBankAccountName => 'Bank Account';

  @override
  String get templateFinancialBankAccountDesc =>
      'Contains bank account information including account number and SWIFT code';

  @override
  String get templateFinancialCardName => 'Card';

  @override
  String get templateFinancialCardDesc =>
      'Contains payment card information including card number and CVV';

  @override
  String get templateFinancialTaxIdName => 'Tax Identification';

  @override
  String get templateFinancialTaxIdDesc =>
      'Contains tax identification information';

  @override
  String get templateProfessionalEducationName => 'Education';

  @override
  String get templateProfessionalEducationDesc =>
      'Records of formal education including degrees, institutions, and fields of study';

  @override
  String get templateProfessionalEmploymentName => 'Employment';

  @override
  String get templateProfessionalEmploymentDesc =>
      'Work history including company, position, responsibilities, and tenure';

  @override
  String get templateProfessionalSkillName => 'Skill';

  @override
  String get templateProfessionalSkillDesc =>
      'Professional skills and proficiency levels';

  @override
  String get templateProfessionalLanguageName => 'Language';

  @override
  String get templateProfessionalLanguageDesc =>
      'Languages and proficiency levels';

  @override
  String get templateProfessionalAwardName => 'Award';

  @override
  String get templateProfessionalAwardDesc =>
      'Professional awards, honors, and recognitions';

  @override
  String get templateProfessionalArticleName => 'Article';

  @override
  String get templateProfessionalArticleDesc =>
      'Academic articles and publications including title, authors, DOI, venue, and citation';

  @override
  String get templateTravelPassportName => 'Passport';

  @override
  String get templateTravelPassportDesc =>
      'Contains passport information including number, issue date, expiry date, and holder details';

  @override
  String get templateTravelVisaName => 'Visa';

  @override
  String get templateTravelVisaDesc =>
      'Contains visa information including type, number, issue date, and expiry date';

  @override
  String get templateTravelHistoryName => 'Travel History';

  @override
  String get templateTravelHistoryDesc =>
      'Records of travel including destination, dates, flights, and travel details';

  @override
  String get templateSecurityName => 'Security';

  @override
  String get templateSecurityDesc =>
      'Security keys and authentication info, including TOTP secrets and recovery codes';

  @override
  String get objectEditorSchemaUpdated =>
      'Property schema has been updated, syncing automatically';

  @override
  String objectEditorShowDeprecated(int count) {
    return '$count deprecated';
  }

  @override
  String get objectEditorHideDeprecated => 'Hide Deprecated';

  @override
  String get objectEditorDeprecatedProperties => 'Deprecated Properties';

  @override
  String get objectEditorDeprecatedBadge => 'Deprecated';

  @override
  String get objectEditorRestoreProperty => 'Restore Property';

  @override
  String operationLogCreatedItem(String name) {
    return 'Created item \"$name\"';
  }

  @override
  String operationLogUpdatedItem(String name) {
    return 'Updated item \"$name\"';
  }

  @override
  String operationLogDeletedItem(String name) {
    return 'Deleted item \"$name\"';
  }

  @override
  String operationLogRestoredItem(String name) {
    return 'Restored \"$name\"';
  }

  @override
  String operationNotifCreated(String name) {
    return 'Added \"$name\"';
  }

  @override
  String operationNotifUpdated(String name) {
    return 'Updated \"$name\"';
  }

  @override
  String operationNotifDeleted(String name) {
    return 'Deleted \"$name\"';
  }

  @override
  String operationNotifRestored(String name) {
    return 'Restored \"$name\"';
  }

  @override
  String operationNotifPurged(String name) {
    return 'Permanently deleted \"$name\"';
  }

  @override
  String get operationNotifUndo => 'Undo';

  @override
  String get operationNotifDismiss => 'Dismiss';

  @override
  String get predefinedCopySuccess => 'Copied to clipboard';

  @override
  String predefinedDeleteFailed(String section) {
    return 'Failed to delete $section';
  }

  @override
  String operationLogSensitivitySet(String field, String level) {
    return 'Set \"$field\" sensitivity to $level';
  }

  @override
  String operationLogSensitivityChanged(
    String field,
    String oldLevel,
    String newLevel,
  ) {
    return 'Changed \"$field\" sensitivity from $oldLevel to $newLevel';
  }

  @override
  String operationLogSensitivityReverted(String field, String oldLevel) {
    return 'Reverted \"$field\" sensitivity to default (was $oldLevel)';
  }

  @override
  String get operationCreatedItem => 'Created item';

  @override
  String get operationUpdatedItem => 'Updated item';

  @override
  String get operationDeletedItem => 'Deleted item';

  @override
  String get operationRestoredItem => 'Restored item';

  @override
  String get operationPurgedItem => 'Permanently deleted item';

  @override
  String get operationChangedSensitivity => 'Changed sensitivity';

  @override
  String get operationUpgradedSensitivity => 'Upgraded sensitivity';

  @override
  String get operationDowngradedSensitivity => 'Downgraded sensitivity';

  @override
  String get operationRevertedSensitivity => 'Reverted sensitivity';

  @override
  String get pluginManagement => 'Plugin Management';

  @override
  String get pluginManagementSubtitle =>
      'Manage installed plugins and browse the marketplace';

  @override
  String get pluginManagementSubtitleIOS =>
      'Plugin execution unavailable on iOS';

  @override
  String get pluginIOSUnsupportedBanner =>
      'Plugin execution is not supported on iOS due to platform restrictions.';

  @override
  String get pluginDashboardTitle => 'Plugin Management';

  @override
  String get pluginTabAll => 'All';

  @override
  String get pluginTabInstalled => 'Installed';

  @override
  String get pluginTabAvailable => 'Available';

  @override
  String get pluginSearchHint => 'Search plugins...';

  @override
  String get pluginEmptyStateTitle => 'No plugins available';

  @override
  String get pluginEmptyStateSubtitle =>
      'Connect to the internet to browse the plugin marketplace';

  @override
  String get pluginOfflineBanner => 'Offline mode. Cannot fetch new plugins.';

  @override
  String get pluginStatusInstalled => 'Installed';

  @override
  String get pluginStatusNotInstalled => 'Not installed';

  @override
  String get pluginStatusUpdateAvailable => 'Update available';

  @override
  String get pluginStatusIncompatible => 'Incompatible';

  @override
  String get pluginStatusRunning => 'Running';

  @override
  String get pluginActionInstall => 'Install';

  @override
  String get pluginActionUpdate => 'Update';

  @override
  String get pluginActionRun => 'Run';

  @override
  String get pluginActionStop => 'Stop';

  @override
  String get pluginActionUninstall => 'Uninstall';

  @override
  String get pluginConsentDialogTitle => 'Plugin Data Authorization';

  @override
  String pluginConsentDialogSubtitle(String pluginName) {
    return 'The plugin \"$pluginName\" requests access to the following data:';
  }

  @override
  String get pluginConsentDialogDataLifetime =>
      'Data is only available during this session and will be automatically destroyed when it expires.';

  @override
  String get pluginConsentButtonDeny => 'Deny';

  @override
  String get pluginConsentButtonAuthorize => 'Authorize';

  @override
  String get pluginSensitivityPublic => 'Public';

  @override
  String get pluginSensitivityInternal => 'Internal';

  @override
  String get pluginSensitivitySensitive => 'Sensitive';

  @override
  String get pluginSensitivityCritical => 'Critical';

  @override
  String get pluginUninstallConfirmTitle => 'Uninstall Plugin?';

  @override
  String get pluginUninstallConfirmMessage =>
      'This will delete the plugin files and revoke all data authorizations. Audit logs will be retained.';

  @override
  String get pluginInstallSuccess => 'Plugin installed successfully';

  @override
  String get pluginUninstallSuccess => 'Plugin uninstalled successfully';

  @override
  String get pluginUpdateSuccess => 'Plugin updated successfully';

  @override
  String get pluginRunSuccess => 'Plugin executed successfully';

  @override
  String get pluginErrorNotFound => 'Plugin not found';

  @override
  String get pluginErrorIncompatible =>
      'Plugin is incompatible with this app version';

  @override
  String get pluginErrorSecurity => 'Security verification failed';

  @override
  String get pluginErrorNetwork =>
      'Network error. Please check your connection.';

  @override
  String get pluginNameAddressFmt => 'Address Formatter';

  @override
  String get pluginNameCalendarEvents => 'Calendar Events';

  @override
  String get pluginNameContactExporter => 'Contact Exporter';

  @override
  String get pluginNameDataCompleteness => 'Data Completeness';

  @override
  String get pluginNameDigitalWill => 'Digital Will';

  @override
  String get pluginNameDocChecklist => 'Doc Checklist';

  @override
  String get pluginNameEmergencyCard => 'Emergency Card';

  @override
  String get pluginNameExpiryGuardian => 'Expiry Guardian';

  @override
  String get pluginNameFormPrefiller => 'Form Prefiller';

  @override
  String get pluginNameIdValidator => 'ID Validator';

  @override
  String get pluginNameIdentityTimeline => 'Identity Timeline';

  @override
  String get pluginNameMrzEncoder => 'MRZ Encoder';

  @override
  String get pluginNameNamecardGen => 'Namecard Generator';

  @override
  String get pluginNamePackingList => 'Packing List';

  @override
  String get pluginNamePhoneFmt => 'Phone Formatter';

  @override
  String get pluginNameResumeBuilder => 'Resume Builder';

  @override
  String get pluginNameTaxProfile => 'Tax Profile';

  @override
  String get pluginNameTotpGen => 'TOTP Generator';

  @override
  String get pluginNameTravelFootprint => 'Travel Footprint';

  @override
  String get pluginNameSlotgo => 'SlotGo';

  @override
  String get pluginDescAddressFmt =>
      'Format all addresses in the Vault according to target country/region standards. Supports 10 countries/regions including China, US, UK, Japan, Germany, France, Canada, Australia, Singapore, and South Korea.';

  @override
  String get pluginDescCalendarEvents =>
      'Extract date-related information from the Vault and generate a calendar event preview list.';

  @override
  String get pluginDescContactExporter =>
      'Export contact information from the Vault into standard vCard format.';

  @override
  String get pluginDescDataCompleteness =>
      'Scan all sections in the Vault, calculate completeness percentage, and generate a report highlighting missing key fields.';

  @override
  String get pluginDescDigitalWill =>
      'Generate digital estate allocation suggestions based on data stored in the Vault.';

  @override
  String get pluginDescDocChecklist =>
      'Infer existing and missing materials from the Vault based on target scenarios (visa application, bank account opening, etc.).';

  @override
  String get pluginDescEmergencyCard =>
      'Generate an emergency medical/contact information card. Data is stored locally with no network required.';

  @override
  String get pluginDescExpiryGuardian =>
      'Scan all documents (passports, visas, ID cards, credit cards) for expiration dates and sort by urgency (30/60/90/180 days).';

  @override
  String get pluginDescFormPrefiller =>
      'Generate a mapping table from Vault fields to target form fields based on the scenario (visa, bank, hotel, etc.).';

  @override
  String get pluginDescIdValidator =>
      'Validate ID number formats and check digits for various countries, including Chinese ID, US SSN, and UK NI Number.';

  @override
  String get pluginDescIdentityTimeline =>
      'Display the user\'s identity evolution over time, including education, work, visas, and asset acquisition milestones.';

  @override
  String get pluginDescMrzEncoder =>
      'Encode passport/ID card information from the Vault into ICAO Doc 9303 standard Machine Readable Zone (MRZ) format.';

  @override
  String get pluginDescNamecardGen =>
      'Generate an encrypted digital business card as a QR code, decryptable only by scanning with SoloSoul.';

  @override
  String get pluginDescPackingList =>
      'Intelligently generate a packing list suggestion based on travel records and destination information in the Vault.';

  @override
  String get pluginDescPhoneFmt =>
      'Format phone numbers from the Vault according to target country/region dialing standards.';

  @override
  String get pluginDescResumeBuilder =>
      'Automatically generate a standard-format resume by extracting education, work experience, skills, and languages from the Vault.';

  @override
  String get pluginDescTaxProfile =>
      'Summarize basic tax filing data based on the user\'s country of residence, income sources, and tax residency status.';

  @override
  String get pluginDescTotpGen =>
      'Generate 6-digit TOTP dynamic verification codes based on the 2FA Secret stored in the Vault, following RFC 6238.';

  @override
  String get pluginDescTravelFootprint =>
      'Analyze visa and travel records in the Vault to generate country visit statistics, favorite regions, and travel timeline reports.';

  @override
  String get pluginDescSlotgo =>
      'UK Visa appointment system assistant framework to help auto-fill and submit booking forms to the TLScontact system.';

  @override
  String get pluginDetailTitleIntro => 'Introduction';

  @override
  String get pluginDetailTitleChangelog => 'Changelog';

  @override
  String get pluginDetailTitleInfo => 'Info';

  @override
  String get pluginDetailFeatureIntro => 'Features';

  @override
  String get pluginDetailRequiredFields => 'Required Data Fields';

  @override
  String get pluginDetailRequired => 'Required';

  @override
  String get pluginDetailOptional => 'Optional';

  @override
  String get pluginDetailVersionCompat => 'Version Compatibility';

  @override
  String get pluginDetailMinAppVersion => 'Min App Version';

  @override
  String get pluginDetailMaxAppVersion => 'Max App Version';

  @override
  String get pluginDetailPluginApiVersion => 'Plugin API Version';

  @override
  String get pluginDetailBasicInfo => 'Basic Info';

  @override
  String get pluginDetailInstallInfo => 'Installation Info';

  @override
  String get pluginDetailPluginId => 'Plugin ID';

  @override
  String get pluginDetailPluginName => 'Name';

  @override
  String get pluginDetailPublisher => 'Publisher';

  @override
  String get pluginDetailHomepage => 'Homepage';

  @override
  String get pluginDetailStatus => 'Status';

  @override
  String get pluginDetailStatusInstalled => 'Installed';

  @override
  String get pluginDetailStatusNotInstalled => 'Not Installed';

  @override
  String get pluginDetailInstalledVersion => 'Installed Version';

  @override
  String get pluginDetailLatestVersion => 'Latest Version';

  @override
  String get pluginDetailInstallTime => 'Installed At';

  @override
  String get pluginDetailLastUsed => 'Last Used';

  @override
  String get pluginDetailNeverUsed => 'Never used';

  @override
  String get pluginDetailNoChangelog => 'No changelog';

  @override
  String get pluginDetailNoVersions => 'No version records';

  @override
  String get pluginDetailCurrent => 'Current';

  @override
  String get pluginVersionHistoryTitle => 'Version History';

  @override
  String pluginVersionCurrentLabel(Object version) {
    return 'v$version (Current)';
  }

  @override
  String get pluginActionDetail => 'Details';

  @override
  String get localSearchTabDocumentScan => 'Document Scan';

  @override
  String get localSearchTabLocalImport => 'Local Import';

  @override
  String get sensitivityOverrideTitle => 'Sensitivity Override';

  @override
  String sensitivityOverrideDescription(
    String pluginName,
    String fieldLabel,
    String actual,
    String required,
  ) {
    return '$pluginName requests access to \"$fieldLabel\" which is set to $actual. The plugin requires sensitivity ≤ $required.';
  }

  @override
  String get sensitivityOverrideDenyTitle => 'Deny Access';

  @override
  String get sensitivityOverrideDenyDesc =>
      'Block plugin from accessing this field';

  @override
  String get sensitivityOverrideMaskTitle => 'Mask Value';

  @override
  String get sensitivityOverrideMaskDesc =>
      'Return masked/redacted value to plugin';

  @override
  String get sensitivityOverrideAllowTitle => 'Allow Access';

  @override
  String get sensitivityOverrideAllowDesc =>
      'Grant plugin access to this field for this session';

  @override
  String get sensitivityOverrideRemember => 'Remember my choice';

  @override
  String get sensitivityOverrideConfirm => 'Confirm Override';

  @override
  String get semanticTypeDuplicateTitle => 'Duplicate Semantic Type';

  @override
  String semanticTypeDuplicateMessage(
    String typeLabel,
    String existingFieldLabel,
  ) {
    return 'The semantic type \"$typeLabel\" is already assigned to \"$existingFieldLabel\". Assigning it here will remove it from the other field.';
  }

  @override
  String get semanticTypeDuplicateHint =>
      'Each semantic type can only be assigned to one field at a time.';

  @override
  String get semanticTypeDuplicateContinue => 'Reassign Anyway';

  @override
  String get semanticTypePickerTitle => 'Select Semantic Type';

  @override
  String get semanticTypeSearchHint => 'Search semantic types...';

  @override
  String get semanticTypeNone => 'None (Remove)';

  @override
  String pluginAccessReviewTitle(String pluginName) {
    return 'Plugin Access Review: $pluginName';
  }

  @override
  String get pluginAccessReviewSubtitle =>
      'Review the fields this plugin wants to access before installation';

  @override
  String get pluginAccessReviewModifySensitivity => 'Modify Sensitivity';

  @override
  String get pluginAccessReviewCreateMissing => 'Create Missing Fields';

  @override
  String get pluginAccessReviewContinue => 'Continue Install';

  @override
  String get pluginAccessReviewHeaderField => 'Field';

  @override
  String get pluginAccessReviewHeaderSection => 'Section';

  @override
  String get pluginAccessReviewHeaderSensitivity => 'Sensitivity';

  @override
  String get pluginAccessReviewHeaderStatus => 'Status';

  @override
  String get pluginAccessReviewNoSection => '(none)';

  @override
  String get pluginAccessReviewMissing => 'Missing';

  @override
  String get pluginAccessReviewExceededTitle => 'Sensitivity Exceeded';

  @override
  String pluginAccessReviewExceededItem(
    String label,
    String actual,
    String required,
  ) {
    return '$label: $actual → requires ≤ $required';
  }

  @override
  String get pluginAccessReviewMissingTitle => 'Missing Fields';

  @override
  String pluginAccessReviewMissingItem(String label) {
    return '$label';
  }

  @override
  String get addAttachment => 'Add attachment';

  @override
  String get attachmentAdded => 'Attachment added';

  @override
  String get attachmentAddFailed => 'Failed to add attachment';

  @override
  String get attachmentReadFailed => 'Failed to read file';

  @override
  String get deleteAttachment => 'Delete attachment';

  @override
  String get deleteAttachmentConfirm =>
      'Are you sure you want to delete this attachment?';

  @override
  String get attachmentDeleted => 'Attachment deleted';

  @override
  String get restoreAttachment => 'Restore';

  @override
  String get permanentlyDeleteAttachment => 'Delete permanently';

  @override
  String get attachmentPermanentlyDeleteConfirm =>
      'This attachment will be permanently deleted and cannot be recovered. Continue?';

  @override
  String get deletedAttachments => 'Deleted attachments';

  @override
  String deletedAtDaysAgo(Object days) {
    return 'Deleted $days days ago';
  }

  @override
  String get noAttachments => 'No attachments';

  @override
  String get attachmentRestored => 'Attachment restored';

  @override
  String attachmentMaxReached(Object count) {
    return 'Maximum $count attachments reached. Please delete some before adding.';
  }

  @override
  String get attachmentDeleteFailed => 'Failed to delete attachment';

  @override
  String get attachmentRestoreFailed => 'Failed to restore attachment';

  @override
  String get downloadAttachment => 'Download';

  @override
  String attachmentDownloaded(Object path) {
    return 'Saved to $path';
  }

  @override
  String get attachmentDownloadFailed => 'Download failed';

  @override
  String get downloadLocation => 'Download Location';

  @override
  String get downloadLocationDesc => 'Attachments will be saved to this folder';

  @override
  String get chooseFolder => 'Choose Folder';

  @override
  String get downloadLocationDefault => 'Default (Downloads)';
}
