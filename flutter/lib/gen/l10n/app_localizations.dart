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

  /// No description provided for @loginDataRecoveryTitle.
  ///
  /// In en, this message translates to:
  /// **'Data Recovery'**
  String get loginDataRecoveryTitle;

  /// No description provided for @loginDataRecoveryMessage.
  ///
  /// In en, this message translates to:
  /// **'Your vault appears to be empty, but a backup exists from {time}. Would you like to restore from this backup?'**
  String loginDataRecoveryMessage(String time);

  /// No description provided for @loginSkip.
  ///
  /// In en, this message translates to:
  /// **'Skip'**
  String get loginSkip;

  /// No description provided for @loginRestoreBackup.
  ///
  /// In en, this message translates to:
  /// **'Restore Backup'**
  String get loginRestoreBackup;

  /// No description provided for @loginRestoreSuccess.
  ///
  /// In en, this message translates to:
  /// **'Restore successful. Your data is now available.'**
  String get loginRestoreSuccess;

  /// No description provided for @loginRestoreFailed.
  ///
  /// In en, this message translates to:
  /// **'Restore failed'**
  String get loginRestoreFailed;

  /// No description provided for @loginBiometricGeneric.
  ///
  /// In en, this message translates to:
  /// **'Biometric'**
  String get loginBiometricGeneric;

  /// No description provided for @loginBiometricFaceId.
  ///
  /// In en, this message translates to:
  /// **'Face ID'**
  String get loginBiometricFaceId;

  /// No description provided for @loginBiometricTouchId.
  ///
  /// In en, this message translates to:
  /// **'Touch ID'**
  String get loginBiometricTouchId;

  /// No description provided for @loginBiometricIris.
  ///
  /// In en, this message translates to:
  /// **'Iris'**
  String get loginBiometricIris;

  /// No description provided for @loginUnlockReason.
  ///
  /// In en, this message translates to:
  /// **'Unlock SoloSoul with {biometricType}'**
  String loginUnlockReason(String biometricType);

  /// No description provided for @loginBiometricFailed.
  ///
  /// In en, this message translates to:
  /// **'Biometric authentication failed or was cancelled'**
  String get loginBiometricFailed;

  /// No description provided for @loginUnlockFailedUsePassword.
  ///
  /// In en, this message translates to:
  /// **'Failed to unlock vault. Please use your master password.'**
  String get loginUnlockFailedUsePassword;

  /// No description provided for @loginPasswordMinLength.
  ///
  /// In en, this message translates to:
  /// **'Password must be at least 8 characters'**
  String get loginPasswordMinLength;

  /// No description provided for @loginInvalidPassword.
  ///
  /// In en, this message translates to:
  /// **'Invalid master password'**
  String get loginInvalidPassword;

  /// No description provided for @loginUnlockFailed.
  ///
  /// In en, this message translates to:
  /// **'Unlock failed: {message}'**
  String loginUnlockFailed(String message);

  /// No description provided for @loginAccountNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Account name is required'**
  String get loginAccountNameRequired;

  /// No description provided for @loginPasswordsDoNotMatch.
  ///
  /// In en, this message translates to:
  /// **'Passwords do not match'**
  String get loginPasswordsDoNotMatch;

  /// No description provided for @loginCreateAccountFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to create account'**
  String get loginCreateAccountFailed;

  /// No description provided for @loginUnlockVaultFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to unlock vault. Please try again.'**
  String get loginUnlockVaultFailed;

  /// No description provided for @loginPasswordHint.
  ///
  /// In en, this message translates to:
  /// **'Password Hint: {hint}'**
  String loginPasswordHint(String hint);

  /// No description provided for @loginNever.
  ///
  /// In en, this message translates to:
  /// **'Never'**
  String get loginNever;

  /// No description provided for @loginToday.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get loginToday;

  /// No description provided for @loginYesterday.
  ///
  /// In en, this message translates to:
  /// **'Yesterday'**
  String get loginYesterday;

  /// No description provided for @loginDaysAgo.
  ///
  /// In en, this message translates to:
  /// **'{count} days ago'**
  String loginDaysAgo(int count);

  /// No description provided for @loginBackToAccountList.
  ///
  /// In en, this message translates to:
  /// **'Back to Account List'**
  String get loginBackToAccountList;

  /// No description provided for @loginAccountName.
  ///
  /// In en, this message translates to:
  /// **'Account Name'**
  String get loginAccountName;

  /// No description provided for @loginAccountNameHint.
  ///
  /// In en, this message translates to:
  /// **'e.g., Personal, Work'**
  String get loginAccountNameHint;

  /// No description provided for @loginMasterPassword.
  ///
  /// In en, this message translates to:
  /// **'Master Password'**
  String get loginMasterPassword;

  /// No description provided for @loginEnterPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter your password'**
  String get loginEnterPassword;

  /// No description provided for @loginCreateStrongPassword.
  ///
  /// In en, this message translates to:
  /// **'Create a strong password'**
  String get loginCreateStrongPassword;

  /// No description provided for @loginConfirmPassword.
  ///
  /// In en, this message translates to:
  /// **'Confirm Password'**
  String get loginConfirmPassword;

  /// No description provided for @loginReenterPassword.
  ///
  /// In en, this message translates to:
  /// **'Re-enter your password'**
  String get loginReenterPassword;

  /// No description provided for @loginPasswordHintOptional.
  ///
  /// In en, this message translates to:
  /// **'Password Hint (Optional)'**
  String get loginPasswordHintOptional;

  /// No description provided for @loginPasswordHintHelp.
  ///
  /// In en, this message translates to:
  /// **'A hint to help you remember'**
  String get loginPasswordHintHelp;

  /// No description provided for @loginShowPasswordHint.
  ///
  /// In en, this message translates to:
  /// **'Show password hint'**
  String get loginShowPasswordHint;

  /// No description provided for @loginNoAccounts.
  ///
  /// In en, this message translates to:
  /// **'No accounts found'**
  String get loginNoAccounts;

  /// No description provided for @loginCreateAccount.
  ///
  /// In en, this message translates to:
  /// **'Create Account'**
  String get loginCreateAccount;

  /// No description provided for @loginLastAccessed.
  ///
  /// In en, this message translates to:
  /// **'Last accessed: {time}'**
  String loginLastAccessed(String time);

  /// No description provided for @loginAccountListEmpty.
  ///
  /// In en, this message translates to:
  /// **'Account list empty'**
  String get loginAccountListEmpty;

  /// No description provided for @loginCreateFirstAccount.
  ///
  /// In en, this message translates to:
  /// **'Create your first account to get started'**
  String get loginCreateFirstAccount;

  /// No description provided for @loginSelectAccountToUnlock.
  ///
  /// In en, this message translates to:
  /// **'Select an account to unlock'**
  String get loginSelectAccountToUnlock;

  /// No description provided for @loginShowLess.
  ///
  /// In en, this message translates to:
  /// **'Show less'**
  String get loginShowLess;

  /// No description provided for @loginShowAllAccounts.
  ///
  /// In en, this message translates to:
  /// **'Show all {count} accounts'**
  String loginShowAllAccounts(int count);

  /// No description provided for @loginNoAccountsYet.
  ///
  /// In en, this message translates to:
  /// **'No accounts yet'**
  String get loginNoAccountsYet;

  /// No description provided for @loginRecent.
  ///
  /// In en, this message translates to:
  /// **'Recent'**
  String get loginRecent;

  /// No description provided for @workspaceObjects.
  ///
  /// In en, this message translates to:
  /// **'Objects'**
  String get workspaceObjects;

  /// No description provided for @workspaceNoItems.
  ///
  /// In en, this message translates to:
  /// **'No items yet'**
  String get workspaceNoItems;

  /// No description provided for @workspaceNoObjects.
  ///
  /// In en, this message translates to:
  /// **'No objects yet'**
  String get workspaceNoObjects;

  /// No description provided for @workspaceAddFirstItem.
  ///
  /// In en, this message translates to:
  /// **'Add your first item'**
  String get workspaceAddFirstItem;

  /// No description provided for @workspaceCreateFirstObject.
  ///
  /// In en, this message translates to:
  /// **'Create your first object to get started'**
  String get workspaceCreateFirstObject;

  /// No description provided for @workspaceDeletePage.
  ///
  /// In en, this message translates to:
  /// **'Delete Page'**
  String get workspaceDeletePage;

  /// No description provided for @workspaceDeleteSection.
  ///
  /// In en, this message translates to:
  /// **'Delete Section'**
  String get workspaceDeleteSection;

  /// No description provided for @workspaceDeleteSectionConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete \"{name}\"?'**
  String workspaceDeleteSectionConfirm(String name);

  /// No description provided for @workspaceDeletePageConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete \"{name}\"? All {count} item(s) inside this page will also be moved to trash.'**
  String workspaceDeletePageConfirm(String name, int count);

  /// No description provided for @workspaceSectionDeleted.
  ///
  /// In en, this message translates to:
  /// **'Section deleted'**
  String get workspaceSectionDeleted;

  /// No description provided for @workspaceMovedToTrash.
  ///
  /// In en, this message translates to:
  /// **'\"{name}\" moved to trash'**
  String workspaceMovedToTrash(String name);

  /// No description provided for @workspaceAddSubPage.
  ///
  /// In en, this message translates to:
  /// **'Add Sub-Page'**
  String get workspaceAddSubPage;

  /// No description provided for @workspaceAddSection.
  ///
  /// In en, this message translates to:
  /// **'Add Section'**
  String get workspaceAddSection;

  /// No description provided for @workspaceAddSectionDialog.
  ///
  /// In en, this message translates to:
  /// **'Add Section'**
  String get workspaceAddSectionDialog;

  /// No description provided for @workspaceSectionName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get workspaceSectionName;

  /// No description provided for @workspaceEnterSectionName.
  ///
  /// In en, this message translates to:
  /// **'Enter section name'**
  String get workspaceEnterSectionName;

  /// No description provided for @workspaceIcon.
  ///
  /// In en, this message translates to:
  /// **'Icon'**
  String get workspaceIcon;

  /// No description provided for @objectEditorEditSection.
  ///
  /// In en, this message translates to:
  /// **'Edit Section'**
  String get objectEditorEditSection;

  /// No description provided for @objectEditorNewSection.
  ///
  /// In en, this message translates to:
  /// **'New Section'**
  String get objectEditorNewSection;

  /// No description provided for @objectEditorType.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get objectEditorType;

  /// No description provided for @objectEditorNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Name is required'**
  String get objectEditorNameRequired;

  /// No description provided for @objectEditorDuplicateProperties.
  ///
  /// In en, this message translates to:
  /// **'Duplicate property names: {names}'**
  String objectEditorDuplicateProperties(String names);

  /// No description provided for @objectEditorSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to save: {message}'**
  String objectEditorSaveFailed(String message);

  /// No description provided for @objectEditorIcon.
  ///
  /// In en, this message translates to:
  /// **'Icon'**
  String get objectEditorIcon;

  /// No description provided for @objectEditorName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get objectEditorName;

  /// No description provided for @objectEditorEnterSectionName.
  ///
  /// In en, this message translates to:
  /// **'Enter section name'**
  String get objectEditorEnterSectionName;

  /// No description provided for @objectEditorSelectType.
  ///
  /// In en, this message translates to:
  /// **'Select type'**
  String get objectEditorSelectType;

  /// No description provided for @objectEditorNoParent.
  ///
  /// In en, this message translates to:
  /// **'No parent (root)'**
  String get objectEditorNoParent;

  /// No description provided for @objectEditorItemProperties.
  ///
  /// In en, this message translates to:
  /// **'Item Properties'**
  String get objectEditorItemProperties;

  /// No description provided for @objectEditorAddProperty.
  ///
  /// In en, this message translates to:
  /// **'Add Property'**
  String get objectEditorAddProperty;

  /// No description provided for @objectEditorKeyName.
  ///
  /// In en, this message translates to:
  /// **'Key name'**
  String get objectEditorKeyName;

  /// No description provided for @objectEditorPropertyTypeText.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get objectEditorPropertyTypeText;

  /// No description provided for @objectEditorPropertyTypeDate.
  ///
  /// In en, this message translates to:
  /// **'Date'**
  String get objectEditorPropertyTypeDate;

  /// No description provided for @objectEditorPropertyTypeNumber.
  ///
  /// In en, this message translates to:
  /// **'Number'**
  String get objectEditorPropertyTypeNumber;

  /// No description provided for @objectEditorPropertyTypeCheckbox.
  ///
  /// In en, this message translates to:
  /// **'Checkbox'**
  String get objectEditorPropertyTypeCheckbox;

  /// No description provided for @objectEditorSensitivity.
  ///
  /// In en, this message translates to:
  /// **'Sensitivity'**
  String get objectEditorSensitivity;

  /// No description provided for @objectEditorDeletePropertyTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete Property'**
  String get objectEditorDeletePropertyTitle;

  /// No description provided for @pageEditorNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Name is required'**
  String get pageEditorNameRequired;

  /// No description provided for @pageEditorEditPage.
  ///
  /// In en, this message translates to:
  /// **'Edit Page'**
  String get pageEditorEditPage;

  /// No description provided for @pageEditorNewPage.
  ///
  /// In en, this message translates to:
  /// **'New Page'**
  String get pageEditorNewPage;

  /// No description provided for @pageEditorName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get pageEditorName;

  /// No description provided for @pageEditorEnterPageName.
  ///
  /// In en, this message translates to:
  /// **'Enter page name'**
  String get pageEditorEnterPageName;

  /// No description provided for @pageEditorIcon.
  ///
  /// In en, this message translates to:
  /// **'Icon'**
  String get pageEditorIcon;

  /// No description provided for @pageEditorParent.
  ///
  /// In en, this message translates to:
  /// **'Parent'**
  String get pageEditorParent;

  /// No description provided for @homeScan.
  ///
  /// In en, this message translates to:
  /// **'Scan'**
  String get homeScan;

  /// No description provided for @homeQuickActions.
  ///
  /// In en, this message translates to:
  /// **'Quick Actions'**
  String get homeQuickActions;

  /// No description provided for @homeEditQuickActions.
  ///
  /// In en, this message translates to:
  /// **'Edit quick actions'**
  String get homeEditQuickActions;

  /// No description provided for @homeEditQuickActionsDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get homeEditQuickActionsDone;

  /// No description provided for @homeSecurityStatus.
  ///
  /// In en, this message translates to:
  /// **'Security Status'**
  String get homeSecurityStatus;

  /// No description provided for @searchTitle.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get searchTitle;

  /// No description provided for @searchHint.
  ///
  /// In en, this message translates to:
  /// **'Search fields...'**
  String get searchHint;

  /// No description provided for @profileType.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get profileType;

  /// No description provided for @profileTypeEmail.
  ///
  /// In en, this message translates to:
  /// **'email'**
  String get profileTypeEmail;

  /// No description provided for @profileTypePhone.
  ///
  /// In en, this message translates to:
  /// **'phone'**
  String get profileTypePhone;

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @settingsDebugModeEnabled.
  ///
  /// In en, this message translates to:
  /// **'Debug mode enabled'**
  String get settingsDebugModeEnabled;

  /// No description provided for @settingsInvalidPassword.
  ///
  /// In en, this message translates to:
  /// **'Invalid password'**
  String get settingsInvalidPassword;

  /// No description provided for @settingsPasswordChangedSuccess.
  ///
  /// In en, this message translates to:
  /// **'Master password changed successfully'**
  String get settingsPasswordChangedSuccess;

  /// No description provided for @settingsOk.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get settingsOk;

  /// No description provided for @settingsEnableDebugMode.
  ///
  /// In en, this message translates to:
  /// **'Enable Debug Mode'**
  String get settingsEnableDebugMode;

  /// No description provided for @settingsEnableDebugModeDesc.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to enable Debug Log.'**
  String get settingsEnableDebugModeDesc;

  /// No description provided for @settingsUseBiometric.
  ///
  /// In en, this message translates to:
  /// **'Use {biometricType}'**
  String settingsUseBiometric(String biometricType);

  /// No description provided for @settingsOr.
  ///
  /// In en, this message translates to:
  /// **'or'**
  String get settingsOr;

  /// No description provided for @settingsMasterPassword.
  ///
  /// In en, this message translates to:
  /// **'Master Password'**
  String get settingsMasterPassword;

  /// No description provided for @settingsShowPasswordHint.
  ///
  /// In en, this message translates to:
  /// **'Show password hint'**
  String get settingsShowPasswordHint;

  /// No description provided for @settingsEnable.
  ///
  /// In en, this message translates to:
  /// **'Enable'**
  String get settingsEnable;

  /// No description provided for @securitySettingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Security Settings'**
  String get securitySettingsTitle;

  /// No description provided for @securitySettingsBiometricFailed.
  ///
  /// In en, this message translates to:
  /// **'Biometric authentication failed or was cancelled'**
  String get securitySettingsBiometricFailed;

  /// No description provided for @securitySettingsBiometricEnabled.
  ///
  /// In en, this message translates to:
  /// **'Biometric unlock enabled'**
  String get securitySettingsBiometricEnabled;

  /// No description provided for @securitySettingsResetToDefaults.
  ///
  /// In en, this message translates to:
  /// **'Reset to Defaults'**
  String get securitySettingsResetToDefaults;

  /// No description provided for @securitySettingsResetTitle.
  ///
  /// In en, this message translates to:
  /// **'Reset Security Settings'**
  String get securitySettingsResetTitle;

  /// No description provided for @securitySettingsResetConfirm.
  ///
  /// In en, this message translates to:
  /// **'This will reset all security settings to their default values. Are you sure?'**
  String get securitySettingsResetConfirm;

  /// No description provided for @securitySettingsReset.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get securitySettingsReset;

  /// No description provided for @securitySettingsNotImplemented.
  ///
  /// In en, this message translates to:
  /// **'Feature not yet implemented'**
  String get securitySettingsNotImplemented;

  /// No description provided for @sensitivitySettingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Sensitivity Settings'**
  String get sensitivitySettingsTitle;

  /// No description provided for @sensitivitySettingsVerify.
  ///
  /// In en, this message translates to:
  /// **'Verify'**
  String get sensitivitySettingsVerify;

  /// No description provided for @sensitivitySettingsConfirmDowngrade.
  ///
  /// In en, this message translates to:
  /// **'Confirm Downgrade'**
  String get sensitivitySettingsConfirmDowngrade;

  /// No description provided for @sensitivitySettingsChangeLevel.
  ///
  /// In en, this message translates to:
  /// **'Change sensitivity level'**
  String get sensitivitySettingsChangeLevel;

  /// No description provided for @sensitivitySettingsSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search fields...'**
  String get sensitivitySettingsSearchHint;

  /// No description provided for @sensitivitySettingsClearSearch.
  ///
  /// In en, this message translates to:
  /// **'Clear search'**
  String get sensitivitySettingsClearSearch;

  /// No description provided for @trashTitle.
  ///
  /// In en, this message translates to:
  /// **'Trash'**
  String get trashTitle;

  /// No description provided for @trashVerify.
  ///
  /// In en, this message translates to:
  /// **'Verify'**
  String get trashVerify;

  /// No description provided for @trashEmptyTrash.
  ///
  /// In en, this message translates to:
  /// **'Empty Trash'**
  String get trashEmptyTrash;

  /// No description provided for @trashConfirmRestore.
  ///
  /// In en, this message translates to:
  /// **'Confirm Restore'**
  String get trashConfirmRestore;

  /// No description provided for @trashRestoreConfirm.
  ///
  /// In en, this message translates to:
  /// **'Restore \"{name}\"?'**
  String trashRestoreConfirm(String name);

  /// No description provided for @trashConfirmPermanentDelete.
  ///
  /// In en, this message translates to:
  /// **'Confirm Permanent Delete'**
  String get trashConfirmPermanentDelete;

  /// No description provided for @trashSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search trash...'**
  String get trashSearchHint;

  /// No description provided for @syncTitle.
  ///
  /// In en, this message translates to:
  /// **'Device Sync'**
  String get syncTitle;

  /// No description provided for @syncNoActiveAccount.
  ///
  /// In en, this message translates to:
  /// **'No active account for sync'**
  String get syncNoActiveAccount;

  /// No description provided for @syncEnterAddressAndKey.
  ///
  /// In en, this message translates to:
  /// **'Enter address and pairing key'**
  String get syncEnterAddressAndKey;

  /// No description provided for @syncInvalidPairingKey.
  ///
  /// In en, this message translates to:
  /// **'Invalid pairing key hex'**
  String get syncInvalidPairingKey;

  /// No description provided for @syncPairingKeyCopied.
  ///
  /// In en, this message translates to:
  /// **'Pairing key copied to clipboard'**
  String get syncPairingKeyCopied;

  /// No description provided for @syncRemoteAddress.
  ///
  /// In en, this message translates to:
  /// **'Remote Address'**
  String get syncRemoteAddress;

  /// No description provided for @syncRemoteAddressHint.
  ///
  /// In en, this message translates to:
  /// **'192.168.1.5:9900'**
  String get syncRemoteAddressHint;

  /// No description provided for @syncPairingKey.
  ///
  /// In en, this message translates to:
  /// **'Pairing Key (hex)'**
  String get syncPairingKey;

  /// No description provided for @syncPairingKeyHint.
  ///
  /// In en, this message translates to:
  /// **'Enter shared pairing key'**
  String get syncPairingKeyHint;

  /// No description provided for @syncGenerateAndCopyKey.
  ///
  /// In en, this message translates to:
  /// **'Generate & Copy Key'**
  String get syncGenerateAndCopyKey;

  /// No description provided for @syncWithDevice.
  ///
  /// In en, this message translates to:
  /// **'Sync with {name}'**
  String syncWithDevice(String name);

  /// No description provided for @syncButton.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get syncButton;

  /// No description provided for @dataManagementTitle.
  ///
  /// In en, this message translates to:
  /// **'Data Management'**
  String get dataManagementTitle;

  /// No description provided for @dataManagementBackupNow.
  ///
  /// In en, this message translates to:
  /// **'Backup Now'**
  String get dataManagementBackupNow;

  /// No description provided for @dataManagementSpecialBackupLimit.
  ///
  /// In en, this message translates to:
  /// **'Special Backup Limit Reached'**
  String get dataManagementSpecialBackupLimit;

  /// No description provided for @dataManagementNameBackup.
  ///
  /// In en, this message translates to:
  /// **'Name Special Backup'**
  String get dataManagementNameBackup;

  /// No description provided for @dataManagementBackupNameHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. Before Major Update'**
  String get dataManagementBackupNameHint;

  /// No description provided for @dataManagementBackupNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Backup name'**
  String get dataManagementBackupNameLabel;

  /// No description provided for @dataManagementCreate.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get dataManagementCreate;

  /// No description provided for @dataManagementRenameBackup.
  ///
  /// In en, this message translates to:
  /// **'Rename Special Backup'**
  String get dataManagementRenameBackup;

  /// No description provided for @dataManagementNewName.
  ///
  /// In en, this message translates to:
  /// **'New name'**
  String get dataManagementNewName;

  /// No description provided for @dataManagementRename.
  ///
  /// In en, this message translates to:
  /// **'Rename'**
  String get dataManagementRename;

  /// No description provided for @dataManagementRestoreBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore Special Backup?'**
  String get dataManagementRestoreBackupTitle;

  /// No description provided for @dataManagementRestoreBackupConfirm.
  ///
  /// In en, this message translates to:
  /// **'Restore special backup \"{name}\"?'**
  String dataManagementRestoreBackupConfirm(String name);

  /// No description provided for @dataManagementDeleteBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete Special Backup?'**
  String get dataManagementDeleteBackupTitle;

  /// No description provided for @dataManagementDeleteBackupConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete special backup \"{name}\"?'**
  String dataManagementDeleteBackupConfirm(String name);

  /// No description provided for @operationLogTitle.
  ///
  /// In en, this message translates to:
  /// **'Operation Log'**
  String get operationLogTitle;

  /// No description provided for @operationLogVerify.
  ///
  /// In en, this message translates to:
  /// **'Verify'**
  String get operationLogVerify;

  /// No description provided for @operationLogClearLogTitle.
  ///
  /// In en, this message translates to:
  /// **'Clear Log'**
  String get operationLogClearLogTitle;

  /// No description provided for @operationLogClear.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get operationLogClear;

  /// No description provided for @operationLogClearLog.
  ///
  /// In en, this message translates to:
  /// **'Clear log'**
  String get operationLogClearLog;

  /// No description provided for @operationLogSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search logs...'**
  String get operationLogSearchHint;

  /// No description provided for @objectEditorDeletePropertyConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete \"{name}\"?'**
  String objectEditorDeletePropertyConfirm(String name);

  /// No description provided for @workspaceAddSectionButton.
  ///
  /// In en, this message translates to:
  /// **'Add Section'**
  String get workspaceAddSectionButton;

  /// No description provided for @workspaceEditPage.
  ///
  /// In en, this message translates to:
  /// **'Edit Page'**
  String get workspaceEditPage;

  /// No description provided for @workspaceDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get workspaceDone;

  /// No description provided for @workspaceReorder.
  ///
  /// In en, this message translates to:
  /// **'Reorder'**
  String get workspaceReorder;

  /// No description provided for @workspaceAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get workspaceAdd;

  /// No description provided for @loginCreateNewAccount.
  ///
  /// In en, this message translates to:
  /// **'Create New Account'**
  String get loginCreateNewAccount;

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

  /// No description provided for @llmChatStatusLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading'**
  String get llmChatStatusLoading;

  /// No description provided for @llmChatStatusError.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get llmChatStatusError;

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

  /// No description provided for @ocrFieldName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get ocrFieldName;

  /// No description provided for @ocrFieldPhone.
  ///
  /// In en, this message translates to:
  /// **'Phone'**
  String get ocrFieldPhone;

  /// No description provided for @ocrFieldEmail.
  ///
  /// In en, this message translates to:
  /// **'Email'**
  String get ocrFieldEmail;

  /// No description provided for @ocrFieldAddress.
  ///
  /// In en, this message translates to:
  /// **'Address'**
  String get ocrFieldAddress;

  /// No description provided for @ocrFieldCompany.
  ///
  /// In en, this message translates to:
  /// **'Company/Organization'**
  String get ocrFieldCompany;

  /// No description provided for @ocrFieldTitle.
  ///
  /// In en, this message translates to:
  /// **'Title/Position'**
  String get ocrFieldTitle;

  /// No description provided for @ocrFieldDate.
  ///
  /// In en, this message translates to:
  /// **'Date'**
  String get ocrFieldDate;

  /// No description provided for @ocrFieldAmount.
  ///
  /// In en, this message translates to:
  /// **'Amount'**
  String get ocrFieldAmount;

  /// No description provided for @ocrFieldInvoiceNumber.
  ///
  /// In en, this message translates to:
  /// **'Invoice/Document Number'**
  String get ocrFieldInvoiceNumber;

  /// No description provided for @ocrFieldWebsite.
  ///
  /// In en, this message translates to:
  /// **'Website/URL'**
  String get ocrFieldWebsite;

  /// No description provided for @ocrFieldIdNumber.
  ///
  /// In en, this message translates to:
  /// **'ID Number'**
  String get ocrFieldIdNumber;

  /// No description provided for @llmChatEmptyResponse.
  ///
  /// In en, this message translates to:
  /// **'The model returned no content. Please check the configuration or try again.'**
  String get llmChatEmptyResponse;

  /// No description provided for @llmChatInferenceFailed.
  ///
  /// In en, this message translates to:
  /// **'Inference failed: {error}'**
  String llmChatInferenceFailed(String error);

  /// No description provided for @sidebarHome.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get sidebarHome;

  /// No description provided for @sidebarSearch.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get sidebarSearch;

  /// No description provided for @sidebarLocalImport.
  ///
  /// In en, this message translates to:
  /// **'Local Import'**
  String get sidebarLocalImport;

  /// No description provided for @sidebarProfile.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get sidebarProfile;

  /// No description provided for @sidebarTravel.
  ///
  /// In en, this message translates to:
  /// **'Travel'**
  String get sidebarTravel;

  /// No description provided for @sidebarFinancial.
  ///
  /// In en, this message translates to:
  /// **'Financial'**
  String get sidebarFinancial;

  /// No description provided for @sidebarProfessional.
  ///
  /// In en, this message translates to:
  /// **'Professional'**
  String get sidebarProfessional;

  /// No description provided for @sidebarAddPage.
  ///
  /// In en, this message translates to:
  /// **'Add Page'**
  String get sidebarAddPage;

  /// No description provided for @sidebarLockVault.
  ///
  /// In en, this message translates to:
  /// **'Lock Vault'**
  String get sidebarLockVault;

  /// No description provided for @sidebarTrash.
  ///
  /// In en, this message translates to:
  /// **'Trash'**
  String get sidebarTrash;

  /// No description provided for @sidebarSync.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get sidebarSync;

  /// No description provided for @sidebarSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get sidebarSettings;

  /// No description provided for @sidebarCollapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get sidebarCollapse;

  /// No description provided for @sidebarExpand.
  ///
  /// In en, this message translates to:
  /// **'Expand'**
  String get sidebarExpand;

  /// No description provided for @sidebarPages.
  ///
  /// In en, this message translates to:
  /// **'PAGES'**
  String get sidebarPages;

  /// No description provided for @sidebarDropToMakeRootPage.
  ///
  /// In en, this message translates to:
  /// **'Drop to make root page'**
  String get sidebarDropToMakeRootPage;

  /// No description provided for @commonBack.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get commonBack;

  /// No description provided for @localSearchTitle.
  ///
  /// In en, this message translates to:
  /// **'Local Search Import'**
  String get localSearchTitle;

  /// No description provided for @localSearchPaths.
  ///
  /// In en, this message translates to:
  /// **'Search Paths'**
  String get localSearchPaths;

  /// No description provided for @localSearchFileTypes.
  ///
  /// In en, this message translates to:
  /// **'File Types'**
  String get localSearchFileTypes;

  /// No description provided for @localSearchScanDepth.
  ///
  /// In en, this message translates to:
  /// **'Scan Depth'**
  String get localSearchScanDepth;

  /// No description provided for @localSearchFilenameOnly.
  ///
  /// In en, this message translates to:
  /// **'Filename only'**
  String get localSearchFilenameOnly;

  /// No description provided for @localSearchFilenameOnlyDesc.
  ///
  /// In en, this message translates to:
  /// **'Fastest — only check filenames'**
  String get localSearchFilenameOnlyDesc;

  /// No description provided for @localSearchFingerprint.
  ///
  /// In en, this message translates to:
  /// **'Filename + Content fingerprint'**
  String get localSearchFingerprint;

  /// No description provided for @localSearchFingerprintDesc.
  ///
  /// In en, this message translates to:
  /// **'Balanced — regex match on content'**
  String get localSearchFingerprintDesc;

  /// No description provided for @localSearchFullText.
  ///
  /// In en, this message translates to:
  /// **'Full text parsing'**
  String get localSearchFullText;

  /// No description provided for @localSearchFullTextDesc.
  ///
  /// In en, this message translates to:
  /// **'Slowest — deep content analysis'**
  String get localSearchFullTextDesc;

  /// No description provided for @localSearchDefaultPaths.
  ///
  /// In en, this message translates to:
  /// **'Use default paths'**
  String get localSearchDefaultPaths;

  /// No description provided for @localSearchDefaultPathsDesc.
  ///
  /// In en, this message translates to:
  /// **'Documents, Desktop, Downloads'**
  String get localSearchDefaultPathsDesc;

  /// No description provided for @localSearchCustomPaths.
  ///
  /// In en, this message translates to:
  /// **'Custom paths'**
  String get localSearchCustomPaths;

  /// No description provided for @localSearchCustomPathsDesc.
  ///
  /// In en, this message translates to:
  /// **'Select specific folders'**
  String get localSearchCustomPathsDesc;

  /// No description provided for @localSearchAddFolder.
  ///
  /// In en, this message translates to:
  /// **'Add folder'**
  String get localSearchAddFolder;

  /// No description provided for @localSearchStartScan.
  ///
  /// In en, this message translates to:
  /// **'Start Scan'**
  String get localSearchStartScan;

  /// No description provided for @localSearchScanning.
  ///
  /// In en, this message translates to:
  /// **'Scanning...'**
  String get localSearchScanning;

  /// No description provided for @localSearchScanned.
  ///
  /// In en, this message translates to:
  /// **'Scanned'**
  String get localSearchScanned;

  /// No description provided for @localSearchFound.
  ///
  /// In en, this message translates to:
  /// **'Found'**
  String get localSearchFound;

  /// No description provided for @localSearchSkipped.
  ///
  /// In en, this message translates to:
  /// **'Skipped'**
  String get localSearchSkipped;

  /// No description provided for @localSearchCancelScan.
  ///
  /// In en, this message translates to:
  /// **'Cancel Scan'**
  String get localSearchCancelScan;

  /// No description provided for @localSearchGoBack.
  ///
  /// In en, this message translates to:
  /// **'Go Back'**
  String get localSearchGoBack;

  /// No description provided for @localSearchScanAgain.
  ///
  /// In en, this message translates to:
  /// **'Scan Again'**
  String get localSearchScanAgain;

  /// No description provided for @localSearchNoResults.
  ///
  /// In en, this message translates to:
  /// **'No Results Found'**
  String get localSearchNoResults;

  /// No description provided for @scanImportComplete.
  ///
  /// In en, this message translates to:
  /// **'Import Complete'**
  String get scanImportComplete;

  /// No description provided for @scanImportGoHome.
  ///
  /// In en, this message translates to:
  /// **'Go Home'**
  String get scanImportGoHome;

  /// No description provided for @scanImportCreated.
  ///
  /// In en, this message translates to:
  /// **'Created'**
  String get scanImportCreated;

  /// No description provided for @scanImportUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated'**
  String get scanImportUpdated;

  /// No description provided for @scanImportFields.
  ///
  /// In en, this message translates to:
  /// **'Fields'**
  String get scanImportFields;

  /// No description provided for @scanImportSkipped.
  ///
  /// In en, this message translates to:
  /// **'Skipped'**
  String get scanImportSkipped;

  /// No description provided for @scanPreviewTitle.
  ///
  /// In en, this message translates to:
  /// **'Preview & Confirm'**
  String get scanPreviewTitle;

  /// No description provided for @scanPreviewNew.
  ///
  /// In en, this message translates to:
  /// **'New'**
  String get scanPreviewNew;

  /// No description provided for @scanPreviewUpdate.
  ///
  /// In en, this message translates to:
  /// **'Update'**
  String get scanPreviewUpdate;

  /// No description provided for @scanPreviewImportAction.
  ///
  /// In en, this message translates to:
  /// **'Import action'**
  String get scanPreviewImportAction;
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
