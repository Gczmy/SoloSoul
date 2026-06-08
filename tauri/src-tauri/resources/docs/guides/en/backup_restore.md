# Backup & Restore

The backup feature creates a complete encrypted snapshot of your vault for disaster recovery.

## Creating a Backup

1. Go to **Settings → Backup & Restore**
2. Click **Create Backup**
3. Enter a backup name (e.g., "Pre-Update Backup")
4. Wait for the backup to complete

Backup files are stored in the vault directory and include:

- All objects and profile data
- Attachment files
- Preferences
- Operation logs

<!--TIP-->
We recommend creating a backup before major operations (bulk imports, version upgrades).
<!--/TIP-->

## Restoring a Backup

1. Go to **Settings → Backup & Restore**
2. Find the target backup in the list
3. Click **Restore**
4. Confirm the restore operation

<!--WARNING-->
Restoring a backup will **overwrite** all current data in the vault. Make sure current data is saved elsewhere if needed.
<!--/WARNING-->

## Deleting Backups

Each backup in the list can be deleted individually to free up storage space.

## Backup vs Export

| Feature | Backup | Export |
|---------|--------|--------|
| Scope | Entire vault | Selectable objects |
| Format | Internal snapshot | `.solosoul` file |
| Purpose | Local disaster recovery | Cross-device migration, external archiving |
| Location | Vault directory | User-specified path |

## Related Docs

<!--CARDS-->
- [Export & Import](export_import.md) — Data migration solution
- [Sync](sync.md) — Multi-device backup
- [Trash](trash.md) — Temporarily protect data
<!--/CARDS-->

