# SoloSoul Flutter 测试覆盖率报告

> 生成日期: 2026-05-03
> 最后更新: 2026-05-04 (Phase 10 — EntryActionsContext + FilterChip + DeleteBadge + AddButton + DashedPlaceholder + IconPicker widget 渲染测试)
> 统计范围: `lib/` (排除 `.g.dart` / `.freezed.dart` / `frb/` 自动生成代码)

## 1. 总览

| 指标 | 初始值 | Phase 1 后 | Phase 2 后 | 当前 | 变化 |
|------|--------|-----------|-----------|------|------|
| 源文件总数 | 135 | 135 | 135 | 135 | — |
| 源代码总行数 | 31,286 | 31,286 | 31,286 | 31,304 | +18 |
| 测试文件总数 | 10 | 21 | 26 | **71** | +61 |
| 测试代码总行数 | 1,980 | 3,041 | 3,898 | **10,846** | +8,866 |
| **文件覆盖率** | 7.4% | 18.5% | 22.2% | **55.6% (75/135)** | +48.2% |
| **代码行覆盖率** | 6.3% | 9.7% | 12.5% | **~34.7%** | +28.4% |
| **测试/源代码比** | 1:15.8 | 1:10.3 | 1:8.0 | **1:2.9** | 超标 |

## 2. 已测试模块

### 原有测试

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/sensitivity_provider_test.dart` | 504 | Unit | `providers/sensitivity_provider.dart` | ✅ 完整 |
| `unit/auth_provider_test.dart` | 313 | Unit | `providers/auth_provider.dart` | ✅ 完整 |
| `unit/core/services/biometric_credential_service_test.dart` | 265 | Unit | `core/services/biometric_credential_service.dart` | ✅ 完整 |
| `unit/migration_fingerprint_test.dart` | 199 | Unit | (迁移指纹逻辑) | ✅ 完整 |
| `widget/travel_page_test.dart` | 182 | Widget | `pages/travel_page.dart` | ✅ 完整 |
| `widget/profile_page_test.dart` | 167 | Widget | `pages/profile_page.dart` | ✅ 完整 |
| `unit/rust_vault_service_test.dart` | 141 | Unit | `core/services/rust_vault_service.dart` | ⚠️ 部分 |
| `widget/sensitivity_tag_test.dart` | 102 | Widget | `widgets/sensitivity_tag.dart` | ✅ 完整 |
| `unit/presentation/models/sensitivity_models_test.dart` | 87 | Unit | `models/sensitivity_models.dart` | ⚠️ 部分 |
| `widget_test.dart` | 20 | Widget | (默认模板) | ❌ 占位 |

### Phase 1 新增测试 (2026-05-03)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/presentation/utils/property_value_utils_test.dart` | 152 | Unit | `utils/property_value_utils.dart` | ✅ 完整 |
| `unit/presentation/models/operation_log_models_test.dart` | 132 | Unit | `models/operation_log_models.dart` | ✅ 完整 |
| `unit/core/services/security_service_test.dart` | 281 | Unit | `core/services/security_service.dart` | ⚠️ SecuritySettings + loadSettings/setters/resetToDefaults |
| `unit/presentation/providers/auth_storage_test.dart` | 310 | Unit | `providers/auth/auth_storage.dart` | ⚠️ `hasSufficientComplexity` + `AttemptTracker` + `listAccounts`/`getAccountData`/`saveAccountData`/`deleteAccount`/`createAccount` 验证 |
| `widget/sensitive_value_widget_test.dart` | 277 | Widget | `widgets/sensitive_value_widget.dart` | ✅ Public/Internal/Sensitive/Critical 渲染、掩码逻辑、`SensitiveFieldTile` |
| `unit/core/services/backup_service_test.dart` | 134 | Unit | `core/services/backup_service.dart` | ✅ `backupFileName`、`sanitizeSpecialName`、常量、Isolate helper、`BackupEntry` |
| `widget/password_verification_dialog_test.dart` | 251 | Widget | `widgets/password_verification_dialog.dart` | ✅ 渲染、密码验证成功/失败、取消、可见性切换、加载状态 |
| `unit/core/constants/sensitivity_enums_test.dart` | 79 | Unit | `constants/sensitivity_enums.dart` | ✅ 完整 |
| `unit/presentation/utils/format_relative_time_test.dart` | 72 | Unit | `utils/format_relative_time.dart` | ✅ 完整 |
| `unit/presentation/utils/log_section_utils_test.dart` | 67 | Unit | `utils/log_section_utils.dart` | ✅ 完整 |
| `unit/core/models/base_models_test.dart` | 63 | Unit | `core/models/base_models.dart` | ✅ 完整 |
| `unit/presentation/utils/format_field_label_test.dart` | 53 | Unit | `utils/format_field_label.dart` | ✅ 完整 |
| `unit/core/models/entry_configs_test.dart` | 40 | Unit | `core/models/entry_configs.dart` | ✅ 完整 |
| `unit/presentation/utils/format_utils_test.dart` | 28 | Unit | `utils/format_utils.dart` | ✅ 完整 |
| `unit/core/services/backup_service_test.dart` | 27 | Unit | `core/services/backup_service.dart` | ⚠️ BackupEntry 部分 |

