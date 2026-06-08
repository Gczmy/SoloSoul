# Plugins

The plugin system allows extending SoloSoul's functionality. The plugin system is currently in development.

## Plugin Market

SoloSoul plugins are distributed through an independent plugin market:

- Plugins run in WebAssembly (WASM) format
- Each plugin executes in a sandboxed environment
- Plugins require explicit authorization to access data fields

## Installing Plugins

1. Go to the **Plugins** page
2. Browse the available plugin list
3. Click **Install** to download a plugin
4. Enable the plugin after installation

## Plugin Authorization

Each plugin requires your authorization before use:

- Field-level authorization: Select specific fields the plugin may access
- Session authorization: Temporary permission granted per use
- Authorization can be revoked at any time in Settings

<!--WARNING-->
Plugins are developed by third parties. Review their permission requests before installing. Only install plugins from trusted sources.
<!--/WARNING-->

## In Development

The plugin system is under continuous development. Future support includes:

- Custom data importers
- External service integrations
- Automation workflows

<!--TIP-->
Plugin-related help documentation will be updated when the system is officially released.
<!--/TIP-->
