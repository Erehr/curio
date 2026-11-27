# Actions & URLs

Keys inside `plugin.json`'s `actions[]` and `urls[]` arrays. See [Actions](../contributions/actions.md) and [URL handlers](../contributions/urls.md).

## actions[].id {#actions-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique action identifier within the plugin. Combined with the plugin ID for shortcut storage and user Action steps.

## actions[].title {#actions-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Display label shown in the item context menu, the **Plugins** menu, the viewer, **Settings › Shortcuts** and the Actions builder.

## actions[].icon {#actions-icon}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Tabler icon name rendered beside the action title.

## actions[].where {#actions-where}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: ["item-menu"]</span></div>
Places where the action appears:

| Value | Where it appears |
|---|---|
| `"item-menu"` | Right-click context menu on selected items (default when omitted). |
| `"toolbar"` | Main toolbar **Plugins** menu (under the plugin's menu). |
| `"viewer"` | Icon button beside an opened item in the viewer toolbar. |
| `"step"` | Reusable step inside user-created [Actions](../../features/organize/actions.md). Supported only for program actions (`run`), not page actions (`opens`). |

## actions[].key {#actions-key}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null (unbound)</span></div>
Default keyboard shortcut string (for example `"Ctrl+Shift+R"`). If it collides with a Curio shortcut, it starts unassigned. Users can rebind or clear it in **Settings › Shortcuts**.

## actions[].selection {#actions-selection}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: "some"</span></div>
Selection count requirement before the action is enabled:

| Value | Enabled when |
|---|---|
| `"one"` | Exactly 1 item is selected. |
| `"many"` | 2 or more items are selected. |
| `"some"` | 1 or more items are selected (default). |
| `"any"` | Always enabled, including with zero items selected. |

## actions[].kinds {#actions-kinds}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all kinds)</span></div>
Item kinds every selected item must belong to (for example `["image"]` or `["video", "audio"]`). Empty allows all kinds.

## actions[].exts {#actions-exts}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all extensions)</span></div>
Lowercase file extensions (without leading dots) every selected item must have. Empty allows all extensions.

## actions[].opens {#actions-opens}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Opens one of the plugin's UI surfaces with the selected items delivered via [`plugin.run`](../hooks/lifecycle.md#plugin-run): pass a window `id` from `windows[]` or `"tab:<id>"` for a tab from `tabs[]`. Mutually exclusive with `run`.

## actions[].run {#actions-run}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Argv array of a program to run on the selected items (requires the `exec` permission). Relative executable paths resolve against the plugin folder. Curio passes the selection payload as JSON on `stdin` and provides a scoped run token via environment variables; see [Action and URL handler programs](../programs/action-url.md). Mutually exclusive with `opens`.

## urls[].id {#urls-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique URL handler identifier within the plugin.

## urls[].title {#urls-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Human-readable description of the URL handler shown in **Settings › Plugins** and job progress notices.

## urls[].patterns {#urls-patterns}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge">Required</span></div>
Full URL wildcard patterns matched case-insensitively when URLs are dropped, pasted or sent via [`item.add.fromUrls`](../../api/operations/item-add.md#item-add-fromurls). Use `*` to match any run of characters (for example `"https://www.youtube.com/watch*"` or `"https://vimeo.com/*"`).

## urls[].opens {#urls-opens}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Window ID or `"tab:<id>"` opened with the matched URLs delivered in [`plugin.run`](../hooks/lifecycle.md#plugin-run). Mutually exclusive with `run`.

## urls[].run {#urls-run}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Argv array of a program executed with the matched URLs on `stdin` (requires the `exec` permission). Relative executable paths resolve against the plugin folder. See [Action and URL handler programs](../programs/action-url.md). Mutually exclusive with `opens`.

## Example

```json
{
  "actions": [
    {
      "id": "upscale",
      "title": "Upscale image",
      "icon": "arrows-maximize",
      "where": ["item-menu", "viewer", "step"],
      "key": "Ctrl+Shift+U",
      "selection": "some",
      "kinds": ["image"],
      "exts": ["png", "jpg", "jpeg", "webp"],
      "run": ["python", "upscale.py"]
    },
    {
      "id": "open-sheet",
      "title": "Contact sheet",
      "icon": "layout-grid",
      "where": ["item-menu", "toolbar"],
      "selection": "many",
      "kinds": ["image"],
      "opens": "sheet"
    }
  ],
  "urls": [
    {
      "id": "youtube",
      "title": "Download YouTube video",
      "patterns": ["https://www.youtube.com/watch*", "https://youtu.be/*"],
      "run": ["python", "download_yt.py"]
    }
  ]
}
```
