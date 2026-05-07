# Flutter i18n 完整迁移 TODO

> 状态追踪：已完成 ~100 key，剩余 ~310 处字符串待迁移。每完成一批勾选对应复选框并 commit。
>
> 工作流：①修改 ARB → `flutter gen-l10n` → ②修改 Dart → `flutter analyze` → ③commit

---

## 批次 1：P0 — 高频核心页面

### 1.1 `presentation/pages/login_page.dart`

- [ ] line ~119 `title: 'Data Recovery'` → `loginDataRecoveryTitle`
- [ ] line ~121 `message: 'Your vault appears to be empty...'` → `loginDataRecoveryMessage`
- [ ] line ~125 `label: 'Skip'` → `loginSkip`
- [ ] line ~129 `label: 'Restore Backup'` → `loginRestoreBackup`
- [ ] line ~147 `content: Text('Restore successful...')` → `loginRestoreSuccess`
- [ ] line ~156 `content: Text('Restore failed')` → `loginRestoreFailed`
- [ ] line ~239-248 Biometric 类型标签 `'Biometric'`/`'Face ID'`/`'Touch ID'`/`'Iris'` → `loginBiometricGeneric` / `loginBiometricFaceId` / `loginBiometricTouchId` / `loginBiometricIris`
- [ ] line ~265 `reason: 'Unlock SoloSoul with $_biometricType'` → `loginUnlockReason`
- [ ] line ~273 `content: 'Biometric authentication failed...'` → `loginBiometricFailed`
- [ ] line ~289 `content: 'Failed to unlock vault...'` → `loginUnlockFailedUsePassword`
- [ ] line ~322 `content: 'Biometric unlock error: $e'` → `loginBiometricUnlockError`
- [ ] line ~349-357 `_formatLastAccessed` 返回值 `'Never'`/`'Today'`/`'Yesterday'`/`'N days ago'` → 保留方法，内部字符串替换为 ARB key 或 intl DateFormat
- [ ] line ~424 `_passwordErrorMessage = 'Password must be at least 8 characters'` → `loginPasswordMinLength`
- [ ] line ~455 `_passwordErrorMessage = 'Invalid master password'` → `loginInvalidPassword`
- [ ] line ~456 `_passwordErrorMessage = 'Unlock failed: $specificError'` → `loginUnlockFailed`
- [ ] line ~473 `_createError = 'Account name is required'` → `loginAccountNameRequired`
- [ ] line ~477 `_createError = 'Password must be at least 8 characters'` → `loginPasswordMinLength`（复用）
- [ ] line ~481 `_createError = 'Passwords do not match'` → `loginPasswordsDoNotMatch`
- [ ] line ~505 `_createError = result.error ?? 'Failed to create account'` → `loginCreateAccountFailed`
- [ ] line ~520 `_createError = 'Failed to unlock vault. Please try again.'` → `loginUnlockVaultFailed`
- [ ] line ~599 `Text('Password Hint: $hint')` → `loginPasswordHint`（参数化）
- [ ] line ~925 `error: (error, _) => Center(child: Text('Error: $error'))` → `commonError`（复用）

### 1.2 `presentation/pages/object_workspace_page.dart`

- [ ] line ~72 `title = currentObject?.name ?? 'Objects'` → `workspaceObjects`
- [ ] line ~84 `'No items yet'` → `workspaceNoItems`
- [ ] line ~85 `'No objects yet'` → `workspaceNoObjects`
- [ ] line ~87 `'Add your first item'` → `workspaceAddFirstItem`
- [ ] line ~88 `'Create your first object to get started'` → `workspaceCreateFirstObject`
- [ ] line ~146 `tooltip: 'Delete'` → `workspaceDelete`（复用 commonDelete 或新建）
- [ ] line ~155 `tooltip: 'Edit Page'` → `workspaceEditPage`
- [ ] line ~164 `tooltip: _isReordering ? 'Done' : 'Reorder'` → `workspaceDone` / `workspaceReorder`
- [ ] line ~174 `tooltip: 'Add'` → `workspaceAdd`
- [ ] line ~208 `title: const Text('Delete Section')` → `workspaceDeleteSectionTitle`
- [ ] line ~209-210 `content: Text('Are you sure you want to delete...')` → `workspaceDeleteSectionConfirm`（参数化 `{name}`）
- [ ] line ~215 `child: const Text('Cancel')` → `commonCancel`（复用）
- [ ] line ~220 `child: const Text('Delete')` → `commonDelete`（复用）
- [ ] line ~230 `content: Text('Section deleted')` → `workspaceSectionDeleted`
- [ ] line ~244 `title: Text(object.typeId == 'page' ? 'Delete Page' : 'Delete Section')` → `workspaceDeletePageTitle` / `workspaceDeleteSectionTitle`
- [ ] line ~245-247 `content: Text(...)` → `workspaceDeletePageConfirm` / `workspaceDeleteSectionConfirm`
- [ ] line ~252 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~257 `child: const Text('Delete')` → `commonDelete`
- [ ] line ~268 `content: Text('"${object.name}" moved to trash')` → `workspaceMovedToTrash`（参数化 `{name}`）
- [ ] line ~298 `title: const Text('Add Sub-Page')` → `workspaceAddSubPage`
- [ ] line ~303 `title: const Text('Add Section')` → `workspaceAddSection`
- [ ] line ~346 `title: const Text('Add Section')` → `workspaceAddSectionDialog`
- [ ] line ~356 `labelText: 'Name'` → `workspaceSectionName`
- [ ] line ~357 `hintText: 'Enter section name'` → `workspaceEnterSectionName`
- [ ] line ~362 `Text('Icon', ...)` → `workspaceIcon`
- [ ] line ~406 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~417 `child: const Text('Add Section')` → `workspaceAddSectionButton`

