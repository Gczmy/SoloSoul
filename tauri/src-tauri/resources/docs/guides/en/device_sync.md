# Device Sync

SoloSoul's device sync lets you securely synchronize **Profiles, Objects, Templates, Trash items, and Attachments** between devices on the **same local network**. All traffic is end-to-end encrypted using **Noise XX**, so neither the SoloSoul team nor any intermediary can decrypt your data.

<!--TIP-->
Sync is completely optional. Your local data remains fully functional even when sync is disabled — SoloSoul is designed local-first.
<!--/TIP-->

---

## 1. Before You Sync

1. **Same account**: both devices must use the same `account_id`. The easiest way is to run two instances from the same account data (for local testing, see "Developer Debugging" below).
2. **Same LAN**: mDNS auto-discovery requires both devices to be on the same Wi-Fi or local network.
3. **Same major version**: keep both apps on the same major version to avoid data-format mismatches.

---

## 2. Enabling Sync

1. Go to **Settings → Device Sync**.
2. Turn on the **Enable Sync** switch at the top.
3. Your device's **fingerprint** is displayed, for example:
   ```
   Your fingerprint: a1b2c3d4...
   ```
   This is the short hash of your device's long-term Noise public key, used for manual verification during pairing.

<!--WARNING-->
**Never trust a fingerprint you do not recognize or cannot verify physically.** An attacker may impersonate your device. Only tap **Trust & Pair** after confirming the other device shows the same fingerprint.
<!--/WARNING-->

---

## 3. Pairing and Trust

SoloSoul uses **TOFU (Trust On First Use)**: the first time a device is seen, you must explicitly verify its fingerprint.

### Auto-discovery (mDNS)

1. When sync is enabled, SoloSoul advertises itself on mDNS (Bonjour/Avahi) and listens for peers.
2. When an untrusted peer is discovered, a **Pair New Device** dialog appears showing:
   - Device name / node_id
   - Peer address
   - Peer fingerprint
3. Verify the fingerprint matches what the other device displays, then tap **Trust & Pair**.
4. Trust is mutual: you must also trust this device from the other side.

### Manual address

If mDNS is blocked by a corporate network, VPN, or firewall, use the manual address fallback:

1. Enter the peer address in the **Sync with Device** field, e.g. `192.168.1.12:54321` or `127.0.0.1:54321`.
2. Tap **Sync**.
3. If this is the first connection, the pairing dialog will appear; verify the fingerprint and trust the peer.

<!--TIP-->
The listening port is assigned by the OS and is not shown directly in the UI. The two-instance smoke-test script (`scripts/dev-two-instances.sh`) and log output provide the port.
<!--/TIP-->

---

## 4. What Syncs and How Conflicts Are Resolved

### Tables synchronized

| Data type | Notes |
|-----------|-------|
| Profiles | The encrypted payload is not decrypted for transport; only a base64-encoded ciphertext is sent and re-encrypted by the receiver. |
| Objects | Includes the `is_deleted` flag, so deletions propagate. |
| User Templates | `properties_json` is decrypted, sent as JSON, and re-encrypted by the receiver. |
| Trash Items | Trash records themselves are business-level tombstones and sync directly. |
| Attachments | After the database records sync, files are transferred separately in 64KB chunks and verified with sha256. |

### Conflict resolution

SoloSoul records a **Hybrid Logical Clock (HLC)** timestamp for every record and uses **Last-Write-Wins (LWW)**:

```text
remote wins iff:
  remote.wall_time_ms > local.wall_time_ms
  or wall_time is equal and remote.counter > local.counter
  or wall_time and counter are equal and remote.node_id > local.node_id
```

When the remote HLC is not newer than the local HLC, the record is marked as a **conflict** and shown in the **Sync Activity** panel:
- Table and record ID
- Local HLC vs remote HLC
- Winner (usually `local`)

The conflict panel **never displays plaintext data** — only metadata so you can decide whether to take manual action.

### Deletion tombstones

Hard-deleting a `profile` or `user_template` writes a `sync_tombstones` entry. When device A deletes a record and device B syncs, B deletes its local copy and keeps the remote HLC as the authoritative deletion time.

---

## 5. Sync Activity Panel

After each sync, the collapsible **Sync Activity** panel below records the last 10 results, including:

- Overall stats: `examined / applied / skipped / conflicts`
- Per-table stats: `{table}: {applied}+{skipped}/{examined}`
- Conflict list (if any)
- Attachment stats: `sent / received / bytes`
- Attachment error messages (if any)

This lets you track what was applied, what was skipped because local data was newer, and whether attachments arrived intact.

---

## 6. Large-Database Chunked Streaming

When a table (e.g. Objects) contains many records, SoloSoul automatically splits it into multiple `Batch` messages:

- Each batch contains at most 100 records (limited by Noise's ~64KB per-message payload size).
- The sender updates `sync_watermarks` after each batch, recording the highest HLC the peer has consumed.
- The receiver applies batches incrementally, avoiding loading the whole table into memory.
- If the connection drops, the next sync resumes from the last watermark, reducing re-transmission.

---

## 7. Security and Privacy

- **End-to-end encryption**: all sync messages are encrypted by the Noise XX `TransportState`.
- **Local-first**: data is already encrypted locally before sync; transported records are ciphertext or re-encrypted.
- **No cloud**: sync never routes through SoloSoul servers; it is purely LAN P2P.
- **Explicit trust**: untrusted devices cannot complete a sync; you must manually verify fingerprints.
- **Audit log**: enabling/disabling sync, trusting/revoking peers, and completed syncs are written to the local audit log.

---

## 8. Troubleshooting

### Devices cannot auto-discover each other

- Confirm both sides use the same `account_id` (same account data).
- Confirm both devices are on the same LAN and mDNS (UDP 5353) is not blocked by a firewall.
- Try manual `host:port` sync.

### Sync shows "Peer is not trusted" / "Peer not trusted"

- Trust the sender's fingerprint on the receiver first, then trust the receiver's fingerprint on the sender.
- Trust state is persisted in the local `sync_peers` table and survives app restarts.

### Attachment sync fails

- Check that the object's `__attachments` field contains `id`, `objectId`, `fileName`, and `sizeBytes`.
- Check that the file exists at `SOLOSOUL_DATA_DIR/<account_id>/attachments/<object_id>/<attachment_id>/`.
- Check the attachment error message in **Sync Activity**.

### Some data did not arrive after sync

- Open **Sync Activity** and look at the `skipped` count. If the local HLC is newer, the remote record is skipped — this is normal conflict resolution.
- Confirm the data was actually saved on the peer with a newer HLC.

---

## 9. Developer Debugging

To debug two-device sync on one machine:

```bash
cd tauri
bash scripts/dev-two-instances.sh
```

The script:
- Uses `SOLOSOUL_DATA_DIR` to give each instance its own data directory.
- Uses different `SOLOSOUL_VITE_PORT` / `SOLOSOUL_VITE_HMR_PORT` values to avoid port collisions.
- Copies the same account into both directories so the `account_id` matches, letting you verify end-to-end sync locally (discovery, pairing, bidirectional sync, attachments, tombstone propagation).

---

## Related Docs

<!--CARDS-->
- [Backup & Restore](backup_restore.md) — Local data safety
- [Export & Import](export_import.md) — Offline migration
- [Object Management](objects.md) — Core synced data
- [Attachment Management](attachments.md) — File sync details
- [Operation Log](audit_log.md) — View sync audit events
<!--/CARDS-->
