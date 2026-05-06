# Changelog

All notable changes to SoloSoul will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.4.6] - 2026-05-06

### Added

- **LLM Integration (P0)** — Full AI chat interface with streaming responses, smart field mapping for object creation, and encrypted usage statistics persistence. Supports multiple providers with per-model configuration, usage tracking, and sparkline chart visualization in settings
- **Local Search Import (P0)** — Import objects from local search results with automatic schema field mapping, batch validation, and CancelToken support to interrupt underlying scan I/O
- **Multi-Device Sync** — FRB-based sync engine featuring CRDT data structures for conflict-free replicated data types, Noise protocol encryption for handshake security, and TCP transport layer. Includes dedicated sync UI for device pairing and connection management
- **Rust Core Engine (Phases 1-5)** — Complete migration from Dart crypto fallback to unified Rust implementation:
  - Phase 1: Unified encryption layer with Argon2id + AES-256-GCM
  - Phase 2: Unified account management with UUID account IDs
  - Phase 3: Eliminated Dart fallback, added KdfPreset configuration
  - Phase 4: Typed FRB bindings replacing JSON relay pattern
  - Phase 5: CRDT sync engine with Noise encryption and TCP transport
- **Anytype-Inspired macOS Features** — Redesigned object workspace and editor interactions following Anytype UX patterns for spatial navigation and relation editing
- **Full Operation Recording** — All user CRUD actions are logged via `OperationLogService` with before/after property snapshots, sensitivity levels, and proper `logSectionForTypeId` mapping for complete audit trails
- **Comprehensive Test Coverage (Phases 1-11)** — Added extensive unit tests for LLM service, local import, auth notifiers, vault operations, sync engine, and widget behavior
- **Structured Sensitive Debug Logging (P028)** — `DebugLogger` now tags sensitive data with structured sensitivity levels for safer diagnostic output
- **Sync UI** — New settings section for managing multi-device synchronization with real-time connection status

### Fixed

- **AI Privacy Protection (P001)** — Smart mapping now blocks critical and restricted sensitivity data from being transmitted to cloud LLM APIs, preventing privacy leakage
- **Startup Black Screen (P0)** — Fixed native library loading race condition caused by incorrect `dlopen` path resolution on macOS app launch
- **Unlock Flow Hangs** — Restructured async unlock sequence to prevent UI hangs caused by `verify_hash` encoding mismatches between old and new account formats. Added automatic `verify_hash` repair for corrupted Keychain entries
- **Account Switch Security** — Password verification is now mandatory when switching accounts from settings, preventing unauthorized access via cached session tokens
- **Password Dialog Error Icon Color** — When verifying identity for critical/sensitive fields, entering an incorrect password now correctly turns the hint (`help_outline`) and visibility toggle (`visibility_outlined`/`visibility_off_outlined`) icons red to match the error text, instead of leaving them white/default
- **Security Audit (S001-S015)** — Path traversal validation on profile IDs, minimum PBKDF2 iteration enforcement, secure key material wipe after account creation, backup file permissions (0600), debug mode security hardening, and constant-time comparison migrated to Rust FFI
- **Performance Audit (PF001-PF010)** — Batch delete for empty trash, O(n²) elimination in list operations, save/log debounce, TextEditingController leak fixes, and timer no-op prevention
- **Code Quality Audit (D001-D011, P001-P055)** — Dead code cleanup, concrete exception types in catch clauses, mounted guards, duplicate code extraction, and removed empty setState calls
- **LLM Stability (P002-P007, P016)** — Proper `http.Client` disposal, input field clearing, stream controller leak fixes, print-to-SoloLog migration, and max 5-file limit for AI mapping to prevent request storms
- **LLM Type Safety (P004, P008-P011, P015)** — Debounced stream rebuilds, type-safe API key handling, proper comment alignment, and API key clearing on model switch
- **Import Integrity** — Ensures imported objects contain all schema-defined fields with correct defaults; prevents stats loss when Vault is locked at startup

### Refactored