### 1.3 `presentation/pages/object_editor_page.dart`

- [ ] line ~184 `title: Text(_isEditing ? 'Edit Section' : 'New Section')` → `objectEditorEditSection` / `objectEditorNewSection`
- [ ] line ~203 `Text('Type', ...)` → `objectEditorType`
- [ ] line ~271 `content: Text('Name is required')` → `objectEditorNameRequired`
- [ ] line ~288 `content: Text('Duplicate property names: ...')` → `objectEditorDuplicateProperties`
- [ ] line ~349 `content: Text('Failed to save: $e')` → `objectEditorSaveFailed`
- [ ] line ~377 `Text('Icon', ...)` → `objectEditorIcon`
- [ ] line ~380 `child: Text('Name', ...)` → `objectEditorName`
- [ ] line ~428 `hintText: 'Enter section name'` → `objectEditorEnterSectionName`
- [ ] line ~463 `hintText: 'Select type'` → `objectEditorSelectType`
- [ ] line ~469 `hint: const Text('Select type')` → `objectEditorSelectType`
- [ ] line ~520 `hintText: 'No parent (root)'` → `objectEditorNoParent`
- [ ] line ~526 `hint: const Text('No parent (root)')` → `objectEditorNoParent`
- [ ] line ~530 `child: Text('No parent (root)')` → `objectEditorNoParent`
- [ ] line ~582 `child: const Text('Save')` → `commonSave`
- [ ] line ~612 `Text('Item Properties', ...)` → `objectEditorItemProperties`
- [ ] line ~616 `tooltip: 'Add Property'` → `objectEditorAddProperty`
- [ ] line ~708 `hintText: 'Key name'` → `objectEditorKeyName`
- [ ] line ~773 `DropdownMenuItem ... Text('Text')` → `objectEditorPropertyTypeText`
- [ ] line ~774 `DropdownMenuItem ... Text('Date')` → `objectEditorPropertyTypeDate`
- [ ] line ~775 `DropdownMenuItem ... Text('Number')` → `objectEditorPropertyTypeNumber`
- [ ] line ~776 `DropdownMenuItem ... Text('Checkbox')` → `objectEditorPropertyTypeCheckbox`
- [ ] line ~792 `tooltip: 'Sensitivity'` → `objectEditorSensitivity`
- [ ] line ~827 `tooltip: 'Delete'` → `commonDelete`
- [ ] line ~833 `title: const Text('Delete Property')` → `objectEditorDeletePropertyTitle`
- [ ] line ~834 `content: Text('Are you sure you want to delete "$keyName"?')` → `objectEditorDeletePropertyConfirm`（参数化 `{name}`）
- [ ] line ~838 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~842 `child: const Text('Delete', ...)` → `commonDelete`

### 1.4 `presentation/pages/page_editor_page.dart`

- [ ] line ~68 `content: Text('Name is required')` → `pageEditorNameRequired`
- [ ] line ~109 `title: Text(_isEditing ? 'Edit Page' : 'New Page')` → `pageEditorEditPage` / `pageEditorNewPage`
- [ ] line ~117 `Text('Name', ...)` → `pageEditorName`
- [ ] line ~122 `hintText: 'Enter page name'` → `pageEditorEnterPageName`
- [ ] line ~129 `Text('Icon', ...)` → `pageEditorIcon`
- [ ] line ~165 `Text('Parent', ...)` → `pageEditorParent`
- [ ] line ~182 `child: const Text('Save')` → `commonSave`

### 1.5 `presentation/pages/home_page.dart`

- [ ] line ~52 `title: Text('SoloSoul')` → `mainAppTitle`（复用）
- [ ] line ~66 `label: const Text('Scan')` → `homeScan`
- [ ] line ~413 `Text('Quick Actions', ...)` → `homeQuickActions`
- [ ] line ~422 `tooltip: _isEditing ? 'Done' : 'Edit quick actions'` → `homeEditQuickActionsDone` / `homeEditQuickActions`
- [ ] line ~461 `Text('Security Status', ...)` → `homeSecurityStatus`

### 1.6 `presentation/pages/search_page.dart`

