# tag

Tag operations list, inspect, rename, merge, recolor and delete tags across a library. Tag references (`tag` or elements of `sources`) accept either the tag's exact trimmed name (`string`) or its integer database ID (`number`). To add or remove tags on specific items, use [`item.addTags`](item-metadata.md#item-addtags) and [`item.removeTags`](item-metadata.md#item-removetags). To organize tags into groups, see [Tag groups](tag-group.md).

Every operation in this namespace targets a single library (see [Choose a library or window](../connect/routing.md)).

| Operation | HTTP Endpoint | Permission | MCP Tool | Summary |
|---|---|---|---|---|
| [`tag.list`](#tag-list) | `POST /api/v1/tag/list` | `metadata.read` | `tag_list` | List every tag with its item count, color and groups. |
| [`tag.get`](#tag-get) | `POST /api/v1/tag/get` | `metadata.read` | `tag_get` | Fetch one tag by name or ID. |
| [`tag.rename`](#tag-rename) | `POST /api/v1/tag/rename` | `organize` | `tag_rename` | Rename a tag, merging automatically if the target name exists. |
| [`tag.merge`](#tag-merge) | `POST /api/v1/tag/merge` | `organize` | `tag_merge` | Merge one or more tags into a single target tag name. |
| [`tag.delete`](#tag-delete) | `POST /api/v1/tag/delete` | `organize` | `tag_delete` | Delete a tag and remove it from every item. |
| [`tag.setColor`](#tag-setcolor) | `POST /api/v1/tag/setColor` | `organize` | `tag_setColor` | Set or clear a tag's own color override. |

## tag.list {#tag-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_list</span>
</div>

Returns every tag in the library ordered alphabetically (case-insensitive) by `name`. Each entry includes the count of non-trashed items carrying the tag, the tag's own `color` (if set), its primary `groupId` (used for inherited color) and all `groupIds` it belongs to.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Tag[]`](../types/organizing.md#tag) — array of all tags in the library.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const tags = await curio.tag.list();
    ```

=== "MCP"

    ```json
    {
      "name": "tag_list",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": 12,
    "name": "lighting",
    "groupId": 2,
    "color": null,
    "count": 37,
    "starred": false,
    "groupIds": [2]
  },
  {
    "id": 15,
    "name": "sky",
    "groupId": 1,
    "color": "#3b82f6",
    "count": 84,
    "starred": true,
    "groupIds": [1, 3]
  }
]
```

**Events and errors**

- **Events emitted**: None (read-only). Subscribe to [`tags.changed`](../events/items.md#tags-changed) to refresh when tags change.
- **Errors**: `unauthorized`, `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline`.
- **Related**: [`tag.get`](#tag-get), [`tagGroup.list`](tag-group.md#taggroup-list).

## tag.get {#tag-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/get</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_get</span>
</div>

Returns a single tag record by its name or integer ID. Name lookup trims surrounding whitespace and matches exact tag spelling.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tag` | `string \| number` | Yes | — | Tag name or integer tag ID. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Tag`](../types/organizing.md#tag) — matching tag object.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/get \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"tag": "sky"}'
    ```

=== "JavaScript"

    ```js
    const tag = await curio.tag.get({ tag: 'sky' });
    ```

=== "MCP"

    ```json
    {
      "name": "tag_get",
      "arguments": {
        "tag": "sky"
      }
    }
    ```

**Example response**

```json
{
  "id": 15,
  "name": "sky",
  "groupId": 1,
  "color": "#3b82f6",
  "count": 84,
  "starred": true,
  "groupIds": [1, 3]
}
```

**Events and errors**

- **Events emitted**: None (read-only).
- **Errors**: `not_found` (`There is no tag <tag>`), `bad_request` (missing `tag`), `not_allowed` (missing `metadata.read`).
- **Related**: [`tag.list`](#tag-list), [`view.show`](view.md#view-show).

## tag.rename {#tag-rename}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/rename</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_rename</span>
</div>

Renames a tag and refreshes the full-text search index for every item carrying it. If another tag with `newName` (after trimming) already exists, the two tags are merged: all items carrying `tag` receive the existing target tag, and the old tag row is deleted. Always pass a non-empty `newName`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tag` | `string \| number` | Yes | — | Existing tag name or integer tag ID. |
| `newName` | `string` | Yes | — | Replacement tag name (trimmed before matching or saving). |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/rename \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"tag": "skies", "newName": "sky"}'
    ```

=== "JavaScript"

    ```js
    await curio.tag.rename({ tag: 'skies', newName: 'sky' });
    ```

=== "MCP"

    ```json
    {
      "name": "tag_rename",
      "arguments": {
        "tag": "skies",
        "newName": "sky"
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

- **Events emitted**: [`tags.changed`](../events/items.md#tags-changed).
- **Errors**: `not_found` (`There is no tag <tag>`), `bad_request` (missing `tag` or `newName`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`tag.merge`](#tag-merge), [`tag.delete`](#tag-delete).

## tag.merge {#tag-merge}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/merge</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_merge</span>
</div>

Merges one or more existing tags (`sources`) into a single `target` tag name in one transaction. If `target` does not yet exist, the first source tag is renamed to `target` and each remaining source is merged into it; if `target` already exists, every source is merged into it and deleted. The full-text search index is refreshed for affected items. Every entry in `sources` must already exist.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `sources` | `(string \| number)[]` | Yes | — | Existing tag names or integer IDs to merge into `target`. |
| `target` | `string` | Yes | — | Destination tag name; created automatically if it does not exist. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/merge \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"sources": ["cloud", "clouds", "overcast"], "target": "sky"}'
    ```

=== "JavaScript"

    ```js
    await curio.tag.merge({ sources: ['cloud', 'clouds', 'overcast'], target: 'sky' });
    ```

=== "MCP"

    ```json
    {
      "name": "tag_merge",
      "arguments": {
        "sources": ["cloud", "clouds", "overcast"],
        "target": "sky"
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

- **Events emitted**: [`tags.changed`](../events/items.md#tags-changed) and [`tagGroups.changed`](../events/items.md#taggroups-changed).
- **Errors**: `not_found` (any entry in `sources` does not exist), `bad_request` (missing `sources` or `target`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`tag.rename`](#tag-rename), [`tag.list`](#tag-list).

## tag.delete {#tag-delete}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/delete</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_delete</span>
</div>

Permanently deletes a tag from the library, removes it from all items and tag groups and refreshes the full-text search index for items that previously carried it.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tag` | `string \| number` | Yes | — | Tag name or integer tag ID to delete. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/delete \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"tag": "temp-import"}'
    ```

=== "JavaScript"

    ```js
    await curio.tag.delete({ tag: 'temp-import' });
    ```

=== "MCP"

    ```json
    {
      "name": "tag_delete",
      "arguments": {
        "tag": "temp-import"
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

- **Events emitted**: [`tags.changed`](../events/items.md#tags-changed) and [`tagGroups.changed`](../events/items.md#taggroups-changed).
- **Errors**: `not_found` (`There is no tag <tag>`), `bad_request` (missing `tag`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`item.removeTags`](item-metadata.md#item-removetags), [`tag.merge`](#tag-merge).

## tag.setColor {#tag-setcolor}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tag/setColor</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tag_setColor</span>
</div>

Sets or clears a tag's own color override. When a tag has no color of its own (`null`), it inherits the color of its primary tag group (`groupId`). Passing `null`, `""` or omitting `color` clears the tag's individual override.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tag` | `string \| number` | Yes | — | Tag name or integer tag ID. |
| `color` | `string` | No | `null` | CSS color string (for example `"#3b82f6"`); omit, pass `null` or pass `""` to clear the override. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tag/setColor \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"tag": "sky", "color": "#3b82f6"}'
    ```

=== "JavaScript"

    ```js
    await curio.tag.setColor({ tag: 'sky', color: '#3b82f6' });
    ```

=== "MCP"

    ```json
    {
      "name": "tag_setColor",
      "arguments": {
        "tag": "sky",
        "color": "#3b82f6"
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

- **Events emitted**: [`tags.changed`](../events/items.md#tags-changed).
- **Errors**: `not_found` (`There is no tag <tag>`), `bad_request` (missing `tag`), `not_allowed` (missing `organize`).
- **Related**: [`tag.get`](#tag-get), [`tagGroup.update`](tag-group.md#taggroup-update).