- **Widget Extraction (P010-P015, P023-P025, P034-P039, P043, P057-P059)** — Extracted 26+ widget classes from 8 oversized files:
  - Settings page: 427 → 35 lines
  - Profile page: 327 → ~50 lines
  - Editor header, bottom save bar, property field rows, contact forms as standalone widgets
- **LLM Service Modularization (P020)** — Split monolithic `llm_service.dart` into focused files: `llm_config_service.dart`, `llm_chat_service.dart`, `llm_mapping_service.dart`, `llm_stats_service.dart`
- **Shared Utilities (Phase 0)** — Extracted `_verifyPassword`, `_postLoginSetup`, dialog overlay helpers, and page templates to eliminate duplication
- **Login Flow (P045)** — Unified `_handleUnlock` and `_handleCreateAccount` post-login setup into shared `_postLoginSetup`
- **Settings Dialogs (P031-P032)** — Extracted `_DebugActivationDialog` reducing dialog builder from 196 → 58 lines

## [1.4.5] - 2026-04-30

### Added

- **Operation Log Search** — Added search bar to operation log page filtering by description, section, and action; shows live result count and supports clear action
- **Trash Property Snapshot** — Purge actions in trash page now capture full property values and sensitivity levels via `OperationLogger.logCustomSection()` for complete audit trail
- **Object Card Title Key Config** — `ObjectCard` now accepts `titlePropertyKey` parameter (default `'Title'`) to support schemas using different title fields (e.g., `fullName` in Identity). Title input controller initialization and save logic now reference this configurable key instead of hard-coded `'Title'`

### Fixed

- **Label Formatting Consistency** — Extracted shared `formatFieldLabel()` utility in `presentation/utils/format_field_label.dart`; applied to `FieldHistoryView` and `OperationTile` so history records and operation log property snapshots display human-readable labels like "Given Name" instead of raw camelCase keys like "givenName", matching the display card formatting

## [1.4.4] - 2026-04-30

### Fixed

- **Trash Purge Snackbar Silent Failure** — `_confirmPurgeUnifiedObject()` previously accepted a `BuildContext` parameter from `ListView.itemBuilder`, which becomes unmounted after `permanentlyDeleteObject()` removes the item from the list. `showOverlaySnackBar()` checks `context.mounted` and silently returns if false. Removed the `BuildContext` parameter from both `_confirmPurgeUnifiedObject()` and `_confirmRestoreUnifiedObject()`; methods now use `_TrashPageState`'s stable `this.context`. Also removed the ineffective `WidgetsBinding.instance.addPostFrameCallback` workaround
- **Trash Card Action Overflow** — Reduced button padding from `EdgeInsets.symmetric(horizontal: 12)` to `6` and wrapped timestamp text in `Flexible` to prevent 13px overflow on medium-width screens
- **Trash Button Alignment** — Replaced `Flexible + Spacer` with `Expanded` so action buttons occupy the full card width consistently
- **Trash Responsive Actions** — Added `LayoutBuilder` with 420px threshold: narrow screens show icon-only buttons with tooltips; wide screens show labeled text buttons
- **Trash History Button State** — Empty history now shows gray icon with "0" count and tap-to-show "No history available" tooltip; non-empty history shows purple icon with count badge
- **Trash Detail/History Dialogs** — Added Details dialog showing fields + sensitivity tags + deletion time; added History dialog reusing `FieldHistoryDialog` with proper field prefix mapping
- **Trash Operation Logging** — Purge, restore, and empty-trash actions now write to `OperationLogService` via `_logSectionForTypeId()` mapping
- **Trash "Untitled" Display** — `PredefinedObjectSection` name resolution now includes `fullName` key in the lookup list
- **flutter_animate Crash** — Removed `.animate().fadeIn()` from `_UnifiedObjectTrashCard` which caused `FractionalTranslation` hit-test assertion during widget removal
- **Operation Log Sensitivity Colors** — Filter chips, `OperationTile`, and detail dialog now use `SensitivityTag` colors: Critical=red.shade900, Internal=blue, Public=green, Sensitive=orange
- **Identity Operation Logging** — `PredefinedObjectSection.onSave()` and `onDidDelete()` now log create/update/delete actions; undo restore also logs
- **Object Editor Sensitivity Dropdown** — `PopupMenuButton` child now shows `Row(SensitivityTag + Icon(Icons.keyboard_arrow_down))` for clearer affordance

