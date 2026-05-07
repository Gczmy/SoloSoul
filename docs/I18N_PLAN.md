# Flutter i18n 完整中文支持方案

> 状态：已建立基础设施（ARB + gen-l10n + LanguageService + languageProvider），P0 高优先级页面已迁移。剩余约 **390 处**硬编码英文 UI 字符串待迁移。

---

## 一、已完成的迁移（当前状态）

| 模块 | 状态 |
|------|------|
| 主入口 `main.dart` | ✅ 已完成 |
| 侧边栏 `app_sidebar.dart` | ✅ 已完成 |
| AI 对话 `llm_chat_page` / `llm_chat_panel` / `llm_chat_bubble` | ✅ 已完成 |
| AI 助手设置 `llm_config_page` | ✅ 已完成 |
| 使用统计 `llm_stats_page` | ✅ 已完成 |
| OCR 扫描 `ocr_scanner_sheet` | ✅ 已完成（Take Photo 桌面端已隐藏） |
| 扫描预览 `scan_preview_page` | ✅ 已完成 |
| 删除账户弹窗 `delete_account_dialog_content` | ✅ 已完成 |
| 设置页语言选择器 `settings_page.dart`（语言相关） | ✅ 已完成 |

---

## 二、剩余待迁移字符串总览（按优先级分组）

### P0 — 高频用户可见页面（建议第一批处理）

#### 2.1 首页 `home_page.dart`
```
line 52:  title: Text('SoloSoul')
line 66:  label: const Text('Scan')
line 413: Text('Quick Actions', ...)
line 422: tooltip: _isEditing ? 'Done' : 'Edit quick actions'
line 461: Text('Security Status', ...)
```
**建议 ARB key**：
- `homeTitle` / `homeScan` / `homeQuickActions` / `homeEditQuickActions` / `homeEditQuickActionsDone` / `homeSecurityStatus`

#### 2.2 登录页 `login_page.dart`
```
line 119: title: 'Data Recovery'
line 121: message: 'Your vault appears to be empty...'
line 125: label: 'Skip'
line 129: label: 'Restore Backup'
line 147: content: Text('Restore successful...')
line 156: content: Text('Restore failed')
line 239: biometricType = 'Biometric' / 'Face ID' / 'Touch ID' / 'Iris'
line 265: reason: 'Unlock SoloSoul with $_biometricType'
line 273: content: 'Biometric authentication failed or was cancelled'
line 289: content: 'Failed to unlock vault. Please use your master password.'
line 322: content: 'Biometric unlock error: $e'
line 349: return 'Never'
line 353: return 'Today'
line 354: return 'Yesterday'
line 355: return '${diff.inDays} days ago'
line 365: SoloLog.d('LOGIN', 'Loading profile...')
line 370: SoloLog.e('LOGIN', 'loadProfile timed out')
line 388: SoloLog.d('LOGIN', 'After loadFromProfile...')
line 390: // 首次启动/空数据检测...（注释，不迁移）
line 424: _passwordErrorMessage = 'Password must be at least 8 characters'
line 455: _passwordErrorMessage = 'Invalid master password'
line 456: _passwordErrorMessage = 'Unlock failed: $specificError'
line 473: _createError = 'Account name is required'
line 477: _createError = 'Password must be at least 8 characters'
line 481: _createError = 'Passwords do not match'
line 505: _createError = result.error ?? 'Failed to create account'
line 520: _createError = 'Failed to unlock vault. Please try again.'
line 599: Text('Password Hint: $hint', ...)
line 925: error: (error, _) => Center(child: Text('Error: $error'))
```
**建议 ARB key**：
- `loginDataRecoveryTitle` / `loginDataRecoveryMessage` / `loginSkip` / `loginRestoreBackup`
- `loginRestoreSuccess` / `loginRestoreFailed` / `loginBiometricFaceId` / `loginBiometricTouchId` / `loginBiometricIris` / `loginBiometricGeneric`
- `loginUnlockReason` / `loginBiometricFailed` / `loginUnlockFailedUsePassword` / `loginBiometricUnlockError`
- `loginNever` / `loginToday` / `loginYesterday` / `loginDaysAgo`
- `loginPasswordMinLength` / `loginInvalidPassword` / `loginUnlockFailed` / `loginAccountNameRequired`
- `loginPasswordsDoNotMatch` / `loginCreateAccountFailed` / `loginUnlockVaultFailed` / `loginPasswordHint`
- `loginLaunchFailed` (已在 ARB 中)

**注意**：登录页日期格式化（Never/Today/Yesterday/days ago）建议保留英文或用 `intl` 的 `DateFormat` 本地化处理，而不是简单字符串替换。

