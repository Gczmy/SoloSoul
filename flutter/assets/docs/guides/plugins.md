<!-- version: 1.6.8 -->

# Plugins

---

## What is a Plugin?

A **plugin** is a WebAssembly (Wasm) extension module that adds new capabilities to SoloSoul. Plugins run inside a secure Wasmtime sandbox and are distributed through the Plugin Market.

> **Note**: Plugins are not the same as objects. An object is a structured record in your vault (passport, bank account, etc.). A plugin is a functional extension that processes or enhances data.

| Concept | What It Is | Example |
|---------|-----------|---------|
| Plugin | Wasm extension module | OCR scanner, data formatter |
| Object | Structured personal record | Passport, contact, address |
| Page | UI grouping of objects | Travel, Financial, Professional |

---

## How Plugins Work

### Security Model

Plugins operate under a strict security model:

1. **Wasm Sandbox**: Each plugin runs in an isolated Wasmtime sandbox with no direct access to your files or network.
2. **SHA-256 Verification**: Every plugin is checksum-verified before loading.
3. **Field-Level Authorization**: Plugins must declare which data fields they need, and you approve each request individually.
4. **Revocable Access**: You can revoke a plugin's permissions at any time.

### Plugin Lifecycle

```
Discover → Install → Authorize → Use → Update/Uninstall
```

---

## Installing a Plugin

1. Go to **Plugins** from the sidebar or home quick actions.
2. Browse the available plugins in the market tab.
3. Tap a plugin to view its details, version, and required permissions.
4. Tap **Install** to download the plugin.
5. After installation, the plugin appears in your **Installed** tab.

> **Tip**: Plugins are downloaded from the official Plugin Market via CDN. No external server is involved.

---

## Authorizing a Plugin

When a plugin requests access to your data:

1. A consent dialog shows which fields the plugin wants to access.
2. Review each field and its sensitivity level.
3. Tap **Allow** to grant access or **Deny** to block it.
4. You can override the sensitivity level for individual fields if needed.

> **Warning**: Only authorize plugins you trust. A plugin with access to sensitive fields can read that data within the sandbox.

---

## Managing Installed Plugins

1. Go to **Plugins** and switch to the **Installed** tab.
2. For each plugin you can:
   - **Run**: Execute the plugin's main function.
   - **Update**: Upgrade to the latest version if available.
   - **Uninstall**: Remove the plugin completely.
3. To review or change permissions, tap the plugin and select **Permissions**.

---

## Plugin Market

The Plugin Market is a public GitHub repository that serves as the distribution source:

- **Zero server cost**: Plugins are delivered via jsDelivr CDN with GitHub Raw fallback.
- **Community contributions**: Third-party developers can publish plugins following the SDK guidelines.
- **Versioning**: Each plugin has semantic versioning. SoloSoul checks compatibility before installation.

> **Tip**: If a plugin fails to load, check that your app version meets the plugin's minimum version requirement.

---

## Troubleshooting

| Issue | Solution |
|-------|---------|
| Plugin fails to install | Check network connection and app version compatibility |
| Plugin shows "unauthorized" | Re-open the plugin and grant the requested field permissions |
| Plugin crashes | Uninstall and reinstall the plugin; check for updates |
| Slow plugin performance | Large Wasm files may take time to load; try restarting the app |
