# SoloSoul (独灵)

**SoloSoul** is a local-first, zero-knowledge personal identity engine. Your data stays encrypted on your device — only you can access it.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Platform: macOS](https://img.shields.io/badge/Platform-macOS-green.svg)](https://flutter.dev)

---

## Overview

SoloSoul is a **Local Digital Twin & Universal Identity Engine** — a decentralized, local-encrypted personal super-profile and automation execution engine.

**Core Philosophy**: "Centralized Schema definition, decentralized data storage"

---

## Features

### Security First

- **Zero-Knowledge Architecture** — Master password never stored, only derived key in memory
- **Argon2id Key Derivation** — Memory-hard KDF resistant to brute-force attacks
- **AES-256-GCM Encryption** — Military-grade encryption for all data
- **Rust Core** — High-performance cryptographic operations via native FFI

### Data Management

- **Profile Editor** — Manage identity, travel, financial, and professional information
- **OCR Scanning** — Auto-extract data from passports, IDs, and visas
- **Sensitive Fields** — Four-tier sensitivity system (Public / Private / Sensitive / Critical)
- **Operation History** — Full audit trail of all changes

### Local-First

- **100% Offline** — All data stored locally in `~/.solosoul/`
- **No Cloud Sync** — Your data never leaves your device
- **Multi-Account** — Each account has independently encrypted storage

---

## Installation

### Requirements

- **macOS** 10.15 (Catalina) or later
- **Apple Silicon** or Intel processor

### Install via DMG

1. Download the latest release from [GitHub Releases](https://github.com/Gczmy/SoloSoul/releases)
2. Double-click to open the DMG
3. Drag **SoloSoul.app** into your Applications folder
4. On first run, right-click SoloSoul in Finder → **Open** → Allow apps from unidentified developers

---

## Quick Start

### 1. Create Your Vault

```
┌─────────────────────────────────────┐
│           SoloSoul                   │
│                                      │
│  ┌─────────────────────────────┐    │
│  │     Create New Vault        │    │
│  └─────────────────────────────┘    │
│                                      │
│  Master Password: ●●●●●●●●●●         │
│  Confirm:        ●●●●●●●●●●         │
│                                      │
│        [ Create Vault ]              │
└─────────────────────────────────────┘
```

### 2. Unlock with Password

Enter your master password to unlock your vault. The password is derived into an encryption key using Argon2id — it's never stored.

### 3. Manage Your Profile

Navigate through tabs to manage different aspects of your identity:

| Tab | Contents |
|-----|----------|
| **Profile** | Name, birthdate, contact info, addresses |
| **Travel** | Passports, visas, travel history |
| **Financial** | Bank accounts, card information |
| **Professional** | Education, employment, skills |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      SoloSoul App                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────────────┐  │
│  │  UI      │───►│ Flutter  │───►│  Rust FFI        │  │
│  │  Layer   │    │  Layer   │    │  (Crypto Core)   │  │
│  └──────────┘    └──────────┘    └──────────────────┘  │
│                                              │              │
│                                              ▼              │
│  ┌──────────┐    ┌──────────┐    ┌──────────────────┐  │
│  │  Plugin  │◄───│ Consent  │◄───│  ~/.solosoul/    │  │
│  │  API     │    │ Manager  │    │  (Encrypted)     │  │
│  └──────────┘    └──────────┘    └──────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Data Storage

```
~/.solosoul/
├── config.json           # Config (salt, version)
├── index.db              # Encrypted index
├── accounts/
│   └── acc_{id}/
│       └── *.enc         # Encrypted profile data
└── plugins/              # Plugin configurations
```

---

## Sensitivity Levels

SoloSoul uses a four-tier sensitivity system for field-level access control:

| Level | Label | Description |
|-------|-------|-------------|
| **Public** | 公开 | Visible without authentication |
| **Private** | 私有 | Requires unlock to view |
| **Sensitive** | 敏感 | Requires password re-verification |
| **Critical** | 机密 | Extra protection, access logged |

All sensitivity changes are recorded in the operation history for security auditing.

---

## Security Model

| Feature | Implementation |
|---------|----------------|
| Key Derivation | Argon2id (64MB, 3 iterations) |
| Encryption | AES-256-GCM |
| Session Token | 24-hour expiry |
| Plugin Consent | Per-field authorization |
| Sensitive Data | Auto-zero after use |

### Zero-Knowledge Guarantee

- Master password is **never stored**
- Salt stored in `~/.solosoul/{account_id}/config.json` for key verification only
- Encryption/decryption happens in memory, then sensitive values are securely zeroed
- External plugins require **explicit user consent** via Consent Manager

---

## Technical Stack

| Component | Technology |
|-----------|------------|
| Frontend | Flutter, Riverpod |
| Crypto Core | Rust, Argon2id, AES-256-GCM |
| Backend | Go, Gin |
| Storage | Local encrypted files |

---

## Documentation

- [User Guide](./docs/USER_GUIDE.md) — Detailed usage instructions
- [Privacy Policy](./docs/PRIVACY_POLICY.md)
- [Terms of Service](./docs/TERMS_OF_SERVICE.md)

---

## License

Apache License 2.0

Copyright (c) 2026 SoloSoul

See [LICENSE](./LICENSE) for full license terms.

---

**SoloSoul - Be the only master of your digital self.**
