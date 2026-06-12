# SoloSoul (独灵)

**SoloSoul** is a local-first, zero-knowledge personal identity engine. Your data stays encrypted on your device — only you can access it.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Platform: macOS | Windows](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-green.svg)]()
![Version](https://img.shields.io/github/v/release/Gczmy/SoloSoul?label=Build&color=blue)

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
- **Rust Core** — High-performance cryptographic operations

### Object Management

- **Template Engine** — Define custom object templates with 8+ field types (text, number, date, URL, email, select, multi-select)
- **Card-Based Workspace** — Interactive cards with inline editing, sensitivity-aware masking, and collapsible sections
- **History & Snapshots** — Automatic version snapshots on every modification. Browse and roll back to any previous version.
- **Recycle Bin** — Soft-delete with time/type filters, batch operations, conflict detection on restore
- **Full-Text Search** — Search across all objects with category, tag, and type filters

### AI & Tools

- **AI Chat** — Multi-provider LLM support (OpenAI, Anthropic, Ollama, and custom endpoints) with streaming responses
- **Attachment System** — Upload, preview (images/PDF/text), rename, download, soft-delete, all files encrypted with vault key
- **OCR Scanning** — Auto-extract data from passports, IDs, and visas
- **Export/Import** — Encrypted export with tag filtering and attachment support

### Local-First

- **100% Offline** — All data stored locally in `~/.solosoul/`
- **No Cloud Sync** — Your data never leaves your device
- **Multi-Account** — Each account has independently encrypted storage
- **Biometric Unlock** — Touch ID / Face ID for quick vault access

---

## Installation

### macOS

<p>
<a href="https://github.com/Gczmy/SoloSoul/releases/latest/download/SoloSoul.dmg" target="_self">
  <img width="200" src="https://img.shields.io/badge/Download-macOS-blue?style=for-the-badge" alt="Download for macOS" />
</a>
</p>

**Requirements**: macOS 10.15 (Catalina) or later, Apple Silicon or Intel processor

Once downloaded, open the `.dmg` and drag **SoloSoul** to your `/Applications` folder.

> [!IMPORTANT]
> We don't have an Apple Developer account (yet), so macOS will warn you that SoloSoul is from an unidentified developer on first launch. This is expected behavior.
>
> You'll need to bypass this before the app will open. You only need to do this once.

#### Recommended: Terminal (Always Works)

After moving SoloSoul to your Applications folder, run:

```bash
xattr -dr com.apple.quarantine /Applications/SoloSoul.app
```

Then open the app normally.

#### Alternative: System Settings

> [!NOTE]
> This method doesn't work for all users. If this doesn't work, use the Terminal method above.

1. Try to open the app — you'll see a security warning.
2. Click **OK** to dismiss it.
3. Open **System Settings** > **Privacy & Security**.
4. Scroll to the bottom and click **Open Anyway** next to the SoloSoul warning.
5. Confirm if prompted.

### Windows

<p>
<a href="https://github.com/Gczmy/SoloSoul/releases/latest/download/SoloSoul_x64-setup.exe" target="_self">
  <img width="200" src="https://img.shields.io/badge/Download-Windows-blue?style=for-the-badge" alt="Download for Windows" />
</a>
</p>

**Requirements**: Windows 10 or later, 64-bit processor

Once downloaded, double-click the `.exe` installer and follow the setup wizard. SoloSoul will be installed to your Start Menu and Desktop.

> [!NOTE]
> Windows may show a SmartScreen warning because the installer is not yet code-signed. Click **More info** > **Run anyway** to proceed.

---

## Quick Start

### 1. Create Your Vault

On first launch, you'll be prompted to create a master password. This password is derived into an encryption key using Argon2id — it's **never stored**.

### 2. Define Templates

Create object templates that match your data: contacts, documents, credentials, or anything else. Each template supports text, number, date, URL, email, phone, select, and multi-select fields.

### 3. Add Objects

Start adding objects using your templates. Each object is encrypted with AES-256-GCM and stored locally in `~/.solosoul/`.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      SoloSoul (Tauri v2)                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐    ┌──────────────────────────────┐   │
│  │  React Frontend  │◄──►│  Rust Backend (Tauri)        │   │
│  │  (TypeScript)    │IPC │  ┌────────────────────────┐  │   │
│  │                  │    │  │  Commands / Services   │  │   │
│  │  • Zustand       │    │  │  SQLite / Vault        │  │   │
│  │  • CSS Modules   │    │  │  Argon2id / AES-256    │  │   │
│  │  • i18next       │    │  └────────────────────────┘  │   │
│  └──────────────────┘    └──────────────────────────────┘   │
│                              │                              │
│                              ▼                              │
│              ┌──────────────────────────────┐               │
│              │  ~/.solosoul/ (Encrypted)     │               │
│              │  ├── config.json              │               │
│              │  ├── index.db (SQLite)        │               │
│              │  └── acc_{id}/                │               │
│              │      ├── vault.enc            │               │
│              │      └── attachments/         │               │
│              └──────────────────────────────┘               │
└─────────────────────────────────────────────────────────────┘
```

---

## Security Model

| Feature | Implementation |
|---------|----------------|
| Key Derivation | Argon2id (64MB, 3 iterations production) |
| Encryption | AES-256-GCM |
| Session Token | 24-hour expiry |
| Sensitivity Levels | Public / Private / Restricted |
| Master Password | **Never stored**, memory-only key derivation |

### Zero-Knowledge Guarantee

- Master password is **never stored**
- Salt stored in config for key verification only
- Encryption/decryption happens in memory, then sensitive values are securely zeroed
- Data only stored locally in `~/.solosoul/`, **never uploaded to cloud**

---

## Technical Stack

| Component | Technology |
|-----------|------------|
| Frontend | React 19, TypeScript, Vite, Zustand |
| Backend | Tauri v2 (Rust) |
| Database | SQLite (rusqlite) |
| Crypto | Argon2id, AES-256-GCM |
| OCR | PP-OCRv4 (ONNX Runtime) |

---

## Documentation

- [Privacy Policy](./docs/en-US/PRIVACY_POLICY.md) — [中文](./docs/zh-CN/PRIVACY_POLICY.md)
- [Terms of Service](./docs/en-US/TERMS_OF_SERVICE.md) — [中文](./docs/zh-CN/TERMS_OF_SERVICE.md)

---

## License

Apache License 2.0

Copyright (c) 2026 SoloSoul

---

**SoloSoul — Be the only master of your digital self.**
