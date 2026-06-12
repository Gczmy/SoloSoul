## [Unreleased]

## [2.0.0] - 2026-06-12

### Changed

- **Complete Rewrite: Flutter → Tauri v2** — SoloSoul is now a Rust/Tauri desktop app with a React/TypeScript frontend. Significantly smaller bundle size (~50MB vs ~200MB), faster startup, and a fully native window experience.

### Added

- **Object Management System** — Create, edit, and organize objects with user-defined templates. Supports 8+ field types including text, number, date, URL, email, select, and multi-select with field-level validation and format hints.
- **Template Engine** — Define and manage custom object templates with per-field types, options, and validation rules. Templates serve as schemas for consistent data entry.
- **Card-Based Workspace** — Objects displayed as interactive cards with inline editing, sensitivity-aware masking, collapsible sections, and drag-to-reorder property fields.
- **History & Snapshots** — Automatic version snapshots on every object modification. Browse and roll back to any previous version with diff summary.
- **Recycle Bin** — Soft-delete with time/type filters, batch operations, conflict detection on restore, and permanent deletion.
- **Full-Text Search** — Search across all objects with category, tag, and type filters.
- **AI Chat Integration** — Multi-provider LLM support (OpenAI, Anthropic, Ollama, and custom endpoints) with streaming responses and conversation management.
- **Attachment System** — Upload, preview (images/PDF/text), rename, download, soft-delete, restore, and permanent delete. All files encrypted with vault key.
- **Export/Import** — Encrypted object export with tag filtering and attachment support.
- **Theme Engine** — 20+ preset themes with light/dark modes, accent color customization, and real-time switching.
- **Biometric Unlock** — Touch ID / Face ID support for quick vault access.
- **Operation Log** — Full audit trail of all CRUD actions with i18n support and structured detail logging.
- **Privacy Policy & Terms of Service** — Bilingual (EN/ZH) documents with language-based auto-routing.

---
## [1.8.0] - 2026-06-03


### Fixed

- **Windows PDF Preview** — PDF attachments can now be previewed on Windows. Migrated from `pdfx` to `pdfrx` to resolve CMake 4.x compatibility issues.
- **i18n & Font Fallback** — Biometric settings and Ollama error messages are now fully localized. Chinese characters on Windows render with correct glyphs via `Microsoft YaHei` fallback instead of Japanese fonts.
- **UI Polish** — Windows version icon corrected from phone to desktop. Recent device list now properly identifies Windows/Linux devices.

### Added

- **Account Short ID** — Login page and password input page now display a short account ID (e.g. `acc_550e84`) next to the account name, making it easy to distinguish accounts with identical names copied from different machines.

### Changed

- **Unified PDF Engine** — macOS and Windows now share the same PDF rendering engine (`pdfrx`), ensuring consistent preview experience across platforms.

## [1.7.1] - 2026-06-03

### Added

- **Windows Support** — SoloSoul now compiles and runs on Windows 10/11. First Windows release packaged as ZIP with portable executable.
- **Dual-Platform Release Pipeline** — Unified build scripts for macOS DMG and Windows ZIP; both auto-read version from `pubspec.yaml` to prevent version mismatch.
- **Windows Window Management** — Enforced minimum window size of 900×600 for consistent desktop experience.

### Fixed

- **Windows Compilation** — Resolved CMake plugin compatibility (`pdfx`, `local_auth_windows` auto-removal), Rust `ort` dependency conflicts via conditional compilation, and VS 2026 C4819 encoding warnings.
- **Plugin Navigation** — Fixed back button routing from plugin dashboard to home page.
- **Backup Reliability** — Ensures attachment sidecar directory exists before writing manifest, preventing restore failures.
- **PDF Rendering** — Graceful fallback on Windows when PDF preview is unavailable (returns null instead of crashing).

### Changed

- **Build Scripts** — All build scripts (`build_dmg.sh`, `build_windows_zip.sh`, `build_windows_zip.ps1`) now auto-detect version from `pubspec.yaml`; manual override still supported.
- **Operation Log** — Extended audit coverage to attachment, backup, and plugin lifecycle operations.
- **Localization** — Backup list summary text and settings labels fully internationalized.

## [1.7.0] - 2026-06-02

### Added

