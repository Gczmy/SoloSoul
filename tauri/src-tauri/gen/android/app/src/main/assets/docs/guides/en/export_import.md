# Export & Import

The Export & Import feature lets you exchange vault data in encrypted `.solosoul` format.

## Exporting Data

1. Go to **Settings → Export & Import**
2. Select pages, objects, and templates to export
3. Choose export options:
   - **Include attachments**: Also export associated files
   - **Include preferences**: Export theme, language, and personal settings
   - **Include behavioral data**: Export operation logs and other audit data
4. Set an export password (at least 8 characters)
5. Click **Export Selected**

<!--STEPPER Export travel section data-->
1. Go to **Settings → Export & Import**
2. In the export area, check the **Travel** page
3. Check **Include attachments** (if you need passport photos, etc.)
4. Set and confirm the export password
5. Click **Export Selected** and choose a save path
<!--/STEPPER-->

## Importing Data

1. Go to **Settings → Export & Import**
2. Switch to the **Import** tab
3. Select a `.solosoul` file
4. Enter the export password
5. Preview the import content and choose an import strategy:
   - **Skip existing**: Keep current data; skip duplicates
   - **Overwrite**: Replace all existing data with imported data
   - **Merge**: Overwrite conflicts; keep non-conflicting items
6. Confirm import

<!--TIP-->
Use the **Preview** feature before importing to review the package contents and avoid accidentally overwriting important data.
<!--/TIP-->

## Export File Security

- `.solosoul` files are encrypted with AES-256-GCM
- The export password is the only key to open the file
- Files can be safely transmitted or stored through any channel

<!--WARNING-->
If the export includes high-sensitivity fields, the password must be sufficiently strong. We recommend 12+ mixed-character passwords.
<!--/WARNING-->

## Related Docs

<!--CARDS-->
- [Backup & Restore](backup_restore.md) — Complete data protection
- [Device Sync](device_sync.md) — Cross-device transfer
- [Trash](trash.md) — Accidental deletion recovery
<!--/CARDS-->