## [1.4.3] - 2026-04-29

### Fixed

- **Vault Initialization Race (Android/Windows)** — `NativeVaultService._initialize()` now stores the async init future in `_initFuture`; all Android/Windows public async methods (`createAccountAsync`, `unlockVaultAsync`, `unlockVaultWithKeyAsync`, `deleteAccountAsync`, `listAccountsAsync`, `getAccountConfigAsync`) await `_ensureInitialized()` before accessing `_fallbackSecureStorage` or `_profilesDir`. Sync fallback `_androidRequest()` now returns `{'success': false, 'error': 'Vault not initialized'}` if called before initialization completes, preventing null-dereference crashes
- **Property Editor Controller Leaks** — `_TextEditor`, `_NumberEditor`, `_RelationEditor`, `_UrlEditor` in `property_editor_factory.dart` were `StatelessWidget`s creating `TextEditingController` on every `build()` without disposal. Converted all four to `StatefulWidget` with `dispose()` calling `controller.dispose()`
- **Object Editor Fire-and-Forget** — `_saveObject()` in `object_editor_page.dart` was `void _saveObject() async`, meaning exceptions from internal `await`s were silently swallowed. Changed to `Future<void> _saveObject() async` with full `try/catch`, logging errors via `DebugLogger` and showing `SnackBar` to user on failure
- **Mounted Checks** — Added `if (mounted)` guards after all `await showModalBottomSheet` / `await showDialog` / `await saveObject()` calls in `object_editor_page.dart` to prevent `setState()` on disposed widgets
- **Search Result Tile Rebuild Logic** — `SearchResultTile` previously watched `accountStyleProvider.select((s) => s.value?.displayMode)`, which does not change when a field is revealed. Changed to watch `accountStyleProvider.select((s) => s.value?.revealedFields)` and `isSensitiveAccessGrantedProvider`, ensuring tiles rebuild correctly when users click "Reveal"

### Performance

- **Sensitivity Settings Cache** — `SensitivitySettingsPage._buildSettingsView()` previously re-sorted all fields (O(n log n)) and performed 4 sensitivity-level filters + search filter on every rebuild. Added `_getEffectiveFields()` and `_getFilteredSections()` to `_SensitivitySettingsPageState` with memoization via `_cachedEffectiveFields`, `_cachedSections`, `_cachedRegistryHash`, `_cachedAccountStyleHash`, and `_cachedSearchQuery`
- **Trash Provider Aggregation** — `TrashPage._buildTrashContent()` previously called `ref.watch(effectiveSensitivityProvider(fieldId))` inside a `for` loop over 12 item types on every rebuild, causing 12 individual provider watches. Added `trashItemSensitivityMapProvider` in `sensitivity_provider.dart` which aggregates all 12 sensitivities into a single `Map<String, SensitivityLevel>`; `trash_page.dart` now watches only this one provider. Also added `_getFilteredTrash()` to cache filtered deleted items/unified objects by search query
- **Predefined Object Section Cache** — `PredefinedObjectSection` was a `ConsumerWidget` that rebuilt `fieldDefs` (schema property → `FormFieldDef` mapping with `FieldRegistry` sensitivity lookup) on every provider change. Converted to `ConsumerStatefulWidget` with `_cachedFieldDefs` and `_cachedTypeDef`, eliminating redundant O(m × n) registry traversals
- **Effective Field Level Select** — `effectiveFieldLevelProvider` in `sensitivity_based_visibility_widget.dart` previously watched the entire `accountStyleProvider` AsyncValue. Narrowed to `accountStyleProvider.select((s) => s.value)`, reducing rebuilds when only the AsyncValue wrapper state (loading/error) changes

### Changed

- **Test Warnings** — `biometric_credential_service_test.dart`: replaced deprecated `setMockMethodCallHandler` with `TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler`; removed unused `dart:typed_data` import. `profile_data_test.dart` and `profile_provider_test.dart`: added `const` to `ProfileData()` constructor calls