- [ ] line ~46 `title: const Text('Search')` → `searchTitle`
- [ ] line ~56 `hintText: 'Search fields...'` → `searchHint`

### 1.7 `presentation/pages/profile_page.dart`

- [ ] line ~450 `labelText: 'Type'` → `profileType`
- [ ] line ~460 `child: Text('email')` → `profileTypeEmail`
- [ ] line ~464 `child: Text('phone')` → `profileTypePhone`
- [ ] line ~495 `child: const Text('Cancel')` → `commonCancel`

---

## 批次 2：P1 — 设置与次级页面

### 2.1 `presentation/pages/settings_page.dart`（剩余部分）

- [ ] line ~198 `Text('Debug mode enabled')` → `settingsDebugModeEnabled`
- [ ] line ~212 `Text('Invalid password')` → `settingsInvalidPassword`
- [ ] line ~227 `title: Text('Settings')` → `settingsTitle`
- [ ] line ~431 `Text('Master password changed successfully')` → `settingsPasswordChangedSuccess`
- [ ] line ~652 `title: Text(feature)` → 动态，需确认 feature 值列表
- [ ] line ~659 `child: const Text('OK')` → `commonOk`
- [ ] line ~700 `leading: const Text('🇺🇸', ...)` → 国旗 emoji，保持不变
- [ ] line ~701 `title: Text(l10n.settingsLanguageEnglish)` → 已迁移
- [ ] line ~712 `leading: const Text('🇨🇳', ...)` → 国旗 emoji，保持不变
- [ ] line ~713 `title: Text(l10n.settingsLanguageChinese)` → 已迁移
- [ ] line ~900 `Text('Enable Debug Mode')` → `settingsEnableDebugMode`
- [ ] line ~907 `const Text('Enter your master password to enable Debug Log.')` → `settingsEnableDebugModeDesc`
- [ ] line ~919 `label: Text('Use $biometricType')` → `settingsUseBiometric`
- [ ] line ~928 `child: Text('or', ...)` → `commonOr`
- [ ] line ~940 `labelText: 'Master Password'` → `settingsMasterPassword`
- [ ] line ~971 `tooltip: 'Show password hint'` → `settingsShowPasswordHint`
- [ ] line ~1000 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~1004 `child: const Text('Enable')` → `settingsEnable`

### 2.2 `presentation/pages/security_settings_page.dart`

- [ ] line ~75 `Text('Biometric authentication failed or was cancelled')` → `securitySettingsBiometricFailed`
- [ ] line ~121 `Text('Biometric unlock enabled')` → `securitySettingsBiometricEnabled`
- [ ] line ~137 `title: const Text('Security Settings')` → `securitySettingsTitle`
- [ ] line ~257 `label: const Text('Reset to Defaults')` → `securitySettingsResetToDefaults`
- [ ] line ~302 `title: const Text('Reset Security Settings')` → `securitySettingsResetTitle`
- [ ] line ~303 `content: const Text('This will reset all security settings...')` → `securitySettingsResetConfirm`
- [ ] line ~307 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~311 `child: const Text('Reset')` → `securitySettingsReset`
- [ ] line ~334 `Text('Feature not yet implemented')` → `securitySettingsNotImplemented`

### 2.3 `presentation/pages/sensitivity_settings_page.dart`

- [ ] line ~117 `title: const Text('Sensitivity Settings')` → `sensitivitySettingsTitle`
- [ ] line ~135 `child: const Text('Verify')` → `sensitivitySettingsVerify`
- [ ] line ~184 `const Text('Confirm Downgrade')` → `sensitivitySettingsConfirmDowngrade`
- [ ] line ~224 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~240 `child: const Text('Confirm')` → `commonConfirm`
- [ ] line ~420 `tooltip: 'Change sensitivity level'` → `sensitivitySettingsChangeLevel`
- [ ] line ~513 `title: const Text('Sensitivity Settings')` → `sensitivitySettingsTitle`
- [ ] line ~529 `hintText: 'Search fields...'` → `sensitivitySettingsSearchHint`
- [ ] line ~690 `child: const Text('Clear search')` → `sensitivitySettingsClearSearch`

### 2.4 `presentation/pages/trash_page.dart`

- [ ] line ~96 `title: const Text('Trash')` → `trashTitle`
- [ ] line ~116 `label: const Text('Verify')` → `trashVerify`
- [ ] line ~154 `const Text('Empty Trash')` → `trashEmptyTrash`
- [ ] line ~198 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~248 `child: const Text('Empty Trash')` → `trashEmptyTrashButton`
- [ ] line ~263 `Text('Confirm Restore')` → `trashConfirmRestore`
- [ ] line ~266 `content: Text('Restore "${object.name}"?')` → `trashRestoreConfirm`（参数化 `{name}`）
- [ ] line ~273 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~277 `child: const Text('Restore')` → `trashRestore`
- [ ] line ~311 `const Text('Confirm Permanent Delete')` → `trashConfirmPermanentDelete`
- [ ] line ~355 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~363 `child: const Text('Delete Permanently')` → `trashDeletePermanently`
- [ ] line ~549 `title: const Text('Trash')` → `trashTitle`
- [ ] line ~561 `hintText: 'Search trash...'` → `trashSearchHint`