- **Attachment Sync** — Devices now sync attachments alongside structured data. No more orphaned references when switching devices.
- **Backup Deduplication** — Backups share a single attachment pool via manifest references. 5 backups with 100 MB attachments now use ~100 MB instead of 500 MB.
- **Data Management Dashboard** — Vault info card now displays attachment size, count, and total footprint for full transparency.
- **AI Chat Enhancements** — Multi-conversation support, trash/recovery for deleted chats, message timestamps, and automatic plugin context injection for richer AI responses.
- **User Guides** — In-app guide system with Notion-style rendering. Settings page and home screen now include quick-access help entries and plugin feature guides.
- **Windows Support** — Rust FFI native library now compiles for Windows (preparation for future Windows client).

### Fixed

- Backups now correctly include attachments (previously only profile data was saved).
- Backup size display no longer double-counts shared attachment pool files.

## [1.6.8] - 2026-05-31

### Added

- **Attachment Trash & Recovery** — Attachments now support soft-delete with a 30-day recovery window. Deleted attachments move to the object-level trash where they can be restored or permanently deleted.
- **One-Click Attachment Download** — New download button in the attachment sheet saves files directly to your system Downloads folder. Automatically handles filename conflicts (e.g. `report (1).pdf`).
- **Custom Download Location** — Settings page now allows configuring a custom download directory. Falls back to system Downloads if the custom path becomes unavailable (e.g. macOS sandbox changes).
- **Attachment Preview Enhancement** — PDF attachments can now be previewed inline using `pdfx`. Image previews support full-screen zoom with pan and dismiss gestures.
- **Inline Add Section Button** — The floating "+" action button has been replaced with a dashed-border "Add Section" card at the end of every page content stream. Works consistently across workspace pages, default pages (Profile/Travel/Financial/Professional), and the root object list.

### Fixed

- **Account Duplicate Name Protection** — Creating an account with an existing name now correctly returns an error instead of silently deleting the old account data.
- **JSON Serialization Consistency** — Fixed `@JsonKey` annotation mismatch for `semanticTypes` and `propertyLabels`, ensuring `toMap()` and `fromJson()` use the same keys (`__semanticTypes`, `__propertyLabels`).
- **Download Robustness** — Added write-permission validation before downloading. Prevents crashes when the configured download directory is no longer writable.
- **Concurrent Download Guard** — Prevents duplicate download triggers when rapidly tapping the download button.
- **Orphan File Prevention** — `permanentlyDeleteAttachment` now throws an exception when `accountId` is null, preventing orphaned encrypted files on disk.
- **macOS Sandbox Fallback** — When the custom download path becomes invalid (common after macOS app restart in sandbox), downloads automatically fall back to the system Downloads folder.

### Refactored

- **Plugin Event Handler** — Split the 285-line `_onRun` method into 6 focused handler methods plus a `_PluginRunSession` state class, reducing nesting depth from 7 to 2 levels.
- **Object Editor Save Flow** — Split `_saveObject` into `_validateSaveInput` and `_buildProperties` with a `_PropertyBuildResult` data class, eliminating 6-layer nested logic.
- **Password Dialog Deduplication** — Extracted `_PasswordDialogBaseMixin` shared by two dialog variants, removing ~100 lines of duplicated field/controller/lifecycle logic.
- **OCR Processing Pipeline** — Extracted `_processOcrBytes` to unify the OCR → MRZ → field extraction flow, removing ~80 lines of duplicated code between image and document pickers.
- **Default Structure Factory** — Extracted `_buildPage` / `_buildSection` factory methods for reuse between `_createDefaultStructure` and `_migrateDefaultSectionSchemas`.
- **LLM Config Deduplication** — Extracted `_activeProfile` helper in `llm_config_service`, eliminating repeated active-profile lookups across 5 getters.
- **Icon Lookup Optimization** — Replaced the 114-line `switch` in `getIconFromName` with a `Map` constant table, reducing code by ~70 lines.
- **Parallel I/O Operations** — Attachment deletions during `updateObject`, `permanentlyDeleteObject`, and `permanentlyDeleteMultiple` now run in parallel via `Future.wait` instead of sequential awaits.
- **O(n) → O(1) Lookups** — `semantic_type_registry.getType` and `recommend` lookups now use pre-built `Map`/`Set` for constant-time access instead of linear scans.

### Test Infrastructure