## [1.4.2] - 2026-04-29

### Fixed

- **Trash Emptying Completeness** — `TrashManager._calculateEmptyTrash()` and `getDeletedItems()` now correctly include `awards` in the professional section
- **Memory Leaks** — Fixed TextEditingController leaks in `ObjectEditorPage` (property field removal) and `ObjectCard` (dummy controller created on every build)
- **Null Safety** — Replaced unsafe `!` operators in `unified_object_provider.dart` children lookups with defensive `whereType<UnifiedObject>()`
- **Error Visibility** — Added error logging to 5 previously silent catch blocks in `native_vault_service.dart` and `native_crypto_service.dart`
- **State Consistency** — `purgeOldDeletedItems()` now returns a new immutable `ProfileData` via `copyWith` instead of mutating the input parameter
- **Operation Log Reliability** — `OperationLogService.addEntry()` now `await`s disk persistence before notifying listeners, preventing log loss on crash
- **Unlock Robustness** — `_handleUnlock()` in login page now wraps `unlockVault()` in try-catch to ensure loading spinner resets on unexpected errors
- **ProfileSectionEditor Safety** — `_markDeletedProfile` and `_markRestoredProfile` now use `identity.copyWith()` instead of fragile manual field reconstruction

### Performance

- **Provider Select Optimization** — `home_page.dart`, `object_editor_page.dart`, and `AccountsVersion` provider now use `.select()` for precise state watching, reducing unnecessary rebuilds

### Removed

- **Dead Code Cleanup** — Removed `UnifiedObjectDataExtension`, `verifyPasswordForRestrictedField`, `animatedSection` helper, `ProfileFieldHistories` typedef, and stale lint suppressions

## [1.4.1] - 2026-04-29

### Fixed

- **Critical Privacy Fix: Cross-Account Data Leakage** — Fixed a severe vulnerability where creating a new account after locking a previous account could display the previous account's data:
  - `main.dart`: Auth state transitions to `locked` now trigger `_wipeSensitiveState()` for both manual and auto-lock paths, ensuring all in-memory sensitive state is cleared
  - `_wipeSensitiveState()`: Now includes `unifiedObjectProvider.reset()` to clear UnifiedObject data previously missed
  - `UnifiedObjectNotifier`: `loadFromProfile()` now resets state to empty when `profile == null` (new account without data)
  - `UnifiedObjectNotifier`: `ref.listen(profileNotifierProvider)` now resets state when profile is cleared to `null` (lock/account switch)

### Performance

- **Isolate Offload for Search** — `SearchProvider._performSearch()` now offloads string matching across all identity/travel/financial/professional/unified fields to a background isolate via `Isolate.run()`, eliminating main-thread jank during search
- **Isolate Offload for Data I/O** — JSON encode/decode and `ProfileData.fromJson()` offloaded to isolates for:
  - `ProfileStorageService.loadProfile()` / `saveProfile()`
  - `BackupService.createBackup()` / `createSpecialBackup()` / `restoreBackup()`
- **Lazy List Rendering** — `AppSidebar`, `TrashPage`, and `ObjectCard` lists converted from eager `ListView` (spread operator) to `ListView.builder`, eliminating pre-build of off-screen children
- **Fine-Grained Object Card Rebuilds** — `_ObjectCardItemTile` now uses `fieldHistoriesProvider.select((h) => h.getHistory(item.id, 'unified'))` so each tile rebuilds independently only when its own history changes

### Fixed

- **Operation Log Live Updates** — `OperationLogProvider` architecture fixed from dead `NotifierProvider<OperationLogServiceNotifier, OperationLogService>` to a version-counter notifier, so entries correctly respond to `addEntry` / `refreshFromDisk` / `clearEntries`

### Changed

