# Changelog

All notable changes to SoloSoul are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[Unreleased]: https://github.com/Gczmy/SoloSoul/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.1.0
[1.0.0]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0
[1.0.0-pre.1]: https://github.com/Gczmy/SoloSoul/releases/tag/v1.0.0-pre.1
