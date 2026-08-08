# Plugins

The plugin system extends SoloSoul's functionality. Plugins run as **WebAssembly (WASM)** modules in a sandboxed environment and access data only through **explicit authorization**.

## Plugin Market

SoloSoul plugins are distributed through an independent plugin market:

- The **Plugins** page shows the market plugin list and installed plugins
- Each plugin includes its name, version, author, and permission description
- Plugins become available only after installation

## Installing and Uninstalling Plugins

1. Go to the **Plugins** page
2. Browse the available plugin list and click **Install**
3. The plugin appears in the **Installed** list
4. To remove it, click **Uninstall**

## Plugin Authorization

Each plugin requires your authorization before use:

- **Field-level authorization**: select the specific fields the plugin may access (Consent)
- **Session authorization**: plugins request temporary permission per run; can be revoked at any time
- **Authorization status**: viewable and revocable in the plugin details

<!--WARNING-->
Plugins are developed by third parties. Review their permission requests before installing. Only install plugins from trusted sources.
<!--/WARNING-->

## Plugin Sessions and Logs

- Plugins run in their own sessions; execution records can be reviewed
- Data access (e.g., attachments) respects permission boundaries; unauthorized resources cannot be read

## Developing Plugins

Developers can refer to `docs/wasm-plugin-development-guide.md` (outside the help docs) and the plugin SDK documentation to write WASM plugins, including host functions, error codes, and field-mapping conventions.

## Related Docs

<!--CARDS-->
- [AI Chat](ai_chat.md) — AI plugin ecosystem
- [LLM Config & Statistics](llm_config.md) — Configure models
- [Object Management](objects.md) — Plugins operate on objects
<!--/CARDS-->
