<!-- version: 1.6.8 -->

# Sync & Backup

---

## Local Backup

SoloSoul stores everything locally. You can create an encrypted backup at any time.

1. Go to **Settings → Data Management → Create Backup**.
2. Choose a destination folder.
3. A timestamped `.solobackup` file is created.

> **Tip**: Store backups on an external drive or encrypted storage for extra safety.

---

## Restoring from Backup

1. Go to **Settings → Data Management → Restore from Backup**.
2. Select a `.solobackup` file.
3. Enter the vault password that was active when the backup was created.
4. The backup contents replace the current vault data.

> **Warning**: Restoring a backup overwrites all current data. Consider creating a new backup of your current state first.

---

## Export

Individual objects or entire sections can be exported as JSON.

1. Open the object or section you want to export.
2. Tap the **Export** button.
3. Choose whether to encrypt the export file with a password.
4. Select a destination folder.

| Export Option | Encrypted | Use Case |
|---------------|-----------|----------|
| With password | Yes | Secure transfer to another device |
| Without password | No | Quick local copy or printing |

---

## Import

Import JSON files from other sources or from a previous SoloSoul export.

1. Go to **Settings → Data Management → Import**.
2. Select a JSON file.
3. During import, map fields to existing object types or create new ones.
4. Review the import summary before confirming.

---

## No Cloud Sync

SoloSoul does not offer built-in cloud synchronization. To sync across devices, manually transfer backup files using your preferred secure channel.

> **Tip**: Recommended secure transfer methods include encrypted USB drives, Signal, or other end-to-end encrypted channels.
