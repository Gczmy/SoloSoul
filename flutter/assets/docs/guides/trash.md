<!-- version: 1.7.0 -->

# Trash

---

## What is the Trash?

The Trash is SoloSoul's **temporary deletion buffer**. When you delete an object (profile, page, travel record, etc.), it does not disappear from your device immediately. Instead, it is moved to the Trash first.

This gives you a **safety window** — you can recover accidentally deleted objects at any time without worrying about instant data loss.

> **Note**: The Trash is a **sensitive page**. You must verify your master password or biometrics (Touch ID / Face ID) before entering, ensuring only you can view deleted content.

---

## Deleting an Object

### From the Object List

1. On the home screen or any category page, find the target object.
2. Long-press the object card (or tap the **More Options** menu in the upper-right corner of the card).
3. Select **Delete**.
4. The object is moved to the Trash and no longer appears in the original list.

### From the Object Editor

1. Open the target object's edit page.
2. Tap the **Delete** button in the bottom toolbar.
3. After confirming, the object enters the Trash.

> **Tip**: Deletion generates an operation log that records the object's complete state before deletion for auditing purposes.

---

## Entering the Trash

1. Tap the **Trash** entry at the bottom of the sidebar.
2. Verify your master password or biometrics.
3. Enter the Trash page to view all deleted objects.

---

## Restoring an Object

1. Find the target object in the Trash list.
2. Tap the **Restore** button on the right side of the object card.
3. Confirm the restore dialog.
4. The object immediately returns to its original category and location, with all data intact.

> **Tip**: Restored objects retain their original creation time, modification history, and linked attachments. No data is lost.

---

## Permanently Deleting an Object

### Deleting a Single Object

1. Find the target object in the Trash.
2. Tap the **Delete Forever** button on the right side of the object card.
3. A **danger warning** dialog appears. After confirming, the object is completely removed.

> **Warning**: Permanent deletion cannot be undone. The object and all its associated data (including attachment references) are immediately cleared.

### Emptying the Trash

1. At the top of the Trash page, tap the **Empty Trash** button.
2. The system displays the total number of objects in the Trash.
3. After confirming, all objects are permanently deleted at once.

> **Warning**: Emptying the Trash is a bulk permanent deletion operation and cannot be reversed. It is recommended to carefully check for any objects you still need to keep before emptying.

---

## Search & Filter

The Trash supports multiple ways to quickly locate deleted objects:

### Keyword Search

- Enter the object name or type keyword in the top search bar.
- Real-time filtering shows the number of matching results.

### Time Filter

| Option | Description |
|--------|-------------|
| All | Show all objects in the Trash |
| Last 7 days | Only show objects deleted within the last week |
| Last 30 days | Only show objects deleted within the last month |
| Last 90 days | Only show objects deleted within the last 3 months |

### Type Filter

- **Page**: Filter all page-type objects
- **Profile**: Filter profile-type objects
- **Travel** / **Financial** / **Professional**: Filter by preset categories
- **Custom Type**: Filter user-defined type objects

> **Tip**: You can combine multiple filter criteria at once. For example: search "passport" + time "Last 30 days" + type "Travel".

---

## Auto-Cleanup

Objects in the Trash are automatically and permanently deleted after **30 days**. This prevents the Trash from growing indefinitely while giving you ample time to discover and recover accidentally deleted content.

| Status | Location | Recoverable | Retention Period |
|--------|----------|-------------|------------------|
| Active | Home / Category list | N/A | Permanent |
| Deleted | Trash | **Yes** | 30 days |
| Permanently deleted | Removed | **No** | Immediate |

---

## Operation Log

Every restore and permanent deletion action in the Trash generates an **operation log** that records:

- Action type (Restore / Permanent Delete)
- Action time
- Object name and type
- Key property values of the object (for audit tracking)

You can view the complete action history on the **Operation Log** page.

---

## FAQ

**Q: Does the Trash take up space?**

Objects in the Trash still occupy Vault storage space (including linked attachments). Space is only freed after the object is **permanently deleted**.

**Q: Does deleting an object delete its attachments?**

Soft-deleting an object does not immediately delete attachment files. The system only cleans up attachments that are no longer referenced by any object when the object is **permanently deleted** (or after the 30-day auto-cleanup).

**Q: Can I extend the Trash retention period?**

The current auto-cleanup period is fixed at 30 days and is not customizable. If you need to keep something long-term, restore it within 30 days.
