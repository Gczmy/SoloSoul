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

  /// No description provided for @commonUndo.
  ///
  /// In en, this message translates to:
  /// **'Undo'**
  String get commonUndo;

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

  /// No description provided for @objectEditorPropertyTypeSelect.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get objectEditorPropertyTypeSelect;

  /// No description provided for @objectEditorPropertyTypeMultiSelect.
  ///
  /// In en, this message translates to:
  /// **'Multi-Select'**
  String get objectEditorPropertyTypeMultiSelect;

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
  /// **'Pairing Key'**
  String get syncPairingKey;

  /// No description provided for @syncPairingKeyHint.
  ///
  /// In en, this message translates to:
  /// **'Generate a shared pairing key to establish a secure connection between devices. Both devices must use the same key.'**
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

  /// No description provided for @sidebarSecurity.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get sidebarSecurity;

  /// No description provided for @sidebarOperationLog.
  ///
  /// In en, this message translates to:
  /// **'Operation Log'**
  String get sidebarOperationLog;

  /// No description provided for @sidebarSensitivity.
  ///
  /// In en, this message translates to:
  /// **'Sensitivity'**
  String get sidebarSensitivity;

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

  /// No description provided for @homeTitle.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get homeTitle;

  /// No description provided for @homeEndToEndEncrypted.
  ///
  /// In en, this message translates to:
  /// **'End-to-End Encrypted'**
  String get homeEndToEndEncrypted;

  /// No description provided for @homeEncryptionDesc.
  ///
  /// In en, this message translates to:
  /// **'AES-256-GCM + Argon2id'**
  String get homeEncryptionDesc;

  /// No description provided for @homeLocalStorage.
  ///
  /// In en, this message translates to:
  /// **'Local Storage'**
  String get homeLocalStorage;

  /// No description provided for @homeLocalStorageDesc.
  ///
  /// In en, this message translates to:
  /// **'Data encrypted and stored locally'**
  String get homeLocalStorageDesc;

  /// No description provided for @homeZeroKnowledge.
  ///
  /// In en, this message translates to:
  /// **'Zero Knowledge'**
  String get homeZeroKnowledge;

  /// No description provided for @homeZeroKnowledgeDesc.
  ///
  /// In en, this message translates to:
  /// **'Master password never stored'**
  String get homeZeroKnowledgeDesc;

  /// No description provided for @profileTitle.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profileTitle;

  /// No description provided for @profileIdentity.
  ///
  /// In en, this message translates to:
  /// **'Identity'**
  String get profileIdentity;

  /// No description provided for @profileContactInfo.
  ///
  /// In en, this message translates to:
  /// **'Contact Information'**
  String get profileContactInfo;

  /// No description provided for @profileIdentityDocuments.
  ///
  /// In en, this message translates to:
  /// **'Identity Documents'**
  String get profileIdentityDocuments;

  /// No description provided for @profileAddresses.
  ///
  /// In en, this message translates to:
  /// **'Addresses'**
  String get profileAddresses;

  /// No description provided for @profileTitleLabel.
  ///
  /// In en, this message translates to:
  /// **'Title'**
  String get profileTitleLabel;

  /// No description provided for @profileTypeLabel.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get profileTypeLabel;

  /// No description provided for @profileValueLabel.
  ///
  /// In en, this message translates to:
  /// **'Value'**
  String get profileValueLabel;

  /// No description provided for @travelTitle.
  ///
  /// In en, this message translates to:
  /// **'Travel'**
  String get travelTitle;

  /// No description provided for @travelPassports.
  ///
  /// In en, this message translates to:
  /// **'Passports'**
  String get travelPassports;

  /// No description provided for @travelVisas.
  ///
  /// In en, this message translates to:
  /// **'Visas'**
  String get travelVisas;

  /// No description provided for @travelHistory.
  ///
  /// In en, this message translates to:
  /// **'Travel History'**
  String get travelHistory;

  /// No description provided for @financialTitle.
  ///
  /// In en, this message translates to:
  /// **'Financial'**
  String get financialTitle;

  /// No description provided for @financialBankAccounts.
  ///
  /// In en, this message translates to:
  /// **'Bank Accounts'**
  String get financialBankAccounts;

  /// No description provided for @financialCards.
  ///
  /// In en, this message translates to:
  /// **'Cards'**
  String get financialCards;

  /// No description provided for @financialTaxIdentification.
  ///
  /// In en, this message translates to:
  /// **'Tax Identification'**
  String get financialTaxIdentification;

  /// No description provided for @professionalTitle.
  ///
  /// In en, this message translates to:
  /// **'Professional'**
  String get professionalTitle;

  /// No description provided for @professionalEducation.
  ///
  /// In en, this message translates to:
  /// **'Education'**
  String get professionalEducation;

  /// No description provided for @professionalEmployment.
  ///
  /// In en, this message translates to:
  /// **'Employment'**
  String get professionalEmployment;

  /// No description provided for @professionalAwards.
  ///
  /// In en, this message translates to:
  /// **'Awards'**
  String get professionalAwards;

  /// No description provided for @professionalSkills.
  ///
  /// In en, this message translates to:
  /// **'Skills'**
  String get professionalSkills;

  /// No description provided for @professionalLanguages.
  ///
  /// In en, this message translates to:
  /// **'Languages'**
  String get professionalLanguages;

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

  /// No description provided for @settingsAccount.
  ///
  /// In en, this message translates to:
  /// **'Account'**
  String get settingsAccount;

  /// No description provided for @settingsCurrentAccount.
  ///
  /// In en, this message translates to:
  /// **'Current Account'**
  String get settingsCurrentAccount;

  /// No description provided for @settingsAllAccounts.
  ///
  /// In en, this message translates to:
  /// **'All Accounts'**
  String get settingsAllAccounts;

  /// No description provided for @settingsDataManagement.
  ///
  /// In en, this message translates to:
  /// **'Data Management'**
  String get settingsDataManagement;

  /// No description provided for @settingsErrorLoadingAccounts.
  ///
  /// In en, this message translates to:
  /// **'Error loading accounts'**
  String get settingsErrorLoadingAccounts;

  /// No description provided for @settingsPleaseRestart.
  ///
  /// In en, this message translates to:
  /// **'Please restart the app'**
  String get settingsPleaseRestart;

  /// No description provided for @settingsAccess.
  ///
  /// In en, this message translates to:
  /// **'Access'**
  String get settingsAccess;

  /// No description provided for @settingsLockVault.
  ///
  /// In en, this message translates to:
  /// **'Lock Vault'**
  String get settingsLockVault;

  /// No description provided for @settingsLockVaultDesc.
  ///
  /// In en, this message translates to:
  /// **'Lock now and require password'**
  String get settingsLockVaultDesc;

  /// No description provided for @settingsChangePassword.
  ///
  /// In en, this message translates to:
  /// **'Change Master Password'**
  String get settingsChangePassword;

  /// No description provided for @settingsChangePasswordDesc.
  ///
  /// In en, this message translates to:
  /// **'Update your vault password'**
  String get settingsChangePasswordDesc;

  /// No description provided for @settingsSecurity.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get settingsSecurity;

  /// No description provided for @settingsAutoLockPrivacy.
  ///
  /// In en, this message translates to:
  /// **'Auto-Lock & Privacy'**
  String get settingsAutoLockPrivacy;

  /// No description provided for @settingsAutoLockPrivacyDesc.
  ///
  /// In en, this message translates to:
  /// **'Configure timeout and privacy settings'**
  String get settingsAutoLockPrivacyDesc;

  /// No description provided for @settingsVerifyPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to access security settings.'**
  String get settingsVerifyPassword;

  /// No description provided for @settingsSensitivity.
  ///
  /// In en, this message translates to:
  /// **'Sensitivity Level Settings'**
  String get settingsSensitivity;

  /// No description provided for @settingsSensitivityDesc.
  ///
  /// In en, this message translates to:
  /// **'Configure field sensitivity'**
  String get settingsSensitivityDesc;

  /// No description provided for @settingsOperationLog.
  ///
  /// In en, this message translates to:
  /// **'Operation Log'**
  String get settingsOperationLog;

  /// No description provided for @settingsOperationLogDesc.
  ///
  /// In en, this message translates to:
  /// **'View activity history'**
  String get settingsOperationLogDesc;

  /// No description provided for @settingsSync.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get settingsSync;

  /// No description provided for @settingsCloudSync.
  ///
  /// In en, this message translates to:
  /// **'Cloud Sync'**
  String get settingsCloudSync;

  /// No description provided for @settingsNotConfigured.
  ///
  /// In en, this message translates to:
  /// **'Not configured'**
  String get settingsNotConfigured;

  /// No description provided for @settingsOfflineMode.
  ///
  /// In en, this message translates to:
  /// **'Offline Mode'**
  String get settingsOfflineMode;

  /// No description provided for @settingsOfflineModeDesc.
  ///
  /// In en, this message translates to:
  /// **'Local data only'**
  String get settingsOfflineModeDesc;

  /// No description provided for @settingsAiAssistant.
  ///
  /// In en, this message translates to:
  /// **'AI Assistant'**
  String get settingsAiAssistant;

  /// No description provided for @settingsLlmConfig.
  ///
  /// In en, this message translates to:
  /// **'LLM Configuration'**
  String get settingsLlmConfig;

  /// No description provided for @settingsLlmConfigDesc.
  ///
  /// In en, this message translates to:
  /// **'Local model or cloud API'**
  String get settingsLlmConfigDesc;

  /// No description provided for @settingsAbout.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get settingsAbout;

  /// No description provided for @settingsVersion.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get settingsVersion;

  /// No description provided for @settingsDebugLog.
  ///
  /// In en, this message translates to:
  /// **'Debug Log'**
  String get settingsDebugLog;

  /// No description provided for @settingsDebugLogDesc.
  ///
  /// In en, this message translates to:
  /// **'View debug log'**
  String get settingsDebugLogDesc;

  /// No description provided for @settingsPrivacyPolicy.
  ///
  /// In en, this message translates to:
  /// **'Privacy Policy'**
  String get settingsPrivacyPolicy;

  /// No description provided for @settingsPrivacyPolicyDesc.
  ///
  /// In en, this message translates to:
  /// **'View our privacy policy'**
  String get settingsPrivacyPolicyDesc;

  /// No description provided for @settingsTermsOfService.
  ///
  /// In en, this message translates to:
  /// **'Terms of Service'**
  String get settingsTermsOfService;

  /// No description provided for @settingsTermsOfServiceDesc.
  ///
  /// In en, this message translates to:
  /// **'View terms of service'**
  String get settingsTermsOfServiceDesc;

  /// No description provided for @settingsLocal.
  ///
  /// In en, this message translates to:
  /// **'Local'**
  String get settingsLocal;

  /// No description provided for @settingsPrivate.
  ///
  /// In en, this message translates to:
  /// **'Private'**
  String get settingsPrivate;

  /// No description provided for @settingsUniversal.
  ///
  /// In en, this message translates to:
  /// **'Universal'**
  String get settingsUniversal;

  /// No description provided for @securityVaultSecurity.
  ///
  /// In en, this message translates to:
  /// **'Vault Security'**
  String get securityVaultSecurity;

  /// No description provided for @securityAutoLockDelay.
  ///
  /// In en, this message translates to:
  /// **'Auto-Lock Delay'**
  String get securityAutoLockDelay;

  /// No description provided for @securityAutoLockDesc.
  ///
  /// In en, this message translates to:
  /// **'Lock vault after inactivity'**
  String get securityAutoLockDesc;

  /// No description provided for @securityBiometricUnlock.
  ///
  /// In en, this message translates to:
  /// **'Biometric Unlock'**
  String get securityBiometricUnlock;

  /// No description provided for @securityPrivacy.
  ///
  /// In en, this message translates to:
  /// **'Privacy'**
  String get securityPrivacy;

  /// No description provided for @securityAppPrivacyScreen.
  ///
  /// In en, this message translates to:
  /// **'App Privacy Screen'**
  String get securityAppPrivacyScreen;

  /// No description provided for @securityAppPrivacyDesc.
  ///
  /// In en, this message translates to:
  /// **'Hide content in app switcher'**
  String get securityAppPrivacyDesc;

  /// No description provided for @securityLockOnBlur.
  ///
  /// In en, this message translates to:
  /// **'Lock on Window Blur'**
  String get securityLockOnBlur;

  /// No description provided for @securityLockOnBlurDesc.
  ///
  /// In en, this message translates to:
  /// **'Lock when switching apps'**
  String get securityLockOnBlurDesc;

  /// No description provided for @securityClipboard.
  ///
  /// In en, this message translates to:
  /// **'Clipboard'**
  String get securityClipboard;

  /// No description provided for @securityAutoClearDelay.
  ///
  /// In en, this message translates to:
  /// **'Auto-Clear Delay'**
  String get securityAutoClearDelay;

  /// No description provided for @securityAutoClearDesc.
  ///
  /// In en, this message translates to:
  /// **'Clear clipboard after copying sensitive data'**
  String get securityAutoClearDesc;

  /// No description provided for @sensitivityVerifyPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to access sensitivity settings.'**
  String get sensitivityVerifyPassword;

  /// No description provided for @sensitivityCritical.
  ///
  /// In en, this message translates to:
  /// **'Restricted'**
  String get sensitivityCritical;

  /// No description provided for @sensitivityCriticalDesc.
  ///
  /// In en, this message translates to:
  /// **'Maximum sensitivity - always masked, requires verification'**
  String get sensitivityCriticalDesc;

  /// No description provided for @sensitivitySensitive.
  ///
  /// In en, this message translates to:
  /// **'Sensitive'**
  String get sensitivitySensitive;

  /// No description provided for @sensitivitySensitiveDesc.
  ///
  /// In en, this message translates to:
  /// **'Personal information requiring protection'**
  String get sensitivitySensitiveDesc;

  /// No description provided for @sensitivityInternal.
  ///
  /// In en, this message translates to:
  /// **'Internal'**
  String get sensitivityInternal;

  /// No description provided for @sensitivityInternalDesc.
  ///
  /// In en, this message translates to:
  /// **'Internal use only - can be hidden by display settings'**
  String get sensitivityInternalDesc;

  /// No description provided for @sensitivityPublic.
  ///
  /// In en, this message translates to:
  /// **'Public'**
  String get sensitivityPublic;

  /// No description provided for @sensitivityPublicDesc.
  ///
  /// In en, this message translates to:
  /// **'Lowest sensitivity - always visible'**
  String get sensitivityPublicDesc;

  /// No description provided for @syncSynchronizing.
  ///
  /// In en, this message translates to:
  /// **'Synchronizing...'**
  String get syncSynchronizing;

  /// No description provided for @syncDeviceDiscovery.
  ///
  /// In en, this message translates to:
  /// **'Device Discovery'**
  String get syncDeviceDiscovery;

  /// No description provided for @syncManualConnection.
  ///
  /// In en, this message translates to:
  /// **'Manual Connection'**
  String get syncManualConnection;

  /// No description provided for @syncLastSync.
  ///
  /// In en, this message translates to:
  /// **'Last Sync'**
  String get syncLastSync;

  /// No description provided for @syncStatus.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get syncStatus;

  /// No description provided for @syncDirection.
  ///
  /// In en, this message translates to:
  /// **'Direction'**
  String get syncDirection;

  /// No description provided for @syncData.
  ///
  /// In en, this message translates to:
  /// **'Data'**
  String get syncData;

  /// No description provided for @syncError.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get syncError;

  /// No description provided for @trashVerifyPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to view the trash.'**
  String get trashVerifyPassword;

  /// No description provided for @trashRestored.
  ///
  /// In en, this message translates to:
  /// **'Restored '**
  String get trashRestored;

  /// No description provided for @trashPermanentlyDeleted.
  ///
  /// In en, this message translates to:
  /// **'Permanently deleted '**
  String get trashPermanentlyDeleted;

  /// No description provided for @operationLogVerifyPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to view the operation log.'**
  String get operationLogVerifyPassword;

  /// No description provided for @dataMgmtRestoreBackup.
  ///
  /// In en, this message translates to:
  /// **'Restore Backup?'**
  String get dataMgmtRestoreBackup;

  /// No description provided for @dataMgmtDeleteBackup.
  ///
  /// In en, this message translates to:
  /// **'Delete Backup?'**
  String get dataMgmtDeleteBackup;

  /// No description provided for @dataMgmtConfirmDeletion.
  ///
  /// In en, this message translates to:
  /// **'Enter your master password to confirm backup deletion.'**
  String get dataMgmtConfirmDeletion;

  /// No description provided for @llmApiEndpoint.
  ///
  /// In en, this message translates to:
  /// **'API Endpoint'**
  String get llmApiEndpoint;

  /// No description provided for @llmModel.
  ///
  /// In en, this message translates to:
  /// **'Model'**
  String get llmModel;

  /// No description provided for @llmAnthropicVersion.
  ///
  /// In en, this message translates to:
  /// **'Anthropic API Version'**
  String get llmAnthropicVersion;

  /// No description provided for @llmOpenAI.
  ///
  /// In en, this message translates to:
  /// **'OpenAI'**
  String get llmOpenAI;

  /// No description provided for @llmAnthropic.
  ///
  /// In en, this message translates to:
  /// **'Anthropic'**
  String get llmAnthropic;

  /// No description provided for @searchUnlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get searchUnlock;

  /// No description provided for @searchDeleted.
  ///
  /// In en, this message translates to:
  /// **'Deleted'**
  String get searchDeleted;

  /// No description provided for @searchReveal.
  ///
  /// In en, this message translates to:
  /// **'Reveal'**
  String get searchReveal;

  /// No description provided for @searchRestrictedHint.
  ///
  /// In en, this message translates to:
  /// **'Restricted - password required to view'**
  String get searchRestrictedHint;

  /// No description provided for @searchPrivateHint.
  ///
  /// In en, this message translates to:
  /// **'Private - reveal to view'**
  String get searchPrivateHint;

  /// No description provided for @sensitivityRestricted.
  ///
  /// In en, this message translates to:
  /// **'Restricted'**
  String get sensitivityRestricted;

  /// No description provided for @commonAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get commonAdd;

  /// No description provided for @commonCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get commonCopy;

  /// No description provided for @dialogCurrentPassword.
  ///
  /// In en, this message translates to:
  /// **'Current Password'**
  String get dialogCurrentPassword;

  /// No description provided for @dialogNewPassword.
  ///
  /// In en, this message translates to:
  /// **'New Password'**
  String get dialogNewPassword;

  /// No description provided for @dialogConfirmNewPassword.
  ///
  /// In en, this message translates to:
  /// **'Confirm New Password'**
  String get dialogConfirmNewPassword;

  /// No description provided for @dialogChange.
  ///
  /// In en, this message translates to:
  /// **'Change'**
  String get dialogChange;

  /// No description provided for @dialogLock.
  ///
  /// In en, this message translates to:
  /// **'Lock'**
  String get dialogLock;

  /// No description provided for @dialogVerifyIdentity.
  ///
  /// In en, this message translates to:
  /// **'Verify Identity'**
  String get dialogVerifyIdentity;

  /// No description provided for @dialogDeleteItem.
  ///
  /// In en, this message translates to:
  /// **'Delete Item'**
  String get dialogDeleteItem;

  /// No description provided for @dialogDeleteSection.
  ///
  /// In en, this message translates to:
  /// **'Delete Section?'**
  String get dialogDeleteSection;

  /// No description provided for @dialogDeleteSectionConfirm.
  ///
  /// In en, this message translates to:
  /// **'This section and its items will be moved to trash.'**
  String get dialogDeleteSectionConfirm;

  /// No description provided for @biometricTestTouchId.
  ///
  /// In en, this message translates to:
  /// **'Test Touch ID'**
  String get biometricTestTouchId;

  /// No description provided for @biometricTestFaceId.
  ///
  /// In en, this message translates to:
  /// **'Test Face ID'**
  String get biometricTestFaceId;

  /// No description provided for @dialogAddQuickAction.
  ///
  /// In en, this message translates to:
  /// **'Add Quick Action'**
  String get dialogAddQuickAction;

  /// No description provided for @homePageEditorSections.
  ///
  /// In en, this message translates to:
  /// **'Sections'**
  String get homePageEditorSections;

  /// No description provided for @homePageEditorIcon.
  ///
  /// In en, this message translates to:
  /// **'Icon'**
  String get homePageEditorIcon;

  /// No description provided for @homePageEditorSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Section Title'**
  String get homePageEditorSectionTitle;

  /// No description provided for @settingsDeleteAccount.
  ///
  /// In en, this message translates to:
  /// **'Delete Account'**
  String get settingsDeleteAccount;

  /// No description provided for @settingsDebugLogCopyTitle.
  ///
  /// In en, this message translates to:
  /// **'Copy Logs to Clipboard'**
  String get settingsDebugLogCopyTitle;

  /// No description provided for @settingsDebugLogCopied.
  ///
  /// In en, this message translates to:
  /// **'Sanitized logs copied to clipboard'**
  String get settingsDebugLogCopied;

  /// No description provided for @settingsDebugLogTitle.
  ///
  /// In en, this message translates to:
  /// **'Debug Log'**
  String get settingsDebugLogTitle;

  /// No description provided for @dialogSelectFolder.
  ///
  /// In en, this message translates to:
  /// **'Select This Folder'**
  String get dialogSelectFolder;

  /// No description provided for @iconPickerTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose Icon'**
  String get iconPickerTitle;

  /// No description provided for @operationDetails.
  ///
  /// In en, this message translates to:
  /// **'Operation Details'**
  String get operationDetails;

  /// No description provided for @trashHistory.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get trashHistory;

  /// No description provided for @dialogUseBiometric.
  ///
  /// In en, this message translates to:
  /// **'Use {biometricType}'**
  String dialogUseBiometric(String biometricType);

  /// No description provided for @dialogDeleteItemConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete \"{name}\"?'**
  String dialogDeleteItemConfirm(String name);

  /// No description provided for @entryHistoryCount.
  ///
  /// In en, this message translates to:
  /// **'History({count})'**
  String entryHistoryCount(int count);

  /// No description provided for @biometricPasswordHint.
  ///
  /// In en, this message translates to:
  /// **'Password Hint: {hint}'**
  String biometricPasswordHint(String hint);

  /// No description provided for @settingsUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get settingsUnknown;

  /// No description provided for @settingsActive.
  ///
  /// In en, this message translates to:
  /// **'Active'**
  String get settingsActive;

  /// No description provided for @settingsCloudSyncSetup.
  ///
  /// In en, this message translates to:
  /// **'Cloud sync setup'**
  String get settingsCloudSyncSetup;

  /// No description provided for @settingsComingSoon.
  ///
  /// In en, this message translates to:
  /// **'This feature will be available in a future update.'**
  String get settingsComingSoon;

  /// No description provided for @settingsTagline.
  ///
  /// In en, this message translates to:
  /// **'Your Local Digital Twin. Privacy-First Universal Identity.'**
  String get settingsTagline;

  /// No description provided for @settingsVerifyIdentityDebug.
  ///
  /// In en, this message translates to:
  /// **'Verify your identity to enable debug mode'**
  String get settingsVerifyIdentityDebug;

  /// No description provided for @loginNoPasswordHint.
  ///
  /// In en, this message translates to:
  /// **'No password hint available'**
  String get loginNoPasswordHint;

  /// No description provided for @commonVerify.
  ///
  /// In en, this message translates to:
  /// **'Verify'**
  String get commonVerify;

  /// No description provided for @commonRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get commonRefresh;

  /// No description provided for @commonShowLess.
  ///
  /// In en, this message translates to:
  /// **'Show less'**
  String get commonShowLess;

  /// No description provided for @debugLogCopyToClipboard.
  ///
  /// In en, this message translates to:
  /// **'Copy to clipboard'**
  String get debugLogCopyToClipboard;

  /// No description provided for @debugLogDisable.
  ///
  /// In en, this message translates to:
  /// **'Disable debug mode'**
  String get debugLogDisable;

  /// No description provided for @debugLogEmpty.
  ///
  /// In en, this message translates to:
  /// **'No debug logs available.'**
  String get debugLogEmpty;

  /// No description provided for @deleteAccountEnterPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter password to confirm'**
  String get deleteAccountEnterPassword;

  /// No description provided for @deleteAccountPasswordRequired.
  ///
  /// In en, this message translates to:
  /// **'Password is required'**
  String get deleteAccountPasswordRequired;

  /// No description provided for @deleteAccountInvalidPassword.
  ///
  /// In en, this message translates to:
  /// **'Invalid password'**
  String get deleteAccountInvalidPassword;

  /// No description provided for @pageEditorPageTitleHint.
  ///
  /// In en, this message translates to:
  /// **'Page title'**
  String get pageEditorPageTitleHint;

  /// No description provided for @pageEditorSaveFirst.
  ///
  /// In en, this message translates to:
  /// **'Save the page first to add sections'**
  String get pageEditorSaveFirst;

  /// No description provided for @pageEditorNoSections.
  ///
  /// In en, this message translates to:
  /// **'No sections yet'**
  String get pageEditorNoSections;

  /// No description provided for @pageEditorEnterSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Enter section title'**
  String get pageEditorEnterSectionTitle;

  /// No description provided for @pageEditorEditSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit Section'**
  String get pageEditorEditSectionTitle;

  /// No description provided for @folderPickerGoUp.
  ///
  /// In en, this message translates to:
  /// **'Go up'**
  String get folderPickerGoUp;

  /// No description provided for @headerLockSensitivity.
  ///
  /// In en, this message translates to:
  /// **'Lock Sensitivity Access'**
  String get headerLockSensitivity;

  /// No description provided for @datePickerClear.
  ///
  /// In en, this message translates to:
  /// **'Clear date'**
  String get datePickerClear;

  /// No description provided for @entryCopyAll.
  ///
  /// In en, this message translates to:
  /// **'Copy All'**
  String get entryCopyAll;

  /// No description provided for @entryNoHistory.
  ///
  /// In en, this message translates to:
  /// **'No history yet'**
  String get entryNoHistory;

  /// No description provided for @operationViewDetails.
  ///
  /// In en, this message translates to:
  /// **'View details'**
  String get operationViewDetails;

  /// No description provided for @scanStopScan.
  ///
  /// In en, this message translates to:
  /// **'Stop scan'**
  String get scanStopScan;

  /// No description provided for @settingsNoHintAvailable.
  ///
  /// In en, this message translates to:
  /// **'No hint available'**
  String get settingsNoHintAvailable;

  /// No description provided for @sensitiveRestrictedMessage.
  ///
  /// In en, this message translates to:
  /// **'Restricted field. Enter your master password to view.'**
  String get sensitiveRestrictedMessage;

  /// No description provided for @syncUnknownError.
  ///
  /// In en, this message translates to:
  /// **'Unknown error'**
  String get syncUnknownError;

  /// No description provided for @syncScanning.
  ///
  /// In en, this message translates to:
  /// **'Scanning...'**
  String get syncScanning;

  /// No description provided for @syncScan.
  ///
  /// In en, this message translates to:
  /// **'Scan'**
  String get syncScan;

  /// No description provided for @syncSyncing.
  ///
  /// In en, this message translates to:
  /// **'Syncing...'**
  String get syncSyncing;

  /// No description provided for @syncConnectSync.
  ///
  /// In en, this message translates to:
  /// **'Connect & Sync'**
  String get syncConnectSync;

  /// No description provided for @scanMappingBoth.
  ///
  /// In en, this message translates to:
  /// **'AI+Rule'**
  String get scanMappingBoth;

  /// No description provided for @scanMappingAi.
  ///
  /// In en, this message translates to:
  /// **'AI'**
  String get scanMappingAi;

  /// No description provided for @mrzDocumentType.
  ///
  /// In en, this message translates to:
  /// **'Document Type'**
  String get mrzDocumentType;

  /// No description provided for @mrzDocumentNumber.
  ///
  /// In en, this message translates to:
  /// **'Document Number'**
  String get mrzDocumentNumber;

  /// No description provided for @mrzSurname.
  ///
  /// In en, this message translates to:
  /// **'Surname'**
  String get mrzSurname;

  /// No description provided for @mrzGivenNames.
  ///
  /// In en, this message translates to:
  /// **'Given Names'**
  String get mrzGivenNames;

  /// No description provided for @mrzNationality.
  ///
  /// In en, this message translates to:
  /// **'Nationality'**
  String get mrzNationality;

  /// No description provided for @mrzDateOfBirth.
  ///
  /// In en, this message translates to:
  /// **'Date of Birth'**
  String get mrzDateOfBirth;

  /// No description provided for @mrzSex.
  ///
  /// In en, this message translates to:
  /// **'Sex'**
  String get mrzSex;

  /// No description provided for @mrzExpiryDate.
  ///
  /// In en, this message translates to:
  /// **'Expiry Date'**
  String get mrzExpiryDate;

  /// No description provided for @changePasswordMinLength.
  ///
  /// In en, this message translates to:
  /// **'Minimum 8 characters'**
  String get changePasswordMinLength;

  /// No description provided for @operationActionCreate.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get operationActionCreate;

  /// No description provided for @operationActionUpdate.
  ///
  /// In en, this message translates to:
  /// **'Update'**
  String get operationActionUpdate;

  /// No description provided for @operationActionDelete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get operationActionDelete;

  /// No description provided for @operationActionRestore.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get operationActionRestore;

  /// No description provided for @operationActionPurge.
  ///
  /// In en, this message translates to:
  /// **'Purge'**
  String get operationActionPurge;

  /// No description provided for @operationPlatformAndroid.
  ///
  /// In en, this message translates to:
  /// **'Android'**
  String get operationPlatformAndroid;

  /// No description provided for @operationPlatformWeb.
  ///
  /// In en, this message translates to:
  /// **'Web'**
  String get operationPlatformWeb;

  /// No description provided for @operationLabelTimestamp.
  ///
  /// In en, this message translates to:
  /// **'Timestamp'**
  String get operationLabelTimestamp;

  /// No description provided for @operationLabelAction.
  ///
  /// In en, this message translates to:
  /// **'Action'**
  String get operationLabelAction;

  /// No description provided for @operationLabelSection.
  ///
  /// In en, this message translates to:
  /// **'Section'**
  String get operationLabelSection;

  /// No description provided for @operationLabelFieldPath.
  ///
  /// In en, this message translates to:
  /// **'Field Path'**
  String get operationLabelFieldPath;

  /// No description provided for @operationLabelDescription.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get operationLabelDescription;

  /// No description provided for @operationLabelDevice.
  ///
  /// In en, this message translates to:
  /// **'Device'**
  String get operationLabelDevice;

  /// No description provided for @versionCurrentVersion.
  ///
  /// In en, this message translates to:
  /// **'Current Version'**
  String get versionCurrentVersion;

  /// No description provided for @versionLatestVersion.
  ///
  /// In en, this message translates to:
  /// **'Latest Version'**
  String get versionLatestVersion;

  /// No description provided for @versionUpdateStatus.
  ///
  /// In en, this message translates to:
  /// **'Update Status'**
  String get versionUpdateStatus;

  /// No description provided for @versionPlatform.
  ///
  /// In en, this message translates to:
  /// **'Platform'**
  String get versionPlatform;

  /// No description provided for @accountCreated.
  ///
  /// In en, this message translates to:
  /// **'Created'**
  String get accountCreated;

  /// No description provided for @accountLastLogin.
  ///
  /// In en, this message translates to:
  /// **'Last Login'**
  String get accountLastLogin;

  /// No description provided for @accountLastOperation.
  ///
  /// In en, this message translates to:
  /// **'Last Operation'**
  String get accountLastOperation;

  /// No description provided for @accountLoginDevices.
  ///
  /// In en, this message translates to:
  /// **'Login Devices'**
  String get accountLoginDevices;

  /// No description provided for @homeDefaultPages.
  ///
  /// In en, this message translates to:
  /// **'Default Pages'**
  String get homeDefaultPages;

  /// No description provided for @homeCustomizedPages.
  ///
  /// In en, this message translates to:
  /// **'Customized Pages'**
  String get homeCustomizedPages;

  /// No description provided for @trashDetailLabel.
  ///
  /// In en, this message translates to:
  /// **'Details'**
  String get trashDetailLabel;

  /// No description provided for @trashRestoreLabel.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get trashRestoreLabel;

  /// No description provided for @trashPurgeLabel.
  ///
  /// In en, this message translates to:
  /// **'Purge'**
  String get trashPurgeLabel;

  /// No description provided for @commonTitle.
  ///
  /// In en, this message translates to:
  /// **'Title'**
  String get commonTitle;

  /// No description provided for @predefinedUnknownType.
  ///
  /// In en, this message translates to:
  /// **'Unknown type: {type}'**
  String predefinedUnknownType(String type);

  /// No description provided for @commonShowPassword.
  ///
  /// In en, this message translates to:
  /// **'Show password'**
  String get commonShowPassword;

  /// No description provided for @commonHidePassword.
  ///
  /// In en, this message translates to:
  /// **'Hide password'**
  String get commonHidePassword;

  /// No description provided for @objectCardAddItem.
  ///
  /// In en, this message translates to:
  /// **'Add Item'**
  String get objectCardAddItem;

  /// No description provided for @dataManagementRestoreBackupTooltip.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get dataManagementRestoreBackupTooltip;

  /// No description provided for @dataManagementSpecialBackupTooltip.
  ///
  /// In en, this message translates to:
  /// **'Save as special backup'**
  String get dataManagementSpecialBackupTooltip;

  /// No description provided for @passwordVerificationRestricted.
  ///
  /// In en, this message translates to:
  /// **'Restricted field. Enter your master password to proceed.'**
  String get passwordVerificationRestricted;

  /// No description provided for @passwordVerificationInvalid.
  ///
  /// In en, this message translates to:
  /// **'Invalid password'**
  String get passwordVerificationInvalid;

  /// No description provided for @trashPasswordRequired.
  ///
  /// In en, this message translates to:
  /// **'Password Required'**
  String get trashPasswordRequired;

  /// No description provided for @trashEmptyConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to permanently delete all {count} items in trash?'**
  String trashEmptyConfirm(int count);

  /// No description provided for @trashEmptyWarning.
  ///
  /// In en, this message translates to:
  /// **'This action cannot be undone. All items will be permanently removed.'**
  String get trashEmptyWarning;

  /// No description provided for @trashEmptyComplete.
  ///
  /// In en, this message translates to:
  /// **'All {count} items permanently deleted'**
  String trashEmptyComplete(int count);

  /// No description provided for @trashRestoreConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to restore \"{name}\"?'**
  String trashRestoreConfirmBody(String name);

  /// No description provided for @trashRestoredItem.
  ///
  /// In en, this message translates to:
  /// **'Restored \"{name}\"'**
  String trashRestoredItem(String name);

  /// No description provided for @trashPermanentDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to permanently delete \"{name}\"?'**
  String trashPermanentDeleteConfirm(String name);

  /// No description provided for @trashPermanentDeleteWarning.
  ///
  /// In en, this message translates to:
  /// **'This action cannot be undone. The item will be permanently removed.'**
  String get trashPermanentDeleteWarning;

  /// No description provided for @trashPermanentDeletedItem.
  ///
  /// In en, this message translates to:
  /// **'Permanently deleted \"{name}\"'**
  String trashPermanentDeletedItem(String name);

  /// No description provided for @trashEmpty.
  ///
  /// In en, this message translates to:
  /// **'Trash is empty'**
  String get trashEmpty;

  /// No description provided for @trashNoMatching.
  ///
  /// In en, this message translates to:
  /// **'No matching items'**
  String get trashNoMatching;

  /// No description provided for @trashDeletedAppear.
  ///
  /// In en, this message translates to:
  /// **'Deleted items will appear here'**
  String get trashDeletedAppear;

  /// No description provided for @trashAdjustSearch.
  ///
  /// In en, this message translates to:
  /// **'Try adjusting your search'**
  String get trashAdjustSearch;

  /// No description provided for @trashFoundResults.
  ///
  /// In en, this message translates to:
  /// **'Found {count} result(s)'**
  String trashFoundResults(int count);

  /// No description provided for @trashNoResults.
  ///
  /// In en, this message translates to:
  /// **'No results found'**
  String get trashNoResults;

  /// No description provided for @trashTotalItems.
  ///
  /// In en, this message translates to:
  /// **'{count} total items in trash'**
  String trashTotalItems(int count);

  /// No description provided for @trashSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Pages & Objects'**
  String get trashSectionTitle;

  /// No description provided for @trashAutoPurgeNotice.
  ///
  /// In en, this message translates to:
  /// **'Items in trash are permanently deleted after 30 days'**
  String get trashAutoPurgeNotice;

  /// No description provided for @trashEmptyTrashButton.
  ///
  /// In en, this message translates to:
  /// **'Empty Trash'**
  String get trashEmptyTrashButton;

  /// No description provided for @operationLogPasswordRequired.
  ///
  /// In en, this message translates to:
  /// **'Password Required'**
  String get operationLogPasswordRequired;

  /// No description provided for @operationLogClearConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to clear all operation history?'**
  String get operationLogClearConfirm;

  /// No description provided for @operationLogFoundResults.
  ///
  /// In en, this message translates to:
  /// **'Found {count} result(s)'**
  String operationLogFoundResults(int count);

  /// No description provided for @operationLogNoMatching.
  ///
  /// In en, this message translates to:
  /// **'No matching entries'**
  String get operationLogNoMatching;

  /// No description provided for @operationLogTryDifferent.
  ///
  /// In en, this message translates to:
  /// **'Try a different search term'**
  String get operationLogTryDifferent;

  /// No description provided for @operationLogAdjustFilters.
  ///
  /// In en, this message translates to:
  /// **'Try adjusting your filters'**
  String get operationLogAdjustFilters;

  /// No description provided for @operationLogFilters.
  ///
  /// In en, this message translates to:
  /// **'Filters'**
  String get operationLogFilters;

  /// No description provided for @sensitivityPasswordRequired.
  ///
  /// In en, this message translates to:
  /// **'Password Required'**
  String get sensitivityPasswordRequired;

  /// No description provided for @sensitivityDowngradeWarning.
  ///
  /// In en, this message translates to:
  /// **'You are about to downgrade \"{name}\" to a lower sensitivity level.'**
  String sensitivityDowngradeWarning(String name);

  /// No description provided for @sensitivityDowngradeConfirm.
  ///
  /// In en, this message translates to:
  /// **'This field will be visible with fewer protections. Continue?'**
  String get sensitivityDowngradeConfirm;

  /// No description provided for @sensitivityMovedToPrivate.
  ///
  /// In en, this message translates to:
  /// **'\"{name}\" moved to Private'**
  String sensitivityMovedToPrivate(String name);

  /// No description provided for @sensitivityNoFields.
  ///
  /// In en, this message translates to:
  /// **'No fields in this section'**
  String get sensitivityNoFields;

  /// No description provided for @sensitivityKeepHighest.
  ///
  /// In en, this message translates to:
  /// **'Keep at Highest'**
  String get sensitivityKeepHighest;

  /// No description provided for @sensitivityMoveHigher.
  ///
  /// In en, this message translates to:
  /// **'Move to Higher'**
  String get sensitivityMoveHigher;

  /// No description provided for @sensitivityKeepLowest.
  ///
  /// In en, this message translates to:
  /// **'Keep at Lowest'**
  String get sensitivityKeepLowest;

  /// No description provided for @sensitivityMoveLower.
  ///
  /// In en, this message translates to:
  /// **'Move to Lower'**
  String get sensitivityMoveLower;

  /// No description provided for @sensitivityMovedHigher.
  ///
  /// In en, this message translates to:
  /// **'\"{name}\" moved to higher sensitivity'**
  String sensitivityMovedHigher(String name);

  /// No description provided for @sensitivityFoundResults.
  ///
  /// In en, this message translates to:
  /// **'Found {count} result(s)'**
  String sensitivityFoundResults(int count);

  /// No description provided for @sensitivityNoResults.
  ///
  /// In en, this message translates to:
  /// **'No results found'**
  String get sensitivityNoResults;

  /// No description provided for @sensitivityAdjustHint.
  ///
  /// In en, this message translates to:
  /// **'Adjust the sensitivity level for each field. Restricted fields require additional verification to view.'**
  String get sensitivityAdjustHint;

  /// No description provided for @sensitivityNoFieldsMatch.
  ///
  /// In en, this message translates to:
  /// **'No fields match \"{query}\"'**
  String sensitivityNoFieldsMatch(String query);

  /// No description provided for @sensitivityFieldsConfigured.
  ///
  /// In en, this message translates to:
  /// **'{count} fields configured'**
  String sensitivityFieldsConfigured(int count);

  /// No description provided for @sensitivityTotalFields.
  ///
  /// In en, this message translates to:
  /// **'{count} total fields'**
  String sensitivityTotalFields(int count);

  /// No description provided for @commonNA.
  ///
  /// In en, this message translates to:
  /// **'N/A'**
  String get commonNA;

  /// No description provided for @accountIdLabel.
  ///
  /// In en, this message translates to:
  /// **'Account ID: {id}'**
  String accountIdLabel(String id);

  /// No description provided for @accountNoRecentOps.
  ///
  /// In en, this message translates to:
  /// **'No recent operations'**
  String get accountNoRecentOps;

  /// No description provided for @accountNoDevices.
  ///
  /// In en, this message translates to:
  /// **'No devices recorded'**
  String get accountNoDevices;

  /// No description provided for @accountRecentDevices.
  ///
  /// In en, this message translates to:
  /// **'Recent Devices'**
  String get accountRecentDevices;

  /// No description provided for @settingsAllAccountsTitle.
  ///
  /// In en, this message translates to:
  /// **'All Accounts'**
  String get settingsAllAccountsTitle;

  /// No description provided for @accountLastLoginLabel.
  ///
  /// In en, this message translates to:
  /// **'Last login: {time}'**
  String accountLastLoginLabel(String time);

  /// No description provided for @versionUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Unavailable'**
  String get versionUnavailable;

  /// No description provided for @versionUpToDate.
  ///
  /// In en, this message translates to:
  /// **'Up to date'**
  String get versionUpToDate;

  /// No description provided for @versionUpdateAvailable.
  ///
  /// In en, this message translates to:
  /// **'Update available'**
  String get versionUpdateAvailable;

  /// No description provided for @settingsAccountCount.
  ///
  /// In en, this message translates to:
  /// **'{count} account(s)'**
  String settingsAccountCount(int count);

  /// No description provided for @debugLogSanitizeWarning.
  ///
  /// In en, this message translates to:
  /// **'Logs will be sanitized before copying, but clipboard content is visible to other apps. The clipboard should be cleared after use.'**
  String get debugLogSanitizeWarning;

  /// No description provided for @debugLogActiveNotice.
  ///
  /// In en, this message translates to:
  /// **'Debug mode is active. Logs are being recorded.'**
  String get debugLogActiveNotice;

  /// No description provided for @homeVaultUnlocked.
  ///
  /// In en, this message translates to:
  /// **'Vault Unlocked'**
  String get homeVaultUnlocked;

  /// No description provided for @homeOnline.
  ///
  /// In en, this message translates to:
  /// **'Online'**
  String get homeOnline;

  /// No description provided for @homeOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get homeOffline;

  /// No description provided for @searchEnterMinChars.
  ///
  /// In en, this message translates to:
  /// **'Enter at least 2 characters to search'**
  String get searchEnterMinChars;

  /// No description provided for @searchNoResultsBody.
  ///
  /// In en, this message translates to:
  /// **'No results found'**
  String get searchNoResultsBody;

  /// No description provided for @searchAdjustFilters.
  ///
  /// In en, this message translates to:
  /// **'Try adjusting your filters or search terms'**
  String get searchAdjustFilters;

  /// No description provided for @syncComplete.
  ///
  /// In en, this message translates to:
  /// **'Sync complete'**
  String get syncComplete;

  /// No description provided for @syncFailed.
  ///
  /// In en, this message translates to:
  /// **'Sync failed: {error}'**
  String syncFailed(String error);

  /// No description provided for @syncDirectionPushed.
  ///
  /// In en, this message translates to:
  /// **'Pushed local changes'**
  String get syncDirectionPushed;

  /// No description provided for @syncDirectionPulled.
  ///
  /// In en, this message translates to:
  /// **'Pulled remote changes'**
  String get syncDirectionPulled;

  /// No description provided for @syncDirectionMerged.
  ///
  /// In en, this message translates to:
  /// **'Merged changes from both devices'**
  String get syncDirectionMerged;

  /// No description provided for @syncDirectionNoChange.
  ///
  /// In en, this message translates to:
  /// **'Already in sync'**
  String get syncDirectionNoChange;

  /// No description provided for @syncDiscoveryHint.
  ///
  /// In en, this message translates to:
  /// **'Scan for nearby SoloSoul devices on your local network.'**
  String get syncDiscoveryHint;

  /// No description provided for @syncFoundDevices.
  ///
  /// In en, this message translates to:
  /// **'Found {count} device(s)'**
  String syncFoundDevices(int count);

  /// No description provided for @syncTestFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get syncTestFailed;

  /// No description provided for @syncDirectionPush.
  ///
  /// In en, this message translates to:
  /// **'Push'**
  String get syncDirectionPush;

  /// No description provided for @syncDirectionPull.
  ///
  /// In en, this message translates to:
  /// **'Pull'**
  String get syncDirectionPull;

  /// No description provided for @syncDirectionMergedShort.
  ///
  /// In en, this message translates to:
  /// **'Merged'**
  String get syncDirectionMergedShort;

  /// No description provided for @syncDirectionNoChangeShort.
  ///
  /// In en, this message translates to:
  /// **'No Change'**
  String get syncDirectionNoChangeShort;

  /// No description provided for @localSearchScanLocalFiles.
  ///
  /// In en, this message translates to:
  /// **'Scan Local Files'**
  String get localSearchScanLocalFiles;

  /// No description provided for @localSearchDescription.
  ///
  /// In en, this message translates to:
  /// **'Search your local files for personal information and import them into your Vault.'**
  String get localSearchDescription;

  /// No description provided for @localSearchSelectHint.
  ///
  /// In en, this message translates to:
  /// **'Tap to select. Long press to adjust size limit.'**
  String get localSearchSelectHint;

  /// No description provided for @localSearchPrivacyNotice.
  ///
  /// In en, this message translates to:
  /// **'All scanning is done locally. No data leaves your device. You will preview all results before importing.'**
  String get localSearchPrivacyNotice;

  /// No description provided for @localSearchSkipLargerThan.
  ///
  /// In en, this message translates to:
  /// **'Skip {label} files larger than:'**
  String localSearchSkipLargerThan(String label);

  /// No description provided for @localSearchScanningFiles.
  ///
  /// In en, this message translates to:
  /// **'Scanning files...'**
  String get localSearchScanningFiles;

  /// No description provided for @localSearchScanCanceled.
  ///
  /// In en, this message translates to:
  /// **'Scan canceled'**
  String get localSearchScanCanceled;

  /// No description provided for @localSearchScanComplete.
  ///
  /// In en, this message translates to:
  /// **'Scan complete'**
  String get localSearchScanComplete;

  /// No description provided for @localSearchNoResultsBody.
  ///
  /// In en, this message translates to:
  /// **'No personal information was found in the scanned files. Try using \"Full text parsing\" mode or adding more folders.'**
  String get localSearchNoResultsBody;

  /// No description provided for @localSearchNoFiles.
  ///
  /// In en, this message translates to:
  /// **'No files'**
  String get localSearchNoFiles;

  /// No description provided for @scanDeselectAll.
  ///
  /// In en, this message translates to:
  /// **'Deselect All'**
  String get scanDeselectAll;

  /// No description provided for @ocrScanDescription.
  ///
  /// In en, this message translates to:
  /// **'Scan passport, ID card, or any document'**
  String get ocrScanDescription;

  /// No description provided for @ocrNoTextDetected.
  ///
  /// In en, this message translates to:
  /// **'No text detected'**
  String get ocrNoTextDetected;

  /// No description provided for @ocrBusinessCardSaved.
  ///
  /// In en, this message translates to:
  /// **'Business card saved'**
  String get ocrBusinessCardSaved;

  /// No description provided for @ocrInvoiceSaved.
  ///
  /// In en, this message translates to:
  /// **'Invoice saved'**
  String get ocrInvoiceSaved;

  /// No description provided for @ocrDocumentSavedAsNote.
  ///
  /// In en, this message translates to:
  /// **'Document saved as a note'**
  String get ocrDocumentSavedAsNote;

  /// No description provided for @ocrBusinessCard.
  ///
  /// In en, this message translates to:
  /// **'Business Card'**
  String get ocrBusinessCard;

  /// No description provided for @ocrInvoice.
  ///
  /// In en, this message translates to:
  /// **'Invoice'**
  String get ocrInvoice;

  /// No description provided for @ocrResume.
  ///
  /// In en, this message translates to:
  /// **'Resume'**
  String get ocrResume;

  /// No description provided for @ocrNoResumeSections.
  ///
  /// In en, this message translates to:
  /// **'No resume sections detected'**
  String get ocrNoResumeSections;

  /// No description provided for @ocrResumeSaved.
  ///
  /// In en, this message translates to:
  /// **'Resume saved'**
  String get ocrResumeSaved;

  /// No description provided for @ocrResumeSavedSections.
  ///
  /// In en, this message translates to:
  /// **'Resume saved with {count} sections'**
  String ocrResumeSavedSections(int count);

  /// No description provided for @ocrSavedSectionsFailed.
  ///
  /// In en, this message translates to:
  /// **'Saved {success} sections, {fail} failed'**
  String ocrSavedSectionsFailed(int success, int fail);

  /// No description provided for @ocrScannedDocument.
  ///
  /// In en, this message translates to:
  /// **'Scanned Document'**
  String get ocrScannedDocument;

  /// No description provided for @ocrUseCamera.
  ///
  /// In en, this message translates to:
  /// **'Use camera to capture document'**
  String get ocrUseCamera;

  /// No description provided for @ocrPhotoOrPdf.
  ///
  /// In en, this message translates to:
  /// **'Photo or PDF file'**
  String get ocrPhotoOrPdf;

  /// No description provided for @commonShowMore.
  ///
  /// In en, this message translates to:
  /// **'Show {count} more'**
  String commonShowMore(int count);

  /// No description provided for @commonCopiedToClipboard.
  ///
  /// In en, this message translates to:
  /// **'Copied to clipboard'**
  String get commonCopiedToClipboard;

  /// No description provided for @commonUntitled.
  ///
  /// In en, this message translates to:
  /// **'Untitled'**
  String get commonUntitled;

  /// No description provided for @predefinedDeletedItem.
  ///
  /// In en, this message translates to:
  /// **'Deleted {title}: {name}'**
  String predefinedDeletedItem(String title, String name);

  /// No description provided for @commonErrorWithMessage.
  ///
  /// In en, this message translates to:
  /// **'Error: {message}'**
  String commonErrorWithMessage(String message);

  /// No description provided for @commonObject.
  ///
  /// In en, this message translates to:
  /// **'Object'**
  String get commonObject;

  /// No description provided for @fieldHistoryLatest.
  ///
  /// In en, this message translates to:
  /// **'Latest'**
  String get fieldHistoryLatest;

  /// No description provided for @commonEmpty.
  ///
  /// In en, this message translates to:
  /// **'(empty)'**
  String get commonEmpty;

  /// No description provided for @lockVaultMessage.
  ///
  /// In en, this message translates to:
  /// **'Locking the vault will require your master password to unlock again.'**
  String get lockVaultMessage;

  /// No description provided for @changePasswordWarning.
  ///
  /// In en, this message translates to:
  /// **'Changing your password will re-encrypt all your data with the new key.'**
  String get changePasswordWarning;

  /// No description provided for @changePasswordCurrentRequired.
  ///
  /// In en, this message translates to:
  /// **'Current password is required'**
  String get changePasswordCurrentRequired;

  /// No description provided for @changePasswordNewRequired.
  ///
  /// In en, this message translates to:
  /// **'New password is required'**
  String get changePasswordNewRequired;

  /// No description provided for @changePasswordMustDiffer.
  ///
  /// In en, this message translates to:
  /// **'New password must be different'**
  String get changePasswordMustDiffer;

  /// No description provided for @changePasswordFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to change password'**
  String get changePasswordFailed;

  /// No description provided for @entryAttachments.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 attachment} other{{count} attachments}}'**
  String entryAttachments(int count);

  /// No description provided for @profileEncryptionDesc.
  ///
  /// In en, this message translates to:
  /// **'Your data is encrypted with AES-256-GCM'**
  String get profileEncryptionDesc;

  /// No description provided for @financialEncryptionDesc.
  ///
  /// In en, this message translates to:
  /// **'Your financial data is encrypted with AES-256-GCM'**
  String get financialEncryptionDesc;

  /// No description provided for @llmStatsPrompt.
  ///
  /// In en, this message translates to:
  /// **'Prompt {tokens}'**
  String llmStatsPrompt(String tokens);

  /// No description provided for @llmStatsCompletion.
  ///
  /// In en, this message translates to:
  /// **'Completion {tokens}'**
  String llmStatsCompletion(String tokens);

  /// No description provided for @llmProviderOllama.
  ///
  /// In en, this message translates to:
  /// **'Ollama'**
  String get llmProviderOllama;

  /// No description provided for @homeNoMorePages.
  ///
  /// In en, this message translates to:
  /// **'No more pages to add'**
  String get homeNoMorePages;

  /// No description provided for @homeDefaultAccountName.
  ///
  /// In en, this message translates to:
  /// **'Account'**
  String get homeDefaultAccountName;

  /// No description provided for @syncEnterPairingKey.
  ///
  /// In en, this message translates to:
  /// **'Enter the pairing key shared from the other device.'**
  String get syncEnterPairingKey;

  /// No description provided for @ocrNoTextDetectedImage.
  ///
  /// In en, this message translates to:
  /// **'No text detected in the image. Please try again with a clearer photo of the document.'**
  String get ocrNoTextDetectedImage;

  /// No description provided for @ocrNoTextDetectedPdf.
  ///
  /// In en, this message translates to:
  /// **'No text detected in the PDF. Please try again with a clearer scanned document.'**
  String get ocrNoTextDetectedPdf;

  /// No description provided for @ocrRecognitionTimeoutImage.
  ///
  /// In en, this message translates to:
  /// **'Recognition timed out. Please try again with a clearer image.'**
  String get ocrRecognitionTimeoutImage;

  /// No description provided for @ocrRecognitionTimeoutPdf.
  ///
  /// In en, this message translates to:
  /// **'Recognition timed out. Please try again with a clearer PDF.'**
  String get ocrRecognitionTimeoutPdf;

  /// No description provided for @ocrPdfRenderFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to render PDF page. The file may be corrupted or password-protected.'**
  String get ocrPdfRenderFailed;

  /// No description provided for @commonAddItem.
  ///
  /// In en, this message translates to:
  /// **'Add Item'**
  String get commonAddItem;

  /// No description provided for @profileAddContact.
  ///
  /// In en, this message translates to:
  /// **'Add Contact'**
  String get profileAddContact;

  /// No description provided for @profileEditContact.
  ///
  /// In en, this message translates to:
  /// **'Edit Contact'**
  String get profileEditContact;

  /// No description provided for @commonAddButton.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get commonAddButton;

  /// No description provided for @profileIdCard.
  ///
  /// In en, this message translates to:
  /// **'ID Card'**
  String get profileIdCard;

  /// No description provided for @profileAddress.
  ///
  /// In en, this message translates to:
  /// **'Address'**
  String get profileAddress;

  /// No description provided for @syncScanHint.
  ///
  /// In en, this message translates to:
  /// **'Scan for nearby SoloSoul devices on your local network.'**
  String get syncScanHint;

  /// No description provided for @syncPairingHint.
  ///
  /// In en, this message translates to:
  /// **'Generate a shared pairing key to establish a secure connection between devices. Both devices must use the same key.'**
  String get syncPairingHint;

  /// No description provided for @syncTestSuccess.
  ///
  /// In en, this message translates to:
  /// **'Success'**
  String get syncTestSuccess;

  /// No description provided for @trashEmptyMessage.
  ///
  /// In en, this message translates to:
  /// **'Trash is empty'**
  String get trashEmptyMessage;

  /// No description provided for @accountDeviceCount.
  ///
  /// In en, this message translates to:
  /// **'{count} device(s)'**
  String accountDeviceCount(int count);

  /// No description provided for @profileEncryptionTitle.
  ///
  /// In en, this message translates to:
  /// **'End-to-End Encrypted'**
  String get profileEncryptionTitle;

  /// No description provided for @profileIdCardSection.
  ///
  /// In en, this message translates to:
  /// **'ID card'**
  String get profileIdCardSection;

  /// No description provided for @profileFormatIdentity.
  ///
  /// In en, this message translates to:
  /// **'Identity\n{data}'**
  String profileFormatIdentity(String data);

  /// No description provided for @profileFormatIdCard.
  ///
  /// In en, this message translates to:
  /// **'ID Card\n{data}'**
  String profileFormatIdCard(String data);

  /// No description provided for @travelFormatPassport.
  ///
  /// In en, this message translates to:
  /// **'Passport\n{data}'**
  String travelFormatPassport(String data);

  /// No description provided for @travelFormatVisa.
  ///
  /// In en, this message translates to:
  /// **'Visa\n{data}'**
  String travelFormatVisa(String data);

  /// No description provided for @travelFormatHistory.
  ///
  /// In en, this message translates to:
  /// **'Travel History\n{data}'**
  String travelFormatHistory(String data);

  /// No description provided for @financialFormatBankAccount.
  ///
  /// In en, this message translates to:
  /// **'Bank Account\n{data}'**
  String financialFormatBankAccount(String data);

  /// No description provided for @financialFormatCard.
  ///
  /// In en, this message translates to:
  /// **'Card\n{data}'**
  String financialFormatCard(String data);

  /// No description provided for @financialFormatTaxId.
  ///
  /// In en, this message translates to:
  /// **'Tax ID\n{data}'**
  String financialFormatTaxId(String data);

  /// No description provided for @professionalFormatEducation.
  ///
  /// In en, this message translates to:
  /// **'Education\n{data}'**
  String professionalFormatEducation(String data);

  /// No description provided for @professionalFormatEmployment.
  ///
  /// In en, this message translates to:
  /// **'Employment\n{data}'**
  String professionalFormatEmployment(String data);

  /// No description provided for @professionalFormatAward.
  ///
  /// In en, this message translates to:
  /// **'Award\n{data}'**
  String professionalFormatAward(String data);

  /// No description provided for @professionalFormatSkill.
  ///
  /// In en, this message translates to:
  /// **'Skill\n{data}'**
  String professionalFormatSkill(String data);

  /// No description provided for @professionalFormatLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language\n{data}'**
  String professionalFormatLanguage(String data);

  /// No description provided for @fieldFullName.
  ///
  /// In en, this message translates to:
  /// **'Full Name'**
  String get fieldFullName;

  /// No description provided for @fieldGivenName.
  ///
  /// In en, this message translates to:
  /// **'Given Name'**
  String get fieldGivenName;

  /// No description provided for @fieldFamilyName.
  ///
  /// In en, this message translates to:
  /// **'Family Name'**
  String get fieldFamilyName;

  /// No description provided for @fieldDateOfBirth.
  ///
  /// In en, this message translates to:
  /// **'Date of Birth'**
  String get fieldDateOfBirth;

  /// No description provided for @fieldGender.
  ///
  /// In en, this message translates to:
  /// **'Gender'**
  String get fieldGender;

  /// No description provided for @fieldNationality.
  ///
  /// In en, this message translates to:
  /// **'Nationality'**
  String get fieldNationality;

  /// No description provided for @fieldTitle.
  ///
  /// In en, this message translates to:
  /// **'Title'**
  String get fieldTitle;

  /// No description provided for @fieldType.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get fieldType;

  /// No description provided for @fieldValue.
  ///
  /// In en, this message translates to:
  /// **'Value'**
  String get fieldValue;

  /// No description provided for @fieldNumber.
  ///
  /// In en, this message translates to:
  /// **'Number'**
  String get fieldNumber;

  /// No description provided for @fieldIdCardNumber.
  ///
  /// In en, this message translates to:
  /// **'ID Card Number'**
  String get fieldIdCardNumber;

  /// No description provided for @fieldIssueDate.
  ///
  /// In en, this message translates to:
  /// **'Issue Date'**
  String get fieldIssueDate;

  /// No description provided for @fieldExpiryDate.
  ///
  /// In en, this message translates to:
  /// **'Expiry Date'**
  String get fieldExpiryDate;

  /// No description provided for @fieldHolderName.
  ///
  /// In en, this message translates to:
  /// **'Holder Name'**
  String get fieldHolderName;

  /// No description provided for @fieldCountry.
  ///
  /// In en, this message translates to:
  /// **'Country'**
  String get fieldCountry;

  /// No description provided for @fieldStreet.
  ///
  /// In en, this message translates to:
  /// **'Street'**
  String get fieldStreet;

  /// No description provided for @fieldCity.
  ///
  /// In en, this message translates to:
  /// **'City'**
  String get fieldCity;

  /// No description provided for @fieldState.
  ///
  /// In en, this message translates to:
  /// **'State'**
  String get fieldState;

  /// No description provided for @fieldPostalCode.
  ///
  /// In en, this message translates to:
  /// **'Postal Code'**
  String get fieldPostalCode;

  /// No description provided for @fieldPassportNumber.
  ///
  /// In en, this message translates to:
  /// **'Passport Number'**
  String get fieldPassportNumber;

  /// No description provided for @fieldIssuingCountry.
  ///
  /// In en, this message translates to:
  /// **'Issuing Country'**
  String get fieldIssuingCountry;

  /// No description provided for @fieldVisaNumber.
  ///
  /// In en, this message translates to:
  /// **'Visa Number'**
  String get fieldVisaNumber;

  /// No description provided for @fieldEntryDate.
  ///
  /// In en, this message translates to:
  /// **'Entry Date'**
  String get fieldEntryDate;

  /// No description provided for @fieldExitDate.
  ///
  /// In en, this message translates to:
  /// **'Exit Date'**
  String get fieldExitDate;

  /// No description provided for @fieldSwiftCode.
  ///
  /// In en, this message translates to:
  /// **'SWIFT Code'**
  String get fieldSwiftCode;

  /// No description provided for @fieldIban.
  ///
  /// In en, this message translates to:
  /// **'IBAN'**
  String get fieldIban;

  /// No description provided for @fieldCardNumber.
  ///
  /// In en, this message translates to:
  /// **'Card Number'**
  String get fieldCardNumber;

  /// No description provided for @fieldCardholderName.
  ///
  /// In en, this message translates to:
  /// **'Cardholder Name'**
  String get fieldCardholderName;

  /// No description provided for @fieldCvv.
  ///
  /// In en, this message translates to:
  /// **'CVV'**
  String get fieldCvv;

  /// No description provided for @fieldTaxIdNumber.
  ///
  /// In en, this message translates to:
  /// **'Tax ID Number'**
  String get fieldTaxIdNumber;

  /// No description provided for @fieldInstitution.
  ///
  /// In en, this message translates to:
  /// **'Institution'**
  String get fieldInstitution;

  /// No description provided for @fieldDegree.
  ///
  /// In en, this message translates to:
  /// **'Degree'**
  String get fieldDegree;

  /// No description provided for @fieldFieldOfStudy.
  ///
  /// In en, this message translates to:
  /// **'Field of Study'**
  String get fieldFieldOfStudy;

  /// No description provided for @fieldStartDate.
  ///
  /// In en, this message translates to:
  /// **'Start Date'**
  String get fieldStartDate;

  /// No description provided for @fieldEndDate.
  ///
  /// In en, this message translates to:
  /// **'End Date'**
  String get fieldEndDate;

  /// No description provided for @fieldCompany.
  ///
  /// In en, this message translates to:
  /// **'Company'**
  String get fieldCompany;

  /// No description provided for @fieldPosition.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get fieldPosition;

  /// No description provided for @fieldCategory.
  ///
  /// In en, this message translates to:
  /// **'Category'**
  String get fieldCategory;

  /// No description provided for @fieldLevel.
  ///
  /// In en, this message translates to:
  /// **'Level'**
  String get fieldLevel;

  /// No description provided for @fieldLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get fieldLanguage;

  /// No description provided for @fieldProficiency.
  ///
  /// In en, this message translates to:
  /// **'Proficiency'**
  String get fieldProficiency;

  /// No description provided for @fieldOrganization.
  ///
  /// In en, this message translates to:
  /// **'Organization'**
  String get fieldOrganization;

  /// No description provided for @fieldPhone.
  ///
  /// In en, this message translates to:
  /// **'Phone'**
  String get fieldPhone;

  /// No description provided for @fieldEmail.
  ///
  /// In en, this message translates to:
  /// **'Email'**
  String get fieldEmail;

  /// No description provided for @fieldContent.
  ///
  /// In en, this message translates to:
  /// **'Content'**
  String get fieldContent;

  /// No description provided for @fieldDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get fieldDone;

  /// No description provided for @fieldDueDate.
  ///
  /// In en, this message translates to:
  /// **'Due Date'**
  String get fieldDueDate;

  /// No description provided for @commonYes.
  ///
  /// In en, this message translates to:
  /// **'Yes'**
  String get commonYes;

  /// No description provided for @commonNo.
  ///
  /// In en, this message translates to:
  /// **'No'**
  String get commonNo;

  /// No description provided for @fieldCountryCode.
  ///
  /// In en, this message translates to:
  /// **'Country Code'**
  String get fieldCountryCode;

  /// No description provided for @fieldPlaceOfIssue.
  ///
  /// In en, this message translates to:
  /// **'Place of Issue'**
  String get fieldPlaceOfIssue;

  /// No description provided for @fieldPlaceOfBirth.
  ///
  /// In en, this message translates to:
  /// **'Place of Birth'**
  String get fieldPlaceOfBirth;

  /// No description provided for @fieldSex.
  ///
  /// In en, this message translates to:
  /// **'Sex'**
  String get fieldSex;

  /// No description provided for @fieldAuthority.
  ///
  /// In en, this message translates to:
  /// **'Authority'**
  String get fieldAuthority;

  /// No description provided for @fieldVisaType.
  ///
  /// In en, this message translates to:
  /// **'Visa Type'**
  String get fieldVisaType;

  /// No description provided for @fieldDestination.
  ///
  /// In en, this message translates to:
  /// **'Destination'**
  String get fieldDestination;

  /// No description provided for @fieldTravelType.
  ///
  /// In en, this message translates to:
  /// **'Travel Type'**
  String get fieldTravelType;

  /// No description provided for @fieldDepartureCity.
  ///
  /// In en, this message translates to:
  /// **'Departure City'**
  String get fieldDepartureCity;

  /// No description provided for @fieldDepartureTime.
  ///
  /// In en, this message translates to:
  /// **'Departure Time'**
  String get fieldDepartureTime;

  /// No description provided for @fieldArrivalTime.
  ///
  /// In en, this message translates to:
  /// **'Arrival Time'**
  String get fieldArrivalTime;

  /// No description provided for @fieldFlightNumber.
  ///
  /// In en, this message translates to:
  /// **'Flight Number'**
  String get fieldFlightNumber;

  /// No description provided for @fieldTicketPrice.
  ///
  /// In en, this message translates to:
  /// **'Ticket Price'**
  String get fieldTicketPrice;

  /// No description provided for @fieldAirline.
  ///
  /// In en, this message translates to:
  /// **'Airline'**
  String get fieldAirline;

  /// No description provided for @fieldCurrency.
  ///
  /// In en, this message translates to:
  /// **'Currency'**
  String get fieldCurrency;

  /// No description provided for @fieldSwiftBic.
  ///
  /// In en, this message translates to:
  /// **'SWIFT/BIC'**
  String get fieldSwiftBic;

  /// No description provided for @fieldSortCode.
  ///
  /// In en, this message translates to:
  /// **'Sort Code'**
  String get fieldSortCode;

  /// No description provided for @fieldCardType.
  ///
  /// In en, this message translates to:
  /// **'Card Type'**
  String get fieldCardType;

  /// No description provided for @fieldTaxIdType.
  ///
  /// In en, this message translates to:
  /// **'Tax ID Type'**
  String get fieldTaxIdType;

  /// No description provided for @fieldIssuingAuthority.
  ///
  /// In en, this message translates to:
  /// **'Issuing Authority'**
  String get fieldIssuingAuthority;

  /// No description provided for @fieldDegreeCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom Degree'**
  String get fieldDegreeCustom;

  /// No description provided for @fieldField.
  ///
  /// In en, this message translates to:
  /// **'Field of Study'**
  String get fieldField;

  /// No description provided for @fieldResponsibilities.
  ///
  /// In en, this message translates to:
  /// **'Responsibilities'**
  String get fieldResponsibilities;

  /// No description provided for @fieldIssuer.
  ///
  /// In en, this message translates to:
  /// **'Issuer'**
  String get fieldIssuer;

  /// No description provided for @fieldDescription.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get fieldDescription;

  /// No description provided for @fieldName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get fieldName;

  /// No description provided for @datePickerSelectDate.
  ///
  /// In en, this message translates to:
  /// **'Select date'**
  String get datePickerSelectDate;

  /// No description provided for @headerSensitiveAccessLocked.
  ///
  /// In en, this message translates to:
  /// **'Sensitive access locked'**
  String get headerSensitiveAccessLocked;

  /// No description provided for @operationLogPropertySnapshot.
  ///
  /// In en, this message translates to:
  /// **'Property Snapshot'**
  String get operationLogPropertySnapshot;

  /// No description provided for @dataMgmtVaultDataSize.
  ///
  /// In en, this message translates to:
  /// **'Vault data size'**
  String get dataMgmtVaultDataSize;

  /// No description provided for @dataMgmtAppVersion.
  ///
  /// In en, this message translates to:
  /// **'App version'**
  String get dataMgmtAppVersion;

  /// No description provided for @dataMgmtRestoreOverwrite.
  ///
  /// In en, this message translates to:
  /// **'This will overwrite your current data with the backup from {time}. A safety backup of the current state will be created first.'**
  String dataMgmtRestoreOverwrite(String time);

  /// No description provided for @dataMgmtRestoreSuccess.
  ///
  /// In en, this message translates to:
  /// **'Restore successful. Please restart the app.'**
  String get dataMgmtRestoreSuccess;

  /// No description provided for @dataMgmtRestoreFailed.
  ///
  /// In en, this message translates to:
  /// **'Restore failed'**
  String get dataMgmtRestoreFailed;

  /// No description provided for @dataMgmtDeleteBackupConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete backup from {time}?'**
  String dataMgmtDeleteBackupConfirm(String time);

  /// No description provided for @dataMgmtBackupCreated.
  ///
  /// In en, this message translates to:
  /// **'Backup created successfully'**
  String get dataMgmtBackupCreated;

  /// No description provided for @dataMgmtBackupFailed.
  ///
  /// In en, this message translates to:
  /// **'Backup failed'**
  String get dataMgmtBackupFailed;

  /// No description provided for @dataMgmtBackupError.
  ///
  /// In en, this message translates to:
  /// **'Backup error: {error}'**
  String dataMgmtBackupError(String error);

  /// No description provided for @dataMgmtBackupDeleted.
  ///
  /// In en, this message translates to:
  /// **'Backup deleted'**
  String get dataMgmtBackupDeleted;

  /// No description provided for @dataMgmtOperationCreatedBackup.
  ///
  /// In en, this message translates to:
  /// **'Created backup'**
  String get dataMgmtOperationCreatedBackup;

  /// No description provided for @dataMgmtOperationRestoredBackup.
  ///
  /// In en, this message translates to:
  /// **'Restored backup'**
  String get dataMgmtOperationRestoredBackup;

  /// No description provided for @dataMgmtOperationDeletedBackup.
  ///
  /// In en, this message translates to:
  /// **'Deleted backup'**
  String get dataMgmtOperationDeletedBackup;

  /// No description provided for @dataMgmtOperationPromotedBackup.
  ///
  /// In en, this message translates to:
  /// **'Promoted backup to special'**
  String get dataMgmtOperationPromotedBackup;

  /// No description provided for @dataMgmtOperationCreatedSpecial.
  ///
  /// In en, this message translates to:
  /// **'Created special backup'**
  String get dataMgmtOperationCreatedSpecial;

  /// No description provided for @dataMgmtOperationRenamedSpecial.
  ///
  /// In en, this message translates to:
  /// **'Renamed special backup'**
  String get dataMgmtOperationRenamedSpecial;

  /// No description provided for @dataMgmtOperationRestoredSpecial.
  ///
  /// In en, this message translates to:
  /// **'Restored special backup'**
  String get dataMgmtOperationRestoredSpecial;

  /// No description provided for @dataMgmtSpecialBackupSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved as special backup \"{name}\"'**
  String dataMgmtSpecialBackupSaved(String name);

  /// No description provided for @dataMgmtSpecialBackupFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to save as special backup'**
  String get dataMgmtSpecialBackupFailed;

  /// No description provided for @dataMgmtSpecialBackupCreated.
  ///
  /// In en, this message translates to:
  /// **'Special backup \"{name}\" created'**
  String dataMgmtSpecialBackupCreated(String name);

  /// No description provided for @dataMgmtSpecialBackupCreateFailed.
  ///
  /// In en, this message translates to:
  /// **'Special backup failed'**
  String get dataMgmtSpecialBackupCreateFailed;

  /// No description provided for @dataMgmtRenamedTo.
  ///
  /// In en, this message translates to:
  /// **'Renamed to \"{name}\"'**
  String dataMgmtRenamedTo(String name);

  /// No description provided for @dataMgmtSpecialBackupLimit.
  ///
  /// In en, this message translates to:
  /// **'You can keep up to {max} special backups. Please delete an existing one before creating a new special backup.'**
  String dataMgmtSpecialBackupLimit(int max);

  /// No description provided for @dataMgmtSpecialBackupPromoteLimit.
  ///
  /// In en, this message translates to:
  /// **'You can keep up to {max} special backups. Please delete an existing one before promoting.'**
  String dataMgmtSpecialBackupPromoteLimit(int max);

  /// No description provided for @dataMgmtSafetyBackupNotice.
  ///
  /// In en, this message translates to:
  /// **'A safety backup of the current state will be created first.'**
  String get dataMgmtSafetyBackupNotice;

  /// No description provided for @dataMgmtSpecialBackupRestored.
  ///
  /// In en, this message translates to:
  /// **'Special backup restored. Please restart the app.'**
  String get dataMgmtSpecialBackupRestored;

  /// No description provided for @dataMgmtOperationDeletedSpecial.
  ///
  /// In en, this message translates to:
  /// **'Deleted special backup'**
  String get dataMgmtOperationDeletedSpecial;

  /// No description provided for @dataMgmtVaultSize.
  ///
  /// In en, this message translates to:
  /// **'Vault size: '**
  String get dataMgmtVaultSize;

  /// No description provided for @dataMgmtBackupEncryptionDesc.
  ///
  /// In en, this message translates to:
  /// **'Backups are encrypted with your vault key. Auto-backup runs on every unlock.'**
  String get dataMgmtBackupEncryptionDesc;

  /// No description provided for @dataMgmtRegularBackups.
  ///
  /// In en, this message translates to:
  /// **'Regular Backups'**
  String get dataMgmtRegularBackups;

  /// No description provided for @dataMgmtNoBackups.
  ///
  /// In en, this message translates to:
  /// **'No backups yet'**
  String get dataMgmtNoBackups;

  /// No description provided for @loginDataYourControl.
  ///
  /// In en, this message translates to:
  /// **'Your data, your control'**
  String get loginDataYourControl;

  /// No description provided for @loginEnterMasterPassword.
  ///
  /// In en, this message translates to:
  /// **'Enter Master Password'**
  String get loginEnterMasterPassword;

  /// No description provided for @loginUnlockYourVault.
  ///
  /// In en, this message translates to:
  /// **'Unlock your vault'**
  String get loginUnlockYourVault;

  /// No description provided for @loginUnlockButton.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get loginUnlockButton;

  /// No description provided for @loginNoPasswordRecovery.
  ///
  /// In en, this message translates to:
  /// **'There is no password recovery. If you forget your master password, your data cannot be accessed.'**
  String get loginNoPasswordRecovery;

  /// No description provided for @loginPleaseEnterPassword.
  ///
  /// In en, this message translates to:
  /// **'Please enter your password'**
  String get loginPleaseEnterPassword;

  /// No description provided for @securityBiometricUnlockSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Use Face ID / Touch ID to unlock'**
  String get securityBiometricUnlockSubtitle;

  /// No description provided for @securityBiometricNotAvailable.
  ///
  /// In en, this message translates to:
  /// **'Biometrics not available on this device'**
  String get securityBiometricNotAvailable;

  /// No description provided for @scanAttachFile.
  ///
  /// In en, this message translates to:
  /// **'Attach original file'**
  String get scanAttachFile;

  /// Relative time: days ago in trash card
  ///
  /// In en, this message translates to:
  /// **'{count}d ago'**
  String trashDaysAgo(Object count);

  /// Relative time: hours ago in trash card
  ///
  /// In en, this message translates to:
  /// **'{count}h ago'**
  String trashHoursAgo(Object count);

  /// Relative time: minutes ago in trash card
  ///
  /// In en, this message translates to:
  /// **'{count}m ago'**
  String trashMinutesAgo(Object count);

  /// No description provided for @trashJustNow.
  ///
  /// In en, this message translates to:
  /// **'Just now'**
  String get trashJustNow;

  /// No description provided for @trashDeletedRecently.
  ///
  /// In en, this message translates to:
  /// **'Deleted recently'**
  String get trashDeletedRecently;

  /// Trash card subtitle indicating when item was deleted
  ///
  /// In en, this message translates to:
  /// **'Deleted {time} ago'**
  String trashDeletedAgo(Object time);

  /// Tooltip for expand button on deleted page trash card
  ///
  /// In en, this message translates to:
  /// **'Show sections'**
  String get trashShowSections;

  /// Tooltip for expand button on deleted section trash card
  ///
  /// In en, this message translates to:
  /// **'Show items'**
  String get trashShowItems;

  /// Display name for 'collection' typeId
  ///
  /// In en, this message translates to:
  /// **'Section'**
  String get typeCollection;

  /// Display name for 'page' typeId
  ///
  /// In en, this message translates to:
  /// **'Page'**
  String get typePage;

  /// Display name for 'item' typeId
  ///
  /// In en, this message translates to:
  /// **'Item'**
  String get typeItem;

  /// Fallback display name for unknown typeId
  ///
  /// In en, this message translates to:
  /// **'Item'**
  String get typeUnknown;

  /// Platform label: macOS
  ///
  /// In en, this message translates to:
  /// **'macOS'**
  String get operationPlatformMacos;

  /// Platform label: iOS
  ///
  /// In en, this message translates to:
  /// **'iOS'**
  String get operationPlatformIos;

  /// Platform label: Windows
  ///
  /// In en, this message translates to:
  /// **'Windows'**
  String get operationPlatformWindows;

  /// Platform label: Linux
  ///
  /// In en, this message translates to:
  /// **'Linux'**
  String get operationPlatformLinux;

  /// No description provided for @logSectionIdentity.
  ///
  /// In en, this message translates to:
  /// **'Identity'**
  String get logSectionIdentity;

  /// No description provided for @logSectionContactInfo.
  ///
  /// In en, this message translates to:
  /// **'Contact'**
  String get logSectionContactInfo;

  /// No description provided for @logSectionAddress.
  ///
  /// In en, this message translates to:
  /// **'Address'**
  String get logSectionAddress;

  /// No description provided for @logSectionIdCard.
  ///
  /// In en, this message translates to:
  /// **'ID Card'**
  String get logSectionIdCard;

  /// No description provided for @logSectionPassport.
  ///
  /// In en, this message translates to:
  /// **'Passport'**
  String get logSectionPassport;

  /// No description provided for @logSectionVisa.
  ///
  /// In en, this message translates to:
  /// **'Visa'**
  String get logSectionVisa;

  /// No description provided for @logSectionTravelHistory.
  ///
  /// In en, this message translates to:
  /// **'Travel History'**
  String get logSectionTravelHistory;

  /// No description provided for @logSectionBankAccount.
  ///
  /// In en, this message translates to:
  /// **'Bank Account'**
  String get logSectionBankAccount;

  /// No description provided for @logSectionCard.
  ///
  /// In en, this message translates to:
  /// **'Card'**
  String get logSectionCard;

  /// No description provided for @logSectionEducation.
  ///
  /// In en, this message translates to:
  /// **'Education'**
  String get logSectionEducation;

  /// No description provided for @logSectionEmployment.
  ///
  /// In en, this message translates to:
  /// **'Employment'**
  String get logSectionEmployment;

  /// No description provided for @logSectionSkill.
  ///
  /// In en, this message translates to:
  /// **'Skill'**
  String get logSectionSkill;

  /// No description provided for @logSectionLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get logSectionLanguage;

  /// No description provided for @logSectionTravel.
  ///
  /// In en, this message translates to:
  /// **'Travel'**
  String get logSectionTravel;

  /// No description provided for @logSectionFinancial.
  ///
  /// In en, this message translates to:
  /// **'Financial'**
  String get logSectionFinancial;

  /// No description provided for @logSectionProfessional.
  ///
  /// In en, this message translates to:
  /// **'Professional'**
  String get logSectionProfessional;

  /// No description provided for @logSectionSensitivity.
  ///
  /// In en, this message translates to:
  /// **'Sensitivity'**
  String get logSectionSensitivity;

  /// No description provided for @logSectionCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get logSectionCustom;

  /// No description provided for @logSectionDefault.
  ///
  /// In en, this message translates to:
  /// **'Section'**
  String get logSectionDefault;

  /// No description provided for @operationFilterLabel.
  ///
  /// In en, this message translates to:
  /// **'Filter'**
  String get operationFilterLabel;

  /// No description provided for @trashFilterLabel.
  ///
  /// In en, this message translates to:
  /// **'Filter:'**
  String get trashFilterLabel;

  /// No description provided for @trashTimeFilterLabel.
  ///
  /// In en, this message translates to:
  /// **'Time:'**
  String get trashTimeFilterLabel;

  /// No description provided for @trashTimeFilterAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get trashTimeFilterAll;

  /// No description provided for @trashTimeFilter10Days.
  ///
  /// In en, this message translates to:
  /// **'Within 10 days'**
  String get trashTimeFilter10Days;

  /// No description provided for @trashTimeFilter1Day.
  ///
  /// In en, this message translates to:
  /// **'Within 1 day'**
  String get trashTimeFilter1Day;

  /// No description provided for @trashTimeFilter6Hours.
  ///
  /// In en, this message translates to:
  /// **'Within 6 hours'**
  String get trashTimeFilter6Hours;

  /// No description provided for @trashTimeFilter1Hour.
  ///
  /// In en, this message translates to:
  /// **'Within 1 hour'**
  String get trashTimeFilter1Hour;

  /// No description provided for @trashTypeFilterLabel.
  ///
  /// In en, this message translates to:
  /// **'Type:'**
  String get trashTypeFilterLabel;

  /// No description provided for @sectionTemplateTitle.
  ///
  /// In en, this message translates to:
  /// **'Section Template'**
  String get sectionTemplateTitle;

  /// No description provided for @sectionTemplateSelected.
  ///
  /// In en, this message translates to:
  /// **'{count} selected'**
  String sectionTemplateSelected(int count);

  /// No description provided for @sectionTemplateSelectButton.
  ///
  /// In en, this message translates to:
  /// **'Select Template'**
  String get sectionTemplateSelectButton;

  /// No description provided for @sectionTemplateEmpty.
  ///
  /// In en, this message translates to:
  /// **'No templates available'**
  String get sectionTemplateEmpty;

  /// No description provided for @sectionTemplateEmptyHint.
  ///
  /// In en, this message translates to:
  /// **'Templates will appear here once configured'**
  String get sectionTemplateEmptyHint;

  /// No description provided for @sectionTemplateApplied.
  ///
  /// In en, this message translates to:
  /// **'Template \"{name}\" applied'**
  String sectionTemplateApplied(String name);

  /// No description provided for @templateChinaBankAccountName.
  ///
  /// In en, this message translates to:
  /// **'China Bank Account'**
  String get templateChinaBankAccountName;

  /// No description provided for @templateChinaBankAccountDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains Chinese bank account information'**
  String get templateChinaBankAccountDesc;

  /// No description provided for @templateUkBankAccountName.
  ///
  /// In en, this message translates to:
  /// **'UK Bank Account'**
  String get templateUkBankAccountName;

  /// No description provided for @templateUkBankAccountDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains UK bank account information (Sort Code + Account Number)'**
  String get templateUkBankAccountDesc;

  /// No description provided for @templateUsBankAccountName.
  ///
  /// In en, this message translates to:
  /// **'US Bank Account'**
  String get templateUsBankAccountName;

  /// No description provided for @templateUsBankAccountDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains US bank account information (Routing Number + Account Number)'**
  String get templateUsBankAccountDesc;

  /// No description provided for @sectionTemplateFieldCount.
  ///
  /// In en, this message translates to:
  /// **'{count} fields'**
  String sectionTemplateFieldCount(int count);

  /// No description provided for @objectEditorDefaultFieldTitle.
  ///
  /// In en, this message translates to:
  /// **'Title'**
  String get objectEditorDefaultFieldTitle;

  /// No description provided for @objectEditorDefaultFieldItemName.
  ///
  /// In en, this message translates to:
  /// **'Item Name'**
  String get objectEditorDefaultFieldItemName;

  /// No description provided for @objectEditorMaxLength.
  ///
  /// In en, this message translates to:
  /// **'{count}'**
  String objectEditorMaxLength(int count);

  /// No description provided for @fieldBankName.
  ///
  /// In en, this message translates to:
  /// **'Bank Name'**
  String get fieldBankName;

  /// No description provided for @fieldAccountNumber.
  ///
  /// In en, this message translates to:
  /// **'Account Number'**
  String get fieldAccountNumber;

  /// No description provided for @fieldAccountHolder.
  ///
  /// In en, this message translates to:
  /// **'Account Holder'**
  String get fieldAccountHolder;

  /// No description provided for @fieldBranchName.
  ///
  /// In en, this message translates to:
  /// **'Branch Name'**
  String get fieldBranchName;

  /// No description provided for @fieldRoutingNumber.
  ///
  /// In en, this message translates to:
  /// **'Routing Number'**
  String get fieldRoutingNumber;

  /// No description provided for @fieldAccountType.
  ///
  /// In en, this message translates to:
  /// **'Account Type'**
  String get fieldAccountType;

  /// No description provided for @fieldChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking'**
  String get fieldChecking;

  /// No description provided for @fieldSavings.
  ///
  /// In en, this message translates to:
  /// **'Savings'**
  String get fieldSavings;

  /// No description provided for @templateProfileIdentityName.
  ///
  /// In en, this message translates to:
  /// **'Identity'**
  String get templateProfileIdentityName;

  /// No description provided for @templateProfileIdentityDesc.
  ///
  /// In en, this message translates to:
  /// **'Personal identity information including name, date of birth, gender, and nationality'**
  String get templateProfileIdentityDesc;

  /// No description provided for @templateProfileContactName.
  ///
  /// In en, this message translates to:
  /// **'Contact Information'**
  String get templateProfileContactName;

  /// No description provided for @templateProfileContactDesc.
  ///
  /// In en, this message translates to:
  /// **'Contact details such as phone number and email address'**
  String get templateProfileContactDesc;

  /// No description provided for @templateProfileIdCardName.
  ///
  /// In en, this message translates to:
  /// **'Identity Documents'**
  String get templateProfileIdCardName;

  /// No description provided for @templateProfileIdCardDesc.
  ///
  /// In en, this message translates to:
  /// **'Identity documents including ID cards, driver\'s license, and passport'**
  String get templateProfileIdCardDesc;

  /// No description provided for @templateProfileAddressName.
  ///
  /// In en, this message translates to:
  /// **'Addresses'**
  String get templateProfileAddressName;

  /// No description provided for @templateProfileAddressDesc.
  ///
  /// In en, this message translates to:
  /// **'Physical addresses including street, city, state, and postal code'**
  String get templateProfileAddressDesc;

  /// No description provided for @templateFinancialBankAccountName.
  ///
  /// In en, this message translates to:
  /// **'Bank Account'**
  String get templateFinancialBankAccountName;

  /// No description provided for @templateFinancialBankAccountDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains bank account information including account number and SWIFT code'**
  String get templateFinancialBankAccountDesc;

  /// No description provided for @templateFinancialCardName.
  ///
  /// In en, this message translates to:
  /// **'Card'**
  String get templateFinancialCardName;

  /// No description provided for @templateFinancialCardDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains payment card information including card number and CVV'**
  String get templateFinancialCardDesc;

  /// No description provided for @templateFinancialTaxIdName.
  ///
  /// In en, this message translates to:
  /// **'Tax Identification'**
  String get templateFinancialTaxIdName;

  /// No description provided for @templateFinancialTaxIdDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains tax identification information'**
  String get templateFinancialTaxIdDesc;

  /// No description provided for @templateProfessionalEducationName.
  ///
  /// In en, this message translates to:
  /// **'Education'**
  String get templateProfessionalEducationName;

  /// No description provided for @templateProfessionalEducationDesc.
  ///
  /// In en, this message translates to:
  /// **'Records of formal education including degrees, institutions, and fields of study'**
  String get templateProfessionalEducationDesc;

  /// No description provided for @templateProfessionalEmploymentName.
  ///
  /// In en, this message translates to:
  /// **'Employment'**
  String get templateProfessionalEmploymentName;

  /// No description provided for @templateProfessionalEmploymentDesc.
  ///
  /// In en, this message translates to:
  /// **'Work history including company, position, responsibilities, and tenure'**
  String get templateProfessionalEmploymentDesc;

  /// No description provided for @templateProfessionalSkillName.
  ///
  /// In en, this message translates to:
  /// **'Skill'**
  String get templateProfessionalSkillName;

  /// No description provided for @templateProfessionalSkillDesc.
  ///
  /// In en, this message translates to:
  /// **'Professional skills and proficiency levels'**
  String get templateProfessionalSkillDesc;

  /// No description provided for @templateProfessionalLanguageName.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get templateProfessionalLanguageName;

  /// No description provided for @templateProfessionalLanguageDesc.
  ///
  /// In en, this message translates to:
  /// **'Languages and proficiency levels'**
  String get templateProfessionalLanguageDesc;

  /// No description provided for @templateProfessionalAwardName.
  ///
  /// In en, this message translates to:
  /// **'Award'**
  String get templateProfessionalAwardName;

  /// No description provided for @templateProfessionalAwardDesc.
  ///
  /// In en, this message translates to:
  /// **'Professional awards, honors, and recognitions'**
  String get templateProfessionalAwardDesc;

  /// No description provided for @templateTravelPassportName.
  ///
  /// In en, this message translates to:
  /// **'Passport'**
  String get templateTravelPassportName;

  /// No description provided for @templateTravelPassportDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains passport information including number, issue date, expiry date, and holder details'**
  String get templateTravelPassportDesc;

  /// No description provided for @templateTravelVisaName.
  ///
  /// In en, this message translates to:
  /// **'Visa'**
  String get templateTravelVisaName;

  /// No description provided for @templateTravelVisaDesc.
  ///
  /// In en, this message translates to:
  /// **'Contains visa information including type, number, issue date, and expiry date'**
  String get templateTravelVisaDesc;

  /// No description provided for @templateTravelHistoryName.
  ///
  /// In en, this message translates to:
  /// **'Travel History'**
  String get templateTravelHistoryName;

  /// No description provided for @templateTravelHistoryDesc.
  ///
  /// In en, this message translates to:
  /// **'Records of travel including destination, dates, flights, and travel details'**
  String get templateTravelHistoryDesc;
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