#### 2.3 对象编辑器 `object_editor_page.dart`
```
line 184: title: Text(_isEditing ? 'Edit Section' : 'New Section')
line 203: Text('Type', ...)
line 271: content: Text('Name is required')
line 288: content: Text('Duplicate property names: ...')
line 349: content: Text('Failed to save: $e')
line 377: Text('Icon', ...)
line 380: child: Text('Name', ...)
line 428: hintText: 'Enter section name'
line 463: hintText: 'Select type'
line 469: hint: const Text('Select type')
line 520: hintText: 'No parent (root)'
line 526: hint: const Text('No parent (root)')
line 530: child: Text('No parent (root)')
line 582: child: const Text('Save')
line 612: Text('Item Properties', ...)
line 616: tooltip: 'Add Property'
line 708: hintText: 'Key name'
line 773-776: DropdownMenuItem Text('Text'/'Date'/'Number'/'Checkbox')
line 792: tooltip: 'Sensitivity'
line 827: tooltip: 'Delete'
line 833: title: const Text('Delete Property')
line 834: content: Text('Are you sure you want to delete "$keyName"?')
line 838: child: const Text('Cancel')
line 842: child: const Text('Delete', ...)
```
**建议 ARB key**：
- `objectEditorEditSection` / `objectEditorNewSection` / `objectEditorType` / `objectEditorNameRequired`
- `objectEditorDuplicateProperties` / `objectEditorSaveFailed` / `objectEditorIcon` / `objectEditorName`
- `objectEditorEnterSectionName` / `objectEditorSelectType` / `objectEditorNoParent`
- `objectEditorSave` / `objectEditorItemProperties` / `objectEditorAddProperty` / `objectEditorKeyName`
- `objectEditorPropertyTypeText` / `objectEditorPropertyTypeDate` / `objectEditorPropertyTypeNumber` / `objectEditorPropertyTypeCheckbox`
- `objectEditorSensitivity` / `objectEditorDeletePropertyTitle` / `objectEditorDeletePropertyConfirm`

#### 2.4 对象工作区 `object_workspace_page.dart`
```
line 72:  title = currentObject?.name ?? 'Objects'
line 82-88: 'No items yet' / 'No objects yet' / 'Add your first item' / 'Create your first object to get started'
line 146: tooltip: 'Delete'
line 155: tooltip: 'Edit Page'
line 164: tooltip: _isReordering ? 'Done' : 'Reorder'
line 174: tooltip: 'Add'
line 208: title: const Text('Delete Section')
line 209-210: content: Text('Are you sure...')
line 215: child: const Text('Cancel')
line 220: child: const Text('Delete')
line 230: content: Text('Section deleted')
line 244: title: Text(object.typeId == 'page' ? 'Delete Page' : 'Delete Section')
line 245-247: content: Text('Are you sure...')
line 252: child: const Text('Cancel')
line 257: child: const Text('Delete')
line 268: content: Text('"${object.name}" moved to trash')
line 298: title: const Text('Add Sub-Page')
line 303: title: const Text('Add Section')
line 346: title: const Text('Add Section')
line 356: labelText: 'Name'
line 357: hintText: 'Enter section name'
line 362: Text('Icon', ...)
line 406: child: const Text('Cancel')
line 417: child: const Text('Add Section')
```
**建议 ARB key**：
- `workspaceObjects` / `workspaceNoItems` / `workspaceNoObjects` / `workspaceAddFirstItem`
- `workspaceCreateFirstObject` / `workspaceDelete` / `workspaceEditPage` / `workspaceDone` / `workspaceReorder`
- `workspaceAdd` / `workspaceDeleteSectionTitle` / `workspaceDeleteSectionConfirm`
- `workspaceSectionDeleted` / `workspaceDeletePageTitle` / `workspaceMovedToTrash`
- `workspaceAddSubPage` / `workspaceAddSection` / `workspaceSectionName` / `workspaceEnterSectionName`
- `workspaceIcon` / `workspaceCancel` / `workspaceAddSectionButton`

#### 2.5 页面编辑器 `page_editor_page.dart`
```
line 68:  content: Text('Name is required')
line 109: title: Text(_isEditing ? 'Edit Page' : 'New Page')
line 117: Text('Name', ...)
line 122: hintText: 'Enter page name'
line 129: Text('Icon', ...)
line 165: Text('Parent', ...)
line 182: child: const Text('Save')
```
**建议 ARB key**：
- `pageEditorNameRequired` / `pageEditorEditPage` / `pageEditorNewPage` / `pageEditorName`
- `pageEditorEnterPageName` / `pageEditorIcon` / `pageEditorParent` / `pageEditorSave`

#### 2.6 个人资料页 `profile_page.dart`
```
line 245: labelText: label / hintText: hint
line 450: labelText: 'Type'
line 460: child: Text('email')
line 464: child: Text('phone')
line 495: child: const Text('Cancel')
```
**建议 ARB key**：
- `profileType` / `profileTypeEmail` / `profileTypePhone` / `profileCancel`

