# Changelog

All notable changes to SoloSoul will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/solosoul/solosoul/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/solosoul/solosoul/releases/tag/v0.1.0
