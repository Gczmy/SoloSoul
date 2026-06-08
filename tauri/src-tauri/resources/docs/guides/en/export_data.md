# How to Export Data

SoloSoul supports exporting your data as an encrypted package for backup and migration.

## Steps

1. Open the **Settings** page
2. Select **Export/Import** from the left menu
3. In the export area, choose what to export:
   - Select pages and objects
   - Include attachments (optional)
   - Include preferences (optional)
   - Include behavioral data (optional)
4. Set an export password (optional, for additional encryption)
5. Click **Start Export** and choose a save location

## Export Format

The export file is in `.solosoul` format, encrypted with AES-256-GCM.

## Notes

- Export is done entirely locally, no data is uploaded to the cloud
- Regular backups are recommended
- Export files are cross-platform (macOS, Windows, Linux)