- **Widget Decomposition (Code Quality)** — Extracted widgets from god files to improve maintainability (no user-facing changes):
  - `settings_page.dart` (1957 → 1105 lines): `CurrentAccountSheet`, `AllAccountsSheet`, `VersionSheet`, `DebugLogSheet`
  - `profile_storage_service.dart` (3500 → 1865 lines): all 22 model classes moved to `core/models/profile_data.dart` with `@JsonSerializable` codegen
  - `home_page.dart` (1253 → 756 lines): `PageEditor`, `IconPicker`, `DashedPlaceholder`
  - `object_card.dart`: `_ObjectCardHeader`, `_ObjectCardPropertiesList`, `_ObjectCardHistorySection`

## [1.4.0] - 2026-04-29

### Added

- **Startup Data Integrity Validation** — `ProfileStorageService.loadProfile()` now runs `_validateAndRepairProfile()` immediately after migration. Automatically repairs:
  - Duplicate `UnifiedObject` IDs (keeps first occurrence)
  - Invalid `childrenIds` references (removes IDs pointing to non-existent objects)
  - Invalid `parentId` references (sets to `null` if parent no longer exists)
  - Repairs are persisted automatically so they don't re-occur on next load
- **Complete Trash Purge Coverage** — `purgeOldDeletedItemsIfNeeded()` and `purgeOldDeletedItems()` now cover all legacy sections:
  - `travel.travelHistory`
  - `professional.skills`, `professional.languages`, `professional.awards`
  - `identity.idCards`, `identity.addresses`, `identity.contact.entries`
  - `unifiedObjects.objects`
- **Field History Orphan Cleanup** — `FieldHistoryService.cleanupOrphanHistories()` removes history entries for permanently deleted items. Wired into `ProfilePersistenceService.loadProfile()` to run automatically on startup.
- **ProfileData.collectAllItemIds()** — Centralized method collecting all item IDs across legacy sections and unified objects, used for cross-section integrity checks.

### Fixed

- **`_calculateEmptyTrash()` completeness** — Now includes `professional.awards` and `unifiedObjects.objects` in permanent deletion.
- **FormHistory unbounded growth** — History entries for deleted items no longer accumulate indefinitely.

---

## [1.3.0] - 2026-04-29

### Added

- **Encrypted Backup & Restore** (`BackupService`) — Full-screen Data Management page. All backups are encrypted with the vault's AES-256-GCM key. Regular backups auto-rotate (max 5) with version-timestamp filenames.
- **Special Backups** — Up to 5 user-named backups stored outside the rotation cycle. Support rename, restore, and delete. Can be created from current state or promoted from any regular backup.
- **Auto-Backup on Unlock** — `AuthNotifier` fires non-blocking backup creation after every successful vault unlock.
- **Auto-Backup on App Upgrade** — `AppVersionTracker` detects version changes and triggers a versioned backup on the first unlock after upgrade.
- **Backup Recovery Prompt** — `LoginPage` detects empty vault + existing backups and offers a restore dialog before creating default items.
- **Account Data Isolation** — `UnifiedObjectNotifier.loadFromProfile()` now resets state to empty when the new account's `unifiedObjects` is null, preventing old account data from leaking into the new account.
- **Default Page Deletion Protection** — `deleteObject()` blocks soft-deletion of `DefaultPageIds` (profile/travel/financial/professional).
- **Default Page Sidebar Filtering** — `AppSidebar` custom pages list now excludes the four built-in default pages.
- **Operation Notification Overlay** — Backup actions use `OperationNotification.show()` (top-floating overlay) instead of `ScaffoldMessenger` SnackBar.

### Changed

- **Data Management** — Moved from BottomSheet (`settings_page.dart`) to standalone page (`data_management_page.dart`) with `AppBar`, `RefreshIndicator`, and full-screen layout.
- **Restore Backup Order** — `restoreBackup()` now reads the target backup file into memory *before* calling `createBackup()` for the protective backup, preventing cleanup from deleting the file being restored.

### Fixed

