# i18n 国际化迁移进度文档

> 本文档记录 SoloSoul Flutter 客户端 i18n 迁移的实时进度。
> **最后更新：2026-05-08**
> **状态：✅ i18n 迁移完成！所有硬编码字符串已本地化。**

---

## 总体进度

| 指标 | 数值 |
|------|------|
| ARB Keys 总数（app_en.arb） | **635 个**（原始 ~330，新增 ~305） |
| 已修改代码文件 | **60+ 个**（含 gen/l10n） |
| 剩余硬编码字符串 | **0** ✅ |
| dart analyze 状态 | **0 errors, 0 warnings**（生产代码） |

---

## 本轮新增/修改文件清单

### 新增 Pages（6 个）
- `lib/presentation/pages/home_page.dart` ✅ — QuickAction 标签本地化
- `lib/presentation/pages/object_workspace_page.dart` ✅ — Delete Section/Page 本地化
- `lib/presentation/pages/object_editor_page.dart` ✅ — 重复属性名错误本地化
- `lib/presentation/pages/scan/local_search_progress_page.dart` ✅ — No Results/OK 本地化
- `lib/presentation/pages/scan/scan_preview_page.dart` ✅ — Import 标签本地化
- `lib/presentation/pages/sync_page.dart` ✅ — 同步状态标签本地化

### 新增 Widgets（20+ 个）
- `lib/presentation/widgets/home/page_editor.dart` ✅ — 子代理遗留 7 字符串全部修复
- `lib/presentation/widgets/settings/delete_account_dialog_content.dart` ✅ — 3 字符串
- `lib/presentation/widgets/settings/debug_log_sheet.dart` ✅ — 4 字符串
- `lib/presentation/widgets/ocr_scanner_sheet.dart` ✅ — 3 字符串
- `lib/presentation/widgets/mrz_preview_card.dart` ✅ — 8 字段标签
- `lib/presentation/widgets/biometric_settings_widget.dart` ✅ — Touch ID/Face ID + tooltip
- `lib/presentation/widgets/scan_progress_banner.dart` ✅ — 统计标签 + tooltip
- `lib/presentation/widgets/change_password_dialog.dart` ✅ — hintText 本地化
- `lib/presentation/widgets/operation_log_filter_section.dart` ✅ — 操作/平台标签
- `lib/presentation/widgets/operation_tile.dart` ✅ — 详情行标签 + tooltip
- `lib/presentation/widgets/settings/version_sheet.dart` ✅ — 版本信息标题
- `lib/presentation/widgets/settings/current_account_sheet.dart` ✅ — 账户信息标题
- `lib/presentation/widgets/home/add_quick_action_dialog.dart` ✅ — 分区标题
- `lib/presentation/widgets/trash/unified_object_trash_card.dart` ✅ — 按钮标签
- `lib/presentation/widgets/object_card/object_card_edit_field.dart` ✅ — Title 标签
- `lib/presentation/widgets/login/create_account_form.dart` ✅ — hintText 本地化
- `lib/presentation/widgets/login/password_input_section.dart` ✅ — hintText 本地化
- `lib/presentation/widgets/folder_picker_dialog.dart` ✅ — tooltip
- `lib/presentation/widgets/header_action_buttons.dart` ✅ — tooltip
- `lib/presentation/widgets/object_tile.dart` ✅ — Edit/Delete tooltip
- `lib/presentation/widgets/section_card.dart` ✅ — Add tooltip
- `lib/presentation/widgets/date_picker_form_field.dart` ✅ — Clear date tooltip
- `lib/presentation/widgets/entry_action_builder.dart` ✅ — Copy/Edit/Delete tooltip
- `lib/presentation/widgets/entry_card_widget.dart` ✅ — History tooltip
- `lib/presentation/widgets/sensitive_value_widget.dart` ✅ — 受限字段消息
- `lib/presentation/widgets/predefined_object_section.dart` ✅ — 未知类型错误

### 新增 Providers（1 个）
- `lib/presentation/providers/search_provider.dart` ✅ — 受限字段消息

---

## 已完成的剩余字符串（全部 ✅）