#### 2.7 搜索页 `search_page.dart`
```
line 46:  title: const Text('Search')
line 56:  hintText: 'Search fields...'
```
**建议 ARB key**：
- `searchTitle` / `searchHint`

---

### P1 — 次级页面与对话框

#### 2.8 操作日志 `operation_log_page.dart`
```
line 59:  title: const Text('Operation Log')
line 77:  child: const Text('Verify')
line 115: title: const Text('Clear Log')
line 122: child: const Text('Cancel')
line 129: child: const Text('Clear', ...)
line 174: title: const Text('Operation Log')
line 181: tooltip: 'Clear log'
line 194: hintText: 'Search logs...'
```
**建议 ARB key**：
- `operationLogTitle` / `operationLogVerify` / `operationLogClearLogTitle`
- `operationLogClear` / `operationLogCancel` / `operationLogSearchHint`

#### 2.9 数据管理 `data_management_page.dart`
```
line 159-160: title/content (AlertDialog 参数化)
line 164: child: const Text('Cancel')
line 271: title: const Text('Special Backup Limit Reached')
line 279: child: const Text('OK')
line 293: title: const Text('Name Special Backup')
line 298: hintText: 'e.g. Before Major Update'
line 299: labelText: 'Backup name'
line 306: child: const Text('Cancel')
line 313: child: const Text('Save')
line 365: title: const Text('Special Backup Limit Reached')
line 373: child: const Text('OK')
line 387: title: const Text('Name Special Backup')
line 392: hintText: 'e.g. Before Major Update'
line 393: labelText: 'Backup name'
line 400: child: const Text('Cancel')
line 407: child: const Text('Create')
line 463: title: const Text('Rename Special Backup')
line 464: content: TextField(labelText: 'New name')
line 475: child: const Text('Cancel')
line 482: child: const Text('Rename')
line 518: title: const Text('Restore Special Backup?')
line 519: content: Text('Restore special backup...')
line 526: child: const Text('Cancel')
line 530: child: const Text('Restore')
line 573: title: const Text('Delete Special Backup?')
line 574: content: Text('Delete special backup...')
line 578: child: const Text('Cancel')
line 586: child: const Text('Delete')
line 613: title: const Text('Data Management')
line 747: label: const Text('Backup Now')
line 896: label: const Text('Create')
```
**建议 ARB key**：
- `dataManagementTitle` / `dataManagementBackupNow` / `dataManagementCreate`
- `dataManagementSpecialBackupLimit` / `dataManagementOk`
- `dataManagementNameBackup` / `dataManagementBackupNameHint` / `dataManagementBackupNameLabel`
- `dataManagementRenameBackup` / `dataManagementNewName` / `dataManagementRename`
- `dataManagementRestoreBackupTitle` / `dataManagementRestoreBackupConfirm`
- `dataManagementDeleteBackupTitle` / `dataManagementDeleteBackupConfirm`

#### 2.10 安全设置 `security_settings_page.dart`
```
line 75:  Text('Biometric authentication failed or was cancelled')
line 121: Text('Biometric unlock enabled')
line 137: title: const Text('Security Settings')
line 257: label: const Text('Reset to Defaults')
line 302: title: const Text('Reset Security Settings')
line 303: content: const Text('This will reset all security settings...')
line 307: child: const Text('Cancel')
line 311: child: const Text('Reset')
line 334: Text('Feature not yet implemented')
```
**建议 ARB key**：
- `securitySettingsTitle` / `securitySettingsBiometricFailed` / `securitySettingsBiometricEnabled`
- `securitySettingsResetToDefaults` / `securitySettingsResetTitle` / `securitySettingsResetConfirm`
- `securitySettingsCancel` / `securitySettingsReset` / `securitySettingsNotImplemented`

#### 2.11 敏感度设置 `sensitivity_settings_page.dart`
```
line 117: title: const Text('Sensitivity Settings')
line 135: child: const Text('Verify')
line 184: const Text('Confirm Downgrade')
line 224: child: const Text('Cancel')
line 240: child: const Text('Confirm')
line 420: tooltip: 'Change sensitivity level'
line 513: title: const Text('Sensitivity Settings')
line 529: hintText: 'Search fields...'
line 690: child: const Text('Clear search')
```
**建议 ARB key**：
- `sensitivitySettingsTitle` / `sensitivitySettingsVerify` / `sensitivitySettingsConfirmDowngrade`
- `sensitivitySettingsCancel` / `sensitivitySettingsConfirm` / `sensitivitySettingsChangeLevel`
- `sensitivitySettingsSearchHint` / `sensitivitySettingsClearSearch`

