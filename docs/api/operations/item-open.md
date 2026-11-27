# item (Open)

Operations for opening an item inside Curio or its default system application, and revealing its file in the OS file manager.

## item.open {#item-open}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/open</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_open</span>
</div>

Open an item in Curio's viewer, in a standalone item window or in the operating system's default application for its file type. For security, executables and scripts can only be launched from Curio's own window (not remotely via API), and saved `.url` links only open when pointing to `http://` or `https://` web pages. To target a specific window's viewer directly or focus a manually grouped member file, use [`viewer.open`](viewer.md#viewer-open).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `how` | `string` | No | `"viewer"` | `viewer` (in the library window's viewer), `window` (new standalone item window) or `app` (OS default app). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/open \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-...","how":"window"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.open({ id: item.id, how: 'window' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_open","arguments":{"id":"0194b7a2-...","how":"viewer"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`viewer.opened`](../events/windows.md#viewer-opened) when `how === "viewer"`. Returns `409 no_window` for `viewer` if no window has the library open; `404 not_found` if missing/offline.

---

## item.reveal {#item-reveal}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/reveal</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_reveal</span>
</div>

Open the operating system's file manager (Windows Explorer, Dolphin, Nautilus or Finder) with the item's file selected.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/reveal \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-..."}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.reveal({ id: item.id })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_reveal","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits no events. Returns `404 not_found` if the item or its file on disk is missing/offline.