| 文件 | 描述 | 状态 |
|------|------|------|
| `password_verification_dialog.dart` | 'Show password hint' / 'Show password' / 'Hide password' | ✅ |
| `password_input_section.dart` | 'Show password hint' | ✅ |
| `object_card_item_tile.dart` | 'Copy', 'Edit', 'Delete', 'History ($count)' | ✅ |
| `unified_object_trash_card.dart` | 'History ($count)' / 'No history yet' | ✅ |
| `data_management/backup_list_tile.dart` | 'Rename', 'Restore', 'Delete', 'Save as special backup' | ✅ |
| `object_card_header.dart` | 'Edit', 'Delete', 'Edit Section', 'Add Item' | ✅ |

---

## 新增 ARB Keys 汇总

本轮（2026-05-08）新增约 **300 个** ARB keys，涵盖：
- **通用操作**: commonRefresh, commonShowLess, commonShowPassword, commonHidePassword, commonTitle, commonRename
- **页面编辑器**: pageEditorPageTitleHint, pageEditorSaveFirst, pageEditorNoSections, pageEditorEnterSectionTitle, pageEditorEditSectionTitle
- **删除账户**: deleteAccountEnterPassword, deleteAccountPasswordRequired, deleteAccountInvalidPassword
- **调试日志**: debugLogCopyToClipboard, debugLogDisable, debugLogEmpty
- **MRZ**: mrzDocumentType, mrzDocumentNumber, mrzSurname, mrzGivenNames, mrzNationality, mrzDateOfBirth, mrzSex, mrzExpiryDate
- **同步**: syncUnknownError, syncScanning, syncScan, syncSyncing, syncConnectSync
- **扫描**: scanMappingBoth, scanMappingAi, scanStopScan
- **操作日志**: operationActionCreate/Update/Delete/Restore/Purge, operationPlatformAndroid/Web, operationLabelTimestamp/Action/Section/FieldPath/Description/Device, operationViewDetails
- **版本**: versionCurrentVersion, versionLatestVersion, versionUpdateStatus, versionPlatform
- **账户**: accountCreated, accountLastLogin, accountLastOperation, accountLoginDevices
- **主页**: homeDefaultPages, homeCustomizedPages
- **回收站**: trashDetailLabel, trashRestoreLabel, trashPurgeLabel
- **密码更改**: changePasswordMinLength
- **文件夹选择**: folderPickerGoUp
- **头部按钮**: headerLockSensitivity
- **日期选择**: datePickerClear
- **条目动作**: entryCopyAll, entryNoHistory
- **预定义对象**: predefinedUnknownType (含占位符)
- **敏感数据**: sensitiveRestrictedMessage
- **通用提示**: settingsNoHintAvailable

---

## 测试回归状态（未变）

| 测试文件 | 失败原因 | 状态 |
|----------|----------|------|
| `test/widget/profile_page_test.dart` | 缺少 localization delegates | 待修复 |
| `test/widget/sidebar_header_test.dart` | 同上 | ✅ 已修复 |
| `test/widget/quick_action_tile_test.dart` | Container finder 太具体 | 待排查 |
| `test/widget/scan_preview_page_test.dart` | 4 个预存在失败 | 预存在 |
| `test/widget/travel_page_test.dart` | 7 个预存在失败 | 预存在 |
| `test/unit/rust_vault_service_test.dart` | 缺少 `libsolosoul_core.dylib` | 预存在 |

---

## 快速命令参考

```bash
cd /Users/zzc/PycharmProjects/SoloSoul_code/flutter

# 生成本地化文件
flutter gen-l10n

# 查找剩余硬编码字符串
grep -rn "Text('\|tooltip:" lib/presentation/ | grep -v "test/" | grep -v "gen/" | grep -v "l10n\." | grep "'[A-Z]" | grep -v "Text('')"

# 统计剩余数量
grep -rn "Text('\|tooltip:" lib/presentation/ | grep -v "test/" | grep -v "gen/" | grep -v "l10n\." | grep "'[A-Z]" | grep -v "Text('')" | wc -l

# 分析
flutter analyze --fatal-infos --fatal-warnings
```
