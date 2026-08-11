# Privacy Policy

**Last updated: 2026-06-01**

## Data Storage

SoloSoul is a **local-first** application. All your data is stored exclusively on your device under `~/.solosoul/`. No data is uploaded to any cloud server.

## Data We Collect

- **None.** SoloSoul does not collect, transmit, or share any personal data.
- Local analytics (e.g., app crash logs) are stored only on your device and never sent externally.

## Third-Party Services

- SoloSoul may connect to local or self-hosted LLM services you configure. These connections are entirely under your control.
- No third-party analytics, telemetry, or advertising SDKs are included.

## Update Checks & Download Proxying

- When the app checks for updates or downloads installers, it **connects directly to GitHub by default**.
- When direct connections are unavailable (e.g., in some network environments where GitHub is unreachable), the app automatically falls back to third-party acceleration proxies. These proxies terminate TLS and forward your requests, so **your IP address, the fact that you use SoloSoul, and the target version number may be exposed to that third-party proxy provider**.
- Downloaded content is verified with cryptographic signatures and hashes whether it comes from a direct connection or a proxy, so proxies cannot tamper with installers; proxying only affects the transport channel, not data integrity.
- A proxy may return stale or altered update metadata (e.g., information about an older version), which could prevent you from learning about a new version in time; this does not affect installer integrity (signature and hash verification still apply) — it may only delay update notifications.
- To fully disable proxy fallback in the app's own download channels (direct connections only), you can set the environment variable `SOLOSOUL_PROXY_PREFIXES` to an empty value — this variable covers the app's built-in GitHub API metadata and installer download relay paths.
- **Note**: The desktop app's built-in update check channel (updater plugin) has direct-connection and multiple proxy fallback endpoints compiled in at build time and is **not controlled by this environment variable**; when direct connections are unavailable it will still try the built-in proxy endpoints, and this behavior cannot be disabled via environment variables (on Android the full flow is governed by the variable, but setting environment variables on mobile is impractical).

## Security

- Your vault is encrypted with AES-256-GCM using a key derived from your master password (Argon2id).
- The master password is never stored — it exists only in memory during your session.

## Your Rights

Since all data is local, you have full control: export, delete, or modify your data at any time.

## Contact

For questions, open an issue at: https://github.com/Gczmy/SoloSoul/issues