### 2.5 `presentation/pages/sync_page.dart`

- [ ] line ~41 `title: const Text('Device Sync')` → `syncTitle`
- [ ] line ~166 `content: Text('No active account for sync')` → `syncNoActiveAccount`
- [ ] line ~187 `content: Text('Enter address and pairing key')` → `syncEnterAddressAndKey`
- [ ] line ~195 `content: Text('Invalid pairing key hex')` → `syncInvalidPairingKey`
- [ ] line ~204 `content: Text('No active account for sync')` → `syncNoActiveAccount`
- [ ] line ~225 `content: Text('Pairing key copied to clipboard')` → `syncPairingKeyCopied`
- [ ] line ~443 `labelText: 'Remote Address'` → `syncRemoteAddress`
- [ ] line ~444 `hintText: '192.168.1.5:9900'` → `syncRemoteAddressHint`
- [ ] line ~454 `labelText: 'Pairing Key (hex)'` → `syncPairingKey`
- [ ] line ~455 `hintText: 'Enter shared pairing key'` → `syncPairingKeyHint`
- [ ] line ~517 `label: const Text('Generate & Copy Key')` → `syncGenerateAndCopyKey`
- [ ] line ~644 `title: Text('Sync with ${widget.device.name}')` → `syncWithDevice`（参数化 `{name}`）
- [ ] line ~656 `labelText: 'Pairing Key (hex)'` → `syncPairingKey`
- [ ] line ~673 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~681 `content: Text('Invalid pairing key hex')` → `syncInvalidPairingKey`
- [ ] line ~688 `child: const Text('Sync')` → `syncButton`

### 2.6 `presentation/pages/data_management_page.dart`

- [ ] line ~159-160 `title: Text(title)` / `content: Text(content)` → 参数化，需确认调用点
- [ ] line ~164 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~271 `title: const Text('Special Backup Limit Reached')` → `dataManagementSpecialBackupLimit`
- [ ] line ~279 `child: const Text('OK')` → `commonOk`
- [ ] line ~293 `title: const Text('Name Special Backup')` → `dataManagementNameBackup`
- [ ] line ~298 `hintText: 'e.g. Before Major Update'` → `dataManagementBackupNameHint`
- [ ] line ~299 `labelText: 'Backup name'` → `dataManagementBackupNameLabel`
- [ ] line ~306 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~313 `child: const Text('Save')` → `commonSave`
- [ ] line ~365 `title: const Text('Special Backup Limit Reached')` → `dataManagementSpecialBackupLimit`
- [ ] line ~373 `child: const Text('OK')` → `commonOk`
- [ ] line ~387 `title: const Text('Name Special Backup')` → `dataManagementNameBackup`
- [ ] line ~392 `hintText: 'e.g. Before Major Update'` → `dataManagementBackupNameHint`
- [ ] line ~393 `labelText: 'Backup name'` → `dataManagementBackupNameLabel`
- [ ] line ~400 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~407 `child: const Text('Create')` → `dataManagementCreate`
- [ ] line ~463 `title: const Text('Rename Special Backup')` → `dataManagementRenameBackup`
- [ ] line ~468 `labelText: 'New name'` → `dataManagementNewName`
- [ ] line ~475 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~482 `child: const Text('Rename')` → `dataManagementRename`
- [ ] line ~518 `title: const Text('Restore Special Backup?')` → `dataManagementRestoreBackupTitle`
- [ ] line ~519 `content: Text('Restore special backup...')` → `dataManagementRestoreBackupConfirm`
- [ ] line ~526 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~530 `child: const Text('Restore')` → `dataManagementRestore`
- [ ] line ~573 `title: const Text('Delete Special Backup?')` → `dataManagementDeleteBackupTitle`
- [ ] line ~574 `content: Text('Delete special backup...')` → `dataManagementDeleteBackupConfirm`
- [ ] line ~578 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~586 `child: const Text('Delete')` → `commonDelete`
- [ ] line ~613 `title: const Text('Data Management')` → `dataManagementTitle`
- [ ] line ~747 `label: const Text('Backup Now')` → `dataManagementBackupNow`
- [ ] line ~896 `label: const Text('Create')` → `dataManagementCreate`

### 2.7 `presentation/pages/operation_log_page.dart`

- [ ] line ~59 `title: const Text('Operation Log')` → `operationLogTitle`
- [ ] line ~77 `child: const Text('Verify')` → `operationLogVerify`
- [ ] line ~115 `title: const Text('Clear Log')` → `operationLogClearLogTitle`
- [ ] line ~122 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~129 `child: const Text('Clear', ...)` → `operationLogClear`
- [ ] line ~174 `title: const Text('Operation Log')` → `operationLogTitle`
- [ ] line ~181 `tooltip: 'Clear log'` → `operationLogClearLog`
- [ ] line ~194 `hintText: 'Search logs...'` → `operationLogSearchHint`

