# Sync

The sync feature helps you discover other devices and exchange data across them.

## Device Discovery

SoloSoul uses mDNS (Bonjour/Avahi) to discover other devices on the same local network:

1. Go to the **Sync** page
2. Click **Scan Devices**
3. The system searches for other SoloSoul instances on the LAN
4. Discovered devices appear in the list

<!--TIP-->
Device discovery requires devices to be on the same local network (Wi-Fi). Devices on different networks cannot auto-discover each other.
<!--/TIP-->

## Enabling Sync

1. On the Sync page, turn on the **Enable Sync** switch
2. Select a device to connect to
3. Confirm pairing

## Sync Content

Sync exchanges the following data:

- Object data (encrypted transmission)
- Profile information
- Attachments (optional)

<!--WARNING-->
Sync is currently in early stages. We recommend creating a local backup before syncing to prevent data conflicts.
<!--/WARNING-->

## Offline Use

SoloSoul is fully functional offline. Sync is an optional feature and does not affect the integrity or availability of local data.
