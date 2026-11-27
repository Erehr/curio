# folder

Folder operations read and organize a library's hierarchical folder tree and list the items inside a folder. Library folders are managed collections inside the library; for watched in-place disk directories, see [Locations](location.md). Every operation in this namespace targets a single library; pass `library` to address a specific library, or omit it to target the library in the most recently focused window (see [Choose a library or window](../connect/routing.md)).

| Operation | HTTP Endpoint | Permission | MCP Tool | Summary |
|---|---|---|---|---|
| [`folder.list`](#folder-list) | `POST /api/v1/folder/list` | `metadata.read` | `folder_list` | List all folders as a flat array with parent links and item counts. |
| [`folder.get`](#folder-get) | `POST /api/v1/folder/get` | `metadata.read` | `folder_get` | Fetch one folder with its direct subfolder IDs. |
| [`folder.recent`](#folder-recent) | `POST /api/v1/folder/recent` | `metadata.read` | `folder_recent` | List folders most recently used in the library's window. |
| [`folder.items`](#folder-items) | `POST /api/v1/folder/items` | `metadata.read` | `folder_items` | Paginate items in a folder, optionally including subfolders. |
| [`folder.create`](#folder-create) | `POST /api/v1/folder/create` | `organize` | `folder_create` | Create a new folder at the top level or under a parent folder. |
| [`folder.rename`](#folder-rename) | `POST /api/v1/folder/rename` | `organize` | `folder_rename` | Rename an existing folder. |
| [`folder.move`](#folder-move) | `POST /api/v1/folder/move` | `organize` | `folder_move` | Move a folder under another parent or to the top level. |
| [`folder.update`](#folder-update) | `POST /api/v1/folder/update` | `organize` | `folder_update` | Update a folder's color, icon or automatic tags. |
| [`folder.delete`](#folder-delete) | `POST /api/v1/folder/delete` | `organize` | `folder_delete` | Delete a folder and its subfolders recursively. |

## folder.list {#folder-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_list</span>
</div>

Returns every folder in the library as a flat array ordered by `sortOrder` and then case-insensitive `name`. Reconstruct the folder tree on the client using each folder's `parentId` (`null` for top-level folders). Each folder's `count` reports non-trashed top-level items directly inside that folder, excluding subfolder totals and grouped member files.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Folder[]`](../types/organizing.md#folder) — flat array of folder objects.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const folders = await curio.folder.list();
    ```

=== "MCP"

    ```json
    {
      "name": "folder_list",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": 1,
    "parentId": null,
    "name": "Reference",
    "color": "#3b9160",
    "sortOrder": 1,
    "count": 42,
    "icon": "palette",
    "autoTags": ["reference"],
    "createdAt": 1732400100000
  },
  {
    "id": 2,
    "parentId": 1,
    "name": "Lighting",
    "color": null,
    "sortOrder": 1,
    "count": 18,
    "icon": null,
    "autoTags": [],
    "createdAt": 1732400200000
  }
]
```

**Events and errors**

- **Events emitted**: None (read-only). Subscribe to [`folders.changed`](../events/items.md#folders-changed) to know when to re-read the folder list.
- **Errors**: `unauthorized`, `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline` (see [Error reference](../connect/errors.md)).
- **Related**: [`folder.get`](#folder-get), [`folder.recent`](#folder-recent), [`folder.create`](#folder-create).

## folder.get {#folder-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/get</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_get</span>
</div>

Returns a single folder record together with the IDs of its direct child subfolders.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Integer folder ID. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Folder`](../types/organizing.md#folder) `& { children: number[] }` — all fields of `Folder` plus:

| Field | Type | Description |
|---|---|---|
| `children` | `number[]` | Direct child folder IDs in their `folder.list` sort order. Empty array if the folder has no subfolders. |

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/get \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 1}'
    ```

=== "JavaScript"

    ```js
    const folder = await curio.folder.get({ id: 1 });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_get",
      "arguments": {
        "id": 1
      }
    }
    ```

**Example response**

```json
{
  "id": 1,
  "parentId": null,
  "name": "Reference",
  "color": "#3b9160",
  "sortOrder": 1,
  "count": 42,
  "icon": "palette",
  "autoTags": ["reference"],
  "createdAt": 1732400100000,
  "children": [2, 5]
}
```

**Events and errors**

- **Events emitted**: None (read-only).
- **Errors**: `not_found` (`There is no folder <id>`), `bad_request` (missing or non-integer `id`), `not_allowed` (missing `metadata.read`).
- **Related**: [`folder.list`](#folder-list), [`folder.items`](#folder-items).

## folder.recent {#folder-recent}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/recent</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_recent</span>
</div>

Returns the folders most recently used in this library's window, ordered newest first. Folders that were subsequently deleted are omitted automatically. If the library is held by a background process without a window or no folders have been used in the current session, returns an empty array.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Folder[]`](../types/organizing.md#folder) — array of recently used folders, newest first.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/recent \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const recent = await curio.folder.recent();
    ```

=== "MCP"

    ```json
    {
      "name": "folder_recent",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": 2,
    "parentId": 1,
    "name": "Lighting",
    "color": null,
    "sortOrder": 1,
    "count": 18,
    "icon": null,
    "autoTags": [],
    "createdAt": 1732400200000
  }
]
```

**Events and errors**

- **Events emitted**: None (read-only).
- **Errors**: `unauthorized`, `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline`.
- **Related**: [`folder.list`](#folder-list), [`item.addToFolders`](item-metadata.md#item-addtofolders).

## folder.items {#folder-items}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/items</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_items</span>
</div>

Returns a paginated slice of non-trashed top-level items in a folder. By default, items in descendant subfolders are included (`subfolders: true`). Items include their filesystem `path` only when the caller also holds `files.read`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Folder ID to read. |
| `subfolders` | `boolean` | No | `true` | When `true`, includes items from all descendant subfolders; when `false`, returns only direct members of the folder. |
| `sort` | `string` | No | `"imported"` | Sort key: `imported`, `modified`, `created`, `name`, `size`, `rating`, `resolution`, `duration` or `field:<key>` for an indexed sortable custom field. |
| `descending` | `boolean` | No | `true` | Sort newest, largest or highest first when `true`; ascending when `false`. |
| `offset` | `number` | No | `0` | Zero-based starting index into the sorted results. |
| `limit` | `number` | No | `50` | Maximum items to return; clamped to `1000`. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Page<Item>`](../types/item.md#page) — paginated list of [`Item`](../types/item.md#item) objects.

| Field | Type | Description |
|---|---|---|
| `total` | `number` | Total matching items across all pages. |
| `offset` | `number` | Offset used for this page. |
| `items` | [`Item[]`](../types/item.md#item) | Items in the requested page. |

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/items \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 1, "subfolders": true, "sort": "rating", "descending": true, "limit": 20}'
    ```

=== "JavaScript"

    ```js
    const page = await curio.folder.items({ id: 1, subfolders: true, sort: 'rating', descending: true, limit: 20 });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_items",
      "arguments": {
        "id": 1,
        "subfolders": true,
        "sort": "rating",
        "descending": true,
        "limit": 20
      }
    }
    ```

**Example response**

```json
{
  "total": 60,
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
- **Errors**: `not_found` (folder `id` does not exist), `bad_request` (unknown `sort` value), `not_allowed` (missing `metadata.read`).
- **Related**: [`item.query`](item-reads.md#item-query), [`smartFolder.items`](smart-folder.md#smartfolder-items), [`folder.get`](#folder-get).

## folder.create {#folder-create}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/create</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_create</span>
</div>

Creates a new folder at the top level or under an existing `parent` folder, appended after existing siblings (`sortOrder = max + 1`). Leading and trailing whitespace is trimmed from `name`. Because folders mirror directories in the managed store, sibling names are compared case-insensitively: if a sibling with the same name already exists, Curio automatically appends a numeric suffix according to the library's duplicate-name style (for example, `Lighting (2)`).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Folder name; trimmed, must not be blank. |
| `parent` | `number` | No | `null` | Parent folder ID; omit or pass `null` to create a top-level folder. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

`{ success: true; id: number }`

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Confirms the folder was created. |
| `id` | `number` | Integer ID of the newly created folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/create \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name": "Rim Lighting", "parent": 2}'
    ```

=== "JavaScript"

    ```js
    const { id } = await curio.folder.create({ name: 'Rim Lighting', parent: 2 });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_create",
      "arguments": {
        "name": "Rim Lighting",
        "parent": 2
      }
    }
    ```

**Example response**

```json
{
  "success": true,
  "id": 9
}
```

**Events and errors**

- **Events emitted**: [`folders.changed`](../events/items.md#folders-changed).
- **Errors**: `bad_request` (`Give it a name` when `name` is empty or whitespace), `not_found` (`parent` folder does not exist), `not_allowed` (missing `organize`), `failed` (write timeout or database error).
- **Related**: [`folder.rename`](#folder-rename), [`folder.move`](#folder-move), [`folder.update`](#folder-update).

## folder.rename {#folder-rename}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/rename</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_rename</span>
</div>

Renames an existing folder. Leading and trailing whitespace is trimmed. Unlike [`folder.create`](#folder-create), renaming refuses a case-insensitive collision with an existing sibling folder under the same parent. In a managed store, Curio reconciles the mirrored directory name on disk asynchronously.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Folder ID to rename. |
| `name` | `string` | Yes | — | New folder name; trimmed, must not be blank or collide with a sibling. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/rename \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 9, "name": "Backlighting"}'
    ```

=== "JavaScript"

    ```js
    await curio.folder.rename({ id: 9, name: 'Backlighting' });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_rename",
      "arguments": {
        "id": 9,
        "name": "Backlighting"
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

- **Events emitted**: [`folders.changed`](../events/items.md#folders-changed).
- **Errors**: `bad_request` (`Give it a name` when `name` is blank), `failed` (folder `id` does not exist or sibling name collision `There is already a folder called "..." here`), `not_allowed` (missing `organize`).
- **Related**: [`folder.create`](#folder-create), [`folder.move`](#folder-move).

## folder.move {#folder-move}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/move</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_move</span>
</div>

Moves a folder (with its entire subtree) under another folder or to the top level. Moving a folder into itself or into one of its own descendants is rejected. If a folder with the same name (case-insensitive) already exists under the new parent, Curio automatically appends a duplicate-name suffix to the moved folder's name. If the folder is already under `parent`, the call succeeds as a no-op.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Folder ID to move. |
| `parent` | `number` | No | `null` | Destination parent folder ID; pass `null` or omit to move to the top level. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/move \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 9, "parent": 1}'
    ```

=== "JavaScript"

    ```js
    await curio.folder.move({ id: 9, parent: 1 });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_move",
      "arguments": {
        "id": 9,
        "parent": 1
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

- **Events emitted**: [`folders.changed`](../events/items.md#folders-changed).
- **Errors**: `failed` (folder `id` does not exist, cycle detected `A folder can't be moved into itself` or nonexistent `parent` folder ID), `not_allowed` (missing `organize`).
- **Related**: [`folder.rename`](#folder-rename), [`item.moveToFolder`](item-metadata.md#item-movetofolder).

## folder.update {#folder-update}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/update</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_update</span>
</div>

Updates a folder's visual styling (`color`, `icon`) and/or its automatic tags (`autoTags`). Omitted properties are left unchanged. Passing an empty string `""` for `color` or `icon` clears that property; passing an empty array `[]` for `autoTags` clears all automatic tags. Note that `autoTags` are joined with commas internally, so individual tag names must not contain commas; automatic tags apply to items added to the folder afterward and do not retroactively tag existing items.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Folder ID to update. |
| `color` | `string` | No | Unchanged | CSS color value (for example `"#3b9160"`); pass `""` to clear. Folder colors can be queried with `folder_color`. |
| `icon` | `string` | No | Unchanged | Tabler icon name (for example `"palette"`); pass `""` to clear. |
| `autoTags` | `string[]` | No | Unchanged | Tag names automatically applied whenever an item is added to this folder; pass `[]` to clear. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/update \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 2, "color": "#e09f3e", "icon": "sun", "autoTags": ["lighting", "reference"]}'
    ```

=== "JavaScript"

    ```js
    await curio.folder.update({ id: 2, color: '#e09f3e', icon: 'sun', autoTags: ['lighting', 'reference'] });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_update",
      "arguments": {
        "id": 2,
        "color": "#e09f3e",
        "icon": "sun",
        "autoTags": ["lighting", "reference"]
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

- **Events emitted**: [`folders.changed`](../events/items.md#folders-changed).
- **Errors**: `bad_request` (invalid parameter types), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`folder.rename`](#folder-rename), [`folder.get`](#folder-get).

## folder.delete {#folder-delete}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/folder/delete</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: folder_delete</span>
</div>

Deletes a folder and all of its descendant subfolders recursively. Items that also belong to a folder outside the deleted subtree remain active and only lose their memberships in the deleted folders.

Items whose only folder memberships were inside the deleted subtree move to the library's Trash without deleting files on disk, and their former folder paths are remembered so [`item.restore`](item-lifecycle.md#item-restore) can recreate the folders. To keep those items active as uncategorized items instead, remove them with [`item.removeFromFolders`](item-metadata.md#item-removefromfolders) before deleting the folder.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Folder ID to delete (along with all descendant subfolders). |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/folder/delete \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 9}'
    ```

=== "JavaScript"

    ```js
    await curio.folder.delete({ id: 9 });
    ```

=== "MCP"

    ```json
    {
      "name": "folder_delete",
      "arguments": {
        "id": 9
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

- **Events emitted**: [`folders.changed`](../events/items.md#folders-changed), and [`items.removed`](../events/items.md#items-removed) (`purged: false`) for any items moved to the Trash.
- **Errors**: `bad_request` (missing or invalid `id`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`item.removeFromFolders`](item-metadata.md#item-removefromfolders), [`item.restore`](item-lifecycle.md#item-restore).