---

## 批次 3：P2 — 扫描与导入相关

### 3.1 `presentation/pages/scan/scan_import_result_page.dart`

- [ ] line ~24 `title: const Text('Import Complete')` → `scanImportComplete`
- [ ] line ~108 `label: const Text('Go Home')` → `scanImportGoHome`
- [ ] line ~117 `label: const Text('Close')` → `commonClose`

### 3.2 `presentation/pages/scan/scan_preview_page.dart`

- [ ] line ~50 `title: const Text('Preview & Confirm')` → `scanPreviewTitle`
- [ ] line ~69 `label: Text('$selectedCount / $totalCandidates')` → `scanPreviewCandidates`
- [ ] line ~457 `tooltip: 'Import action'` → `scanPreviewImportAction`
- [ ] line ~538 `child: const Text('Back')` → `commonBack`
- [ ] line ~681 `label: Text('Import ($selectedCount)')` → `scanPreviewImport`

### 3.3 `presentation/pages/scan/local_search_config_page.dart`

- [ ] line ~80 `title: Text('Local Search Import')` → `localSearchTitle`
- [ ] line ~151 `title: const Text('Filename only')` → `localSearchFilenameOnly`
- [ ] line ~152 `subtitle: const Text('Fastest — only check filenames')` → `localSearchFilenameSubtitle`
- [ ] line ~158 `title: const Text('Filename + Content fingerprint')` → `localSearchFingerprint`
- [ ] line ~159 `subtitle: const Text('Balanced — regex match on content')` → `localSearchFingerprintSubtitle`
- [ ] line ~165 `title: const Text('Full text parsing')` → `localSearchFullText`
- [ ] line ~166 `subtitle: const Text('Slowest — deep content analysis')` → `localSearchFullTextSubtitle`
- [ ] line ~247 `title: Text('$label size limit')` → `localSearchSizeLimit`
- [ ] line ~277 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~281 `child: const Text('Save')` → `commonSave`
- [ ] line ~433 `title: const Text('Use default paths')` → `localSearchDefaultPaths`
- [ ] line ~434 `subtitle: const Text('Documents, Desktop, Downloads')` → `localSearchDefaultPathsSubtitle`
- [ ] line ~442 `title: const Text('Custom paths')` → `localSearchCustomPaths`
- [ ] line ~443 `subtitle: const Text('Select specific folders')` → `localSearchCustomPathsSubtitle`
- [ ] line ~467 `title: const Text('Add folder')` → `localSearchAddFolder`
- [ ] line ~530 `label: const Text('Start Scan')` → `localSearchStartScan`

### 3.4 `presentation/pages/scan/local_search_progress_page.dart`

- [ ] line ~67 `title: const Text('Scanning...')` → `scanProgressScanning`
- [ ] line ~176 `label: const Text('Cancel Scan')` → `scanProgressCancelScan`
- [ ] line ~185 `label: const Text('Go Back')` → `scanProgressGoBack`
- [ ] line ~193 `label: const Text('Scan Again')` → `scanProgressScanAgain`
- [ ] line ~208 `title: const Text('No Results Found')` → `scanProgressNoResults`
- [ ] line ~219 `child: const Text('OK')` → `commonOk`

---

## 批次 4：P2 — Widget 级组件

### 4.1 密码/安全相关对话框

**`presentation/widgets/change_password_dialog.dart`**
- [ ] line ~32 `const Text('Change Master Password')` → `changePasswordTitle`
- [ ] line ~67 `labelText: 'Current Password'` → `changePasswordCurrent`
- [ ] line ~94 `labelText: 'New Password'` → `changePasswordNew`
- [ ] line ~96 `hintText: 'Minimum 8 characters'` → `changePasswordNewHint`
- [ ] line ~122 `labelText: 'Confirm New Password'` → `changePasswordConfirmNew`
- [ ] line ~148 `labelText: 'New Password Hint (Optional)'` → `changePasswordHintOptional`
- [ ] line ~150 `hintText: 'A hint to help you remember'` → `changePasswordHintHelp`
- [ ] line ~183 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~244 `: const Text('Change')` → `changePasswordButton`

**`presentation/widgets/password_verification_dialog.dart`**
- [ ] line ~227 `const Text('Verify Identity')` → `verifyIdentityTitle`
- [ ] line ~261 `labelText: 'Master Password'` → `verifyIdentityMasterPassword`
- [ ] line ~281 `tooltip: 'Show password hint'` → `verifyIdentityShowHint`
- [ ] line ~296 `tooltip: _obscurePassword ? 'Show password' : 'Hide password'` → `verifyIdentityShowPassword` / `verifyIdentityHidePassword`
- [ ] line ~308 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~318 `: const Text('Verify')` → `verifyIdentityVerify`
- [ ] line ~433 `const Text('Verify Identity')` → `verifyIdentityTitle`
- [ ] line ~471 `label: Text('Use $biometricType')` → `verifyIdentityUseBiometric`
- [ ] line ~480 `child: Text('or', ...)` → `commonOr`
- [ ] line ~493 `labelText: 'Master Password'` → `verifyIdentityMasterPassword`
- [ ] line ~513 `tooltip: 'Show password hint'` → `verifyIdentityShowHint`
- [ ] line ~528 `tooltip: _obscurePassword ? 'Show password' : 'Hide password'` → `verifyIdentityShowPassword` / `verifyIdentityHidePassword`
- [ ] line ~540 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~550 `: const Text('Verify')` → `verifyIdentityVerify`

