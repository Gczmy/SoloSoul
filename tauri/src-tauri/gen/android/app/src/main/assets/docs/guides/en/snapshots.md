# Snapshots and History

SoloSoul automatically creates a snapshot every time an object is modified, letting you roll back to any historical version.

## Automatic Snapshots

Snapshots are created automatically when:

- An object is modified and saved
- Object fields change

Each snapshot contains the object's complete state (name, properties, attachment associations, etc.).

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

1. Open the object details → click the history icon to enter the history viewer
2. Select the target snapshot version
3. Click **Restore** and confirm

<!--TIP-->
Snapshot storage size can be viewed in **Settings → Data Management**, where you can also set a snapshot retention limit.
<!--/TIP-->

## Related Docs

<!--CARDS-->
- [Object Management](objects.md) — Snapshots work on objects
- [Workspace](workspace.md) — View object changes
- [Trash](trash.md) — Recover accidentally deleted items
<!--/CARDS-->

