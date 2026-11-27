# First plugin

Scaffold, load and run a Curio plugin with the built-in starter generator or by creating a folder with `plugin.json`.

## Scaffold

1. Open **Settings › Plugins** (++ctrl+comma++ › **Plugins**).
2. Click **New example plugin** (the `+` document icon in the top-right bar).
3. Choose one of the six starter templates (**Window**, **Tab**, **Widget**, **Format viewer**, **Program** or **URL handler**), pick a parent folder on disk and create it. Curio writes a working plugin folder, registers it and enables it for the current library.

## Structure

A plugin is an ordinary directory containing `plugin.json` at its root, plus any HTML, CSS, JavaScript, localization (`locales/<lang>.json`) or executable files it references:

```text
my-plugin/
├── plugin.json          # Required manifest declaring id, name, api, permissions and contributions
├── index.html           # Optional UI page
├── app.js               # External script (inline <script> is blocked by CSP)
└── locales/
    └── en.json          # Optional translated manifest strings
```

Minimal working plugin:

`plugin.json`:

```json
{
  "id": "com.example.hello",
  "name": "Hello Curio",
  "version": "1.0.0",
  "api": 1,
  "description": "Lists the five newest items in the library.",
  "author": "You",
  "icon": "sparkles",
  "permissions": ["metadata.read"],
  "windows": [
    { "id": "main", "title": "Hello Curio", "page": "index.html", "width": 520, "height": 420 }
  ]
}
```

`index.html`:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="/curio/ui/curio.css">
  <script src="/curio/ui/curio.js"></script>
  <script type="module" src="app.js"></script>
</head>
<body><ul id="list"></ul></body>
</html>
```

`app.js`:

```javascript
const curio = Curio.connect()
const { items } = await curio.item.query({ sort: 'imported', limit: 5 })
document.getElementById('list').replaceChildren(
  ...items.map((item) => Object.assign(document.createElement('li'), { textContent: item.name })),
)
```

## Load & enable

1. In **Settings › Plugins**, click **Add plugin** and select your plugin directory (the folder containing `plugin.json`).
2. Curio validates `plugin.json`, registers the folder path, enables the plugin for the current library and grants its declared `permissions`.
3. Alternatively, place the plugin folder inside Curio's app plugins folder (`%APPDATA%\Curio\plugins` on Windows, `~/.local/share/Curio/plugins` on Linux, `~/Library/Application Support/Curio/plugins` on macOS) or inside `<Library>.curio/plugins/`, then click **Reload** and toggle it on.

## Run

- **Launch a window or tab**: Open the **Plugins** menu in the toolbar, click the window's open button inside its expanded row in **Settings › Plugins** or select a tab from the Library sidebar.
- **Place a widget**: Open **Edit layout** and drag your widget from the palette into the Inspector or Library sidebar, or set `default_visible: true` so it appears when matching items are selected.

## Permissions

Expand your plugin's row in **Settings › Plugins** to review or toggle individual permissions (`metadata.read`, `metadata.write`, `organize`, `files.read`, `files.write`, `files.outside`, `ui`, `exec`, `network`). See [Permissions](permissions.md) for scope rules and library-local approval.

## Reload & debug

- **Reloading**: After editing `plugin.json`, HTML, CSS or scripts, click **Reload** in **Settings › Plugins** to load manifest and contribution changes. Reopening a page also reloads its HTML and JavaScript.
- **No inline scripts**: Content Security Policy blocks inline `<script>` tags and `eval`. Load all page code from external `.js` files referenced by relative path.
- **Auto-disable on repeated failures**: Five consecutive failures in a program, extractor page or thumbnail page disable the plugin until you fix the cause and re-enable it in **Settings › Plugins**.
- **Logging**: Write diagnostic messages from pages or programs with [`plugin.log`](../../api/operations/plugin.md#plugin-log). Messages appear in `curio.log` tagged with your plugin ID (`%APPDATA%\Curio\curio.log` on Windows, `~/.config/Curio/curio.log` on Linux, `~/Library/Application Support/Curio/curio.log` on macOS). Child process `stderr` is written to the same log.
- **Safe testing**: Develop against a disposable test library or portable mode (`--portable`) before enabling write or `exec` plugins on your main library.

## Next steps

- [Page runtime](../pages/runtime.md): Frame roles, origins, CSP and `Curio.connect()`.
- [Permissions](permissions.md): Permission scopes, `exec` trust boundaries and library-local approval.
- [`plugin.json` reference](../manifest/index.md): Top-level manifest keys and contribution schemas.
- [Starter templates](../examples/starters.md): Overview of all six templates in **Settings › Plugins › New example plugin**.
