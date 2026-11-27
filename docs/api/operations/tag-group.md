# tagGroup

Tag group operations organize tags into named collections with optional colors, icons and descriptions. A tag can belong to multiple tag groups simultaneously; its first group (`groupId`) supplies its fallback color when the tag has no color override of its own.

Every operation in this namespace targets a single library (see [Choose a library or window](../connect/routing.md)).

| Operation | HTTP Endpoint | Permission | MCP Tool | Summary |
|---|---|---|---|---|
| [`tagGroup.list`](#taggroup-list) | `POST /api/v1/tagGroup/list` | `metadata.read` | `tagGroup_list` | List all tag groups in display order. |
| [`tagGroup.create`](#taggroup-create) | `POST /api/v1/tagGroup/create` | `organize` | `tagGroup_create` | Create a new tag group. |
| [`tagGroup.update`](#taggroup-update) | `POST /api/v1/tagGroup/update` | `organize` | `tagGroup_update` | Update a tag group's name, color, icon or description. |
| [`tagGroup.delete`](#taggroup-delete) | `POST /api/v1/tagGroup/delete` | `organize` | `tagGroup_delete` | Delete a tag group while keeping all its tags. |
| [`tagGroup.addTags`](#taggroup-addtags) | `POST /api/v1/tagGroup/addTags` | `organize` | `tagGroup_addTags` | Add existing or new tags to a tag group. |
| [`tagGroup.removeTags`](#taggroup-removetags) | `POST /api/v1/tagGroup/removeTags` | `organize` | `tagGroup_removeTags` | Remove tags from a tag group. |

## tagGroup.list {#taggroup-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_list</span>
</div>

Returns every tag group in the library in display order, then by `name`. To see which tags belong to each group, cross-reference `groupIds` on entries from [`tag.list`](tag.md#tag-list).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`TagGroup[]`](../types/organizing.md#taggroup) — array of tag group objects.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const groups = await curio.tagGroup.list();
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_list",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": 1,
    "name": "Environment",
    "color": "#3b9160",
    "icon": "trees",
    "description": "Outdoor weather, skies, and terrain"
  },
  {
    "id": 2,
    "name": "Lighting",
    "color": "#e09f3e",
    "icon": "sun",
    "description": null
  }
]
```

**Events and errors**

- **Events emitted**: None (read-only). Subscribe to [`tagGroups.changed`](../events/items.md#taggroups-changed) to detect changes.
- **Errors**: `unauthorized`, `not_allowed` (missing `metadata.read`), `unknown_library`, `library_offline`.
- **Related**: [`tag.list`](tag.md#tag-list), [`tagGroup.create`](#taggroup-create).

## tagGroup.create {#taggroup-create}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/create</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_create</span>
</div>

Creates a new tag group with a trimmed non-empty name and an optional CSS color. To set an `icon` or `description`, follow up with [`tagGroup.update`](#taggroup-update).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Tag group name; trimmed, must not be blank. |
| `color` | `string` | No | `null` | Optional CSS color value (for example `"#3b9160"`). |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

`{ success: true; id: number }`

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Confirms the tag group was created. |
| `id` | `number` | Integer ID of the newly created tag group. |

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/create \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name": "Materials", "color": "#6366f1"}'
    ```

=== "JavaScript"

    ```js
    const { id } = await curio.tagGroup.create({ name: 'Materials', color: '#6366f1' });
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_create",
      "arguments": {
        "name": "Materials",
        "color": "#6366f1"
      }
    }
    ```

**Example response**

```json
{
  "success": true,
  "id": 3
}
```

**Events and errors**

- **Events emitted**: [`tagGroups.changed`](../events/items.md#taggroups-changed).
- **Errors**: `bad_request` (`Give it a name` when `name` is blank), `not_allowed` (missing `organize`), `failed` (duplicate group name or database error).
- **Related**: [`tagGroup.update`](#taggroup-update), [`tagGroup.addTags`](#taggroup-addtags).

## tagGroup.update {#taggroup-update}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/update</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_update</span>
</div>

Renames a tag group or updates its `color`, `icon` or `description`. Omitted fields retain their current values. Passing an empty string `""` for `color` or `icon` clears that attribute to `null`; passing `""` for `description` sets it to an empty string.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Integer tag group ID to update. |
| `name` | `string` | No | Unchanged | New group name; trimmed, must not be blank when supplied. |
| `color` | `string` | No | Unchanged | CSS color value; pass `""` to clear. |
| `icon` | `string` | No | Unchanged | Tabler icon name; pass `""` to clear. |
| `description` | `string` | No | Unchanged | Descriptive note shown in the tag manager. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/update \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 3, "icon": "texture", "description": "Surface shaders and fabrics"}'
    ```

=== "JavaScript"

    ```js
    await curio.tagGroup.update({ id: 3, icon: 'texture', description: 'Surface shaders and fabrics' });
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_update",
      "arguments": {
        "id": 3,
        "icon": "texture",
        "description": "Surface shaders and fabrics"
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

- **Events emitted**: [`tagGroups.changed`](../events/items.md#taggroups-changed).
- **Errors**: `bad_request` (`Give it a name` if `name` is blank), `failed` (`There is no such tag group` when `id` does not exist, or duplicate group name), `not_allowed` (missing `organize`).
- **Related**: [`tagGroup.list`](#taggroup-list), [`tagGroup.delete`](#taggroup-delete).

## tagGroup.delete {#taggroup-delete}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/delete</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_delete</span>
</div>

Deletes a tag group and removes its membership links. All tags that belonged to the group remain in the library and on their items; any tag whose primary `groupId` pointed to the deleted group falls back to another group it still belongs to, or becomes ungrouped (`null`).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Integer tag group ID to delete. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/delete \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"id": 3}'
    ```

=== "JavaScript"

    ```js
    await curio.tagGroup.delete({ id: 3 });
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_delete",
      "arguments": {
        "id": 3
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

- **Events emitted**: [`tagGroups.changed`](../events/items.md#taggroups-changed).
- **Errors**: `bad_request` (missing or invalid `id`), `not_allowed` (missing `organize`), `failed` (database error).
- **Related**: [`tagGroup.create`](#taggroup-create), [`tagGroup.removeTags`](#taggroup-removetags).

## tagGroup.addTags {#taggroup-addtags}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/addTags</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_addTags</span>
</div>

Adds one or more tags to a tag group without removing them from any other groups they already belong to. String tag names that do not yet exist in the library are created automatically; numeric tag IDs are verified upfront and must already exist. If a tag previously had no primary `groupId`, `groupId` is set to this group.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `groupId` | `number` | Yes | — | Destination tag group ID. |
| `tags` | `(string \| number)[]` | Yes | — | Tag names (created if new) or existing integer tag IDs. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/addTags \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"groupId": 3, "tags": ["metal", "wood", 12]}'
    ```

=== "JavaScript"

    ```js
    await curio.tagGroup.addTags({ groupId: 3, tags: ['metal', 'wood', 12] });
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_addTags",
      "arguments": {
        "groupId": 3,
        "tags": ["metal", "wood", 12]
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
- **Errors**: `not_found` (any numeric tag ID in `tags` does not exist), `failed` (blank tag name `empty tag` or nonexistent `groupId`), `not_allowed` (missing `organize`).
- **Related**: [`tagGroup.removeTags`](#taggroup-removetags), [`tag.list`](tag.md#tag-list).

## tagGroup.removeTags {#taggroup-removetags}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tagGroup/removeTags</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tagGroup_removeTags</span>
</div>

Removes one or more tags from a tag group. Every tag referenced in `tags` must exist in the library. Removing a tag from a group leaves the tag itself and all item assignments intact; if the tag's primary `groupId` was this group, it updates to another remaining group or `null`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `groupId` | `number` | Yes | — | Tag group ID to remove the tags from. |
| `tags` | `(string \| number)[]` | Yes | — | Existing tag names or integer tag IDs to detach from the group. |
| `library` | `string` | No | Active window's library | Library ID from `library.list`, folder path or library name. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -sS -X POST http://127.0.0.1:41733/api/v1/tagGroup/removeTags \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"groupId": 3, "tags": ["wood", 12]}'
    ```

=== "JavaScript"

    ```js
    await curio.tagGroup.removeTags({ groupId: 3, tags: ['wood', 12] });
    ```

=== "MCP"

    ```json
    {
      "name": "tagGroup_removeTags",
      "arguments": {
        "groupId": 3,
        "tags": ["wood", 12]
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
- **Errors**: `not_found` (any entry in `tags` does not exist), `bad_request` (missing `groupId` or `tags`), `not_allowed` (missing `organize`).
- **Related**: [`tagGroup.addTags`](#taggroup-addtags), [`tagGroup.delete`](#taggroup-delete).