**`presentation/widgets/lock_vault_dialog.dart`**
- [ ] line ~14 `Text('Lock Vault?')` → `lockVaultTitle`
- [ ] line ~23 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~27 `child: const Text('Lock')` → `lockVaultLock`

**`presentation/widgets/biometric_settings_widget.dart`**
- [ ] line ~68 `title: const Text('Master Password')` → `biometricMasterPassword`
- [ ] line ~79 `labelText: 'Master Password'` → `biometricMasterPassword`
- [ ] line ~90 `content: Text('Password Hint: $hint')` → `biometricPasswordHint`
- [ ] line ~98 `tooltip: ... 'Show password hint' / 'No hint available'` → `biometricShowHint` / `biometricNoHint`
- [ ] line ~117 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~121 `child: const Text('Confirm')` → `commonConfirm`
- [ ] line ~413 `label: const Text('Test Touch ID')` → `biometricTestTouchId`
- [ ] line ~432 `label: const Text('Test Face ID')` → `biometricTestFaceId`

### 4.2 登录组件

**`presentation/widgets/login/password_input_section.dart`**
- [ ] line ~204 `labelText: 'Master Password'` → `loginMasterPassword`
- [ ] line ~205 `hintText: 'Enter your password'` → `loginEnterPassword`
- [ ] line ~285 `tooltip: 'Show password hint'` → `loginShowPasswordHint`

**`presentation/widgets/login/create_account_form.dart`**
- [ ] line ~68 `labelText: 'Account Name'` / `hintText: 'e.g., Personal, Work'` → `loginAccountName` / `loginAccountNameHint`
- [ ] line ~86 `labelText: 'Master Password'` / `hintText: 'Create a strong password'` → `loginMasterPassword` / `loginMasterPasswordHint`
- [ ] line ~120 `labelText: 'Confirm Password'` / `hintText: 'Re-enter your password'` → `loginConfirmPassword` / `loginConfirmPasswordHint`
- [ ] line ~158 `labelText: 'Password Hint (Optional)'` / `hintText: 'A hint to help you remember'` → `loginPasswordHintOptional` / `loginPasswordHintHelp`
- [ ] line ~236 `child: const Text('Back to Account List')` → `loginBackToAccountList`

**`presentation/widgets/login/account_list_section.dart`**
- [ ] line ~52 `Text('No accounts found')` → `loginNoAccounts`
- [ ] line ~57 `label: const Text('Create Account')` → `loginCreateAccount`
- [ ] line ~118 `Text('Last accessed: ...')` → `loginLastAccessed`
- [ ] line ~148 `Text('Account list empty')` → `loginAccountListEmpty`
- [ ] line ~151 `Text('Create your first account to get started')` → `loginCreateFirstAccount`

### 4.3 首页 Widget

**`presentation/widgets/home/page_editor.dart`**
- [ ] line ~125 `title: const Text('Delete Section?')` → `homePageEditorDeleteSection`
- [ ] line ~126 `content: const Text('This section and its items will be moved to trash.')` → `homePageEditorDeleteSectionDesc`
- [ ] line ~128 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~132 `child: const Text('Delete')` → `commonDelete`
- [ ] line ~151 `title: Text(page != null ? 'Edit Page' : 'New Page')` → `homePageEditorEditPage` / `homePageEditorNewPage`
- [ ] line ~165 `child: const Text('Save')` → `commonSave`
- [ ] line ~187 `hintText: 'Page title'` → `homePageEditorPageTitle`
- [ ] line ~201 `Text('Sections', ...)` → `homePageEditorSections`
- [ ] line ~205 `label: const Text('Add Section')` → `homePageEditorAddSection`
- [ ] line ~250 `subtitle: Text('${section.childrenIds.length} items')` → `homePageEditorItems`（参数化 `{count}`）
- [ ] line ~307 `title: Text(widget.initialTitle == null ? 'Add Section' : 'Edit Section')` → `homePageEditorAddSectionTitle` / `homePageEditorEditSectionTitle`
- [ ] line ~316 `labelText: 'Section Title'` → `homePageEditorSectionTitle`
- [ ] line ~317 `hintText: 'Enter section title'` → `homePageEditorEnterSectionTitle`
- [ ] line ~322 `Text('Icon', ...)` → `homePageEditorIcon`
- [ ] line ~333 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~343 `child: const Text('Save')` → `commonSave`

