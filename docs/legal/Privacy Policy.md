# Privacy Policy

**Last updated: 2026-09-28 (AI data-flow description)**

## Data Storage

SoloSoul is a **local-first** application that stores vault data in the device’s data directory. When you use a configured AI service, request content is sent to that service; local storage does not mean every AI request is processed on this device.

## Data We Collect

- SoloSoul does not collect vault content as analytics or advertising data. The AI data sent to your selected service is described below; this must not be read as a promise that no personal data is ever transmitted.
- Local analytics (e.g., app crash logs) are stored only on your device and never sent externally.

## Third-Party Services

- You can configure an on-device, remote, or self-hosted AI service. The actual address and any onward forwarding by that service determine where data goes; its name or a “local” label does not guarantee offline processing.
- Sending a chat message includes the current input and user/assistant conversation history. Sensitive text you type, paste, or retain in history is not automatically redacted by the object-field sensitivity filter.
- With automatic context enabled, requests may also include candidate public objects’ names, types and public fields, selected preferences, software version/platform/language, and retrieved help excerpts. Internal, sensitive, critical and unknown-level object fields are excluded from automatic attachment. Turning this option off still sends your input and history.
- Help-document Embedding requests may send the original query to the configured service; rebuilding the index may send bundled help excerpts. “Prefer local Embedding” can fall back to the configured service when no installed model is selected. It does not change the destination of chat requests.
- API keys are saved in local encrypted configuration and sent to the selected service when authentication is needed. Connection tests, online checks and Embedding support checks may also contact the configured service.
- The initial risk notice is not a second confirmation for every message. Enabled automatic context can accompany a user-initiated request. The selected service’s policies and configuration govern its processing, retention and deletion; SoloSoul does not promise a retention period or exclusion from training.
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

You can export, delete, or modify local vault data. Turning off AI or automatic context, or deleting a local conversation, does not recall content already sent or delete copies held by the selected service. Requests concerning those copies must use that service’s available procedures.

## Contact

For questions, open an issue at: https://github.com/Gczmy/SoloSoul/issues