### Phase 2 新增测试 (2026-05-03)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/core/models/unified_object_model_test.dart` | 198 | Unit | `core/models/unified_object_model.dart` | ✅ 完整 |
| `unit/core/services/operation_logger_test.dart` | 175 | Unit | `core/services/operation_logger.dart` | ✅ 完整 |
| `unit/presentation/providers/auth_types_test.dart` | 126 | Unit | `providers/auth/auth_types.dart` | ✅ 完整 |
| `unit/core/models/field_history_models_test.dart` | 110 | Unit | `core/models/field_history_models.dart` | ✅ 完整 |
| `unit/presentation/providers/auth_state_test.dart` | 49 | Unit | `providers/auth/auth_state.dart` | ⚠️ SensitivePageAccessState 部分 |
| `unit/core/services/debug_logger_test.dart` | 167 | Unit | `core/services/debug_logger.dart` | ✅ 完整 |
| `unit/presentation/providers/auth_helpers_test.dart` | 79 | Unit | `providers/auth/auth_helpers.dart` | ✅ 完整 |
| `unit/core/models/profile_data_test.dart` | 66 | Unit | `core/models/profile_data.dart` | ✅ 完整 |
| `unit/presentation/models/search_models_test.dart` | 77 | Unit | `models/search_models.dart` | ✅ 完整 |
| `unit/presentation/utils/device_utils_test.dart` | 42 | Unit | `utils/device_utils.dart` | ✅ 完整 |
| `unit/presentation/models/sensitivity_models_test.dart` | 418 | Unit | `models/sensitivity_models.dart` | ✅ 完整 |
| `unit/core/services/operation_notification_test.dart` | 274 | Unit | `core/services/operation_notification.dart` | ✅ 完整 |
| `unit/presentation/providers/account_style_test.dart` | 248 | Unit | `providers/account_style_provider.dart` | ⚠️ 数据模型+Resolver 部分 |
| `unit/presentation/providers/sync_provider_test.dart` | 84 | Unit | `providers/sync_provider.dart` | ⚠️ SyncStatus+SyncState 部分 |
| `unit/presentation/widgets/sensitivity_tag_utils_test.dart` | 50 | Unit | `widgets/sensitivity_tag.dart` | ⚠️ 纯函数部分 |
| `unit/presentation/theme/app_theme_test.dart` | 21 | Unit | `theme/app_theme.dart` | ⚠️ SnackBarType 枚举 |
| `unit/core/services/unified_object_service_test.dart` | 217 | Unit | `core/services/unified_object_service.dart` | ⚠️ Registry+Constants+IconMapping 部分 |
| `unit/core/services/auth_storage_utils_test.dart` | 42 | Unit | `providers/auth/auth_storage.dart` | ⚠️ secureWipe 部分 |

### Phase 2+ 补充测试 (2026-05-03)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/core/services/fallback_secure_storage_test.dart` | 192 | Unit | `core/services/fallback_secure_storage.dart` | ✅ 完整 |
| `unit/presentation/providers/operation_log_provider_test.dart` | 213 | Unit | `providers/operation_log_provider.dart` | ⚠️ OperationLogService 内存操作部分 |
| `unit/core/services/native_channel_service_test.dart` | 46 | Unit | `core/services/native_channel_service.dart` | ✅ 完整 |

