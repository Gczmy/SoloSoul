# Changelog

All notable changes to SoloSoul are documented in this file.

## [1.6.4] - 2026-05-23

### Added

- **Icon Library Expansion to 96 Icons** — `icon_picker_sheet.dart` now supports 96 icons across 12 categories (travel, finance, identity, education, technology, health, media, objects, nature, symbols, arrows, misc). Added search filtering, categorized grid display, and collapsible filter bar.
- **Default Page Custom Sections** — Profile/Travel/Financial/Professional pages now support adding custom sections via "+" button alongside the sensitivity mode button. Reuses `AddSectionDialog` and `custom_sections_widget.dart`. New sections appear at the bottom and support rename, delete, and property editing.
- **Article Section Template** — New predefined template for articles and notes with Title, Author, Source, URL, and Content fields.
- **Password Hint-Only Changes** — `change_password_dialog.dart` now supports updating only the password hint without changing the master password. Added `updatePasswordHintOnly` flow in `rust_vault_service.dart`.
- **Global Backoff Protection** — Password verification dialog now enforces a 30-second global cooldown after 5 failed attempts. Backoff state is persisted via `SharedPreferences` and survives dialog close/reopen across the entire app.
- **Unified Password Verification Dialog** — Extracted shared `password_verification_dialog.dart` with `showPasswordVerificationDialog()` API. Replaced duplicated password dialogs on search page, settings page, and all protected pages.

### Fixed

- **macOS Hot Restart Compilation Error** — `packages/liquid_glass_widgets/lib/src/renderer/shaders.dart` had `const String _shadersRoot = !kIsWeb && isTestEnvironment ? ...` which failed on IO builds because `isTestEnvironment` is `final` (not `const`) in `_env_io.dart`. Changed `_shadersRoot` and `ShaderKeys` fields to `final` with `ignore: prefer_const_declarations` to prevent analyzer false-positives.
- **Default Page Alignment (Phase 3)** — Custom sections now visually align with predefined sections on Profile/Travel/Financial/Professional pages. Removed hardcoded padding differences.
- **Old Account Auto-Migration** — Accounts created before schema v2 now automatically get missing default pages and sections (Identity, Contact, Address, ID Card, Passport, Visa, etc.) on next unlock.
- **macOS Sandbox Data Isolation** — Fixed `path_provider` data directory resolution for sandboxed release builds. Data now correctly stores in `~/Library/Containers/...` when sandbox is enabled.
- **URL Property Type** — `UrlProperty` now has proper validation regex and displays clickable links in `ObjectCard`.
- **Sidebar Alignment** — Fixed vertical alignment of sidebar items with icons of varying widths. Added `IntrinsicWidth` wrapper for consistent label positioning.
- **Filter Bar Collapse** — Filter sections on operation log, search, and trash pages now properly collapse/expand without layout jumps.
- **Delete Account Flow** — Improved confirmation dialog with warning text and 3-second delay before allowing deletion.
- **New Account Default Pages** — Fixed missing default pages when creating a brand new account after app reinstall.

### Refactored

- **Code Quality Audit (Round 1)** — Comprehensive static analysis and automated fix cycle:
  - Fixed 4 P0 test compilation errors: `llm_query_enhancer_test.dart` (dead file removed), `local_search_service_test.dart` (wrong class reference), `property_value_utils_test.dart` (missing import), `sensitivity_tag_test.dart` (removed `getSensitivityLabel` restored)
  - Fixed P1 warnings: unused variables/imports in `scan_import_service.dart`, `sensitivity_settings_page.dart`, `predefined_object_section.dart`
  - Fixed P1 potential bugs: `use_build_context_synchronously` in `llm_config_page.dart`, `unawaited_futures` in `account_style_provider.dart`
  - Fixed P1 deprecated API usages: `dangling_library_doc_comments` in `mrz_date_utils.dart`, missing `fake_async` dependency
  - Bulk P2 fixes via `dart fix --apply`: 160 fixes across 41 files (`prefer_const_constructors`, `prefer_const_declarations`, `no_leading_underscores_for_local_identifiers`, `unnecessary_import`, `unnecessary_to_list_in_spreads`, etc.)
  - Fixed `build_dmg.sh` entitlements path: `Runner/Release.entitlements` → `macos/Runner/Release.entitlements`
- **Filter Sections Unification** — Extracted shared `FilterSection` widget pattern across operation log, search, and trash pages. Eliminated duplicated filter logic.

### Internal

- Added `currentObjects` public getter to `UnifiedObjectNotifier` to avoid external access to protected `state` property.
- Restored `getSensitivityLabel(SensitivityLevel)` top-level helper in `sensitivity_tag.dart` for test compatibility.
- Generated `CODE_ANALYSIS_REPORT.md` and `CODE_ANALYSIS_REPORT_FINAL.md` documenting 20 identified issues and 17 resolutions.

## [1.6.3] - 2026-05-10

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

- **LLM Config Page** — Fixed redirecting to home page in release mode (removed from `debugOnlyRoutes` guard)
- **Local Search & Scan Routes** — Opened to production builds

## [1.4.8] - 2026-05-08

### Added

- **Auto Language Detection** — OS locale auto-detection on first launch (Chinese OS → zh, all others → en)
- **Version Auto-Injection** — Version auto-injected into DMG builds, with update notification linking to GitHub Releases
- **935 ARB Keys** — Comprehensive i18n completion across all pages and widgets with 0 hardcoded strings remaining

### Fixed

- Date picker localization, password dialog width consistency, search empty state colors, untranslated "Vault" references

## [1.4.7] - 2026-05-06

### Added

- **Liquid Glass UI Overhaul** — Complete cross-platform UI redesign using liquid glass material design. All 20+ protected pages, AppBars, cards, buttons, dialogs, and sidebar now use glass-morphism effects with Notion+Anytype bright color palette
- **Login UI Refresh** — Redesigned login page with gradient background, decorative orbs, vertical centering, and hover effects on all interactive elements
- **Back Navigation** — SoloGlassAppBar now supports `backRoute` for proper back button behavior on deep-linked pages

### Fixed

- **Sensitivity Lock Enforcement** — Locking sensitive access now simultaneously enforces data masking and collapses all expanded history records
- **Sidebar Rename Bug** — Editing a custom page name no longer persists when navigating away; double-tap renamed to long-press for faster click response
- **Sidebar Drag Performance** — Cached descendant lookups during drag-and-drop and simplified drag placeholder to reduce jank
- **LLM Stats Persistence** — Skips LLM usage statistics persistence when vault is locked to prevent errors
- **Object Editor Character Counter** — Fixed character counter showing literal text instead of actual number
- **History Timestamp Alignment** — Full timestamps in history records are now right-aligned for consistency

### Refactored

- **Sensitivity Model Consolidation** — `sensitivity_models.dart` moved to `core/models/` for cleaner architecture
- **Scan Service Refactoring** — `local_search_service` now uses `FieldRegistry` as the single source of truth for field sensitivity levels