**`presentation/widgets/home/add_quick_action_dialog.dart`**
- [ ] line ~15 `title: const Text('Add Quick Action')` → `homeAddQuickAction`
- [ ] line ~42 `child: const Text('Cancel')` → `commonCancel`

### 4.4 对象相关 Widget

**`presentation/widgets/object_card.dart`**
- [ ] line ~333 `title: const Text('Delete Item')` → `objectCardDeleteItem`
- [ ] line ~334 `content: Text('Are you sure you want to delete "$itemName"?')` → `objectCardDeleteItemConfirm`
- [ ] line ~338 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~342 `child: Text('Delete', ...)` → `commonDelete`
- [ ] line ~556 `title: const Text('Delete Section')` → `objectCardDeleteSection`
- [ ] line ~557-558 `content: Text(...)` → `objectCardDeleteSectionConfirm`
- [ ] line ~563 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~568 `child: const Text('Delete')` → `commonDelete`
- [ ] line ~578 `content: Text('Section deleted')` → `objectCardSectionDeleted`
- [ ] line ~962 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~967 `child: const Text('Add')` → `objectCardAdd`
- [ ] line ~1065 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~1070 `child: const Text('Save')` → `commonSave`

**`presentation/widgets/object_card/object_card_edit_field.dart`**
- [ ] line ~77 `labelText: isTitle ? 'Title' : formatLabel(propertyKey)` → `objectCardEditTitle`

**`presentation/widgets/object_card/object_card_header.dart`**
- [ ] line ~57 `tooltip: 'Edit'` → `objectCardHeaderEdit`
- [ ] line ~66 `tooltip: 'Delete'` → `objectCardHeaderDelete`
- [ ] line ~78 `tooltip: 'Edit Section'` → `objectCardHeaderEditSection`
- [ ] line ~87 `tooltip: 'Add Item'` → `objectCardHeaderAddItem`

**`presentation/widgets/object_card/object_card_item_tile.dart`**
- [ ] line ~128 `tooltip: 'Copy'` → `objectCardItemCopy`
- [ ] line ~134 `tooltip: 'Edit'` → `objectCardItemEdit`
- [ ] line ~158 `tooltip: hasHist ? 'History ($count)' : 'No history yet'` → `objectCardItemHistory` / `objectCardItemNoHistory`
- [ ] line ~165 `tooltip: 'Delete'` → `objectCardItemDelete`
- [ ] line ~223 `tooltip: count == 1 ? '1 attachment' : '$count attachments'` → `objectCardItemAttachment` / `objectCardItemAttachments`

**`presentation/widgets/object_tile.dart`**
- [ ] line ~97 `tooltip: 'Edit'` → `objectTileEdit`
- [ ] line ~107 `tooltip: 'Delete'` → `objectTileDelete`

### 4.5 条目与附件

**`presentation/widgets/entry_action_builder.dart`**
- [ ] line ~29 `tooltip: 'Copy All'` → `entryActionCopyAll`
- [ ] line ~45 `tooltip: 'Edit'` → `entryActionEdit`
- [ ] line ~66 `tooltip: 'Delete'` → `entryActionDelete`

**`presentation/widgets/entry_card_widget.dart`**
- [ ] line ~320 `tooltip: hasHist ? 'History ($count)' : 'No history yet'` → `entryCardHistory` / `entryCardNoHistory`
- [ ] line ~373 `label: Text('History(${history?.entries.length ?? 0})')` → `entryCardHistoryLabel`
- [ ] line ~454 `tooltip: count == 1 ? '1 attachment' : '$count attachments'` → `entryCardAttachment` / `entryCardAttachments`

**`presentation/widgets/field_history_dialog.dart`**
- [ ] line ~142 `child: const Text('Close')` → `commonClose`

**`presentation/widgets/attachment_list_sheet.dart`**
- 动态文件名和格式化日期，无需 i18n

### 4.6 侧边栏与头部

**`presentation/widgets/sidebar/sidebar_header.dart`**
- [ ] line ~57 `tooltip: 'Collapse'` → `sidebarCollapse`
- [ ] line ~68 `tooltip: 'Expand'` → `sidebarExpand`

**`presentation/widgets/header_action_buttons.dart`**
- [ ] line ~30 `tooltip: 'Lock Sensitivity Access'` → `headerLockSensitivity`

### 4.7 搜索与过滤

**`presentation/widgets/search_filters.dart`**
- [ ] line ~38 `label: const Text('Public')` → `searchFilterPublic`
- [ ] line ~49 `label: const Text('Internal')` → `searchFilterInternal`
- [ ] line ~60 `label: const Text('Sensitive')` → `searchFilterSensitive`
- [ ] line ~71 `label: const Text('Restricted')` → `searchFilterRestricted`
- [ ] line ~82 `label: const Text('Unlock')` → `searchFilterUnlock`

**`presentation/widgets/search_result_tile.dart`**
- [ ] line ~89 `label: const Text('Reveal')` → `searchResultReveal`