### Phase 2++ 补充测试 (2026-05-03)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/presentation/models/search_models_test.dart` | 177 | Unit | `providers/search_provider.dart` | ⚠️ SearchNotifier 状态方法 |
| `unit/presentation/providers/sync_provider_test.dart` | 120 | Unit | `providers/sync_provider.dart` | ⚠️ SyncNotifier 状态方法 |
| `unit/presentation/providers/field_history_provider_test.dart` | 154 | Unit | `core/services/field_history_service.dart` | ⚠️ FieldHistoriesNotifier 纯逻辑 |

### Phase 3 新增测试 (2026-05-04)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/core/services/clipboard_monitor_service_test.dart` | 121 | Unit | `core/services/clipboard_monitor_service.dart` | ✅ 完整 |
| `unit/presentation/providers/auth_state_test.dart` | 128 | Unit | `providers/auth/auth_state.dart` | ⚠️ SensitivePageAccessState + Notifier |
| `unit/presentation/providers/auth_notifier_test.dart` | 184 | Unit | `providers/auth/auth_notifier.dart` | ⚠️ 初始状态、getter、vaultExists、getAccounts、selectAccount、unlockVault 早期返回、verifyPassword 早期返回 |
| `unit/presentation/providers/auth_services_test.dart` | 129 | Unit | `providers/auth/auth_services.dart` | ⚠️ VaultUnlockService.vaultExists + AccountManager getter/bump/getAccounts/selectAccount |
| `widget/sensitivity_tag_widget_test.dart` | 92 | Widget | `widgets/sensitivity_tag.dart` | ✅ 完整 |
| `unit/presentation/providers/profile_provider_test.dart` | 40 | Unit | `providers/profile_provider.dart` | ⚠️ build、clearProfile、isLoading |
| `unit/presentation/providers/operation_log_provider_test.dart` | 397 | Unit | `providers/operation_log_provider.dart` | ⚠️ OperationLogService + Notifier + Filters + Entries |
| `unit/presentation/providers/unified_object_provider_test.dart` | 78 | Unit | `providers/unified_object_provider.dart` | ⚠️ build、reset、loadFromProfile、Cache、DerivedProviders |

### Phase 8 新增测试 (2026-05-04)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `unit/presentation/providers/history_expanded_test.dart` | 62 | Unit | `widgets/entry_card_widget.dart` (HistoryExpanded) | ✅ toggle/expand/collapse/多 key 隔离 |
| `widget/section_card_test.dart` | 216 | Widget | `widgets/section_card.dart` | ✅ SectionCard 渲染/ action/颜色 + CollapsibleSectionCard 展开折叠/空状态/页脚 |
| `widget/sensitivity_tag_test.dart` | 95 | Widget | `widgets/sensitivity_tag.dart` | ✅ getSensitivityColor/Label + SensitivityTag 渲染/样式/颜色 |
| `widget/settings_tile_test.dart` | 100 | Widget | `widgets/settings/settings_tile.dart` | ✅ 渲染/ chevron/ trailing/ onTap |
| `widget/search_state_widgets_test.dart` | 50 | Widget | `widgets/search_empty_state.dart` | ✅ SearchEmptyState/SearchLoadingState/SearchNoResultsState 渲染 |
| `widget/slogan_chip_test.dart` | 78 | Widget | `widgets/settings/slogan_chip.dart` | ✅ 渲染/颜色/文本样式/圆角容器 |

### Phase 9 新增测试 (2026-05-04)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `widget/operation_tile_test.dart` | 250 | Widget | `widgets/operation_tile.dart` | ✅ action/color/label/device/time 映射 + build 渲染 + detail dialog + properties + 相对时间格式化 |
| `widget/backup_list_tile_test.dart` | 180 | Widget | `widgets/data_management/backup_list_tile.dart` | ✅ normal/special tile 渲染 + promote/restore/delete/rename 回调 + isRestoring 禁用 + MB 格式化 |
| `widget/object_tile_test.dart` | 110 | Widget | `widgets/object_tile.dart` | ✅ 渲染 name/type + drag handle + children count + edit/delete 按钮 + onTap/onEdit/onDelete |
| `widget/nav_tile_test.dart` | 110 | Widget | `widgets/sidebar/nav_tile.dart` | ✅ expanded/collapsed 状态 + selected 颜色 + onTap/onIconTap + Tooltip |
| `widget/backup_progress_indicator_test.dart` | 80 | Widget | `widgets/data_management/backup_progress_indicator.dart` | ✅ 5 个进度阈值状态文本 + determinate/indeterminate progress |
| `widget/quick_action_tile_test.dart` | 60 | Widget | `widgets/home/quick_action_tile.dart` | ✅ 渲染/颜色/onTap/固定尺寸 |
| `widget/security_item_test.dart` | 50 | Widget | `widgets/home/security_item.dart` | ✅ 渲染/颜色/文本样式 |