#### 2.12 设备同步 `sync_page.dart`
```
line 41:  title: const Text('Device Sync')
line 166: content: Text('No active account for sync')
line 187: content: Text('Enter address and pairing key')
line 195: content: Text('Invalid pairing key hex')
line 204: content: Text('No active account for sync')
line 225: content: Text('Pairing key copied to clipboard')
line 443: labelText: 'Remote Address'
line 444: hintText: '192.168.1.5:9900'
line 454: labelText: 'Pairing Key (hex)'
line 455: hintText: 'Enter shared pairing key'
line 517: label: const Text('Generate & Copy Key')
line 644: title: Text('Sync with ${widget.device.name}')
line 656: labelText: 'Pairing Key (hex)'
line 673: child: const Text('Cancel')
line 681: content: Text('Invalid pairing key hex')
line 688: child: const Text('Sync')
```
**建议 ARB key**：
- `syncTitle` / `syncNoActiveAccount` / `syncEnterAddressAndKey`
- `syncInvalidPairingKey` / `syncPairingKeyCopied` / `syncRemoteAddress`
- `syncRemoteAddressHint` / `syncPairingKey` / `syncPairingKeyHint`
- `syncGenerateAndCopyKey` / `syncWithDevice` / `syncCancel` / `syncButton`

#### 2.13 回收站 `trash_page.dart`
```
line 96:  title: const Text('Trash')
line 116: label: const Text('Verify')
line 154: const Text('Empty Trash')
line 198: child: const Text('Cancel')
line 248: child: const Text('Empty Trash')
line 263: Text('Confirm Restore')
line 266: content: Text('Restore "${object.name}"?')
line 273: child: const Text('Cancel')
line 277: child: const Text('Restore')
line 311: const Text('Confirm Permanent Delete')
line 355: child: const Text('Cancel')
line 363: child: const Text('Delete Permanently')
line 549: title: const Text('Trash')
line 561: hintText: 'Search trash...'
```
**建议 ARB key**：
- `trashTitle` / `trashVerify` / `trashEmptyTrash` / `trashCancel`
- `trashConfirmRestore` / `trashRestoreConfirm` / `trashRestore`
- `trashConfirmPermanentDelete` / `trashDeletePermanently` / `trashSearchHint`

---

### P2 — Widget 级复用组件与低频页面

#### 2.14 扫描相关
**`scan_import_result_page.dart`**
```
line 24:  title: const Text('Import Complete')
line 108: label: const Text('Go Home')
line 117: label: const Text('Close')
```
- `scanImportComplete` / `scanImportGoHome` / `scanImportClose`

**`scan_preview_page.dart`**
```
line 50:  title: const Text('Preview & Confirm')
line 69:  label: Text('$selectedCount / $totalCandidates')
line 457: tooltip: 'Import action'
line 538: child: const Text('Back')
line 681: label: Text('Import ($selectedCount)')
```
- `scanPreviewTitle` / `scanPreviewCandidates` / `scanPreviewImportAction` / `scanPreviewBack` / `scanPreviewImport`

**`local_search_config_page.dart`**
```
line 80:  title: Text('Local Search Import')
line 151: title: const Text('Filename only') / subtitle: const Text('Fastest — only check filenames')
line 158: title: const Text('Filename + Content fingerprint') / subtitle: const Text('Balanced — regex match on content')
line 165: title: const Text('Full text parsing') / subtitle: const Text('Slowest — deep content analysis')
line 247: title: Text('$label size limit')
line 277: child: const Text('Cancel')
line 281: child: const Text('Save')
line 433: title: const Text('Use default paths') / subtitle: const Text('Documents, Desktop, Downloads')
line 442: title: const Text('Custom paths') / subtitle: const Text('Select specific folders')
line 467: title: const Text('Add folder')
line 530: label: const Text('Start Scan')
```
- `localSearchTitle` / `localSearchFilenameOnly` / `localSearchFilenameSubtitle`
- `localSearchFingerprint` / `localSearchFingerprintSubtitle`
- `localSearchFullText` / `localSearchFullTextSubtitle`
- `localSearchSizeLimit` / `localSearchCancel` / `localSearchSave`
- `localSearchDefaultPaths` / `localSearchDefaultPathsSubtitle`
- `localSearchCustomPaths` / `localSearchCustomPathsSubtitle`
- `localSearchAddFolder` / `localSearchStartScan`

**`local_search_progress_page.dart`**
```
line 67:  title: const Text('Scanning...')
line 176: label: const Text('Cancel Scan')
line 185: label: const Text('Go Back')
line 193: label: const Text('Scan Again')
line 208: title: const Text('No Results Found')
line 219: child: const Text('OK')
line 343: title: Text(...) / subtitle: Text(...)
```
- `scanProgressScanning` / `scanProgressCancelScan` / `scanProgressGoBack`
- `scanProgressScanAgain` / `scanProgressNoResults` / `scanProgressOk`