- **Restore Oldest Backup Failed** — When 5 regular backups existed, the protective backup's cleanup would delete the oldest backup before it could be read. Fixed by reordering read-before-protect.
- **Date Masking Leak** — `_maskedValue()` threshold was 8 chars, causing `1997-08-19` (10 chars) to show `1997••••••••8-19`. Threshold raised to 12 chars for full masking of dates and short IDs.
- **Object Workspace Pop Crash** — `build()` auto-navigate and `_deleteCurrentObject()` both called `context.pop()`, causing double-pop `GoError`. Removed pop from delete, let build handle navigation.
- **Migration `StateError` Crash** — `_migrateProfileDataToUnified._sens()` used `firstWhere` with `on Exception catch`, but `StateError` (from missing field in `FieldRegistry.defaultFields`) is an `Error`, not `Exception`. Fixed to `catch (_)` and added `FormFieldRegistry.getField()` fallback.
- **Account Switch Data Leak** — New account login would display previous account's custom pages because `UnifiedObjectNotifier` state was never cleared when `profile.unifiedObjects == null`.

---

## [1.2.0] - 2026-04-27

### Added
- **Unified Object Model** - Everything is a `UnifiedObject` with `parentId`/`childrenIds` tree structure. Replaces legacy `FlexibleSection`/`FlexibleItem` models.
- **Persistent Sidebar (`AppSidebar`)** - Drag-resizable (180–400px), collapse/expand, with tree-structured custom pages (expand/collapse for nested sub-pages)
- **Object Workspace Page** - Page-centric UI showing children as cards with inline property editing; non-page children shown as list tiles
- **Object Editor Page** - Generic editor for creating/editing any `UnifiedObject` with icon picker, type selection (create-only), and parent assignment
- **Icon Picker Sheet** - Shared bottom-sheet component for selecting from 26 predefined Material icons
- **Lock Vault Confirmation Dialog** - Unified confirmation dialog before locking, with cancel/confirm buttons and tap-outside-to-dismiss
- **Data Size Display** - Settings page Account section now shows total vault data size (B/KB/MB/GB)
- **Property Editor Factory** - Type-aware inline property editors (text, number, date, checkbox, select, multi-select, relation, URL)
- **CHANGELOG.md** - Version history documentation
- **Rust FFI for Argon2id** (`crypto-argon2/`) - High-performance Argon2id key derivation using Rust SIMD optimizations for Apple Silicon
- **Change Password API** - `POST /api/auth/password` endpoint with full data re-encryption
- **Shared Header Component** - Left sidebar with navigation, account badge, and lock button shared across all auth pages
- **Request Timeout** - 10-second timeout on all API client requests to prevent hanging

### Changed
- **Schema Version** - Bumped to v3; `ProfileData` now uses `unifiedObjects` field for all object storage
- **Home Page** - Simplified to main dashboard only; quick actions fixed at 90×90; inline page editor moved to object workspace
- **Custom Page Trash** - Page-type children no longer display in parent workspace (hierarchy visible only in sidebar tree)
- **Object Editor Save** - Save button moved from AppBar to bottom-centered outline button
- **Multi-account Session Persistence** - `sessionToken` and `currentAccount` are now persisted to localStorage
- **Session Validation** - Fixed closure capture bug in auth page redirects using `getState()` pattern
- **Settings Page** - Change Master Password section is now collapsed by default
- **Dashboard** - Simplified toolbar, account info moved to shared Header
- **Vault ChangePassword** - Now properly re-encrypts all profile data with new key

### Fixed
- **Data Persistence on Login** - `UnifiedObjectNotifier` now auto-loads from `ProfileData` via `ref.listen` and explicit `loadFromProfile()` calls on login
- **Lock Button Stucking** - Added try-finally in settings page to ensure button state is restored on error
- **Change Password Flow** - Wrong current password now returns proper error message instead of hanging
- **SetVaultPath Index Loading** - Fixed missing `loadIndex()` call when switching account vaults
- **Account Deletion Lock** - Delete account now properly locks and redirects to login

### Security
- **Argon2id Performance** - Rust SIMD implementation provides ~3x speedup on Apple M-series chips
- **Password Change Re-encryption** - All vault data is now properly re-encrypted when password changes

---

## [1.1.0] - 2026-04-24

### Added

- **Riverpod 3.0 Upgrade** — Upgraded from Riverpod 2.6.1 to 3.0.3
- **Disable Debug Mode Button** — Added power button in debug log sheet to exit debug mode

### Bug Fixes