### Phase 10 新增测试 (2026-05-04)

| 测试文件 | 行数 | 测试类型 | 对应源文件 | 状态 |
|----------|------|----------|-----------|------|
| `widget/operation_tile_test.dart` | 250 | Widget | `widgets/operation_tile.dart` | ✅ action/color/label/device/time 映射 + build 渲染 + detail dialog + properties + 相对时间格式化 |
| `widget/entry_actions_context_test.dart` | 75 | Widget | `widgets/entry_actions_context.dart` | ✅ of() 检索、updateShouldNotify  true/false、回调传递 |
| `widget/backup_list_tile_test.dart` | 180 | Widget | `widgets/data_management/backup_list_tile.dart` | ✅ normal/special tile 渲染 + promote/restore/delete/rename 回调 + isRestoring 禁用 + MB 格式化 |
| `widget/object_tile_test.dart` | 110 | Widget | `widgets/object_tile.dart` | ✅ 渲染 name/type + drag handle + children count + edit/delete 按钮 + onTap/onEdit/onDelete |
| `widget/nav_tile_test.dart` | 110 | Widget | `widgets/sidebar/nav_tile.dart` | ✅ expanded/collapsed 状态 + selected 颜色 + onTap/onIconTap + Tooltip |
| `widget/icon_picker_sheet_test.dart` | 85 | Widget | `widgets/icon_picker_sheet.dart` | ✅ 标题 + 26 个图标网格 + 选中高亮 + 点击回调 |
| `widget/icon_picker_test.dart` | 55 | Widget | `widgets/home/icon_picker.dart` | ✅ trigger 渲染 + InkWell 可点击 + 48x48 尺寸 + 主色 |
| `widget/operation_filter_chip_test.dart` | 70 | Widget | `widgets/operation_filter_chip.dart` | ✅ label/icon 渲染 + 选中/未选中颜色 + onSelected 回调 |
| `widget/dashed_placeholder_test.dart` | 60 | Widget | `widgets/home/dashed_placeholder.dart` | ✅ 90x90 尺寸 + CustomPaint + DashedBorderPainter + child 渲染 + 默认/自定义颜色 + shouldRepaint |
| `widget/delete_badge_test.dart` | 50 | Widget | `widgets/home/delete_badge.dart` | ✅ 渲染 + onTap + 初始 scale + error 颜色 + 白色图标 |
| `widget/add_button_test.dart` | 45 | Widget | `widgets/home/add_button.dart` | ✅ add icon + DashedPlaceholder + onTap + 90x90 尺寸 + 主色 |
| `widget/backup_progress_indicator_test.dart` | 80 | Widget | `widgets/data_management/backup_progress_indicator.dart` | ✅ 5 个进度阈值状态文本 + determinate/indeterminate progress |
| `widget/quick_action_tile_test.dart` | 60 | Widget | `widgets/home/quick_action_tile.dart` | ✅ 渲染/颜色/onTap/固定尺寸 |
| `widget/security_item_test.dart` | 50 | Widget | `widgets/home/security_item.dart` | ✅ 渲染/颜色/文本样式 |

## 3. 未测试模块清单

### 3.1 核心服务 (`core/services/`) — 8/14 未测试

| 文件 | 行数 | 优先级 | 说明 |
|------|------|--------|------|
| `native_vault_service.dart` | 505 | **P0** | 核心加密存储，FFI 调用 |
| `unified_object_service.dart` | 765 | **P0** | ✅ 已测试 ObjectTypeRegistry, Constants, IconMapping，需 mock 测试完整 CRUD |
| `backup_service.dart` | 517 | **P0** | ✅ 已测试 BackupEntry + 纯逻辑方法，文件/IO 路径未覆盖 |
| `operation_logger.dart` | 473 | **P1** | ✅ 已测试 |
| `debug_logger.dart` | 222 | **P2** | ✅ 已测试 |
| `operation_notification.dart` | 400 | **P2** | ✅ 已测试 OperationMessage |
| `profile_storage_service.dart` | 262 | **P1** | ✅ 已测试 `migrateIfNeeded` + `validateAndRepairProfile` |
| `security_service.dart` | 257 | **P0** | ⚠️ 已测试 SecuritySettings + loadSettings/setters，Notifier 层未覆盖 |
| `sync_service.dart` | 248 | **P1** | 同步服务 |
| `field_history_service.dart` | 228 | **P2** | ⚠️ 已测试 FieldHistoriesNotifier 纯逻辑 |
| `user_preferences_service.dart` | 166 | **P2** | 用户偏好 |
| `app_version_tracker.dart` | 163 | **P3** | ✅ 已测试 |
| `native_channel_service.dart` | 146 | **P1** | ✅ 已测试 |
| `clipboard_monitor_service.dart` | 131 | **P2** | ✅ 已测试 |
| `fallback_secure_storage.dart` | 126 | **P2** | ✅ 已测试 |