#### 2.15 设置页（非语言相关部分）
```
line 198: Text('Debug mode enabled')
line 212: Text('Invalid password')
line 227: title: Text('Settings')
line 431: Text('Master password changed successfully')
line 652: title: Text(feature)
line 659: child: const Text('OK')
line 900: Text('Enable Debug Mode')
line 907: const Text('Enter your master password to enable Debug Log.')
line 919: label: Text('Use $biometricType')
line 928: child: Text('or', style: TextStyle(color: Colors.grey))
line 940: labelText: 'Master Password'
line 971: tooltip: 'Show password hint'
line 1000: child: const Text('Cancel')
line 1004: child: const Text('Enable')
```
- `settingsTitle` / `settingsDebugModeEnabled` / `settingsInvalidPassword`
- `settingsPasswordChangedSuccess` / `settingsOk` / `settingsEnableDebugMode`
- `settingsEnableDebugModeDesc` / `settingsUseBiometric` / `settingsOr`
- `settingsMasterPassword` / `settingsShowPasswordHint` / `settingsCancel` / `settingsEnable`

#### 2.16 账户管理对话框
**`change_password_dialog.dart`**
```
line 32:  const Text('Change Master Password')
line 67:  labelText: 'Current Password'
line 94:  labelText: 'New Password'
line 96:  hintText: 'Minimum 8 characters'
line 122: labelText: 'Confirm New Password'
line 148: labelText: 'New Password Hint (Optional)'
line 150: hintText: 'A hint to help you remember'
line 183: child: const Text('Cancel')
line 244: : const Text('Change')
```
- `changePasswordTitle` / `changePasswordCurrent` / `changePasswordNew`
- `changePasswordNewHint` / `changePasswordConfirmNew` / `changePasswordHintOptional`
- `changePasswordHintHelp` / `changePasswordCancel` / `changePasswordButton`

**`password_verification_dialog.dart`**
```
line 227: const Text('Verify Identity')
line 261: labelText: 'Master Password'
line 281: tooltip: 'Show password hint'
line 296: tooltip: _obscurePassword ? 'Show password' : 'Hide password'
line 308: child: const Text('Cancel')
line 318: : const Text('Verify')
line 433: const Text('Verify Identity')
line 471: label: Text('Use $biometricType')
line 480: child: Text('or', ...)
line 493: labelText: 'Master Password'
line 513: tooltip: 'Show password hint'
line 528: tooltip: _obscurePassword ? 'Show password' : 'Hide password'
line 540: child: const Text('Cancel')
line 550: : const Text('Verify')
```
- `verifyIdentityTitle` / `verifyIdentityMasterPassword` / `verifyIdentityShowHint`
- `verifyIdentityShowPassword` / `verifyIdentityHidePassword` / `verifyIdentityCancel`
- `verifyIdentityVerify` / `verifyIdentityUseBiometric` / `verifyIdentityOr`

**`lock_vault_dialog.dart`**
```
line 14: Text('Lock Vault?')
line 23: child: const Text('Cancel')
line 27: child: const Text('Lock')
```
- `lockVaultTitle` / `lockVaultCancel` / `lockVaultLock`

**`biometric_settings_widget.dart`**
```
line 68:  title: const Text('Master Password')
line 79:  labelText: 'Master Password'
line 90:  content: Text('Password Hint: $hint')
line 98:  tooltip: hint != null ... ? 'Show password hint' : 'No hint available'
line 117: child: const Text('Cancel')
line 121: child: const Text('Confirm')
line 413: label: const Text('Test Touch ID')
line 432: label: const Text('Test Face ID')
```
- `biometricMasterPassword` / `biometricPasswordHint` / `biometricShowHint`
- `biometricNoHint` / `biometricCancel` / `biometricConfirm`
- `biometricTestTouchId` / `biometricTestFaceId`

#### 2.17 对象卡片与条目
**`object_card.dart`**
```
line 333: title: const Text('Delete Item')
line 334: content: Text('Are you sure you want to delete "$itemName"?')
line 338: child: const Text('Cancel')
line 342: child: Text('Delete', ...)
line 556: title: const Text('Delete Section')
line 557-558: content: Text(...)
line 563: child: const Text('Cancel')
line 568: child: const Text('Delete')
line 578: content: Text('Section deleted')
line 962: child: const Text('Cancel')
line 967: child: const Text('Add')
line 1065: child: const Text('Cancel')
line 1070: child: const Text('Save')
```
- `objectCardDeleteItem` / `objectCardDeleteItemConfirm` / `objectCardCancel`
- `objectCardDelete` / `objectCardDeleteSection` / `objectCardDeleteSectionConfirm`
- `objectCardSectionDeleted` / `objectCardAdd` / `objectCardSave`

