# Surfaces

Plugins can open standalone desktop windows, mount workspace tabs beside the main grid, embed custom Settings panels inside **Settings › Plugins** and add declarative query panels to the Library sidebar.

See [Window, tab and panel declarations](../manifest/surfaces.md) and [plugin.json `settings`](../manifest/index.md#settings) for the full key schemas.

## Windows

Declare standalone windows under `windows[]`:

```json
{
  "windows": [
    {
      "id": "convert",
      "title": "Convert pictures",
      "page": "convert.html",
      "icon": "transform",
      "width": 640,
      "height": 520,
      "multiple": false
    }
  ]
}
```

- **Single instance (`multiple: false`)**: Opening the window again focuses the existing window and sends a fresh [`plugin.run`](../hooks/lifecycle.md#plugin-run) event with the new items or URLs (default).
- **Multiple instances (`multiple: true`)**: Each launch opens an independent window.
- **Discovery**: Every declared window appears in the **Plugins** toolbar menu and in **Edit layout** as a placeable toolbar button.

## Tabs

Declare workspace tabs under `tabs[]`:

```json
{
  "tabs": [
    {
      "id": "stats",
      "title": "Library stats",
      "page": "stats.html",
      "icon": "chart-bar"
    }
  ]
}
```

- A tab plugin mounts in the center pane beside the item grid and **Tag Manager** (`view: "tab:<plugin>/<tab>"`).
- Switching away from the tab keeps the iframe loaded in memory and notifies it via [`plugin.hide`](../hooks/lifecycle.md#plugin-hide) and [`plugin.show`](../hooks/lifecycle.md#plugin-show).
- While a tab is active, calling [`selection.set`](../../api/operations/selection.md#selection-set) selects any item in the library so Curio's Inspector displays that item's details beside your tab UI.

## Launching

Windows and tabs launch in three ways:

1. From the **Plugins** menu or a toolbar button placed in **Edit layout** (receives `{ window: "<id>", items: [selectedIds] }` or `{ tab: "<id>" }`).
2. From an [`actions[]`](actions.md) entry using `"opens": "<windowId>"` or `"opens": "tab:<tabId>"` (receives `{ action: "<actionId>", items: [ids] }`).
3. From a [`urls[]`](urls.md) handler using `"opens": "<windowId>"` or `"opens": "tab:<tabId>"` (receives `{ handler: "<handlerId>", urls: [...], place: { ... } }`).

## Settings

Set top-level `"settings": "settings.html"` in `plugin.json`:

- Curio embeds `settings.html` inside the plugin's expanded card in **Settings › Plugins** and resizes its iframe height to fit its content.
- Read and write persistent computer-wide preferences with [`plugin.settings.get`](../../api/operations/plugin.md#plugin-settings-get) and [`plugin.settings.set`](../../api/operations/plugin.md#plugin-settings-set) without extra permissions. Other open pages of your plugin receive [`plugin.settingsChanged`](../hooks/lifecycle.md#plugin-settingschanged).

## Context & close

- **Launch context**: Listen to [`plugin.run`](../hooks/lifecycle.md#plugin-run). Because it is sticky, registering a listener after page load still receives the launch payload, and re-triggering an action while the window or tab is open delivers the new payload without reloading the page.
- **Closing a window or popover**: Call [`plugin.close()`](../../api/operations/plugin.md#plugin-close) to close a standalone plugin window, or `plugin.close({ result })` inside a [`ui.popover`](../../api/operations/ui.md#ui-popover) page to resolve the caller's promise with a structured result.

## Panels

For sidebar query shortcuts without HTML or JavaScript, declare [`panels[]`](search.md#panels) with a `title`, optional `text` and `filters[]` array. Curio renders the panel natively inside the Library sidebar.
