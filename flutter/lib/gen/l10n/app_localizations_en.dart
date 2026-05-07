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
  String get commonLoading => 'Loading...';

  @override
  String get commonSuccess => 'Success';

  @override
  String get commonDelete => 'Delete';

  @override
  String get commonEdit => 'Edit';

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
}
