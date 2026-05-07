import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('zh'),
  ];

  /// No description provided for @commonCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get commonCancel;

  /// No description provided for @commonConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get commonConfirm;

  /// No description provided for @commonSave.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get commonSave;

  /// No description provided for @commonImport.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get commonImport;

  /// No description provided for @commonError.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get commonError;

  /// No description provided for @commonRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get commonRetry;

  /// No description provided for @commonClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get commonClose;

  /// No description provided for @commonLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading...'**
  String get commonLoading;

  /// No description provided for @commonSuccess.
  ///
  /// In en, this message translates to:
  /// **'Success'**
  String get commonSuccess;

  /// No description provided for @commonDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get commonDelete;

  /// No description provided for @commonEdit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get commonEdit;

  /// Settings tile label for language selection
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsLanguage;

  /// No description provided for @settingsLanguageSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Select your preferred language'**
  String get settingsLanguageSubtitle;

  /// No description provided for @settingsLanguageEnglish.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get settingsLanguageEnglish;

  /// No description provided for @settingsLanguageChinese.
  ///
  /// In en, this message translates to:
  /// **'中文 (Chinese)'**
  String get settingsLanguageChinese;

  /// No description provided for @settingsAiChat.
  ///
  /// In en, this message translates to:
  /// **'AI Chat'**
  String get settingsAiChat;

  /// No description provided for @settingsAiChatSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Chat with local or cloud models'**
  String get settingsAiChatSubtitle;

  /// No description provided for @settingsDeleteAccountWarning.
  ///
  /// In en, this message translates to:
  /// **'After deleting the account, all data will be cleared. Are you sure you want to delete it?'**
  String get settingsDeleteAccountWarning;

  /// No description provided for @mainAppTitle.
  ///
  /// In en, this message translates to:
  /// **'SoloSoul'**
  String get mainAppTitle;

  /// No description provided for @mainSplashTagline.
  ///
  /// In en, this message translates to:
  /// **'Orchestrate your life data, reshape your digital origin'**
  String get mainSplashTagline;

  /// No description provided for @mainLaunchFailed.
  ///
  /// In en, this message translates to:
  /// **'Launch failed'**
  String get mainLaunchFailed;

  /// No description provided for @sidebarAiChat.
  ///
  /// In en, this message translates to:
  /// **'AI Chat'**
  String get sidebarAiChat;

  /// Page title for LLM configuration
  ///
  /// In en, this message translates to:
  /// **'AI Assistant Settings'**
  String get llmConfigTitle;

  /// No description provided for @llmConfigNotLoaded.
  ///
  /// In en, this message translates to:
  /// **'Config not loaded'**
  String get llmConfigNotLoaded;

  /// No description provided for @llmConfigOllamaNotRunning.
  ///
  /// In en, this message translates to:
  /// **'Ollama service is not running\nPlease make sure Ollama is installed and running'**
  String get llmConfigOllamaNotRunning;

  /// No description provided for @llmConfigOllamaModelNotInstalled.
  ///
  /// In en, this message translates to:
  /// **'Ollama is running, but model {model} is not installed\nInstalled models: {models}'**
  String llmConfigOllamaModelNotInstalled(String model, String models);

  /// No description provided for @llmConfigLocalSuccess.
  ///
  /// In en, this message translates to:
  /// **'Local model connected successfully!'**
  String get llmConfigLocalSuccess;

  /// No description provided for @llmConfigConnectionFailed.
  ///
  /// In en, this message translates to:
  /// **'Connection failed: {message}'**
  String llmConfigConnectionFailed(String message);

  /// No description provided for @llmConfigUnknownError.
  ///
  /// In en, this message translates to:
  /// **'Unknown error: {message}'**
  String llmConfigUnknownError(String message);

  /// No description provided for @llmConfigSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Save failed: {message}'**
  String llmConfigSaveFailed(String message);

  /// No description provided for @llmConfigDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete Configuration'**
  String get llmConfigDeleteTitle;

  /// No description provided for @llmConfigDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete \"{name}\"? This action cannot be undone.'**
  String llmConfigDeleteConfirm(String name);

  /// No description provided for @llmConfigExperimental.
  ///
  /// In en, this message translates to:
  /// **'Experimental'**
  String get llmConfigExperimental;

  /// No description provided for @llmConfigLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Load failed: {message}'**
  String llmConfigLoadFailed(String message);

  /// No description provided for @llmConfigInferenceBackend.
  ///
  /// In en, this message translates to:
  /// **'Inference Backend'**
  String get llmConfigInferenceBackend;

  /// No description provided for @llmConfigModelName.
  ///
  /// In en, this message translates to:
  /// **'Model Name'**
  String get llmConfigModelName;

  /// No description provided for @llmConfigInstructions.
  ///
  /// In en, this message translates to:
  /// **'Instructions'**
  String get llmConfigInstructions;

  /// No description provided for @llmConfigInstructionsOllama.
  ///
  /// In en, this message translates to:
  /// **'1. Install Ollama: https://ollama.com\n2. Pull model: ollama pull qwen2.5:1.5b\n3. Keep Ollama running in the background'**
  String get llmConfigInstructionsOllama;

  /// No description provided for @llmConfigCloudConfig.
  ///
  /// In en, this message translates to:
  /// **'Cloud Configuration'**
  String get llmConfigCloudConfig;

  /// No description provided for @llmConfigAddProfile.
  ///
  /// In en, this message translates to:
  /// **'Add Configuration'**
  String get llmConfigAddProfile;

  /// No description provided for @llmConfigCloudConsent.
  ///
  /// In en, this message translates to:
  /// **'Consent to Cloud Processing'**
  String get llmConfigCloudConsent;

  /// No description provided for @llmConfigCloudConsentDesc.
  ///
  /// In en, this message translates to:
  /// **'I confirm that the current batch does not contain critical-level fields, and agree to send data to the specified enterprise/private API endpoint.'**
  String get llmConfigCloudConsentDesc;

  /// No description provided for @llmConfigStatsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'View token consumption, conversation count, etc.'**
  String get llmConfigStatsSubtitle;

  /// No description provided for @llmConfigTesting.
  ///
  /// In en, this message translates to:
  /// **'Testing...'**
  String get llmConfigTesting;

  /// No description provided for @llmConfigTestConnection.
  ///
  /// In en, this message translates to:
  /// **'Test Connection'**
  String get llmConfigTestConnection;

  /// No description provided for @llmConfigModelInfo.
  ///
  /// In en, this message translates to:
  /// **'Model: {model}'**
  String llmConfigModelInfo(String model);

  /// No description provided for @llmConfigEndpointInfo.
  ///
  /// In en, this message translates to:
  /// **'Endpoint: {endpoint}'**
  String llmConfigEndpointInfo(String endpoint);

  /// No description provided for @llmConfigNoProfiles.
  ///
  /// In en, this message translates to:
  /// **'No cloud configurations yet'**
  String get llmConfigNoProfiles;

  /// No description provided for @llmConfigNoProfilesHint.
  ///
  /// In en, this message translates to:
  /// **'Tap the button below to create your first cloud API configuration'**
  String get llmConfigNoProfilesHint;

  /// No description provided for @llmConfigNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Please enter a configuration name'**
  String get llmConfigNameRequired;

  /// No description provided for @llmConfigApiKeyRequired.
  ///
  /// In en, this message translates to:
  /// **'API Key is required for new configurations'**
  String get llmConfigApiKeyRequired;

  /// No description provided for @llmConfigEndpointModelRequired.
  ///
  /// In en, this message translates to:
  /// **'Endpoint and Model cannot be empty'**
  String get llmConfigEndpointModelRequired;

  /// No description provided for @llmConfigEditProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit Configuration'**
  String get llmConfigEditProfile;

  /// No description provided for @llmConfigProfileName.
  ///
  /// In en, this message translates to:
  /// **'Configuration Name'**
  String get llmConfigProfileName;

  /// No description provided for @llmConfigProfileNameHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. OpenAI Production'**
  String get llmConfigProfileNameHint;

  /// No description provided for @llmConfigApiKeySet.
  ///
  /// In en, this message translates to:
  /// **'API Key (configured)'**
  String get llmConfigApiKeySet;

  /// No description provided for @llmConfigApiKeyNew.
  ///
  /// In en, this message translates to:
  /// **'API Key *'**
  String get llmConfigApiKeyNew;

  /// No description provided for @llmConfigApiKeyHintNew.
  ///
  /// In en, this message translates to:
  /// **'Enter a new value to replace the existing key'**
  String get llmConfigApiKeyHintNew;

  /// No description provided for @llmConfigApiKeyHintKeep.
  ///
  /// In en, this message translates to:
  /// **'Leave blank to keep the existing key'**
  String get llmConfigApiKeyHintKeep;

  /// No description provided for @llmConfigSave.
  ///
  /// In en, this message translates to:
  /// **'Save Changes'**
  String get llmConfigSave;

  /// No description provided for @llmConfigCreate.
  ///
  /// In en, this message translates to:
  /// **'Create Configuration'**
  String get llmConfigCreate;

  /// No description provided for @llmConfigBackendLocal.
  ///
  /// In en, this message translates to:
  /// **'Local Model'**
  String get llmConfigBackendLocal;

  /// No description provided for @llmConfigBackendCloud.
  ///
  /// In en, this message translates to:
  /// **'Cloud API'**
  String get llmConfigBackendCloud;

  /// No description provided for @llmStatsTitle.
  ///
  /// In en, this message translates to:
  /// **'Usage Statistics'**
  String get llmStatsTitle;

  /// No description provided for @llmStatsCurrentModel.
  ///
  /// In en, this message translates to:
  /// **'Current Model'**
  String get llmStatsCurrentModel;

  /// No description provided for @llmStatsSessionStats.
  ///
  /// In en, this message translates to:
  /// **'Session Statistics'**
  String get llmStatsSessionStats;

  /// No description provided for @llmStatsAccountStats.
  ///
  /// In en, this message translates to:
  /// **'Account Statistics'**
  String get llmStatsAccountStats;

  /// No description provided for @llmStatsTokenBreakdown.
  ///
  /// In en, this message translates to:
  /// **'Token Breakdown'**
  String get llmStatsTokenBreakdown;

  /// No description provided for @llmStatsDailyTrend.
  ///
  /// In en, this message translates to:
  /// **'Daily Token Trend (Last 14 Days)'**
  String get llmStatsDailyTrend;

  /// No description provided for @llmStatsModelUsage.
  ///
  /// In en, this message translates to:
  /// **'Model Usage'**
  String get llmStatsModelUsage;

  /// No description provided for @llmStatsReset.
  ///
  /// In en, this message translates to:
  /// **'Reset Statistics'**
  String get llmStatsReset;

  /// No description provided for @llmStatsResetConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to reset all usage statistics? This action cannot be undone.'**
  String get llmStatsResetConfirm;

  /// No description provided for @llmStatsResetSuccess.
  ///
  /// In en, this message translates to:
  /// **'Statistics have been reset'**
  String get llmStatsResetSuccess;

  /// No description provided for @llmStatsUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get llmStatsUnknown;

  /// No description provided for @llmStatsNotLoaded.
  ///
  /// In en, this message translates to:
  /// **'Not Loaded'**
  String get llmStatsNotLoaded;

  /// No description provided for @llmStatsLocalModelOllama.
  ///
  /// In en, this message translates to:
  /// **'Local Model (Ollama)'**
  String get llmStatsLocalModelOllama;

  /// No description provided for @llmStatsModelLabel.
  ///
  /// In en, this message translates to:
  /// **'Model'**
  String get llmStatsModelLabel;

  /// No description provided for @llmStatsProviderLabel.
  ///
  /// In en, this message translates to:
  /// **'Provider'**
  String get llmStatsProviderLabel;

  /// No description provided for @llmStatsConversationCount.
  ///
  /// In en, this message translates to:
  /// **'Conversations'**
  String get llmStatsConversationCount;

  /// No description provided for @llmStatsTokenConsumption.
  ///
  /// In en, this message translates to:
  /// **'Token Consumption'**
  String get llmStatsTokenConsumption;

  /// No description provided for @llmStatsLastLoaded.
  ///
  /// In en, this message translates to:
  /// **'Last Loaded'**
  String get llmStatsLastLoaded;

  /// No description provided for @llmStatsLastUsed.
  ///
  /// In en, this message translates to:
  /// **'Last Used'**
  String get llmStatsLastUsed;

  /// No description provided for @llmStatsTotalConversations.
  ///
  /// In en, this message translates to:
  /// **'Total Conversations'**
  String get llmStatsTotalConversations;

  /// No description provided for @llmStatsTotalTokens.
  ///
  /// In en, this message translates to:
  /// **'Total Tokens'**
  String get llmStatsTotalTokens;

  /// No description provided for @llmStatsSession.
  ///
  /// In en, this message translates to:
  /// **'Session'**
  String get llmStatsSession;

  /// No description provided for @llmStatsAccountTotal.
  ///
  /// In en, this message translates to:
  /// **'Account Total'**
  String get llmStatsAccountTotal;

  /// No description provided for @llmStatsAllModels.
  ///
  /// In en, this message translates to:
  /// **'All Models'**
  String get llmStatsAllModels;

  /// No description provided for @llmChatTitle.
  ///
  /// In en, this message translates to:
  /// **'AI Chat'**
  String get llmChatTitle;

  /// No description provided for @llmChatBackendCloud.
  ///
  /// In en, this message translates to:
  /// **'Cloud'**
  String get llmChatBackendCloud;

  /// No description provided for @llmChatBackendLocal.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get llmChatBackendLocal;

  /// No description provided for @llmChatModelNotConfigured.
  ///
  /// In en, this message translates to:
  /// **'Not configured'**
  String get llmChatModelNotConfigured;

  /// No description provided for @llmChatModelNotLoaded.
  ///
  /// In en, this message translates to:
  /// **'Model not loaded. Please configure LLM first.'**
  String get llmChatModelNotLoaded;

  /// No description provided for @llmChatClearSession.
  ///
  /// In en, this message translates to:
  /// **'Clear session'**
  String get llmChatClearSession;

  /// No description provided for @llmChatThinking.
  ///
  /// In en, this message translates to:
  /// **'Thinking...'**
  String get llmChatThinking;

  /// No description provided for @llmChatNoResponse.
  ///
  /// In en, this message translates to:
  /// **'No response received'**
  String get llmChatNoResponse;

  /// No description provided for @llmChatInputHintReady.
  ///
  /// In en, this message translates to:
  /// **'Type a message...'**
  String get llmChatInputHintReady;

  /// No description provided for @llmChatInputHintNotReady.
  ///
  /// In en, this message translates to:
  /// **'Model not ready'**
  String get llmChatInputHintNotReady;

  /// No description provided for @llmChatStatusReady.
  ///
  /// In en, this message translates to:
  /// **'Ready'**
  String get llmChatStatusReady;

  /// No description provided for @llmChatStatusNotReady.
  ///
  /// In en, this message translates to:
  /// **'Not ready'**
  String get llmChatStatusNotReady;

  /// No description provided for @llmChatLoadingConfig.
  ///
  /// In en, this message translates to:
  /// **'Loading model configuration...'**
  String get llmChatLoadingConfig;

  /// No description provided for @llmChatStartConversation.
  ///
  /// In en, this message translates to:
  /// **'Start chatting with AI'**
  String get llmChatStartConversation;

  /// No description provided for @llmChatConnectCloudModel.
  ///
  /// In en, this message translates to:
  /// **'Connect cloud model'**
  String get llmChatConnectCloudModel;

  /// No description provided for @llmChatStartLocalModel.
  ///
  /// In en, this message translates to:
  /// **'Start local model'**
  String get llmChatStartLocalModel;

  /// No description provided for @llmChatGoToConfig.
  ///
  /// In en, this message translates to:
  /// **'Go to LLM Config'**
  String get llmChatGoToConfig;

  /// No description provided for @llmErrorConfigNotLoaded.
  ///
  /// In en, this message translates to:
  /// **'Configuration not loaded'**
  String get llmErrorConfigNotLoaded;

  /// No description provided for @llmErrorCloudConfigIncomplete.
  ///
  /// In en, this message translates to:
  /// **'Cloud configuration incomplete: please check API Key and privacy consent'**
  String get llmErrorCloudConfigIncomplete;

  /// No description provided for @llmErrorNoActiveCloudProfile.
  ///
  /// In en, this message translates to:
  /// **'No active cloud configuration'**
  String get llmErrorNoActiveCloudProfile;

  /// No description provided for @llmErrorApiKeyEmpty.
  ///
  /// In en, this message translates to:
  /// **'API Key is empty'**
  String get llmErrorApiKeyEmpty;

  /// No description provided for @llmCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get llmCopy;

  /// No description provided for @llmCopied.
  ///
  /// In en, this message translates to:
  /// **'Copied'**
  String get llmCopied;

  /// No description provided for @llmInferenceError.
  ///
  /// In en, this message translates to:
  /// **'Inference Error'**
  String get llmInferenceError;

  /// No description provided for @ocrScanDocument.
  ///
  /// In en, this message translates to:
  /// **'Scan Document'**
  String get ocrScanDocument;

  /// No description provided for @ocrTakePhoto.
  ///
  /// In en, this message translates to:
  /// **'Take Photo'**
  String get ocrTakePhoto;

  /// No description provided for @ocrSelectDocument.
  ///
  /// In en, this message translates to:
  /// **'Select Document'**
  String get ocrSelectDocument;

  /// Checkbox label in OCR scanner sheet to enable LLM assistance
  ///
  /// In en, this message translates to:
  /// **'Use LLM to assist extraction'**
  String get ocrLlmAssist;

  /// No description provided for @ocrLlmAssistSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Improve field recognition accuracy'**
  String get ocrLlmAssistSubtitle;

  /// No description provided for @ocrNoModelAvailable.
  ///
  /// In en, this message translates to:
  /// **'No LLM model available'**
  String get ocrNoModelAvailable;

  /// No description provided for @ocrGoToConfig.
  ///
  /// In en, this message translates to:
  /// **'Go to Config'**
  String get ocrGoToConfig;

  /// No description provided for @ocrLlmConfig.
  ///
  /// In en, this message translates to:
  /// **'LLM Config'**
  String get ocrLlmConfig;

  /// No description provided for @ocrModelSelectorLabel.
  ///
  /// In en, this message translates to:
  /// **'Select Model'**
  String get ocrModelSelectorLabel;

  /// No description provided for @ocrPrivacyNotice.
  ///
  /// In en, this message translates to:
  /// **'All recognition is done locally on your device. Images are never uploaded to any server. Travel documents and ID cards will be automatically detected.'**
  String get ocrPrivacyNotice;

  /// No description provided for @ocrTip.
  ///
  /// In en, this message translates to:
  /// **'Tip: For best results, ensure the text is clearly visible and the image is well-lit.'**
  String get ocrTip;

  /// No description provided for @ocrRecognizing.
  ///
  /// In en, this message translates to:
  /// **'Recognizing text...'**
  String get ocrRecognizing;

  /// No description provided for @ocrRecognitionFailed.
  ///
  /// In en, this message translates to:
  /// **'Recognition Failed'**
  String get ocrRecognitionFailed;

  /// No description provided for @ocrTryAgain.
  ///
  /// In en, this message translates to:
  /// **'Try Again'**
  String get ocrTryAgain;

  /// No description provided for @ocrTravelDocumentDetected.
  ///
  /// In en, this message translates to:
  /// **'Travel document detected'**
  String get ocrTravelDocumentDetected;

  /// No description provided for @ocrRescan.
  ///
  /// In en, this message translates to:
  /// **'Rescan'**
  String get ocrRescan;

  /// No description provided for @scanGoToConfig.
  ///
  /// In en, this message translates to:
  /// **'Go to Config'**
  String get scanGoToConfig;

  /// No description provided for @scanAiMappingComplete.
  ///
  /// In en, this message translates to:
  /// **'AI mapping completed'**
  String get scanAiMappingComplete;

  /// No description provided for @scanAiMapping.
  ///
  /// In en, this message translates to:
  /// **'AI Smart Mapping'**
  String get scanAiMapping;

  /// No description provided for @llmStatsTotalFormatted.
  ///
  /// In en, this message translates to:
  /// **'Total: {total}'**
  String llmStatsTotalFormatted(String total);

  /// No description provided for @llmStatsModelSummary.
  ///
  /// In en, this message translates to:
  /// **'{count} models · Total {total} tokens'**
  String llmStatsModelSummary(int count, String total);

  /// No description provided for @llmStatsModelDetail.
  ///
  /// In en, this message translates to:
  /// **'{provider} · {total} tokens · {count} calls'**
  String llmStatsModelDetail(String provider, String total, int count);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
