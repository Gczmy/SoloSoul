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

## Export & Import

SoloSoul supports exporting your account data to an encrypted `.solosoul` package file for backup or migration, and importing data from such files into your current account.

### Export

1. Go to **Settings → Data Management → Export / Import**.
2. Switch to the **Export** tab.
3. Tap the **Export** button and verify your master password.
4. Choose a save location. The default file name is `{AccountName}_{AccountID}.solosoul`.

### Import

1. Go to **Settings → Data Management → Export / Import**.
2. Switch to the **Import** tab.
3. Select a `.solosoul` file and enter the export password.
4. Review the import preview (select collections, map target pages).
5. Confirm to complete the import.

> **Note**: For detailed instructions, see the **Import & Export** guide. The `.solosoul` format uses Argon2id + AES-256-GCM encryption and supports selective import with attachment migration.

---

## No Cloud Sync

SoloSoul does not offer built-in cloud synchronization. To sync across devices, manually transfer backup files using your preferred secure channel.

> **Tip**: Recommended secure transfer methods include encrypted USB drives, Signal, or other end-to-end encrypted channels.
