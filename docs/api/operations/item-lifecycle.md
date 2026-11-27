# item (Lifecycle)

Operations for duplicating items, transferring items across libraries, exporting files to disk and moving items through Trash, Restore and Purge.

## item.duplicate {#item-duplicate}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/duplicate</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_duplicate</span>
</div>

Create independent duplicate items with their own copied file bytes and identical tags, folders, ratings, notes, source URLs and custom fields.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to duplicate. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; ids: string[] }` — UUIDs of the new duplicate items.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/duplicate \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    const { ids } = await curio.item.duplicate({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_duplicate","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7d0-1111-7222-8333-444455556666"] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added) (`how: "duplicate"`).

---

## item.copyToLibrary {#item-copytolibrary}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/copyToLibrary</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_copyToLibrary</span>
</div>

Copy items (and their grouped companions, tags, ratings, notes and matching custom fields) into another open or closed library on this computer. The caller must also be permitted to access the destination library.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Source item IDs. |
| `to` | `string` | Yes | — | Destination library ID from [`library.list`](libraries.md#library-list), folder path or name. |
| `library` | `string` | No | Default library | Source library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; count: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/copyToLibrary \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"to":"Archive"}'
    ```

=== "JavaScript"

    ```javascript
    const { count } = await curio.item.copyToLibrary({ ids: [item.id], to: 'Archive' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_copyToLibrary","arguments":{"ids":["0194b7a2-..."],"to":"Archive"}}}
    ```

**Example response**

```json
{ "success": true, "count": 1 }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added) on the destination library. Returns `404 unknown_library` or `409 library_offline` if destination cannot be reached; `403 not_allowed` if token cannot reach `to`.

---

## item.moveToLibrary {#item-movetolibrary}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/moveToLibrary</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_moveToLibrary</span>
</div>

Copy items to another library (identically to `item.copyToLibrary`) and then move the source items into the source library's Trash.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Source item IDs. |
| `to` | `string` | Yes | — | Destination library ID from [`library.list`](libraries.md#library-list), folder path or name. |
| `library` | `string` | No | Default library | Source library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; count: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/moveToLibrary \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"to":"Archive"}'
    ```

=== "JavaScript"

    ```javascript
    const { count } = await curio.item.moveToLibrary({ ids: [item.id], to: 'Archive' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_moveToLibrary","arguments":{"ids":["0194b7a2-..."],"to":"Archive"}}}
    ```

**Example response**

```json
{ "success": true, "count": 1 }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added) on destination and [`items.removed`](../events/items.md#items-removed) (`purged: false`) on source.

---

## item.export {#item-export}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/export</span>
  <span class="curio-badge">Permission: files.outside</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_export</span>
</div>

Copy items' files into a folder on disk (creating the folder if missing and appending numeric suffixes rather than overwriting existing files).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to export. |
| `folder` | `string` | Yes | — | Destination directory path on disk. |
| `members` | `boolean` | No | `true` | Also export companion/grouped member files. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; count: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/export \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"folder":"C:\\Exports\\Selected","members":true}'
    ```

=== "JavaScript"

    ```javascript
    const { count } = await curio.item.export({ ids: [item.id], folder: 'C:\\Exports\\Selected' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_export","arguments":{"ids":["0194b7a2-..."],"folder":"C:\\Exports\\Selected"}}}
    ```

**Example response**

```json
{ "success": true, "count": 1 }
```

**Events and errors** Read-only in the database; writes only to `folder`. Returns `403 not_allowed` without `files.outside`.

---

## item.trash {#item-trash}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/trash</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_trash</span>
</div>

Move items to the library Trash. Original files remain intact until [`item.purge`](#item-purge) empties the Trash.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to trash. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/trash \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.trash({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_trash","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.removed`](../events/items.md#items-removed) (`purged: false`).

---

## item.restore {#item-restore}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/restore</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_restore</span>
</div>

Restore trashed items back to the library and their previous folders.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Trashed item IDs to restore. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/restore \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.restore({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"item_restore","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added) (`how: "restore"`).

---

## item.purge {#item-purge}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/purge</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_purge</span>
</div>

Permanently remove trashed items from the library (or empty the entire Trash). Managed copies owned by Curio are sent to the system trash / Recycle Bin; linked location originals on disk are never deleted.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Unless `all: true` | — | Specific trashed item IDs to purge. |
| `all` | `boolean` | No | `false` | Empty the entire Trash. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/purge \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.purge({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"item_purge","arguments":{"all":true}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.removed`](../events/items.md#items-removed) (`purged: true`).