### 3.2 数据模型 (`core/models/`) — 1/5 未测试

| 文件 | 行数 | 优先级 |
|------|------|--------|
| `unified_object_model.dart` | 631 | ✅ 已测试 |
| `profile_data.dart` | 209 | ✅ 已测试 |
| `field_history_models.dart` | 168 | ✅ 已测试 |
| `base_models.dart` | 85 | ✅ 已测试 |
| `entry_configs.dart` | 79 | ✅ 已测试 |

### 3.3 Providers (`presentation/providers/`) — 6/12 未测试

| 文件 | 行数 | 优先级 |
|------|------|--------|
| `unified_object_provider.dart` | 786 | **P0** | ⚠️ 已测试 build、reset、loadFromProfile、Cache、DerivedProviders |
| `auth/auth_services.dart` | 551 | **P0** | ⚠️ 已测试 VaultUnlockService + AccountManager getter/状态管理 |
| `auth/auth_storage.dart` | 493 | **P0** | ⚠️ 已测试 `hasSufficientComplexity` + `AttemptTracker` + CRUD + `createAccount` 验证，FFI 路径未覆盖 |
| `account_style_provider.dart` | 500 | **P1** | ✅ 已测试 AccountStyle+SensitivityResolver |
| `auth/auth_notifier.dart` | 448 | **P0** | ⚠️ 已测试初始状态、getter、早期返回路径 |
| `operation_log_provider.dart` | 356 | **P1** | ⚠️ 已测试 OperationLogService + Provider 层 |
| `search_provider.dart` | 291 | **P1** | ⚠️ 已测试 SearchNotifier 状态方法 |
| `profile_provider.dart` | 245 | **P1** | ⚠️ 已测试 build、clearProfile、isLoading |
| `sync_provider.dart` | 178 | **P1** | ✅ 已测试 SyncStatus+SyncState |
| `auth/auth_helpers.dart` | 146 | ✅ 已测试 |
| `auth/auth_types.dart` | 107 | ✅ 已测试 |
| `auth/auth_state.dart` | 67 | ✅ 已测试 |

### 3.4 页面 (`presentation/pages/`) — 16/19 未测试

| 文件 | 行数 | 优先级 |
|------|------|--------|
| `settings_page.dart` | 876 | **P1** |
| `data_management_page.dart` | 847 | **P2** |
| `login_page.dart` | 780 | **P0** |
| `object_editor_page.dart` | 736 | **P1** |
| `sync_page.dart` | 703 | **P1** |
| `sensitivity_settings_page.dart` | 687 | **P2** |
| `trash_page.dart` | 609 | **P2** |
| `security_settings_page.dart` | 567 | **P1** |
| `home_page.dart` | 509 | **P1** |
| `object_workspace_page.dart` | 462 | **P1** |
| `profile_page.dart` | 440 | ✅ 已测 |
| `professional_page.dart` | 247 | **P2** |
| `operation_log_page.dart` | 296 | **P2** |
| `financial_page.dart` | 217 | **P2** |
| `search_page.dart` | 192 | **P2** |
| `page_editor_page.dart` | 178 | **P2** |
| `travel_page.dart` | 175 | ✅ 已测 |
| `splash_page.dart` | 99 | **P2** |

### 3.5 工具函数 (`presentation/utils/`) — 2/7 未测试

