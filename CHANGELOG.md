# Changelog

All notable changes to SoloSoul are documented in this file.

## [1.6.0] - 2026-05-09

### Added

- **Custom Sections on Default Pages** — Added "+" button to each default page's right side (alongside sensitivity mode button), reusing `AddSectionDialog` to add custom sections to Profile/Travel/Financial/Professional pages. New sections appear at the bottom of the page and support all standard section operations (rename, delete, add/edit properties).
  - `custom_sections_widget.dart`: New shared widget wrapping `ObjectCard` list with add/edit/delete controls
  - `add_section_dialog.dart`: New dialog for naming and creating a custom section on any page
  - Updated all 4 default pages: `profile_page.dart`, `travel_page.dart`, `financial_page.dart`, `professional_page.dart`
- **OCR Scan: Save Original File** — `saveOriginalFile` checkbox in scan document sheet allows saving the scanned image as an encrypted attachment via `saveAttachment()`. Attachment is encrypted with vault key and stored in `UnifiedObject.attachments` map.
  - `scan_document_button.dart`: Added checkbox for save-original-file with translated label
  - `object_card_fields_sheet.dart`: Attachment UI displays saved filename with open/open-location actions
  - `unified_object_model.dart`: Added `attachments` field (`Map<String, String>`) to `UnifiedObject`
  - `base_models.dart`: Added `attachments` persistence field to `UnifiedObjectData`
  - `operation_logger.dart`: Added `logCustomSection()` for property-level audit logging
- **MRZ Scan Section Selector** — When importing from MRZ scan, user can override the default section via dropdown menu in the preview dialog. Validated against existing page sections and dynamically created sections.
  - `mrz_preview_card.dart`: Section selector dropdown + validation
  - `predefined_object_section_helpers.dart`: Added `findSectionByName` + `suggestSectionForType` for smart section routing
  - `entry_card_widget.dart`: Added `currentPageSections` parameter support
  - `travel_page.dart`: Passes page sections to MRZ preview
- **Operation Log i18n** — All action labels, time labels, device labels, and section names in operation log page now localized:
  - Filter: `'Action:'` → `'${l10n.operationLabelAction}:'`, `'Device:'` → `'${l10n.operationLabelDevice}:'`
  - Filter chips: `'macOS'` → `l10n.operationPlatformMacos`, `'iOS'` → `l10n.operationPlatformIos`
  - Tile badges: `_actionLabel` (hardcoded 'Created'/'Updated'/'Deleted'/'Restored'/'Purged') → `_actionLabel(l10n)` using `l10n.operationAction*`
  - Time labels: `_formatTime` (hardcoded 'Just now'/'Xm ago'/'Xh ago'/'Xd ago') → reuses `l10n.trashJustNow`/`trashMinutesAgo`/`trashHoursAgo`/`trashDaysAgo`
  - Device tags: `_getDeviceLabel` (hardcoded 'macOS'/'iOS'/'Android'/'Windows'/'Linux'/'Web') → `l10n.operationPlatform*`
  - Section display: `entry.section.toUpperCase()` → `_sectionLabel(l10n)` mapping via `logSection*` l10n keys
  - Detail dialog: All labels (`_actionLabel`, `_sectionLabel`, `_getDeviceLabel`) use l10n
- **New ARB Keys**: Added 23 new keys to both `app_en.arb` and `app_zh.arb`:
  - `operationPlatformMacos`, `operationPlatformIos`, `operationPlatformWindows`, `operationPlatformLinux`
  - `logSectionIdentity` through `logSectionCustom` (19 section labels)

### Fixed

- **MRZ Visa Routing + Double-Save Guard** — `import_result_page.dart` now correctly routes visa items to the travel page section instead of creating orphan objects. Added prevention for multiple MRZ scans of the same document path creating duplicate entries.
- **Section Deletion Notification** — Fixed snackbar disappearing after section soft-delete. Root cause: ObjectCard gets removed from widget tree after `deleteObject` updates provider state, making `context.mounted == false`. Fix: capture `Overlay.of(context)` before any `await`, pass via `forOverlay` parameter to `showOverlaySnackBar`, and add `BuildContext?` / `OverlayState?` dual-path API in `app_theme.dart`.
- **Trash Children Sorting** — `deletedChildrenProvider` now sorts by `deletedAt` descending (matching `trashRootDeletedObjectsProvider` pattern), ensuring newest deletions appear first.
- **Custom Section Title i18n** — Added `'Title'` case to `translateFieldLabel` switch in `format_field_label.dart` (case-sensitive: custom sections use capitalized `'Title'` key).
- **Trash Detail Dialog Empty Properties** — Removed `l10n.commonEmpty` text for empty property values in `unified_object_trash_card.dart`; labels and sensitivity tags remain visible.
- **ObjectCardPropertiesList i18n** — Changed from `formatFieldLabel(key)` (algorithmic Title Case) to `translateFieldLabel(key, l10n)` (i18n-aware) with proper imports.

## [1.5.1] - 2026-05-08

### Fixed

- **Password Hint Persistence** — Account creation now saves hint to Rust vault via `updatePasswordHint` in both normal and fallback paths. Previously the hint was only in Keychain; if Keychain was unavailable, the hint was lost on re-login.
- **Display Card Field Labels** — Added `toFormattedStringLocalized(l10n)` to `FormattableEntry` mixin using `translateFieldLabel`. Updated all 13 `formatAllFields` callbacks across travel, profile, financial, and professional pages.
- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to DMG signing entitlements (v1.5.0).

## [1.5.0] - 2026-05-08

### Fixed

- **File Picker in Release Builds** — Added `com.apple.security.files.user-selected.read-write` and `com.apple.security.files.bookmarks.app-scope` to `build_dmg.sh` entitlements template. These were missing, preventing `file_picker` and `image_picker` from working in release DMG builds.

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

