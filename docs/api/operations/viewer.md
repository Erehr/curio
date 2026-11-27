# viewer

Operations for opening, closing and inspecting Curio's built-in item viewer, plus the page-only operation viewer plugins use to populate Curio's native toolbar.

## viewer.setTools {#viewer-settools}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App (host window)</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin viewer pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Publish the buttons, dropdown panels, progress slider, text labels and active `viewer_keys` rendered in Curio's native viewer toolbar while a plugin viewer page is open. Each control `id` maps to a matching `viewer_tools[]` declaration in `plugin.json`. Clicks and value changes arrive back at the viewer page via [`plugin.tool`](../../plugins/hooks/interaction.md#plugin-tool). Call with `{}` to clear all viewer tools.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `buttons` | `{ id: string; label: string; icon?: string; on?: boolean; panel?: unknown[] }[]` | No | `[]` | Up to 12 toolbar buttons, each with an optional native popover `panel`. |
| `slider` | `{ id: string; label?: string; min: number; max: number; step?: number; value: number; dir?: 'ltr' \| 'rtl' }` | No | `null` | Native toolbar progress slider (supports `dir: 'rtl'` for right-to-left comics/manga). |
| `labels` | `{ id: string; text: string }[]` | No | `[]` | Up to 12 text readouts matching `viewer_tools` with `kind: "text"` (e.g. chapter title or page counter). |
| `keys` | `string[]` | No | All declared `viewer_keys` | Up to 64 active `viewer_keys` IDs currently enabled. |

A button `panel` accepts either a flat list of controls or up to 6 tabs (`{ tab, icon?, controls }`). Supported control types are `section`, `separator`, `slider`, `toggle`, `choice`, `select` and `swatches`.

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.viewer.setTools({
      labels: [{ id: 'chapter', text: 'Chapter 3 · Page 14 / 48' }],
      slider: { id: 'place', min: 1, max: 48, step: 1, value: 14, dir: 'ltr' },
      buttons: [
        { id: 'double', label: 'Double page', icon: 'book', on: true },
        {
          id: 'look',
          label: 'Appearance',
          icon: 'adjustments-horizontal',
          panel: [
            { type: 'section', label: 'Display' },
            { type: 'slider', id: 'zoom', label: 'Zoom', min: 50, max: 200, step: 10, value: 100, unit: '%' },
            { type: 'toggle', id: 'rtl', label: 'Right to left', value: false },
          ],
        },
      ],
      keys: ['double', 'rtl', 'bookmark'],
    })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Returns `403 not_allowed` if called from a non-viewer frame role (such as a widget or tab).

---

## viewer.open {#viewer-open}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/viewer/open</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: viewer_open</span>
</div>

Open an item in the target window's viewer. If the item is not currently shown in the window's active view, the window switches to **All** first. Passing a manually grouped member ID opens its parent item focused directly on that member file. To open an item in a standalone window or its default system application, use [`item.open`](item-open.md#item-open).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item (or grouped member) UUID or row ID. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/viewer/open \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-..."}'
    ```

=== "JavaScript"

    ```javascript
    await curio.viewer.open({ id: item.id })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"viewer_open","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`viewer.opened`](../events/windows.md#viewer-opened). Returns `403 not_allowed` without `ui`, `404 not_found` if the item or `window` does not exist (or the item is in Trash or hidden by a locked filter), `404 unknown_library`, `409 no_window` if no window is open, `422 failed` if the item's file is missing or its storage is offline; or `503 busy`.

---

## viewer.close {#viewer-close}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/viewer/close</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: viewer_close</span>
</div>

Close the target window's viewer back to the item grid.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ closed: boolean }` (`false` if the viewer was already closed).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/viewer/close \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const { closed } = await curio.viewer.close()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"viewer_close","arguments":{}}}
    ```

**Example response**

```json
{ "closed": true }
```

**Events and errors** Emits [`viewer.closed`](../events/windows.md#viewer-closed) when `closed === true`. Returns `403 not_allowed` without `ui`, `404 not_found` for an unknown `window`, `404 unknown_library`, `409 no_window` or `503 busy`.

---

## viewer.current {#viewer-current}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/viewer/current</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: viewer_current</span>
</div>

Return the item ID and specific member file ID currently displayed in the target window's viewer (`null` when the viewer is closed).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ id: string | null; file: string | null }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/viewer/current \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const { id, file } = await curio.viewer.current()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"viewer_current","arguments":{}}}
    ```

**Example response**

```json
{ "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012", "file": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012" }
```

**Events and errors** Read-only. Returns `403 not_allowed` without `ui`, `404 not_found` for an unknown `window`, `404 unknown_library`, `409 no_window` if no window is open or `503 busy`.
