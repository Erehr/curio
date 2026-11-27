# smartFolder

Smart folder operations read, create, edit, delete and evaluate saved [query language](../query/syntax.md) views. Smart folders form a hierarchy via `parentId`: a child smart folder's query narrows its ancestors' queries, while a parent smart folder with an empty query acts as a union of its subfolders' queries. Smart folders also support `groupBy` to generate live virtual subfolders grouped by a built-in property or custom field.

Every operation in this namespace targets a single library (see [Choose a library or window](../connect/routing.md)).

| Operation | HTTP Endpoint | Permission | MCP Tool | Summary |
|---|---|---|---|---|
| [`smartFolder.list`](#smartfolder-list) | `POST /api/v1/smartFolder/list` | `metadata.read` | `smartFolder_list` | List all smart folders with their queries and grouping settings. |
| [`smartFolder.get`](#smartfolder-get) | `POST /api/v1/smartFolder/get` | `metadata.read` | `smartFolder_get` | Fetch one smart folder with its live item count. |
| [`smartFolder.create`](#smartfolder-create) | `POST /api/v1/smartFolder/create` | `organize` | `smartFolder_create` | Create a smart folder from a query, optional parent and optional `groupBy`. |
| [`smartFolder.update`](#smartfolder-update) | `POST /api/v1/smartFolder/update` | `organize` | `smartFolder_update` | Update a smart folder's name, query, icon or `groupBy`. |
| [`smartFolder.delete`](#smartfolder-delete) | `POST /api/v1/smartFolder/delete` | `organize` | `smartFolder_delete` | Delete a smart folder and its child smart folders. |
| [`smartFolder.items`](#smartfolder-items) | `POST /api/v1/smartFolder/items` | `metadata.read` | `smartFolder_items` | Paginate the items matched by a smart folder. |

## smartFolder.list {#smartfolder-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_list</span>
</div>

Returns every smart folder in the library as a flat array ordered by `sortOrder` and case-insensitive `name`. For fast listing, `count` is always `null` on entries returned by `smartFolder.list`; call [`smartFolder.get`](#smartfolder-get) when you need the evaluated item count.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`SmartFolder[]`](../types/organizing.md#smartfolder) — flat array of smart folder objects (`count` is `null`; `groupBy` is `string | null`).

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const smartFolders = await curio.smartFolder.list();
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_list",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": 1,
    "name": "Top Rated Pictures",
    "query": "kind = image AND rating >= 4",
    "sort": null,
    "icon": "star",
    "groupBy": "year:added",
    "parentId": null,
    "sortOrder": 1,
    "count": null
  }
]
```

**Events and errors**

- **Events emitted**: None (read-only). Subscribe to [`smartFolders.changed`](../events/items.md#smartfolders-changed) to detect changes.
- **Errors**: `unauthorized`, `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline`.
- **Related**: [`smartFolder.get`](#smartfolder-get), [`smartFolder.create`](#smartfolder-create).

## smartFolder.get {#smartfolder-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/get</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_get</span>
</div>

Returns a single smart folder with its live evaluated `count` populated. Because Curio evaluates the smart folder's full ancestor chain (or subfolder union when its query is blank), `count` reflects the exact number of items [`smartFolder.items`](#smartfolder-items) returns.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Integer smart folder ID. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`SmartFolder`](../types/organizing.md#smartfolder) — smart folder record with `count` populated as a `number` (or `null` if the stored query failed to compile).

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/get \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 1}'
    ```

=== "JavaScript"

    ```js
    const smart = await curio.smartFolder.get({ id: 1 });
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_get",
      "arguments": {
        "id": 1
      }
    }
    ```

**Example response**

```json
{
  "id": 1,
  "name": "Top Rated Pictures",
  "query": "kind = image AND rating >= 4",
  "sort": null,
  "icon": "star",
  "groupBy": "year:added",
  "parentId": null,
  "sortOrder": 1,
  "count": 124
}
```

**Events and errors**

- **Events emitted**: None (read-only).
- **Errors**: `not_found` (`There is no smart folder <id>`), `bad_request` (missing or non-integer `id`), `not_allowed` (missing `metadata.read`).
- **Related**: [`smartFolder.list`](#smartfolder-list), [`smartFolder.items`](#smartfolder-items).

## smartFolder.create {#smartfolder-create}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/create</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_create</span>
</div>

Creates a new smart folder from a [query language](../query/syntax.md) expression. The `query` is compiled and validated against the library's field catalog before writing; an empty string `""` is valid and acts as a group container that unions its child smart folders (or matches nothing if it has no children).

When `parentId` is provided, the new smart folder is nested inside that parent and narrows the parent's query with logical `AND`. When `groupBy` is set, Curio displays up to 200 live virtual subfolders beneath the smart folder in the sidebar, grouped by the chosen property or custom field:

- Date buckets: `year:added`, `month:added`, `year:created`, `month:created`, `year:modified`, `month:modified`
- Built-in properties: `rating`, `kind`, `ext`, `tag`
- Custom fields: `field:<key>` (for example `field:genre` or `field:model`)

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Smart folder name; trimmed, must not be blank. |
| `query` | `string` | Yes | — | Query language expression (may be `""` for a grouping container); validated before saving. |
| `parentId` | `number` | No | `null` | Parent smart folder ID to narrow; omit or pass `null` for top level. |
| `groupBy` | `string` | No | `null` | Virtual subfolder grouping key (`year:added`, `month:added`, `year:created`, `month:created`, `year:modified`, `month:modified`, `rating`, `kind`, `ext`, `tag` or `field:<key>`). Blank strings are stored as `null`. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

`{ success: true; id: number }`

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Confirms the smart folder was created. |
| `id` | `number` | Integer ID of the newly created smart folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/create \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name": "Top Rated Pictures", "query": "kind = image AND rating >= 4", "groupBy": "year:added"}'
    ```

=== "JavaScript"

    ```js
    const { id } = await curio.smartFolder.create({
      name: 'Top Rated Pictures',
      query: 'kind = image AND rating >= 4',
      groupBy: 'year:added',
    });
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_create",
      "arguments": {
        "name": "Top Rated Pictures",
        "query": "kind = image AND rating >= 4",
        "groupBy": "year:added"
      }
    }
    ```

**Example response**

```json
{
  "success": true,
  "id": 4
}
```

**Events and errors**

- **Events emitted**: [`smartFolders.changed`](../events/items.md#smartfolders-changed).
- **Errors**: `bad_request` (invalid query syntax, unknown field in query or blank `name`), `not_allowed` (missing `organize`), `failed` (nonexistent `parentId` or database error).
- **Related**: [`query.validate`](query.md#query-validate), [`smartFolder.update`](#smartfolder-update).

## smartFolder.update {#smartfolder-update}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/update</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_update</span>
</div>

Updates a smart folder's `name`, `query`, `icon` or `groupBy`. Omitted properties keep their existing values. If `query` is provided, it is validated against the field catalog before writing. Passing an empty string `""` for `icon` or `groupBy` clears that property to `null`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Smart folder ID to update. |
| `name` | `string` | No | Unchanged | New name; trimmed, must not be blank when provided. |
| `query` | `string` | No | Unchanged | New query expression; validated before saving. |
| `icon` | `string` | No | Unchanged | Tabler icon name; pass `""` to clear. |
| `groupBy` | `string` | No | Unchanged | Virtual subfolder grouping key (see [`smartFolder.create`](#smartfolder-create)); pass `""` to clear. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/update \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 4, "query": "kind = image AND rating = 5", "icon": "sparkles", "groupBy": "field:model"}'
    ```

=== "JavaScript"

    ```js
    await curio.smartFolder.update({
      id: 4,
      query: 'kind = image AND rating = 5',
      icon: 'sparkles',
      groupBy: 'field:model',
    });
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_update",
      "arguments": {
        "id": 4,
        "query": "kind = image AND rating = 5",
        "icon": "sparkles",
        "groupBy": "field:model"
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Events and errors**

- **Events emitted**: [`smartFolders.changed`](../events/items.md#smartfolders-changed).
- **Errors**: `bad_request` (invalid `query` or blank `name`), `failed` (smart folder `id` not found when updating `name`, `query` or `groupBy`), `not_allowed` (missing `organize`).
- **Related**: [`smartFolder.create`](#smartfolder-create), [`smartFolder.get`](#smartfolder-get).

## smartFolder.delete {#smartfolder-delete}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/delete</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_delete</span>
</div>

Deletes a smart folder and all of its descendant smart folders recursively. Because smart folders are saved queries rather than item containers, no items are modified or moved to Trash.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Smart folder ID to delete. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/delete \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 4}'
    ```

=== "JavaScript"

    ```js
    await curio.smartFolder.delete({ id: 4 });
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_delete",
      "arguments": {
        "id": 4
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Events and errors**

- **Events emitted**: [`smartFolders.changed`](../events/items.md#smartfolders-changed).
- **Errors**: `bad_request` (missing or invalid `id`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`smartFolder.list`](#smartfolder-list), [`smartFolder.create`](#smartfolder-create).

## smartFolder.items {#smartfolder-items}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/smartFolder/items</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: smartFolder_items</span>
</div>

Evaluates a smart folder's query (including any ancestor smart folders' queries, or the union of its child smart folders if its own query is empty) and returns a paginated slice of matching items in the library's default sort order (newest imported first). Items include their filesystem `path` only when the caller also holds `files.read`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Smart folder ID to evaluate. |
| `offset` | `number` | No | `0` | Zero-based starting index into the matched items. |
| `limit` | `number` | No | `50` | Maximum items to return; clamped to `1000`. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Page<Item>`](../types/item.md#page) — paginated list of [`Item`](../types/item.md#item) objects.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/smartFolder/items \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 1, "offset": 0, "limit": 25}'
    ```

=== "JavaScript"

    ```js
    const page = await curio.smartFolder.items({ id: 1, offset: 0, limit: 25 });
    ```

=== "MCP"

    ```json
    {
      "name": "smartFolder_items",
      "arguments": {
        "id": 1,
        "offset": 0,
        "limit": 25
      }
    }
    ```

**Example response**

```json
{
  "total": 124,
  "offset": 0,
  "items": [
    {
      "id": "8e1d7f4e-9b25-4c0f-a824-1b7e6b21a301",
      "name": "Studio Key Light",
      "fileName": "studio_key_light.png",
      "ext": "png",
      "kind": "image",
      "size": 1842910,
      "width": 2048,
      "height": 1365,
      "duration": null,
      "rating": 5,
      "favorite": false,
      "explicit": false,
      "annotation": "Warm bounce fill on left",
      "url": "",
      "tags": ["reference", "lighting"],
      "folders": [{"id": 2, "name": "Lighting"}],
      "status": "Available",
      "imported": 1732401500000,
      "modified": 1732399000000,
      "parts": 0
    }
  ]
}
```

**Events and errors**

- **Events emitted**: None (read-only).
- **Errors**: `bad_request` (missing `id` or invalid stored query), `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline`.
- **Related**: [`smartFolder.get`](#smartfolder-get), [`item.query`](item-reads.md#item-query), [`view.show`](view.md#view-show).
