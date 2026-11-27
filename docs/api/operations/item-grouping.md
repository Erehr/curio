# item (Grouping)

Operations for manually grouping multiple files into one item, detaching or splitting grouped items, forgetting missing member files and re-enabling automatic grouping rules on manually split items. For library-wide rule inspection and batch execution, see [Library grouping rules](library-grouping.md).

## item.group {#item-group}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/group</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_group</span>
</div>

Collapse `members` into the primary item `id`. The primary item absorbs members' tags, folders and notes while preserving each member's original metadata for restoration if split later.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Primary item UUID or row ID. |
| `members` | `(string \| number)[]` | Yes | — | Item IDs to attach as grouped files of `id`. |
| `role` | `string` | No | `"part"` | Companion role label (letters, digits, `-` or `_`, max 40 characters; e.g. `preview`, `metadata` or `part`). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/group \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-...","members":["0194b7a3-..."],"role":"part"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.group({ id: primaryId, members: [altId], role: 'part' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_group","arguments":{"id":"0194b7a2-...","members":["0194b7a3-..."]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) for the primary item and [`items.removed`](../events/items.md#items-removed) (`purged: false`) for the grouped members. Returns `400 bad_request` for invalid roles, `404 not_found` if any ID is missing or `422 failed` if the files cannot be grouped (such as self-grouping or items in different locations).

---

## item.split {#item-split}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/split</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_split</span>
</div>

Split grouped items back into independent items, restoring each member's prior metadata and marking them `solo` so automatic companion rules do not immediately regroup them. To release a single member file while leaving the rest of the group intact, use [`item.detach`](#item-detach).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Grouped item IDs to split. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ ids: string[] }` — UUIDs of the newly independent member items.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/split \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    const { ids } = await curio.item.split({ ids: [primaryId] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_split","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "ids": ["0194b7a3-..."] }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) and [`items.added`](../events/items.md#items-added) (`how: "split"`).

---

## item.detach {#item-detach}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/detach</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_detach</span>
</div>

Detach a single member file from its parent grouped item, turning that member back into an independent item while leaving other members attached. Like [`item.split`](#item-split), the detached item is marked `solo` so automatic rules do not immediately regroup it (clear with [`item.allowGrouping`](#item-allowgrouping)); to split all members at once, use [`item.split`](#item-split).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Member item UUID (from `ItemWithFields.parts[].id`). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/detach \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a3-..."}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.detach({ id: memberId })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_detach","arguments":{"id":"0194b7a3-..."}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) for the former parent item and [`items.added`](../events/items.md#items-added) (`how: "split"`) for the detached item. Returns `404 not_found` if `id` does not exist or `422 failed` if it is not a grouped member.

---

## item.forgetFile {#item-forgetfile}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/forgetFile</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_forgetFile</span>
</div>

Remove a missing or offline member file from its parent grouped item and from the library database without affecting the primary item; future rescans will not re-add it unless the file reappears changed.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Missing/offline member UUID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/forgetFile \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a3-..."}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.forgetFile({ id: missingMemberId })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_forgetFile","arguments":{"id":"0194b7a3-..."}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed). Returns `400 bad_request` if the member file is currently available on disk or is not a grouped member.

---

## item.allowGrouping {#item-allowgrouping}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/allowGrouping</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_allowGrouping</span>
</div>

Clear the manual `solo` exclusion flag on previously split items so automatic companion rules (`library.grouping.apply` and future scans) may group them again.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to re-allow for automatic grouping. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/allowGrouping \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-...","0194b7a3-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.allowGrouping({ ids: [idA, idB] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_allowGrouping","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed).