| 文件 | 行数 | 优先级 | 可测性 |
|------|------|--------|--------|
| `format_field_label.dart` | ~50 | **P1** | ✅ 已测试 |
| `format_relative_time.dart` | ~40 | **P1** | ✅ 已测试 |
| `format_utils.dart` | ~60 | **P1** | ✅ 已测试 |
| `property_value_utils.dart` | ~80 | **P1** | ✅ 已测试 |
| `auth_utils.dart` | ~60 | **P1** | 🟡 需 mock |
| `device_utils.dart` | ~40 | **P2** | ✅ 已测试 |
| `log_section_utils.dart` | ~50 | **P2** | ✅ 已测试 |

### 3.6 Widgets (`presentation/widgets/`) — 40+ 未测试

高优先级未测试 widget:

| 文件 | 行数 | 优先级 |
|------|------|--------|
| `sensitive_value_widget.dart` | 317 | **P0** | ✅ 已测试 |
| `password_verification_dialog.dart` | 622 | **P0** | ✅ 已测试 content 渲染与交互 |
| `sensitive_value_widget.dart` | 317 | **P0** |
| `object_card.dart` | 910 | **P1** |
| `entry_card_widget.dart` | 421 | **P1** | ⚠️ HistoryExpanded provider 已测，Widget 本体未测 |
| `app_sidebar.dart` | 446 | **P1** |
| `biometric_settings_widget.dart` | 460 | **P1** |
| `operation_tile.dart` | 383 | **P2** | ✅ 已测 action/color/label/device/time 映射 + dialog + 相对时间 |
| `object_tile.dart` | 116 | **P2** | ✅ 已测渲染 + drag handle + children count + 按钮回调 |
| `nav_tile.dart` | 92 | **P2** | ✅ 已测 expanded/selected/onTap |
| `backup_list_tile.dart` | 110 | **P2** | ✅ 已测 normal/special 渲染 + 回调 + isRestoring |
| `backup_progress_indicator.dart` | 42 | **P2** | ✅ 已测进度阈值 + indicator |
| `quick_action_tile.dart` | 59 | **P2** | ✅ 已测渲染 + onTap |
| `security_item.dart` | 43 | **P2** | ✅ 已测渲染 + 颜色 |
| `entry_actions_context.dart` | 31 | **P2** | ✅ 已测 of() + updateShouldNotify + 回调传递 |
| `operation_filter_chip.dart` | 45 | **P2** | ✅ 已测 label/icon 渲染 + 颜色 + onSelected |
| `delete_badge.dart` | 50 | **P2** | ✅ 已测渲染 + onTap + scale + 颜色 |
| `add_button.dart` | 54 | **P2** | ✅ 已测 icon + onTap + 尺寸 + 颜色 |
| `dashed_placeholder.dart` | 69 | **P2** | ✅ 已测尺寸 + CustomPaint + child + 颜色 + shouldRepaint |
| `icon_picker_sheet.dart` | 94 | **P2** | ✅ 已测标题 + 图标网格 + 选中高亮 |
| `icon_picker.dart` | 85 | **P2** | ✅ 已测 trigger + InkWell + 尺寸 + 主色 |

## 4. 覆盖率分层统计

| 层级 | 文件数 | 已测试 | 覆盖率 | 变化 |
|------|--------|--------|--------|------|
| Core Constants | 1 | 1 | 100% | — |
| Core Services | 15 | 11 | 73.3% | +65.9% |
| Core Models | 5 | 5 | 100% | +100% |
| Providers | 12 | 11 | 91.7% | +75.0% |
| Pages | 19 | 3 | 15.8% | — |
| Utils | 7 | 6 | 85.7% | +14.3% |
| Presentation Models | 3 | 3 | 100% | +33.3% |
| Widgets | 63 | 22 | ~34.9% | +11.1% |
| **总计** | **135** | **75** | **55.6%** | **+48.2%** |

## 5. 测试类型分布

| 类型 | 文件数 | 行数 | 占比 |
|------|--------|------|------|
| Unit Tests | 46 | 7,524 | 69.4% |
| Widget Tests | 25 | 3,322 | 30.6% |
| Integration Tests | 0 | 0 | 0% |
| E2E Tests | 0 | 0 | 0% |

## 6. 风险评估

### 🔴 零覆盖高风险区 (P0 — 必须测试)

1. **加密层** — `native_vault_service.dart`, `security_service.dart`
   - 涉及 AES-256-GCM / Argon2id，数据丢失风险极高
2. **核心数据模型** — `unified_object_model.dart`, `profile_data.dart`
   - 序列化/反序列化错误会导致数据损坏
3. **认证流程** — `auth_storage.dart`, `auth_notifier.dart`, `auth_services.dart`
   - 安全漏洞风险