- **Injectable FFI Wrappers** — Added `_saltGenerator` and `_keyDeriver` function pointers to `SecureAccountStorage`, allowing unit tests to mock Rust crypto operations without initializing `flutter_rust_bridge`.
- **Full Test Suite Green** — All 902 unit tests now pass (0 failures), up from 16 pre-existing failures caused by stale `typeId` formats and missing FFI initialization.

## [1.6.7] - 2026-05-31

### Fixed

- **Crash Prevention** — Eliminated ~166 potential null-dereference crash sites across the codebase through safer null-handling patterns
- **Security Hardening** — Validated file paths before external command execution to prevent injection attacks
- **Memory Stability** — Fixed timer and stream subscription leaks that could accumulate during extended app usage

### Refactored

- **Code Quality Audit** — Complete static analysis cleanup: 0 errors, 0 warnings, 0 info. Migrated deprecated Radio APIs and split oversized UI methods for better maintainability

## [1.6.6] - 2026-05-26

### Fixed

- **Identity Section Title Fields** — Fixed duplicate title inputs and missing fullName when adding items to identity sections
- **Preset Section Schemas** — All preset sections now include a Title field for consistent item naming
- **Title Sensitivity Tag** — Title input fields now display the sensitivity level (Public) for unified UI
- **macOS Keyboard Assertion** — Suppressed harmless HardwareKeyboard assertion errors
- **Address Formatter Logs** — Removed technical prefix from user-facing error messages

### Added

- **Sidebar Plugin Entry** — Added Plugins shortcut to the sidebar for quick access to plugin management

### Refactored

- **Skill & Language Schemas** — Removed redundant 'name' property; Title now serves as the display name

## [1.6.5] - 2026-05-25

### Added

- **Plugin Ecosystem v2** — 20 plugins available (TOTP Generator, Emergency Card, Address Formatter, MRZ Encoder, Expiry Guardian, ID Validator, and more) with semantic type system and field-level access review
- **Plugin Market** — Browse and install plugins directly from GitHub with CDN fallback; works offline after first sync
- **Drag-to-Reorder Fields** — Reorder property fields in section editor by dragging
- **Move Sections Between Pages** — Change a section's parent page from the editor
- **Editable Field Names** — Rename display labels without affecting underlying data keys
- **Plugins Quick Action** — New shortcut on home page to quickly open plugins

### Fixed

- **Section Move Reliability** — Moving a section between pages no longer leaves ghost copies or causes unexpected deletions
- **Account Re-creation** — Deleting and re-creating an account with the same name now works correctly
- **Card UI Polish** — Collapse threshold lowered to 1 item; persistent add button; cleaner header layout
- **Title Handling** — Fixed duplicate Title fields and ensured Title always appears first in item cards
- **First-Launch Offline** — Plugin market works without internet on first launch

### Refactored

- Code quality audit resolved 160+ analyzer warnings across 41 files
- Reduced app bundle size by removing unnecessary native build artifacts from repository


# Changelog

All notable changes to SoloSoul are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.6.4] - 2026-05-23

### Added

- **Icon Library Expansion** — 96 icons across 12 categories with search filtering and categorized display
- **Default Page Custom Sections** — Add, edit, and delete custom sections on Profile/Travel/Financial/Professional pages
- **Article Section Template** — New predefined template for articles and notes
- **Password Hint-Only Changes** — Update password hint without changing the master password
- **Global Backoff Protection** — 30-second cooldown after 5 failed password attempts, persisted across dialog sessions
- **Unified Password Dialog** — Shared password verification dialog across search, settings, and all protected pages

### Fixed

- **macOS Compilation** — Fixed `Not a constant expression` error in `liquid_glass_widgets` that prevented app launch
- **Default Page Alignment** — Custom sections now align with predefined sections on default pages
- **Old Account Migration** — Automatically creates missing default pages and sections for accounts created before schema v2
- **macOS Sandbox Data Isolation** — Fixed data path resolution for sandboxed release builds
- **URL Property Type** — Proper validation and display for URL fields
- **Sidebar Alignment** — Visual alignment improvements for sidebar items

### Refactored

- **Code Quality Audit** — Comprehensive static analysis cleanup: fixed test compilation errors, removed dead code, resolved unawaited futures, and eliminated 160+ analyzer warnings across 41 files
- **Filter Sections** — Unified filter section pattern across operation log, search, and trash pages