**`entry_action_builder.dart`**
```
line 29: tooltip: 'Copy All'
line 45: tooltip: 'Edit'
line 66: tooltip: 'Delete'
```
- `entryActionCopyAll` / `entryActionEdit` / `entryActionDelete`

**`object_tile.dart`**
```
line 97: tooltip: 'Edit'
line 107: tooltip: 'Delete'
```
- `objectTileEdit` / `objectTileDelete`

#### 2.18 附件与历史
**`attachment_list_sheet.dart`**
```
line 85:  title: Text(fileName)
line 187: title: Text(a.fileName)
line 188: subtitle: Text('${_formatSize(a.size)} • ${_formatDate(a.createdAt)}')
```
- 动态内容，无需 i18n

**`field_history_dialog.dart`**
```
line 142: child: const Text('Close')
```
- `fieldHistoryClose`

#### 2.19 搜索与过滤器
**`search_filters.dart`**
```
line 38: label: const Text('Public')
line 49: label: const Text('Internal')
line 60: label: const Text('Sensitive')
line 71: label: const Text('Restricted')
line 82: label: const Text('Unlock')
```
- `searchFilterPublic` / `searchFilterInternal` / `searchFilterSensitive`
- `searchFilterRestricted` / `searchFilterUnlock`

**`search_result_tile.dart`**
```
line 89: label: const Text('Reveal')
```
- `searchResultReveal`

#### 2.20 图标选择器
**`icon_picker_sheet.dart`**
```
line 51: Text('Choose Icon', ...)
```
- `iconPickerTitle`

#### 2.21 首页组件
**`page_editor.dart`** (widgets/home/)
```
line 125: title: const Text('Delete Section?')
line 126: content: const Text('This section and its items will be moved to trash.')
line 128: child: const Text('Cancel')
line 132: child: const Text('Delete')
line 151: title: Text(page != null ? 'Edit Page' : 'New Page')
line 165: child: const Text('Save')
line 187: hintText: 'Page title'
line 201: Text('Sections', ...)
line 205: label: const Text('Add Section')
line 250: title: Text(section.name) / subtitle: Text('${section.childrenIds.length} items')
line 307: title: Text(widget.initialTitle == null ? 'Add Section' : 'Edit Section')
line 316: labelText: 'Section Title'
line 317: hintText: 'Enter section title'
line 322: Text('Icon', ...)
line 333: child: const Text('Cancel')
line 343: child: const Text('Save')
```
- `homePageEditorDeleteSection` / `homePageEditorDeleteSectionDesc` / `homePageEditorCancel`
- `homePageEditorDelete` / `homePageEditorEditPage` / `homePageEditorNewPage`
- `homePageEditorSave` / `homePageEditorPageTitle` / `homePageEditorSections`
- `homePageEditorAddSection` / `homePageEditorItems` / `homePageEditorAddSectionTitle`
- `homePageEditorEditSection` / `homePageEditorSectionTitle` / `homePageEditorEnterSectionTitle`
- `homePageEditorIcon`

**`add_quick_action_dialog.dart`**
```
line 15: title: const Text('Add Quick Action')
line 42: child: const Text('Cancel')
line 92: title: Text(action.label)
```
- `homeAddQuickAction` / `homeAddQuickActionCancel`

#### 2.22 侧边栏
**`sidebar_header.dart`**
```
line 57: tooltip: 'Collapse'
line 68: tooltip: 'Expand'
```
- `sidebarCollapse` / `sidebarExpand`

#### 2.23 备份列表
**`backup_list_tile.dart`**
```
line 48: tooltip: 'Rename'
line 53: tooltip: 'Restore'
line 58: tooltip: 'Delete'
line 89: tooltip: 'Save as special backup'
line 94: tooltip: 'Restore'
line 99: tooltip: 'Delete'
```
- `backupRename` / `backupRestore` / `backupDelete` / `backupSaveAsSpecial`

#### 2.24 操作瓦片
**`operation_tile.dart`**
```
line 22: const Expanded(child: Text('Operation Details'))
line 112: child: const Text('Close')
line 344: tooltip: 'View details'
```
- `operationDetails` / `operationClose` / `operationViewDetails`

#### 2.25 调试日志
**`debug_log_sheet.dart`**
```
line 41: title: const Text('Copy Logs to Clipboard')
line 50: child: const Text('Cancel')
line 54: child: const Text('Copy')
line 70: Text('Sanitized logs copied to clipboard')
line 138: Text('Debug Log', ...)
line 144: tooltip: 'Refresh'
line 152: tooltip: 'Copy to clipboard'
line 160: tooltip: 'Disable debug mode'
```
- `debugCopyLogsTitle` / `debugCancel` / `debugCopy` / `debugLogsCopied`
- `debugLogTitle` / `debugRefresh` / `debugCopyToClipboard` / `debugDisableDebugMode`

