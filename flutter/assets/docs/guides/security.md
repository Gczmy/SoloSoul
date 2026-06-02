<!-- version: 1.6.8 -->

# Security & Privacy

---

## Encryption

All data in SoloSoul is encrypted with **AES-256-GCM** at the application level. Your master password is never stored — it is used only to derive the encryption key via **Argon2id**.

| Component | Technology | Purpose |
|-----------|------------|---------|
| Data encryption | AES-256-GCM | Protects all stored data |
| Key derivation | Argon2id | Derives key from your password |
| Password storage | None | Your password is never saved |

---

## Vault Locking

### Manual Lock

1. Tap the lock icon in the app header.
2. Alternatively, use the keyboard shortcut if available.

### Auto-Lock

1. Go to **Settings → Security → Auto-Lock**.
2. Choose a time interval:
   - `1 minute`
   - `5 minutes`
   - `15 minutes`
   - `Never`

> **Tip**: Shorter intervals provide better security. Choose based on your environment.

### Background Lock

Enable **Lock on Background** in Security Settings. The vault automatically locks when the app enters the background.

---

## Biometric Unlock

1. Go to **Settings → Security → Biometric Unlock**.
2. Enable Touch ID / Face ID.
3. Authenticate with your device biometric system.

> **Note**: Your biometric data is stored in the device's secure enclave. SoloSoul cannot access it directly.

---

## Clipboard Auto-Clear

Sensitive data copied from SoloSoul is automatically cleared from the system clipboard after a configurable delay.

1. Go to **Settings → Security → Clipboard Clear**.
2. Choose a delay:
   - `10 seconds`
   - `30 seconds`
   - `60 seconds`
   - `Never`

> **Warning**: Setting to `Never` means sensitive data remains in the clipboard indefinitely. Use with caution.

---

## Changing Your Password

1. Go to **Settings → Security → Change Password**.
2. Enter your current password.
3. Enter and confirm your new password.
4. The vault re-encrypts all data with the new key.

> **Warning**: There is no password recovery mechanism. If you forget your password, your data is permanently inaccessible. Store your password in a safe place.

---

## Privacy Guarantee

SoloSoul is fully offline:

- No data is uploaded to any server.
- No analytics, trackers, or telemetry.
- No cloud dependencies.
- All processing happens on your device.