### 4.8 图标与文件夹选择

**`presentation/widgets/icon_picker_sheet.dart`**
- [ ] line ~51 `Text('Choose Icon', ...)` → `iconPickerTitle`

**`presentation/widgets/folder_picker_dialog.dart`**
- [ ] line ~134 `tooltip: 'Go up'` → `folderPickerGoUp`
- [ ] line ~174 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~179 `child: const Text('Select This Folder')` → `folderPickerSelectFolder`

### 4.9 备份与数据管理 Widget

**`presentation/widgets/data_management/backup_list_tile.dart`**
- [ ] line ~48 `tooltip: 'Rename'` → `backupRename`
- [ ] line ~53 `tooltip: 'Restore'` → `backupRestore`
- [ ] line ~58 `tooltip: 'Delete'` → `backupDelete`
- [ ] line ~89 `tooltip: 'Save as special backup'` → `backupSaveAsSpecial`
- [ ] line ~94 `tooltip: 'Restore'` → `backupRestore`
- [ ] line ~99 `tooltip: 'Delete'` → `backupDelete`

### 4.10 操作瓦片与 Section

**`presentation/widgets/operation_tile.dart`**
- [ ] line ~22 `const Expanded(child: Text('Operation Details'))` → `operationDetails`
- [ ] line ~112 `child: const Text('Close')` → `commonClose`
- [ ] line ~344 `tooltip: 'View details'` → `operationViewDetails`

**`presentation/widgets/section_card.dart`**
- [ ] line ~79 `tooltip: 'Add'` → `sectionCardAdd`
- [ ] line ~187 `tooltip: 'Add'` → `sectionCardAdd`

### 4.11 调试日志

**`presentation/widgets/settings/debug_log_sheet.dart`**
- [ ] line ~41 `title: const Text('Copy Logs to Clipboard')` → `debugCopyLogsTitle`
- [ ] line ~50 `child: const Text('Cancel')` → `commonCancel`
- [ ] line ~54 `child: const Text('Copy')` → `debugCopy`
- [ ] line ~70 `Text('Sanitized logs copied to clipboard')` → `debugLogsCopied`
- [ ] line ~138 `Text('Debug Log', ...)` → `debugLogTitle`
- [ ] line ~144 `tooltip: 'Refresh'` → `debugRefresh`
- [ ] line ~152 `tooltip: 'Copy to clipboard'` → `debugCopyToClipboard`
- [ ] line ~160 `tooltip: 'Disable debug mode'` → `debugDisableDebugMode`

### 4.12 回收站卡片

**`presentation/widgets/trash/unified_object_trash_card.dart`**
- [ ] line ~196 `title: Text(object.name)` → 动态
- [ ] line ~299 `child: const Text('Close')` → `commonClose`
- [ ] line ~380 `tooltip: hasHist ? 'History ($count)' : 'No history yet'` → `trashHistory` / `trashNoHistory`
- [ ] line ~393 `label: const Text('History')` → `trashHistory`

### 4.13 日期选择器与进度

**`presentation/widgets/date_picker_form_field.dart`**
- [ ] line ~67 `labelText: label` → 动态
- [ ] line ~73 `tooltip: 'Clear date'` → `datePickerClear`

**`presentation/widgets/scan_progress_banner.dart`**
- [ ] line ~87 `tooltip: 'Stop scan'` → `scanStop`

---

## 批次 5：P3 — 剩余清理

### 5.1 `presentation/theme/glass_adapters.dart`
- [ ] line ~385 `tooltip: 'Back'` → `commonBack`

### 5.2 `presentation/theme/app_theme.dart`
- [ ] line ~540 `labelText: labelText` / line ~541 `hintText: hintText` → 参数化通用组件，无需迁移

### 5.3 `core/services/operation_notification.dart`
- [ ] line ~389 `tooltip: 'Dismiss'` → `commonDismiss`

---

## 附录 A：可复用 Key 清单

以下字符串在多个页面重复出现，优先复用已有 key，避免新增：

| 英文原文 | 已有 ARB Key |
|----------|-------------|
| Cancel | `commonCancel` |
| Save | `commonSave` |
| Delete | `commonDelete` |
| Close | `commonClose` |
| Confirm | `commonConfirm` |
| Edit | `commonEdit` |
| Loading... | `commonLoading` |
| Error | `commonError` |
| Retry | `commonRetry` |
| Success | `commonSuccess` |
| Import | `commonImport` |

---

## 附录 B：参数化字符串 ARB 模板

对于带变量的确认消息，使用 placeholder：

```json
"workspaceDeleteSectionConfirm": "Are you sure you want to delete \"{name}\"?",
"@workspaceDeleteSectionConfirm": {
  "placeholders": { "name": { "type": "String" } }
}
```

Dart 调用：
```dart
AppLocalizations.of(context).workspaceDeleteSectionConfirm(itemName)
```

---

*文档路径：`docs/I18N_TODO.md`*
*创建时间：2026-05-06*
