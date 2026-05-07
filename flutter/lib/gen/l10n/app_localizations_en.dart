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
