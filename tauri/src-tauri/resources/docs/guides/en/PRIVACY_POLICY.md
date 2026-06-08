# Privacy Policy

**Last Updated: 2026-06-08**

## Data Storage

SoloSoul uses a **local-first** architecture:

- All personal data is stored only on your local device
- Data is encrypted with AES-256-GCM; the key is derived from your master password
- No data is uploaded to SoloSoul's servers or cloud

## Zero-Knowledge Architecture

SoloSoul is designed to ensure:

- Developers cannot access your vault contents
- Servers cannot decrypt your data
- Even if the device is lost, the vault cannot be read without the master password

## AI Feature Data Processing

When you use the AI Chat feature:

- Conversation content and system prompts are sent to your configured third-party AI provider
- Only `public`-level object data is injected into system prompts
- Sensitive / restricted / critical data is **never** sent to AI
- Conversation history is stored locally only

## Biometric Data

- Biometric credentials (e.g., Touch ID) are stored securely by the operating system
- SoloSoul does not store any biometric data
- Biometrics are used only for unlocking the local vault

## Data Export and Deletion

- You can export all data as an encrypted `.solosoul` file at any time
- You can delete your account and vault at any time
- After deletion, all data is permanently removed from the local device

## Contact

For privacy-related questions, please open an Issue on our GitHub repository.