## [1.6.3] - 2026-05-10

### Added

- **Global Backoff Protection** — Password verification now has global brute-force protection with 30-second cooldown after 5 failed attempts, persisted across dialog close/reopen
- **Unified Password Dialog** — Shared password verification dialog extracted for reuse across the app (search page, settings page, etc.)

### Fixed

- **Icon Colors** — Key icon, hint button, and visibility button now use consistent theme colors (error: red, focus: primary blue)
- **Biometric Independence** — TouchID/FaceID remains available even during password backoff cooldown

### Refactored

- **Filter Sections** — Unified filter section pattern across operation log, search, and trash pages

## [1.6.2] - 2026-05-10

### Added

- **Section Template Browser** — 15 predefined section templates (passport, visa, bank accounts, ID card, education, etc.) with localized names
- **Trash Empty Placeholders** — Blank titles show "Title: (empty)"; blank property values show "(empty)" in gray italic
- **Operation Log Filter** — Added Windows and Linux device platform options
- **Field Key Translation** — All template field keys now localized in both English and Chinese
- **Debug Logger** — Always records to circular buffer; activation prints buffered logs for pre-bug diagnostics

### Fixed

- **Per-Section Schema Independence** — Each section has its own property schema; no cross-section property leakage. New sections start with only Title
- **Section Editor** — Deleting properties truly removes them (no deprecated toggle); re-creating works without "duplicate" error
- **Settings Page i18n** — Account count, backup summaries, operation descriptions, device names fully localized
- **Device Name Resolution** — Shows actual system hostname for distinguishing multiple Macs; platform labels like [macOS] added
- **Template Field Display** — ObjectCard shows localized labels instead of raw camelCase keys
- **Privacy Mode & Sidebar Drag** — Fixed timeout and reordering bugs

### Changed

- **liquid_glass_widgets** — Vendored locally to remove pub.dev dependency, suppressed verbose init logs

## [1.6.1] - 2026-05-09

### Added

- **Trash Filter Section** — Filter by time (10 days/1 day/6 hours/1 hour) and type (Page/Section/Item)
- **Trash Hierarchy** — Expand pages to see sections, expand sections to see items

### Fixed

- **Delete Dialog i18n** — Localized delete confirmation dialogs
- **Item Type Filter** — Fixed "Item" filter matching predefined section items

## [1.6.0] - 2026-05-09

### Added

- **Custom Sections on Default Pages** — Add custom sections via "+" button on Profile/Travel/Financial/Professional pages, reused from the custom page section system
- **OCR Scan: Save Original File** — Scanned images can now be saved as encrypted attachments in the vault
- **MRZ Scan Section Selector** — Manual override for import target section when scanning MRZ documents
- **Operation Log i18n** — Action labels (Create/Update/Delete/Restore/Purge), time labels (Just now/Xm ago/Xh ago/Xd ago), device platform labels, and section names all localized to Chinese

### Fixed

- **MRZ Visa Routing** — Fixed double-save guard for MRZ scan imports to prevent duplicate entries
- **Section Deletion Notification** — Fixed snackbar disappearing after section soft-delete by capturing overlay before async gap
- **Trash Children Sorting** — Deleted children now sorted by deletion time (newest first)
- **Custom Section Title i18n** — Property key 'Title' now correctly resolves to localized label
- **Trash Detail Dialog** — Empty properties no longer show "(empty)" text, reducing visual noise for sections

### i18n

- **Display Card Labels** — All property labels on Profile/Travel/Financial/Professional card views are now localized using `translateFieldLabel`
- **Operation Log** — Filter labels, action badges, time stamps, device tags, and section labels fully localized

## [1.5.1] - 2026-05-08

### Fixed

- **Password Hint Persistence** — Account creation now saves hint to Rust vault on both normal and fallback paths
- **Display Card Field Labels** — Added `toFormattedStringLocalized(l10n)` for i18n support across all default page display cards

## [1.5.0] - 2026-05-08

### Fixed

- **File Picker in Release Builds** — Added file access entitlements to DMG signing template for file_picker and image_picker
- **LLM Config Page** — Fixed redirecting to home page in release mode
- **Local Search & Scan Routes** — Opened to production builds

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
