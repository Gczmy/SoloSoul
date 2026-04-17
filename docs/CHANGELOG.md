# Changelog

All notable changes to SoloSoul will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **CHANGELOG.md** - Version history documentation
- **Rust FFI for Argon2id** (`crypto-argon2/`) - High-performance Argon2id key derivation using Rust SIMD optimizations for Apple Silicon
- **Change Password API** - `POST /api/auth/password` endpoint with full data re-encryption
- **Shared Header Component** - Left sidebar with navigation, account badge, and lock button shared across all auth pages
- **Request Timeout** - 10-second timeout on all API client requests to prevent hanging

### Changed
- **Multi-account Session Persistence** - `sessionToken` and `currentAccount` are now persisted to localStorage
- **Session Validation** - Fixed closure capture bug in auth page redirects using `getState()` pattern
- **Settings Page** - Change Master Password section is now collapsed by default
- **Dashboard** - Simplified toolbar, account info moved to shared Header
- **Vault ChangePassword** - Now properly re-encrypts all profile data with new key

### Fixed
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
