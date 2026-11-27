# selection

Operations for reading and changing the active item selection in an open Curio window. Both operations target a **window** (accepting optional `window` or `library`) and require the `ui` permission.

## selection.get {#selection-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/selection/get</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: selection_get</span>
</div>

Return the full `Item` objects currently selected in the target window, in selection order.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Item[]`](../types/item.md#item).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/selection/get \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const selected = await curio.selection.get()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"selection_get","arguments":{}}}
    ```

**Example response**

```json
[
  {
    "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
    "name": "Dawn clouds",
    "fileName": "dawn_clouds.png",
    "ext": "png",
    "kind": "image",
    "size": 2457600,
    "width": 2560,
    "height": 1440,
    "duration": null,
    "rating": 5,
    "favorite": true,
    "explicit": false,
    "annotation": "",
    "url": "",
    "tags": ["sky"],
    "folders": [{ "id": 3, "name": "References" }],
    "status": "Available",
    "imported": 1763900000000,
    "modified": 1763890000000,
    "parts": 0
  }
]
```

**Events and errors** Read-only. Returns `403 not_allowed` without `ui` (or if a plugin passes `window` or `library`), `404 not_found` for an unknown `window`, `404 unknown_library` or `409 no_window` if the target library has no open desktop window. Follow changes live with [`selection.changed`](../events/windows.md#selection-changed).

---

## selection.set {#selection-set}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/selection/set</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: selection_set</span>
</div>

Replace the active selection in the target window. Items not present in the window's current grid view are skipped and counted in `notShown` (unless the window is showing a Tab plugin page, which can select any item in the library). Pass `ids: []` to clear selection.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to select. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ selected: number; notShown: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/selection/set \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-...","0194b7a3-..."]}'
    ```

=== "JavaScript"

    ```javascript
    const { selected, notShown } = await curio.selection.set({ ids: [idA, idB] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"selection_set","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "selected": 2, "notShown": 0 }
```

**Events and errors** Emits [`selection.changed`](../events/windows.md#selection-changed) and updates any selection-aware plugin widgets via [`plugin.selection`](../../plugins/hooks/interaction.md#plugin-selection). Returns `403 not_allowed` without `ui`, `404 not_found` if any item ID or `window` does not exist, `404 unknown_library`, `409 no_window` if no window is open or `503 busy` if the window does not answer within 15 seconds.
