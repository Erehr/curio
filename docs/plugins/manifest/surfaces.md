# Surfaces

Keys inside `plugin.json`'s `windows[]`, `tabs[]` and `panels[]` arrays. See [Windows, tabs and sidebar panels](../contributions/surfaces.md).

## windows[].id {#windows-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique window identifier within the plugin. Referenced by `actions[].opens` and `urls[].opens`. In **Edit layout**, a window launcher button uses the widget ID `win:<plugin>/<id>`.

## windows[].title {#windows-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Title shown in the native window titlebar, the **Plugins** menu and **Edit layout**.

## windows[].page {#windows-page}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Relative path to the HTML entry file inside the plugin folder (for example `"ui/window.html"`), served from the plugin's origin (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows).

## windows[].icon {#windows-icon}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Tabler icon name shown in the **Plugins** menu and when the window launcher is placed in a toolbar or sidebar (defaults to `"app-window"` when empty).

## windows[].width {#windows-width}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null (900)</span></div>
Initial window width in logical pixels when opened (defaults to `900`, clamped to `320`–`4000`).

## windows[].height {#windows-height}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null (640)</span></div>
Initial window height in logical pixels when opened (defaults to `640`, clamped to `240`–`3000`).

## windows[].multiple {#windows-multiple}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: false</span></div>
When `true`, opening this window creates a new instance every time. When `false`, opening it again focuses the existing open window (and delivers a fresh [`plugin.run`](../hooks/lifecycle.md#plugin-run) message if triggered by an action or URL handler).

## tabs[].id {#tabs-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique tab identifier within the plugin. Referenced by `actions[].opens` and `urls[].opens` as `"tab:<id>"`, and in **Edit layout** as `tab:<plugin>/<id>`.

## tabs[].title {#tabs-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Display label for the tab in the Library sidebar navigation and the toolbar title.

## tabs[].page {#tabs-page}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Relative path to the HTML entry file inside the plugin folder, served from the plugin's origin (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows) and mounted in the main center pane beside the item grid and Tag Manager.

## tabs[].icon {#tabs-icon}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Tabler icon name shown beside the tab title in the Library sidebar.

## panels[].id {#panels-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique identifier for a lightweight declarative sidebar panel.

## panels[].title {#panels-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Section heading shown inside the Library sidebar's **Plugin panels** widget.

## panels[].filters {#panels-filters}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
List of clickable query presets rendered inside the sidebar panel. Clicking a preset applies its `query` to the active view.

## panels[].text {#panels-text}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Optional short informational note displayed inside the sidebar panel.

## panels[].filters[].id {#panels-filters-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique identifier for the panel filter preset within the panel.

## panels[].filters[].label {#panels-filters-label}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Clickable label displayed inside the sidebar panel.

## panels[].filters[].query {#panels-filters-query}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Curio [query language](../../api/query/syntax.md) expression executed when the user selects this preset.

## Example

```json
{
  "windows": [
    {
      "id": "batch",
      "title": "Batch Renamer",
      "page": "ui/batch.html",
      "icon": "forms",
      "width": 720,
      "height": 540,
      "multiple": false
    }
  ],
  "tabs": [
    {
      "id": "board",
      "title": "Moodboard",
      "page": "ui/board.html",
      "icon": "layout-collage"
    }
  ],
  "panels": [
    {
      "id": "reading",
      "title": "Reading List",
      "text": "Quick shelves for books and comics.",
      "filters": [
        { "id": "reading-now", "label": "Currently reading", "query": "read_state = reading" },
        { "id": "unread", "label": "Unread", "query": "kind = document AND NOT read_state = finished" }
      ]
    }
  ]
}
```