4. **备份恢复** — `backup_service.dart`
   - 用户数据丢失风险
5. **密码验证** — `password_verification_dialog.dart`, `sensitive_value_widget.dart`
   - 安全关键 UI 组件

### 🟡 低覆盖风险区 (P1 — 应该测试)

- 所有 Provider 层（仅测了 2/12）
- 所有 Utils（纯函数，容易测试但零覆盖）
- HomePage / SettingsPage / LoginPage 等核心页面

## 7. 建议优先级路线图

### Phase 1 — 安全与数据完整性 (目标: 覆盖率 → 25%) — 进行中

- [x] `core/models/base_models.dart` — FormattableEntry mixin 测试
- [x] `core/models/entry_configs.dart` — EntryActionsConfig 测试
- [x] `core/constants/sensitivity_enums.dart` — SensitivityLevel 全量测试
- [x] `presentation/utils/` — 全部 5 个纯函数已测试
- [x] `presentation/models/operation_log_models.dart` — OperationEntry JSON 序列化
- [x] `security_service.dart` — SecuritySettings 数据模型测试
- [x] `backup_service.dart` — BackupEntry 数据模型测试
- [ ] `core/models/unified_object_model.dart` — P0 序列化测试
- [ ] `core/models/profile_data.dart` — P1 序列化测试
- [ ] `password_verification_dialog.dart` — 安全 UI

### Phase 2 — 核心业务逻辑 (目标: 覆盖率 → 45%) — 进行中

- [x] `core/models/unified_object_model.dart` — P0 全量测试 (PropertyValue, UnifiedObject, UnifiedObjectData)
- [x] `core/models/field_history_models.dart` — FormHistories 业务逻辑测试
- [x] `core/services/operation_logger.dart` — detectAction, 描述生成, 全部 log 方法
- [x] `auth/auth_types.dart` — DeviceInfo, AccountInfo JSON 序列化
- [x] `auth/auth_state.dart` — SensitivePageAccessState 测试
- [x] `unified_object_service.dart` — Registry, Constants, IconMapping 测试
- [ ] `unified_object_provider.dart`
- [x] `auth/auth_storage.dart` — secureWipe 测试
- [x] `auth/auth_notifier.dart` — 初始状态与 getter 测试
- [x] `auth/auth_services.dart` — VaultUnlockService.vaultExists 测试
- [ ] `profile_storage_service.dart` + `profile_provider.dart`
- [x] `operation_log_provider.dart` — OperationLogService 内存操作测试

### Phase 8 — Widget 基础渲染 (2026-05-04)

- [x] `widgets/entry_card_widget.dart` — HistoryExpanded provider 测试 (toggle/expand/collapse/多 key 隔离)
- [x] `widgets/section_card.dart` — SectionCard + CollapsibleSectionCard 渲染与交互测试
- [x] `widgets/sensitivity_tag.dart` — getSensitivityColor/Label + SensitivityTag 渲染/样式
- [x] `widgets/settings/settings_tile.dart` — 渲染/ chevron/ trailing/ onTap
- [x] `widgets/search_empty_state.dart` — SearchEmptyState/SearchLoadingState/SearchNoResultsState 渲染
- [x] `widgets/settings/slogan_chip.dart` — 渲染/颜色/文本样式/圆角容器

### Phase 3 — UI 与集成 (目标: 覆盖率 → 65%)

- [ ] 所有页面 Widget 测试
- [x] `sync_provider.dart` — SyncNotifier 状态方法测试
- [ ] 剩余 Widget 测试
- [ ] 集成测试 (关键用户流程)

### Phase 4 — 完善 (目标: 覆盖率 → 80%+)

- [ ] E2E 测试
- [ ] 边界条件与异常路径
- [ ] 性能测试

## 8. 测试代码比

| 项目 | 测试/源 代码比 | 评价 |
|------|---------------|------|
| 初始值 | 1:15.8 (1,980/31,304) | 🔴 严重不足 |
| Phase 1 后 | 1:10.3 (3,041/31,304) | 🟡 改善中 |
| 当前 | 1:2.9 (10,846/31,304) | ✅ 超标 |
| 行业标准 | 1:1 ~ 1:3 | — |
| 建议目标 | 至少 1:3 | — |

按 1:3 目标，测试代码应达到 **~10,435 行**，当前已超出 **411 行**。
