# Changelog

All notable changes to SoloSoul are documented in this file.

## [1.4.9] - 2026-05-08

### Fixed

- **LLM Config Page** — Fixed redirecting to home page in release mode. `AppRoutes.llmConfig` was incorrectly included in `debugOnlyRoutes` guard set in `app_router.dart`, causing the redirect to return `AppRoutes.home` in non-debug builds.
- **Local Search & Scan Routes** — Removed the entire `debugOnlyRoutes` guard block. `localSearch`, `localSearchProgress`, `scanPreview`, `scanImportResult` are now accessible in production builds.

## [1.4.8] - 2026-05-08

## [1.4.8] - 2026-05-08

### Added

- **Auto Language Detection** — `LanguageNotifier.build()` now detects OS locale via `PlatformDispatcher.instance.locale`. Chinese OS → zh, all others → en. Added `hasStoredPreference()` to `LanguageService` to distinguish first launch from user choice.
- **Version Auto-Injection** — `build_dmg.sh` now injects VERSION into `pubspec.yaml` before `flutter build macos --release` and restores via `trap EXIT`.
- **Update Notification** — Version sheet uses semantic version comparison (`_isUpdateAvailable`), shows "update available" button that opens GitHub Releases in browser (macOS) or copies URL to clipboard.
- **Password Dialog Fixed Width** — Replaced `showDialog` (which uses `IntrinsicWidth`) with `showGeneralDialog` + `Center` + `SizedBox(width: 360)` inside AlertDialog content; prevents dialog resizing on password input.

### i18n — Comprehensive Completion

- **935 ARB Keys** (up from ~330), 60+ files modified, 0 hardcoded strings remaining
- **Pages**: home, search, sync, trash, operation_log, sensitivity_settings, settings, data_management, login, profile, travel, financial, professional
- **Widgets**: password_verification_dialog, ocr_scanner_sheet, scan_document_button, search_empty_state, section_card, entry_card_widget, object_tile, predefined_object_section, change_password_dialog, field_history_dialog, lock_vault_dialog, search_result_tile, scan_progress_banner, date_picker_form_field, header_action_buttons, folder_picker_dialog, entry_action_builder, mrz_preview_card, version_sheet, current_account_sheet, all_accounts_sheet, debug_log_sheet, delete_account_dialog_content, add_quick_action_dialog, backup_list_tile, object_card_edit_field, create_account_form, password_input_section, empty_profiles_state, scan_document_button
- **Field Label Translation** — `translateFieldLabel(key, l10n)` switch-based lookup for ~80 built-in property keys; used in `object_card_edit_field`, `operation_tile`, `trash/unified_object_trash_card`
- **Sensitivity Labels Unified** — `sensitivityCritical` → "Restricted"/"受限"; added `localizedLabel(l10n)` to `SensitivityLevelExtension`
- **Quick Action Labels** — `QuickAction.localizedLabel()` maps routes to sidebar l10n keys
- **Privacy Policy & Terms** — Chinese `PRIVACY_POLICY_zh.md` and `TERMS_OF_SERVICE_zh.md` created; locale-aware loading in `settings_page.dart`

### Fixed

- Quick action editor showing English page names
- Login page: Enter Master Password, Unlock, biometric labels, password recovery warning
- Data management: all backup operation labels, dialogs, confirm messages
- Delete account button label
- Date picker: "Select date", "Cancel", "OK" l10n
- Search empty state text color too faint
- Security/Sensitivity route paths in quick action labels (corrected to `/security_settings`, `/sensitivity_settings`)
- Untranslated "Vault" in home page and local search description

### Code Quality

- **S001**: PowerShell injection path validation in `windows_search_service.dart`
- **S002**: Security hardening + one-time warning log in `fallback_secure_storage.dart`
- **S004**: `print()` → `SoloLog` in `ocr_service.dart`
- **S006**: `cleanupStaleTimers()` added to `solo_log.dart`
- **P001**: `dispose()` method added to `scan_background_service.dart`
- **P002**: LRU eviction (max 3) in `profile_storage_service.dart`
- **P003**: `_configCache` in-memory cache in `llm_config_service.dart`
- **D001-D005**: Deleted unused files: `llm_privacy_filter.dart`, `llm_query_enhancer.dart`, `llm_stub_provider.dart`, `streaming_text_widget.dart`, `ocr_result_preview.dart`
- **O011**: Removed duplicate `_getDeviceIcon` wrappers
- **O012**: Extracted `_buildPropertyList` from `_showDetailDialog`
- **O013**: Extracted `_performEmptyTrash` from `_confirmEmptyTrash`
- **O014**: Extracted `_buildResultActions` from `_buildResultState`
- **O016**: Merged duplicate for-each loops in `trash_page.dart`
- **O017**: Extracted `shouldRetryStats` boolean variable
- **O018**: Migrated deprecated Radio API to `RadioGroup`
- **O019**: Added `mounted` checks after async gaps in `llm_config_page.dart` and `login_page.dart`
- **O020**: Added `const` constructors to 6 locations
- **O010**: Extracted `EmptyProfilesState` to `llm/` subdirectory (822→795 lines)

### UI

- Language picker emoji flags replaced with `Icons.language`
- Version sheet platform icon changed from `Icons.phone_android` to `Icons.laptop_mac`
- Create account back button moved to top-left with arrow, matching login page style
- Biometric unlock text simplified to "使用 {biometricType}" (removed "解锁 SoloSoul" suffix)

