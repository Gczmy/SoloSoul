# Local Two-Instance Sync Smoke Test

> This guide explains how to run two SoloSoul app instances locally to verify end-to-end device sync (mDNS discovery, Noise handshake, bidirectional data sync, attachment sync, and deletion tombstone propagation).

---

## Prerequisites

- macOS or Windows development environment is configured.
- Front-end dependencies are installed (`npm install`).
- Rust dependencies are compiled at least once (`cargo build`).
- Both instances can reach each other on the same LAN (loopback is sufficient).

---

## Quick Start

An automation script is provided:

```bash
cd tauri
bash scripts/dev-two-instances.sh
```

The script will:

1. Check whether device-a account data exists under `SOLOSOUL_SMOKE_DIR` (default `/tmp/solosoul-smoke`).
2. If not, start device-a alone and prompt you to create an account; close the window and press Enter when done.
3. Copy device-a data to device-b so both share the same `account_id` and master password.
4. Launch two `tauri dev` instances simultaneously:
   - device-a: data dir `/tmp/solosoul-smoke/device-a`, Vite port `1420`
   - device-b: data dir `/tmp/solosoul-smoke/device-b`, Vite port `1430`

---

## Manual Verification Steps

### 1. Unlock both instances

Unlock the same account with the same master password in both windows.

### 2. Enable sync

Turn on **Enable Sync** in the **Device Sync** page of both instances and note each fingerprint.

### 3. Trust the peer

- If mDNS works, the untrusted device appears in the known-devices list.
- Tap **Trust & Pair**.
- Compare the peer fingerprint against what the other window displays.

### 4. Trigger sync

In either instance:

- Enter the peer `host:port` (e.g. `127.0.0.1:<port>`; the port is not shown directly in the UI but can be found in logs or via mDNS), then tap Sync; or
- Tap the Sync button on the known-device row if the UI provides one.

### 5. Verify data consistency

On device-a:

- Create a profile, an object, a user template, and upload an attachment.
- Tap Sync.

On device-b:

- Wait for sync to finish, then refresh the relevant pages and confirm the data and attachment arrived.
- Delete the profile on device-a, sync again, and confirm it is also removed on device-b (verifying tombstone propagation).

---

## Customizing Ports and Data Directories

The script uses the following environment variables, which you can override:

| Variable | Default | Description |
|----------|---------|-------------|
| `SOLOSOUL_SMOKE_DIR` | `/tmp/solosoul-smoke` | Root directory for test data |
| `SOLOSOUL_VITE_PORT` | `1420` / `1430` | Vite dev server port |
| `SOLOSOUL_VITE_HMR_PORT` | `1421` / `1431` | Vite HMR WebSocket port |
| `SOLOSOUL_DATA_DIR` | `~/.solosoul` | App data directory |

> Note: the two instances must use different `SOLOSOUL_DATA_DIR` values to avoid file conflicts.

---

## Troubleshooting

### Instances cannot discover each other

- Confirm both sides have the exact same `account_id` (achieved by copying the same account data).
- Some networks block mDNS; use `host:port` manual sync instead.
- Check that the firewall allows `5353/udp` and the random TCP port.

### Sync shows "Peer is not trusted"

- Trust the sender's fingerprint on the receiver first, then trust the receiver's fingerprint on the sender.
- Trust state is persisted in the `sync_peers` table and survives app restarts.

### Attachment sync fails

- Check that the object's `__attachments` field contains valid `id`, `objectId`, `fileName`, and `sizeBytes`.
- Confirm the file exists at `SOLOSOUL_DATA_DIR/<account_id>/attachments/<object_id>/<attachment_id>/`.
- Look for `Attachment exchange failed` warnings in the Rust logs.

### Vite port conflicts

- If `1420/1421/1430/1431` are already in use, edit the script or override via environment variables.

---

## Related Files

| File | Description |
|------|-------------|
| `tauri/scripts/dev-two-instances.sh` | Two-instance launch script |
| `tauri/src-tauri/src/services/vault_service.rs` | `SOLOSOUL_DATA_DIR` support |
| `tauri/vite.config.ts` | `SOLOSOUL_VITE_PORT` / `SOLOSOUL_VITE_HMR_PORT` support |
| `tauri/crates/solosoul-sync/src/manager.rs` | mDNS discovery, Noise handshake, sync sessions |
| `tauri/src/pages/sync/SyncPage.tsx` | Sync settings page |
