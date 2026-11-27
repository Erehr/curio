# Connections

Use this checklist when the Curio window does not appear, the local API or browser extension cannot connect, MCP tools are missing or a plugin fails to load.

## Tray & port

- **Curio is running in the tray or menu bar**: Curio stays hidden when started with `--tray`, started in the background by **Send to Curio** or closed to the tray or menu bar. Click the leaf icon in the system tray (Windows and Linux) or menu bar (macOS) to show it.
- **Off-screen or broken window styling**: If a custom theme or Custom CSS breaks the window, launch once with `--safe-mode` to ignore custom styling. Run `Curio.exe --safe-mode` (Windows), `curio --safe-mode` (Linux) or `/Applications/Curio.app/Contents/MacOS/curio --safe-mode` (macOS).
- **API port already in use**: Curio binds to `127.0.0.1:41733` by default. If another program uses that port, open **Settings › API** and pick another loopback port (such as `41734`), then update **Send to Curio › Options** and your scripts or MCP client. Verify reachability with:

```bash
curl http://127.0.0.1:41733/api/v1/ping
```

## Auth & scope

When an API request returns `401 unauthorized`, `403 forbidden` or `not_allowed`:

1. Confirm **Enable API** is on in **Settings › API** and your `Authorization: Bearer curio_...` header matches an active token.
2. Check the token's permissions in **Settings › API**. Importing from local paths requires both `files.write` and `files.outside`; window commands require `ui`.
3. Check the token's allowed **Libraries**. Calls targeting a library outside that list fail with `forbidden`.
4. Check allowed **Origins** for browser extensions and scripts. Paired extension tokens pin to `chrome-extension://<id>`. If an unpacked extension's ID changed, click **Disconnect** in Extension Options and pair again.
5. Check whether the target library is offline (`library_offline`) or has no open window (`no_window` for window-only calls).

See [Error reference](../../api/connect/errors.md) for every error code and recovery step.

## MCP

If your AI assistant connects to `POST http://127.0.0.1:41733/mcp` but sees fewer tools than expected or none at all:

1. Open **Settings › MCP** and confirm **Enable MCP** is on.
2. Check which tool families and individual operations are switched on in **Settings › MCP**.
3. Check that your client's Bearer token grants the permissions those tools need.

Available tools are the intersection of eligible operations (up to 140 MCP tools), tools enabled in **Settings › MCP** and your token's permissions. Page-only, plugin-only, binary non-image and token-revocation calls are never exposed over MCP.

See [Connect an MCP client](../../api/connect/mcp.md).

## Plugins

- **Plugin does not appear**: Make sure you selected the plugin's own folder containing `plugin.json` directly (not its parent folder), then click **Reload** in **Settings › Plugins**. Verify `plugin.json` is valid JSON with a unique `id`, `name` and `api: 1`.
- **Plugin shows an Issue badge**: Expand the plugin row in **Settings › Plugins** to read the exact problem. Common causes include a missing dependency, an unsupported API version, an invalid manifest or pending approval for a library-local plugin.
- **Disabled after repeated failures**: Curio turns off a plugin automatically if its program or extractor crashes repeatedly. Check `curio.log`, fix the script or tool path under **Settings › Plugins › Tools**, reload and re-enable the plugin.
- **Refused API call inside a plugin**: Confirm every required permission is declared in `plugin.json` and checked in **Settings › Plugins** for the active library. Plugins can never hold `library.manage`.

## Logs

Curio writes diagnostic logs (including Auto-Import activity, scanner and reader errors, API router events and plugin logs) to `curio.log`:

| Platform | Log path |
|---|---|
| **Windows (installed)** | `%APPDATA%\Curio\curio.log` |
| **Linux** | `~/.config/Curio/curio.log` (or `$XDG_CONFIG_HOME/Curio/curio.log`) |
| **macOS** | `~/Library/Application Support/Curio/curio.log` |
| **Portable mode** | `<Curio folder>\Curio data\curio.log` |

When reporting an issue, include:

- Curio version (from **Settings › About**) and OS version (Windows 10/11, Linux distribution and Wayland/X11, or macOS).
- Whether the library uses managed storage or indexed Locations, and whether files are on an internal SSD, external drive or NAS.
- Relevant lines from `curio.log` around the error timestamp with personal file paths and API tokens redacted.
- Minimal steps to reproduce the issue, plus any request JSON or `plugin.json` snippet for API or plugin problems.
