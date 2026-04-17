# SoloSoul Privacy Policy

**Effective Date: April 17, 2026**

## 1. Introduction

SoloSoul ("we," "our," or "us") is committed to protecting your privacy. SoloSoul is a local-encrypted personal data management application that stores all your data locally on your device. This Privacy Policy explains how we collect, use, and safeguard your information when you use our software.

**Key Principle**: SoloSoul is designed with a "local-first" architecture. Your personal data never leaves your device unless you explicitly choose to export it. We do not operate servers that store your data, and we do not use cloud services to synchronize your information without your explicit consent.

## 2. Information We Collect

### 2.1 Information You Provide
- **Account Information**: When you create a SoloSoul account, you provide an account name. Your master password is derived into an encryption key locally and is never transmitted or stored on any external server.
- **Profile Data**: You may voluntarily enter personal information into SoloSoul, including but not limited to:
  - Travel documents (passport numbers, visa information)
  - Financial account information
  - Professional credentials and identifiers
  - Contact information
  - Travel history

### 2.2 Automatically Collected Information
- **Local Device Information**: We may collect basic device information (device name, operating system) solely for the purpose of tracking device access to your vault. This information is stored locally in your vault and is not transmitted externally.
- **Usage Analytics**: We do not collect usage analytics, telemetry, or behavioral data. SoloSoul operates completely offline with respect to user data collection.

## 3. How We Use Your Information

Your information is used exclusively for the following purposes:

1. **Vault Encryption**: Your master password is used to derive an encryption key (using Argon2id) that encrypts all your data locally using AES-256-GCM.
2. **Account Management**: Account metadata is stored locally to manage access to your vault.
3. **Feature Functionality**: Profile data enables the core features of SoloSoul, including document storage, identity management, and personal data organization.

**We do not use your personal information for advertising, marketing, or any commercial purposes beyond the core functionality of the application.**

## 4. Data Storage and Security

### 4.1 Local Storage
All data managed by SoloSoul is stored locally on your device in an encrypted format. The encryption is performed using industry-standard algorithms:
- **Key Derivation**: Argon2id with 16384 KiB memory, 1 iteration, and 4 parallelism
- **Encryption**: AES-256-GCM

### 4.2 Data Location
Your data is stored in your system's user directory under `~/.solosoul/`. Each account has its own subdirectory with encrypted data files. **This directory remains under your full control and can be deleted at any time to remove all SoloSoul data.**

### 4.3 No Cloud Storage (Default)
By default, SoloSoul does not synchronize your data to any cloud service. All data stays on your local device.

### 4.4 Optional Cloud Sync
If you choose to enable cloud synchronization features (when available), you will be required to provide credentials for your own cloud storage service. SoloSoul acts only as an intermediary that encrypts your data before transmission. Your cloud storage provider's privacy policy will govern their handling of your encrypted data.

## 5. Data Sharing and Disclosure

### 5.1 No Disclosure to Third Parties
We do not sell, trade, or otherwise transfer your personal information to third parties. Your data remains under your control at all times.

### 5.2 Legal Requirements
We will not disclose your personal information to law enforcement or government agencies unless required by valid legal process (such as a court order or subpoena). Even in such cases, the data provided would only be the encrypted vault files stored locally on your device, which we may be compelled to provide if legally required.

## 6. Your Rights

As a SoloSoul user, you have the following rights regarding your data:

1. **Access**: You can access all your data at any time through the SoloSoul application.
2. **Correction**: You can edit or correct your personal information at any time.
3. **Deletion**: You can delete your SoloSoul account and all associated data at any time through the application settings or by deleting the `~/.solosoul/` directory.
4. **Portability**: You can export your data from SoloSoul in standard formats (when export features are available).

## 7. Children's Privacy

SoloSoul is not intended for use by children under the age of 13 (or the minimum age of digital consent in your jurisdiction). We do not knowingly collect personal information from children. If you become aware that a child has provided us with personal information, please contact us so we can delete that data.

## 8. International Data Transfers

Since SoloSoul stores all data locally on your device, there are no international data transfers under normal operation. If you choose to enable cloud synchronization and store your encrypted data with a cloud provider located in another jurisdiction, that transfer would be governed by your chosen cloud provider's policies.

## 9. Data Breach Notification

In the event of a security breach that compromises your encrypted data, we will notify you through the contact information associated with your account within 72 hours of becoming aware of the breach. Due to the encrypted nature of our storage, a breach would only expose encrypted data, which would require significant computational resources to attempt to decrypt.

## 10. Third-Party Services

### 10.1 Local Authentication
SoloSoul may use your device's biometric authentication capabilities (Face ID, Touch ID, or similar) for convenient unlock. Biometric data is processed locally by your device's operating system and is never accessed or stored by SoloSoul or transmitted externally.

### 10.2 Cloud Storage (Optional)
If you enable cloud synchronization, your data will be stored with your chosen cloud storage provider. Please review their privacy policies.

## 11. Changes to This Privacy Policy

We may update this Privacy Policy from time to time to reflect changes in our practices or legal requirements. When we make material changes, we will:
- Update the "Effective Date" at the top of this policy
- Provide prominent notice within the application

Your continued use of SoloSoul after such changes constitutes your acceptance of the updated Privacy Policy.

## 12. Contact Us

If you have questions about this Privacy Policy or SoloSoul's privacy practices, please contact us at:

**Email**: privacy@solosoul.app

**Mail**: SoloSoul Privacy
[Company Address]

## 13. Jurisdiction-Specific Disclosures

### For European Union Users (GDPR)
If you are located in the European Union, you have additional rights under the General Data Protection Regulation, including:
- The right to be informed about data processing
- The right of access to your personal data
- The right to rectification
- The right to erasure ("right to be forgotten")
- The right to data portability
- The right to restrict processing
- The right to object

Since SoloSoul stores all data locally and does not process it on external servers, most GDPR obligations relate to your local device. To exercise your rights, you may export or delete your data directly through the application or by removing the local storage directory.

### For California Users (CCPA/CPRA)
If you are a California resident, you have the right to:
- Know what personal information is collected about you
- Know whether your personal information is sold or disclosed
- Say "no" to the sale of personal information
- Access your personal information
- Request deletion of your personal information
- Not be discriminated against for exercising your privacy rights

SoloSoul does not sell your personal information. Your data remains exclusively on your local device.

### For Users in Other Jurisdictions
The privacy laws of your jurisdiction may provide additional rights or requirements. SoloSoul is designed to respect user privacy as a core principle, and we will respond to legitimate privacy-related requests in accordance with applicable law.