#### 2.26 文件夹选择器
**`folder_picker_dialog.dart`**
```
line 134: tooltip: 'Go up'
line 174: child: const Text('Cancel')
line 179: child: const Text('Select This Folder')
line 258: title: Text(name)
```
- `folderPickerGoUp` / `folderPickerCancel` / `folderPickerSelectFolder`

#### 2.27 头部操作按钮
**`header_action_buttons.dart`**
```
line 30: tooltip: 'Lock Sensitivity Access'
```
- `headerLockSensitivity`

#### 2.28 日期选择器
**`date_picker_form_field.dart`**
```
line 67: labelText: label
line 73: tooltip: 'Clear date'
```
- `datePickerClear`

#### 2.29 扫描进度横幅
**`scan_progress_banner.dart`**
```
line 87: tooltip: 'Stop scan'
```
- `scanStop`

#### 2.30 登录组件
**`password_input_section.dart`**
```
line 204: labelText: 'Master Password'
line 205: hintText: 'Enter your password'
line 285: tooltip: 'Show password hint'
```
- `loginMasterPassword` / `loginEnterPassword` / `loginShowPasswordHint`

**`create_account_form.dart`**
```
line 68: labelText: 'Account Name' / hintText: 'e.g., Personal, Work'
line 86: labelText: 'Master Password' / hintText: 'Create a strong password'
line 120: labelText: 'Confirm Password' / hintText: 'Re-enter your password'
line 158: labelText: 'Password Hint (Optional)' / hintText: 'A hint to help you remember'
line 236: child: const Text('Back to Account List')
```
- `loginAccountName` / `loginAccountNameHint` / `loginMasterPassword`
- `loginMasterPasswordHint` / `loginConfirmPassword` / `loginConfirmPasswordHint`
- `loginPasswordHintOptional` / `loginPasswordHintHelp` / `loginBackToAccountList`

#### 2.31 对象卡片编辑字段
**`object_card_edit_field.dart`**
```
line 77: labelText: isTitle ? 'Title' : formatLabel(propertyKey)
```
- `objectCardEditTitle`（formatLabel 可能是动态生成的，需检查）

#### 2.32 对象卡片头部
**`object_card_header.dart`**
```
line 57: tooltip: 'Edit'
line 66: tooltip: 'Delete'
line 78: tooltip: 'Edit Section'
line 87: tooltip: 'Add Item'
```
- `objectCardHeaderEdit` / `objectCardHeaderDelete` / `objectCardHeaderEditSection` / `objectCardHeaderAddItem`

#### 2.33 对象卡片条目瓦片
**`object_card_item_tile.dart`**
```
line 128: tooltip: 'Copy'
line 134: tooltip: 'Edit'
line 158: tooltip: hasHist ? 'History ($count)' : 'No history yet'
line 165: tooltip: 'Delete'
line 223: tooltip: count == 1 ? '1 attachment' : '$count attachments'
```
- `objectCardItemCopy` / `objectCardItemEdit` / `objectCardItemHistory`
- `objectCardItemNoHistory` / `objectCardItemDelete` / `objectCardItemAttachment`
- `objectCardItemAttachments`

#### 2.34 回收站卡片
**`unified_object_trash_card.dart`**
```
line 196: title: Text(object.name)
line 299: child: const Text('Close')
line 380: tooltip: hasHist ? 'History ($count)' : 'No history yet'
line 393: label: const Text('History')
line 426: tooltip: label
```
- `trashClose` / `trashHistory` / `trashNoHistory`

#### 2.35 Section 卡片
**`section_card.dart`**
```
line 79: tooltip: 'Add'
line 187: tooltip: 'Add'
```
- `sectionCardAdd`

#### 2.36 分类页面
**`object_category_page.dart`**
```
line 26: title: Text(title)
```
- 动态标题，可能不需要 i18n

#### 2.37 条目卡片
**`entry_card_widget.dart`**
```
line 320: tooltip: hasHist ? 'History ($count)' : 'No history yet'
line 373: label: Text('History(${history?.entries.length ?? 0})')
line 454: tooltip: count == 1 ? '1 attachment' : '$count attachments'
```
- `entryCardHistory` / `entryCardNoHistory` / `entryCardAttachment` / `entryCardAttachments`

#### 2.38 账户列表
**`account_list_section.dart`**
```
line 52:  Text('No accounts found')
line 57:  label: const Text('Create Account')
line 118: Text('Last accessed: ...')
line 148: Text('Account list empty')
line 151: Text('Create your first account to get started')
```
- `loginNoAccounts` / `loginCreateAccount` / `loginLastAccessed`
- `loginAccountListEmpty` / `loginCreateFirstAccount`

#### 2.39 登录页面其他
**`login_page.dart` 中的其他字符串**
```
line 350: return 'Never'
line 353: return 'Today'
line 354: return 'Yesterday'
line 355: return '${diff.inDays} days ago'
```
这些建议保留为日期格式化逻辑，使用 `intl` 的 `DateFormat` 处理。

