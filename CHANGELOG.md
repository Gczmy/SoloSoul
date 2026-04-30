# Changelog

All notable changes to SoloSoul are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.4] - 2026-04-30

### Fixed

- **Trash Purge/Restore Notifications** — Fixed missing snackbar notification after purge by using State-level context instead of item builder context, which gets unmounted when the item is permanently deleted
- **Trash UI Polish** — Added detail/history dialogs, responsive action buttons (icon-only on narrow screens), and proper operation logging for purge/restore/empty actions
- **Operation Log Consistency** — Sensitivity tag colors now match across filter chips, list tiles, and detail dialogs
- **Layout Stability** — Fixed button overflow in trash cards and removed problematic fade-in animation that caused hit-test crashes

## [1.4.3] - 2026-04-29

### Fixed

- **Vault Initialization Race** — Fixed a race condition where Android/Windows fallback vault could be accessed before async initialization completed
- **Memory Leaks** — Fixed TextEditingController leaks in property editors (text, number, relation, URL)
- **Build-Time Crashes** — Fixed fire-and-forget async exceptions in object editor save flow; added mounted checks after all async operations

### Performance

- **Faster Sensitivity Settings** — Field list sorting and filtering is now cached, eliminating redundant computation on every UI rebuild
- **Faster Trash** — Replaced 12 individual provider watches with a single aggregated sensitivity map; trash filtering results are cached
- **Reduced Rebuilds** — Search result tiles now watch only revealed field state instead of the entire account style; object workspace and predefined sections use more precise provider watching

## [1.4.2] - 2026-04-29

### Fixed

- **Memory Leaks** — Fixed controller leaks in object editor and object card
- **Trash Completeness** — Trash emptying now correctly removes awards
- **Null Safety** — Safer object tree lookups prevent crashes on stale data
- **Error Logging** — Previously silent failures in vault/crypto operations now log errors
- **State Consistency** — Trash purge no longer mutates input data in place
- **Unlock Robustness** — Login page recovers gracefully from unexpected unlock errors

### Performance

- **Reduced Rebuilds** — Home page, editor dropdown, and account list use more precise provider watching

## [1.4.1] - 2026-04-29

### Fixed

- **Critical Privacy Fix** — Fixed cross-account data leakage when creating a new account after locking a previous one. All sensitive in-memory state is now properly cleared on lock, including unified objects.
- **Operation Log Live Updates** — Operation log page now correctly reflects new entries in real time

### Performance

- **Smoother Search** — Global search now runs on a background thread, eliminating UI freeze on large vaults
- **Faster Data Operations** — Profile load/save and backup create/restore now run JSON processing on background threads
- **Smoother Lists** — Sidebar, trash, and object cards now use lazy list rendering for better scrolling performance

## [1.4.0] - 2026-04-29

### Added

- **Startup Data Integrity Check** — Automatic repair of UnifiedObject tree corruption (duplicate IDs, broken parent/child references)
- **Complete Trash Cleanup** — 30-day auto-purge now covers all sections including travel history, skills, languages, awards, identity cards, addresses, contacts, and unified objects
- **Field History Cleanup** — Automatically removes history entries for permanently deleted items

### Fixed

- Trash emptying now correctly removes awards and unified objects
- Form history no longer grows indefinitely when items are deleted

---

## [1.3.0] - 2026-04-29

### Added

- **Encrypted Backup System** — Full-screen Data Management page with encrypted backup/restore (AES-256-GCM)
- **Special Backups** — Pin up to 5 named backups outside the auto-rotation cycle; rename and restore individually
- **Promote to Special Backup** — Save any regular backup as a special backup with a custom name
- **Auto-Backup on Unlock** — Automatic versioned backup created on every vault unlock
- **Auto-Backup on Upgrade** — App version change triggers automatic backup before first unlock
- **Backup Recovery Prompt** — If vault is empty but backups exist, login offers restore dialog
- **Account Data Isolation** — Switching accounts now properly resets unified object state (no cross-account data leakage)
- **Default Page Protection** — Built-in pages (Profile/Travel/Financial/Professional) are protected from deletion
- **Default Page Sidebar Filtering** — Default pages no longer appear in the Custom Pages section

### Fixed

- **Restore Oldest Backup** — Protective backup creation no longer deletes the target backup being restored
- **Date Masking** — Short values like dates (`1997-08-19`) are now fully masked instead of partially exposed
- **Object Workspace Pop Crash** — Fixed `GoError: There is nothing to pop` when deleting objects
- **Migration Crash** — Fixed `StateError` from `firstWhere` on missing `FormFieldRegistry` fields during v3→v4 migration
- **Overlay Notifications** — Backup success/failure messages now use top-floating overlay instead of SnackBar

---

## [1.2.0] - 2026-04-27

### Added

- **Unified Object Model** — Everything is a `UnifiedObject` with `parentId`/`childrenIds` tree structure
- **Persistent Sidebar** — Drag-resizable sidebar with tree-structured custom pages (expand/collapse)
- **Object Workspace** — Page-centric UI with inline property editing for children
- **Object Editor** — Generic editor for creating/editing objects with icon picker
- **Lock Vault Dialog** — Confirmation dialog before locking with tap-outside-to-dismiss
- **Data Size Display** — Settings page shows total vault data size (B/KB/MB/GB)

### Changed

- **Schema Version** — Bumped to v3; `ProfileData` uses `unifiedObjects` field
- **Home Page** — Simplified to main dashboard only

### Fixed

- **Data Persistence** — Objects now correctly load after login/unlock

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

[Unreleased]: https://github.com/Gczmy/SoloSoul/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.4.0
[1.3.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.3.0
[1.2.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.2.0
[1.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.1.0
[1.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0
[1.0.0-pre.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0-pre.1
