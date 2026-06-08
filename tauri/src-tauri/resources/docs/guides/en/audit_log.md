# Operation Log

The Operation Log records all significant actions on the vault for auditing and security tracing.

## Viewing Logs

1. Go to **Settings → Operation Log**
2. Logs are listed in reverse chronological order
3. Each entry includes: time, action type, entity, performer, and details

## Filtering Logs

Filter by the following conditions:

- **Action Type**: Create, Update, Delete, Restore, Permanent Delete, Rollback, etc.
- **Entity Type**: Object, Page, Profile, Biometric, Export, Import, etc.
- **Performer**: User action / System automatic

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

Click **Export Logs** to export the current filtered log range as a text file.

<!--TIP-->
Operation logs are read-only. Users cannot modify or delete log entries. This ensures audit integrity.
<!--/TIP-->

## Log Retention

Operation logs are retained long-term locally. To clean up historical logs, use the relevant feature on the **Debug Log** page.
