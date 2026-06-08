# Snapshots and History

SoloSoul automatically creates a snapshot every time an object is modified, letting you roll back to any historical version.

## Automatic Snapshots

Snapshots are created automatically when:

- An object is modified and saved
- Object fields change

Each snapshot contains the object's complete state (name, properties, tags).

## Viewing History

1. Find the target object in the workspace
2. Click the clock icon (history) on the object card
3. A flip-book style history browser opens
4. Use the left/right arrows to navigate between versions

<!--STEPPER View an object's history-->
1. Find the object in the workspace
2. Click the clock icon (shows snapshot count badge)
3. Browse versions in the history viewer
4. Click the left arrow for older versions, right arrow for newer
5. Click **Close** to exit the history viewer
<!--/STEPPER-->

## Sensitive Data in History

The history browser follows the same sensitivity rules:

- `sensitive` / `critical` fields are blurred
- Viewing `critical` fields requires password verification

## Restoring from Snapshot

To roll back an object to a previous version:

1. Go to **Settings → Data Management**
2. Or use the standalone **History** page (accessed from object details)
3. Select the target snapshot and click **Restore to this version**

<!--TIP-->
Snapshots don't consume much space; the system manages them automatically. You can view snapshot storage usage in **Data Management**.
<!--/TIP-->