- Fixed address save not persisting new entries (missing list update logic)
- Fixed soft delete confirmation dialog not showing (alreadyConfirmed flag was wrong)
- Fixed debug mode being lost on page navigation (provider now uses keepAlive)
- Fixed debug mode password dialog dismissing on outside tap (barrierDismissible: false)

### Technical

- StateNotifier → Notifier migration (4 classes)
- ChangeNotifierProvider → NotifierProvider migration
- Auto-retry disabled in ProviderScope
- All generated code regenerated for Riverpod 3.0 compatibility

---

## [1.0.0] - 2026-04-24

### Added

- **macOS DMG Installer** — Official v1.0.0 release with drag-and-drop installation
- **Debug Mode** — Hidden debug log sheet (tap version 5 times to reveal) with colored log levels
- **Improved Keychain Handling** — Better fallback mechanism for non-notarized distribution
- **Biometric Authentication** — Face ID unlock support with password verification fallback
- **Debug Logger Colors** — Color-coded log levels (INFO: cyan, WARN: yellow, ERROR: red, DEBUG: gray)

### Bug Fixes

- Fixed macOS Keychain probe false-negative issues
- Fixed debug log copy button functionality
- Fixed biometric toggle requiring password verification
- Fixed password dialog ghost overlay when cancelled
- Fixed duplicate hint button in message boxes

### Build

- Non-notarized distribution support (sandbox + identity signing disabled)
- DMG packaging with create-dmg tool

---

## [1.0.0-pre.1] - 2026-04-22

### Added

- **Flutter macOS Application** — Native macOS client with full feature set
- **Zero-Knowledge Security Architecture** — Master password never stored
- **Rust FFI Crypto Core** — High-performance Argon2id + AES-256-GCM via native FFI
- **Profile Management** — Identity, travel, financial, and professional data
- **OCR Scanning** — Auto-extract data from passports, IDs, and visas
- **Four-Tier Sensitivity System** — Public / Private / Sensitive / Critical
- **Operation History** — Full audit trail of all changes including sensitivity settings
- **Multi-Account Support** — Each account with independently encrypted storage
- **Local Storage Only** — All data in `~/.solosoul/`, no cloud sync

### Features

- **Profile Editor** — Intuitive tab-based interface for managing all profile data
- **Travel Module** — Passports, visas, travel history management
- **Financial Module** — Bank accounts, card information
- **Professional Module** — Education, employment, skills, languages
- **Sensitivity Settings** — Per-field sensitivity level configuration
- **Operation Log Page** — Searchable history of all profile and settings changes
- **Password Verification Dialog** — Re-authentication for sensitive operations

### Security

- **Argon2id Key Derivation** — Memory-hard KDF (64MB, 3 iterations)
- **AES-256-GCM Encryption** — Military-grade symmetric encryption
- **Secure Memory Handling** — Sensitive values zeroed after use
- **24-Hour Session Tokens** — Automatic expiry for plugin sessions
- **Plugin Consent System** — Per-field authorization for third-party access

### Technical Stack

| Component | Technology |
|-----------|------------|
| Frontend | Flutter, Riverpod |
| Crypto Core | Rust, Argon2id, AES-256-GCM |
| Backend | Go, Gin |
| Storage | Local encrypted files |

### Known Issues

- macOS only (other platforms coming soon)
- Touch ID not yet functional

---

## [0.1.0] - 2026-04-09

### Added
- Core crypto: Argon2id KDF, AES-256-GCM encryption, secure memory
- Vault storage system with file-based implementation
- CLI tool: init, unlock, lock, status, profile commands
- gRPC API server for vault operations
- Plugin management system with consent flow
- OCR module with MRZ parsing and PaddleOCR adapter
- Next.js web UI with login, dashboard, profile editor, vault, OCR, plugins, settings pages
- Multi-account support with independent vault directories
- Comprehensive test suite

[Unreleased]: https://github.com/Gczmy/SoloSoul/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.0
[1.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.3.0
[1.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.2.0
[1.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.1.0
[1.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0
[1.0.0-pre.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0-pre.1
[0.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v0.1.0