---

### P3 — 低优先级的工具提示和内部标签

#### 2.40 `glass_adapters.dart`
```
line 385: tooltip: 'Back'
```
- `commonBack`

#### 2.41 `operation_notification.dart`
```
line 389: tooltip: 'Dismiss'
```
- `commonDismiss`

---

## 三、实施建议

### 3.1 工作流

1. **逐文件修改**：一次聚焦一个页面/组件，避免大量文件同时改动导致 review 困难
2. **先生成 ARB**：在 `app_en.arb` 和 `app_zh.arb` 中成对添加 key，然后运行 `flutter gen-l10n`
3. **再改 Dart**：替换硬编码字符串为 `AppLocalizations.of(context).keyName`
4. **注意 `const`**：包含 `AppLocalizations.of(context)` 的 widget 不能是 `const`，需要移除 `const` 关键字
5. **每批提交**：每完成一个页面/组件就 analyze 检查，然后 commit

### 3.2 命名规范

| 前缀 | 适用范围 |
|------|----------|
| `common*` | 跨页面通用（Cancel, Save, Delete, Close, OK, Back） |
| `login*` | 登录/账户相关 |
| `home*` | 首页/工作台/页面编辑器 |
| `object*` | 对象/条目/section 编辑器和工作区 |
| `settings*` | 设置页（已部分完成） |
| `security*` | 安全设置/生物识别/密码验证 |
| `sensitivity*` | 敏感度设置 |
| `trash*` | 回收站 |
| `sync*` | 设备同步 |
| `scan*` | 扫描导入/预览/结果 |
| `localSearch*` | 本地搜索配置/进度 |
| `ocr*` | OCR 扫描（已完成） |
| `llm*` | AI 对话/配置/统计（已完成） |
| `dataManagement*` | 数据管理/备份 |
| `operationLog*` | 操作日志 |
| `debug*` | 调试日志 |
| `biometric*` | 生物识别设置 |
| `folderPicker*` | 文件夹选择器 |
| `search*` | 搜索页/过滤器/结果 |
| `iconPicker*` | 图标选择器 |
| `attachment*` | 附件列表 |
| `fieldHistory*` | 字段历史 |

### 3.3 重复利用已有 key

以下通用字符串应优先复用已有的 `common*` key，而非新增：
- `Cancel` → `commonCancel`（已存在）
- `Save` → `commonSave`（已存在）
- `Delete` → `commonDelete`（已存在）
- `Close` → `commonClose`（已存在）
- `Confirm` → `commonConfirm`（已存在）
- `Edit` → `commonEdit`（已存在）
- `Loading...` → `commonLoading`（已存在）
- `Error` → `commonError`（已存在）
- `Retry` → `commonRetry`（已存在）
- `Success` → `commonSuccess`（已存在）
- `Import` → `commonImport`（已存在）

### 3.4 参数化字符串处理

对于带变量的字符串（如 `"Delete \"${name}\"?"`），使用 ARB placeholder 机制：

```json
"objectCardDeleteItemConfirm": "Are you sure you want to delete \"{name}\"?",
"@objectCardDeleteItemConfirm": {
  "placeholders": { "name": { "type": "String" } }
}
```

Dart 调用：
```dart
AppLocalizations.of(context).objectCardDeleteItemConfirm(itemName)
```

### 3.5 日期格式化

`login_page.dart` 中的 `_formatLastAccessed` 方法包含 `Never`/`Today`/`Yesterday`/`N days ago`。建议：
- 保留 `_formatLastAccessed` 方法
- 将其中的硬编码字符串替换为 ARB key
- 或使用 `intl` 的 `DateFormat` 进行相对日期格式化

### 3.6 预估工作量

| 优先级 | 页面数 | 字符串数 | 预估时间 |
|--------|--------|----------|----------|
| P0 | 7 | ~120 | 2-3 小时 |
| P1 | 6 | ~150 | 2-3 小时 |
| P2 | ~25 | ~120 | 3-4 小时 |
| P3 | ~5 | ~20 | 30 分钟 |
| **总计** | **~43** | **~410** | **8-10 小时** |

---

## 四、建议执行顺序

1. **第一批（P0）**：`login_page`, `object_workspace_page`, `object_editor_page`, `page_editor_page`, `home_page`, `profile_page`, `search_page`
2. **第二批（P1）**：`settings_page`（剩余部分）, `security_settings_page`, `sensitivity_settings_page`, `trash_page`, `sync_page`, `data_management_page`
3. **第三批（P2）**：所有 widgets 和次级页面
4. **第四批（P3）**：tooltip 和剩余清理

---

*生成时间：2026-05-06*
*基于代码库扫描：~410 处硬编码英文 UI 字符串*
