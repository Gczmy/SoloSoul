# Operation Log

The Operation Log records all significant actions on the vault for auditing and security tracing.

## Viewing Logs

1. Go to **Settings → Operation Log**
2. Logs are listed in reverse chronological order
3. Each entry includes: time, action type, entity, performer, and details

## Filtering Logs

Filter by the following conditions:

- **Entity Type**: Object, Page, Template, Attachment, Biometric, Export, Import, etc.
- **Keyword search**: search by action type, entity name, etc.

## Log Content

Common operation examples:

| Action | Detail Description |
|--------|--------------------|
| Create Object | Shows the section it belongs to |
| Update Object | Records a summary of field changes |
| Delete Object | Marked as moved to Trash |
| Restore Object | Marked as restored to original location |
| Permanent Delete | Records the deleted object name |
| Enable Biometric | Records the enablement location |

## Exporting Logs

Click **Export Logs** to export the current logs as a JSON file (default filename `audit_log_export.json`) for archiving or audit analysis.

<!--TIP-->
Operation logs are read-only. Users cannot modify or delete log entries. This ensures audit integrity.
<!--/TIP-->

## Related Docs

<!--CARDS-->
- [Security Settings](security.md) — Audit security events
- [Sensitivity & Privacy](sensitivity.md) — Protection level change records
- [Trash](trash.md) — Deletion operation audit
<!--/CARDS-->

