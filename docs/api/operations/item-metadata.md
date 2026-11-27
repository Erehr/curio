# item (Metadata)

Operations for updating item metadata, renaming multiple items and managing tag and folder memberships. All operations target a **library** and require `metadata.write`. Writes addressed to a grouped member ID automatically apply to its parent item.

## item.update {#item-update}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/update</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_update</span>
</div>

Update display name, notes, source URL, star rating, favorite/explicit flags or custom field values on one or more items. Omitted fields remain unchanged.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | One of `id` / `ids` | — | Single item UUID or row ID. |
| `ids` | `(string \| number)[]` | One of `id` / `ids` | — | Multiple item IDs all receiving the same updates. |
| `name` | `string` | No | — | New display name (also renames managed files and `organize` location files on disk). |
| `annotation` | `string` | No | — | New notes text. |
| `url` | `string` | No | — | New source URL (`item.update` uses `url`, matching `Item.url`, while `item.add.*` uses `sourceUrl`). |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Set or clear the Favorite heart flag. |
| `explicit` | `boolean` | No | — | Set or clear the Explicit blur flag. |
| `fields` | `Record<string, unknown>` | No | — | Map of custom field keys to values (`null` clears a field; writing to a file-reading field marks it as a manual override; writing to an unindexed field is refused). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/update \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012","rating":5,"favorite":true,"fields":{"artist":"M. Chen","draft":null}}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.update({
      id: item.id,
      rating: 5,
      favorite: true,
      fields: { artist: 'M. Chen', draft: null },
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_update","arguments":{"id":"0194b7a2-...","rating":5,"favorite":true}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "metadata"`). Returns `400 bad_request` if neither `id` nor `ids` is provided or if a field fails validation/is unindexed; `404 not_found` if any item does not exist.

---

## item.rename {#item-rename}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/rename</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_rename</span>
</div>

Assign distinct names to multiple items in a single atomic transaction (used by Batch Rename).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `items` | `{ id: string \| number; name: string }[]` | Yes | — | List of `{ id, name }` rename entries. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/rename \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"items":[{"id":"0194b7a2-...","name":"hero_001"},{"id":"0194b7a3-...","name":"hero_002"}]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.rename({
      items: [
        { id: idA, name: 'hero_001' },
        { id: idB, name: 'hero_002' },
      ],
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_rename","arguments":{"items":[{"id":"0194b7a2-...","name":"hero_001"}]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "metadata"`).

---

## item.addTags {#item-addtags}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/addTags</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_addTags</span>
</div>

Attach one or more tags to items, creating any tag names that do not exist yet.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `tags` | `string[]` | Yes | — | Tag names to add (case-insensitive deduplication). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/addTags \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"tags":["lighting","approved"]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.addTags({ ids: [item.id], tags: ['lighting', 'approved'] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_addTags","arguments":{"ids":["0194b7a2-..."],"tags":["lighting","approved"]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`tags.changed`](../events/items.md#tags-changed).

---

## item.removeTags {#item-removetags}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/removeTags</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_removeTags</span>
</div>

Remove specific tags, or all tags, from items.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `tags` | `(string \| number)[]` | Unless `all: true` | — | Tag names or numeric IDs to remove. |
| `all` | `boolean` | No | `false` | Remove every tag from the given items. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/removeTags \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"tags":["draft"]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.removeTags({ ids: [item.id], tags: ['draft'] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_removeTags","arguments":{"ids":["0194b7a2-..."],"all":true}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`tags.changed`](../events/items.md#tags-changed).

---

## item.addToFolders {#item-addtofolders}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/addToFolders</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_addToFolders</span>
</div>

Add items to one or more folders without removing existing folder memberships (applies target folders' auto-tags and creates managed `files/` hard links).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `folders` | `number[]` | Yes | — | Destination folder IDs. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/addToFolders \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"folders":[3,7]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.addToFolders({ ids: [item.id], folders: [3, 7] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_addToFolders","arguments":{"ids":["0194b7a2-..."],"folders":[3,7]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`folders.changed`](../events/items.md#folders-changed). Returns `404 not_found` if any folder ID does not exist.

---

## item.removeFromFolders {#item-removefromfolders}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/removeFromFolders</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_removeFromFolders</span>
</div>

Remove items from the specified folders while keeping the items in the library (and in any other folders they belong to).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `folders` | `number[]` | Yes | — | Folder IDs to remove items from. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/removeFromFolders \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"folders":[7]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.removeFromFolders({ ids: [item.id], folders: [7] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"item_removeFromFolders","arguments":{"ids":["0194b7a2-..."],"folders":[7]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`folders.changed`](../events/items.md#folders-changed).

---

## item.moveToFolder {#item-movetofolder}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/moveToFolder</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_moveToFolder</span>
</div>

Place items into `folder` while removing them from `from` (including its subfolders), or from every other folder when `from` is omitted.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `folder` | `number` | Yes | — | Destination folder ID. |
| `from` | `number` | No | *(all other folders)* | Source folder ID to leave (with its subfolders). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/moveToFolder \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"folder":12,"from":3}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.moveToFolder({ ids: [item.id], folder: 12, from: 3 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"item_moveToFolder","arguments":{"ids":["0194b7a2-..."],"folder":12,"from":3}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`folders.changed`](../events/items.md#folders-changed).
