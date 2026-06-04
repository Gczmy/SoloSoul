<!-- version: 1.0.0 -->

# Import & Export

---

## Overview

SoloSoul allows you to **export** your account data into an encrypted `.solosoul` package file for backup or migration to another device. You can also **import** data from a `.solosoul` file into your current account.

> **Note**: Import/Export is different from **Local Backup**. Backups are full Vault snapshots, while `.solosoul` packages are per-account data bundles that support selective import.

---

## Exporting Data

### Steps

1. Go to **Settings → Data Management → Export / Import**.
2. Switch to the **Export** tab.
3. Review the account information displayed.
4. Tap the **Export** button.
5. Enter your **master password** in the verification dialog.
6. Choose a save location. The default file name is `{AccountName}_{AccountID}.solosoul`.
7. Wait for the export to complete. A success message will appear when finished.

> **Tip**: The `.solosoul` file contains your account data, profile objects, custom types, and attachments — all encrypted. Only someone who knows the master password can decrypt it.

---

## Importing Data

### Steps

1. Go to **Settings → Data Management → Export / Import**.
2. Switch to the **Import** tab.
3. Tap **Select File** and choose the `.solosoul` file you want to import.
4. The system displays the number of objects and attachments in the file.
5. Enter the **master password** used when the file was exported, then tap **Preview**.
6. In the import preview screen, select the collections you want to import and review target page mappings.
7. Tap **Confirm Import** and wait for the operation to finish.

> **Warning**: The import process automatically creates a silent backup of your current account before proceeding. However, we still recommend manually creating a backup before importing.

---

## Import Preview

Before confirming the import, the system shows a preview screen to help you understand what will be imported:

### Collection List

Each collection represents a group of related objects (e.g., travel records, financial accounts). You can:

- **Check/uncheck** individual collections to control which data is imported
- View the **object count** for each collection
- View the **highest sensitivity level** in the collection:
  - 🟢 Green = Public
  - 🟡 Yellow = Internal
  - 🟠 Orange = Sensitive
  - 🔴 Red = Critical

### Relation Property Warnings

If a collection contains relation properties (links to other objects), the preview shows:

- **Total relation properties**: The number of relation fields across all objects in the collection
- **Cross-partition relations**: Relations pointing to objects that do not exist in your current account (these will become invalid after import)

> **Tip**: Cross-partition relations will not block the import, but those relation fields will become empty after import. To preserve complete relation chains, make sure all related objects already exist in your current account, or import them together.

### Target Page Mapping

For each selected collection, you can specify a **target page** (e.g., Profile, Travel, Financial, Professional) where the imported data should be placed. If not specified, the system will automatically assign pages based on data type.

---

## Security

### Encryption

`.solosoul` files use the following encryption scheme:

| Component | Technology | Description |
|-----------|------------|-------------|
| Key derivation | Argon2id | Derives encryption key from your master password |
| Data encryption | AES-256-GCM | Military-grade encryption standard |
| Integrity check | SHA-256 | Prevents file tampering |

### Password Verification

- Both export and import require verification of your master password
- The password is only used in memory and is never stored inside the file
- If the wrong password is entered during import, the system will display a "Wrong password" error and cannot decrypt the data

### File Security Recommendations

| Scenario | Recommendation |
|----------|----------------|
| Local backup | Store `.solosoul` files on an encrypted disk or external drive |
| Cross-device transfer | Use Signal, encrypted USB drives, or other end-to-end encrypted channels |
| Cloud storage | If uploading to cloud storage, consider adding a second layer of encryption |

---

## FAQ

**Q: What's the difference between Import/Export and Local Backup?**

Local backups (`.solobackup`) are complete Vault snapshots containing all account data. Restoring a backup overwrites all current data. In contrast, `.solosoul` import/export targets a single account and supports selective import, making it suitable for cross-account data migration.

**Q: Can I import data into a different account?**

Yes. `.solosoul` files are not tied to a specific account. As long as you know the master password, you can import the file into any SoloSoul account. The system automatically handles ID remapping to avoid conflicts with existing data.

**Q: What if I forget the password?**

`.solosoul` files are encrypted with the master password used at export time. If you forget the password, the file cannot be decrypted. We recommend recording the password hint or storing the password in a password manager when exporting.

**Q: Where does imported data go?**

Imported objects are organized according to the **target page** you specified in the preview screen. If no target page was specified, the system automatically assigns them to Profile, Travel, Financial, or Professional pages based on data type.

**Q: Will importing delete my existing data?**

No. Importing only **adds** new data and will not delete or overwrite existing objects. If an imported custom type has the same name as an existing one, the system intelligently merges the type definitions.

**Q: Are attachments imported too?**

Yes. `.solosoul` files include all attachment data. During import, attachments are re-encrypted and stored in your current account's attachment pool.
