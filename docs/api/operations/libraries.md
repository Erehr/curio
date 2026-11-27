# library

Operations in the `library` namespace list and inspect libraries on this computer, create, open, rename, reorder, style, forget and switch libraries in windows, and compute aggregate item and byte statistics. For file-grouping rules inside a library, see [Library grouping rules](./library-grouping.md).

## library.list {#library-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/list</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_list</span>
</div>

Lists every library registered on this computer that the caller is allowed to reach, along with each library's stable ID, open state, whether it is the default routing target and its window ID when shown in a window. If the caller's token is restricted to specific libraries, only those libraries are returned. The filesystem `path` field is included only when the caller holds `files.read`.

**Parameters**

This operation takes no parameters (`{}`).

**Returns**

[`Library[]`](../types/library.md#library):

| Field | Type | Description |
|---|---|---|
| `id` | `string \| null` | Stable UUID across renames and moves, or `null` if the library has not been opened yet. |
| `name` | `string` | Display name derived from the `.curio` library folder name. |
| `icon` | `string \| null` | Custom Tabler icon name, or `null` if default. |
| `color` | `string \| null` | Custom CSS color, or `null` if default. |
| `state` | `'window' \| 'background' \| 'closed' \| 'offline'` | `window`: open in a GUI window; `background`: held by a headless background process; `closed`: on disk and ready to wake on demand; `offline`: folder is currently unreachable. |
| `default` | `boolean` | `true` for the library that receives requests omitting `library`. |
| `window` | `string` | Present only when `state` is `"window"` and the window has registered its ID; pass to `window`-targeted operations. |
| `path` | `string` | Present only when the caller has `files.read`. Full path to the `.curio` folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const libraries = await curio.library.list();
    const active = libraries.find((lib) => lib.default);
    ```

=== "MCP"

    ```json
    {
      "name": "library_list",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "id": "8a2f7f10-3f6c-4a0d-a519-0e2c3f6f7a11",
    "name": "Reference",
    "icon": "palette",
    "color": "#3b9160",
    "state": "window",
    "default": true,
    "window": "win-1a2b3c",
    "path": "C:/Users/Alice/AppData/Roaming/Curio/Libraries/Reference.curio"
  },
  {
    "id": "4b9c1e22-9d0a-4781-9f32-11a7c4d0e890",
    "name": "Archive",
    "icon": null,
    "color": null,
    "state": "closed",
    "default": false
  }
]
```

**Side effects, events and errors**

- Emits no public events. Subscribe to [`library.listChanged`](../events/libraries.md#library-listchanged), [`library.opened`](../events/libraries.md#library-opened) and [`library.closed`](../events/libraries.md#library-closed) to observe changes.
- Errors: `unauthorized`, `not_allowed` (missing `library.read`), `bad_request` (if `library` or `window` is passed).
- Related: [`library.info`](#library-info), [`library.open`](#library-open), [Choose a library or window](../connect/routing.md).

---

## library.info {#library-info}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/info</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_info</span>
</div>

Returns metadata, schema version and summary item and size counts for a single library. If the target library is closed on disk, Curio wakes a headless background holder process to answer (waiting up to 30 seconds). The `path` field is included only when the caller holds `files.read`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers (plugins always target their own library). |

**Returns**

[`LibraryInfo`](../types/library.md#libraryinfo):

| Field | Type | Description |
|---|---|---|
| `id` | `string \| null` | Stable library UUID, or `null` if unassigned. |
| `name` | `string \| null` | Display name of the library folder. |
| `schema` | `number` | Current library schema version. |
| `floor` | `number` | Minimum schema version required to open this library safely. |
| `counts` | `object` | Item and storage counts: `all`, `favorites`, `uncategorized`, `untagged`, `trash`, `missing`, `recent`, `tags` (distinct tag count) and `size` (total bytes of non-trashed files). |
| `path` | `string` | Present only when the caller has `files.read`. Full filesystem path to the `.curio` library folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/info \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Reference"}'
    ```

=== "JavaScript"

    ```js
    const info = await curio.library.info({ library: 'Reference' });
    console.log(info.counts.all, info.counts.size);
    ```

=== "MCP"

    ```json
    {
      "name": "library_info",
      "arguments": {
        "library": "Reference"
      }
    }
    ```

**Example response**

```json
{
  "id": "8a2f7f10-3f6c-4a0d-a519-0e2c3f6f7a11",
  "name": "Reference",
  "schema": 42,
  "floor": 40,
  "counts": {
    "all": 1420,
    "favorites": 64,
    "uncategorized": 185,
    "untagged": 310,
    "trash": 12,
    "missing": 0,
    "recent": 48,
    "tags": 96,
    "size": 4829104210
  },
  "path": "C:/Users/Alice/AppData/Roaming/Curio/Libraries/Reference.curio"
}
```

**Side effects, events and errors**

- May spawn a background holder process if the library was `closed` (emitting [`library.opened`](../events/libraries.md#library-opened) with `state: "background"`).
- Errors: `unknown_library`, `library_closed`, `library_offline`, `busy` (library still opening), `not_allowed` (missing `library.read` or library outside token scope).
- Related: [`library.list`](#library-list), [`library.stats`](#library-stats).

---

## library.open {#library-open}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/open</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Entry</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_open</span>
</div>

Opens a registered library in its own Curio window. If a window already has that library open, that window is raised to the foreground instead of spawning a duplicate window. Because this is an `Entry` operation, `library` is required.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | Yes | — | Target library ID from [`library.list`](#library-list), folder path or unique name. |

**Returns**

[`Done`](../types/runtime.md#done) (`{ "success": true }`).

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/open \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Archive"}'
    ```

=== "JavaScript"

    ```js
    await curio.library.open({ library: 'Archive' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_open",
      "arguments": {
        "library": "Archive"
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Side effects, events and errors**

- Spawns a new Curio GUI window for the library or raises the existing window.
- Events: [`library.opened`](../events/libraries.md#library-opened) (with `state: "window"`) when a new window opens.
- Errors: `bad_request` (missing `library`), `unknown_library`, `not_allowed` (missing `library.manage` or library outside token scope), `failed` (could not spawn window process).
- Related: [`library.switch`](#library-switch), [`library.create`](#library-create).

---

## library.create {#library-create}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/create</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_create</span>
</div>

Creates a new empty `.curio` library folder, initializes its `library.db` with a newly generated stable UUID, registers it on this computer's library list and optionally opens it in a new window. The name is sanitized for filesystem safety and suffixed with `.curio`. Because this operation targets the app, passing `library` or `window` is rejected.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Display name for the new library (trimmed; must not be empty). The created folder is `<sanitized-name>.curio`. |
| `folder` | `string` | No | Curio's `Libraries` data directory | Parent directory in which to create `<name>.curio`. |
| `open` | `boolean` | No | `false` | When `true`, opens the new library immediately in a window of its own. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Always `true`. |
| `id` | `string` | Stable UUID assigned to the new library. |
| `name` | `string` | Final library name derived from the created `.curio` folder. |
| `path` | `string` | Present only when the caller has `files.read`. Full path to the new `.curio` folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/create \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"name": "Client Shots", "open": true}'
    ```

=== "JavaScript"

    ```js
    const created = await curio.library.create({ name: 'Client Shots', open: true });
    console.log(created.id, created.name);
    ```

=== "MCP"

    ```json
    {
      "name": "library_create",
      "arguments": {
        "name": "Client Shots",
        "open": true
      }
    }
    ```

**Example response**

```json
{
  "success": true,
  "id": "e1c4a9b2-5318-48de-9f1a-77c3b204d190",
  "name": "Client Shots",
  "path": "C:/Users/Alice/AppData/Roaming/Curio/Libraries/Client Shots.curio"
}
```

**Side effects, events and errors**

- Creates `<folder>/<name>.curio/library.db` on disk and appends it to Curio's library list.
- Events: [`library.listChanged`](../events/libraries.md#library-listchanged), plus [`library.opened`](../events/libraries.md#library-opened) if `open` is `true`.
- Errors: `bad_request` (empty `name`, library folder already exists at target path or routing `library`/`window` passed), `failed` (cannot create folder or initialize library database), `not_allowed` (missing `library.manage`).
- Related: [`library.open`](#library-open), [`library.rename`](#library-rename), [`library.forget`](#library-forget).

---

## library.rename {#library-rename}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/rename</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Entry</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_rename</span>
</div>

Renames a library's `.curio` folder on disk beside its current location and updates the library list entry while preserving its stable `id`, icon and color. The library cannot be renamed while a GUI window has it open; if only a background holder process holds it, Curio releases the background holder first.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | Yes | — | Target library ID, folder path or unique name. |
| `name` | `string` | Yes | — | New display name; the folder becomes `<name>.curio` beside the previous folder. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Always `true`. |
| `name` | `string` | Resulting display name after renaming. |
| `path` | `string` | Present only when the caller has `files.read`. Full path to the renamed `.curio` folder. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/rename \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Archive", "name": "Archive 2026"}'
    ```

=== "JavaScript"

    ```js
    const renamed = await curio.library.rename({ library: 'Archive', name: 'Archive 2026' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_rename",
      "arguments": {
        "library": "Archive",
        "name": "Archive 2026"
      }
    }
    ```

**Example response**

```json
{
  "success": true,
  "name": "Archive 2026",
  "path": "C:/Users/Alice/AppData/Roaming/Curio/Libraries/Archive 2026.curio"
}
```

**Side effects, events and errors**

- Renames the `.curio` folder on disk and updates saved library references (default library, custom look and ID mapping).
- Events: [`library.listChanged`](../events/libraries.md#library-listchanged) (and [`library.closed`](../events/libraries.md#library-closed) if a background holder was released).
- Errors: `bad_request` (missing `library`), `unknown_library`, `failed` (library is currently open in a window, invalid name or destination folder already exists), `not_allowed` (missing `library.manage` or unreachable library).
- Related: [`library.list`](#library-list), [`library.forget`](#library-forget).

---

## library.forget {#library-forget}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/forget</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Entry</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_forget</span>
</div>

Removes a library from this computer's library list without deleting or altering its `.curio` folder on disk. If the library is held by another GUI window, the call fails until that window is closed; a background holder is released automatically, and if the process answering the call holds the library itself, it closes it first.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | Yes | — | Target library ID, folder path or unique name. |

**Returns**

[`Done`](../types/runtime.md#done) (`{ "success": true }`).

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/forget \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Archive 2026"}'
    ```

=== "JavaScript"

    ```js
    await curio.library.forget({ library: 'Archive 2026' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_forget",
      "arguments": {
        "library": "Archive 2026"
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Side effects, events and errors**

- Removes the entry, saved look and ID mapping from Curio's library list; files on disk remain untouched.
- Events: [`library.listChanged`](../events/libraries.md#library-listchanged) (and [`library.closed`](../events/libraries.md#library-closed) if a holder closed).
- Errors: `bad_request` (missing `library`), `unknown_library`, `failed` (another window still has the library open), `not_allowed` (missing `library.manage` or library outside token scope).
- Related: [`library.list`](#library-list), [`library.create`](#library-create).

---

## library.place {#library-place}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/place</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Entry</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_place</span>
</div>

Reorders a library in this computer's library list by moving it immediately before another library, or to the end of the list when `before` is omitted.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | Yes | — | Library to move (ID, folder path or unique name). |
| `before` | `string` | No | End of list | Library ID, folder path or unique name to place `library` immediately before. Omit to move to the end. |

**Returns**

[`Done`](../types/runtime.md#done) (`{ "success": true }`).

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/place \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Client Shots", "before": "Reference"}'
    ```

=== "JavaScript"

    ```js
    await curio.library.place({ library: 'Client Shots', before: 'Reference' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_place",
      "arguments": {
        "library": "Client Shots",
        "before": "Reference"
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Side effects, events and errors**

- Updates the saved library list ordering.
- Events: [`library.listChanged`](../events/libraries.md#library-listchanged).
- Errors: `bad_request` (missing `library`), `unknown_library` (either `library` or `before` not found), `failed` (library not on the list), `not_allowed` (missing `library.manage` or either library outside token scope).
- Related: [`library.list`](#library-list), [`library.setLook`](#library-setlook).

---

## library.setLook {#library-setlook}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/setLook</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Entry</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_setLook</span>
</div>

Sets or clears the custom icon and accent color shown for a library in Curio's library switcher and lists. Omitting a property leaves its current value unchanged; passing an empty string `""` clears it back to the default.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | Yes | — | Target library ID, folder path or unique name. |
| `icon` | `string` | No | Unchanged | Tabler icon name (for example `"camera"` or `"palette"`). Pass `""` to clear. |
| `color` | `string` | No | Unchanged | CSS color value (for example `"#3b9160"`). Pass `""` to clear. |

**Returns**

[`Done`](../types/runtime.md#done) (`{ "success": true }`).

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/setLook \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"library": "Reference", "icon": "palette", "color": "#3b9160"}'
    ```

=== "JavaScript"

    ```js
    await curio.library.setLook({ library: 'Reference', icon: 'palette', color: '#3b9160' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_setLook",
      "arguments": {
        "library": "Reference",
        "icon": "palette",
        "color": "#3b9160"
      }
    }
    ```

**Example response**

```json
{
  "success": true
}
```

**Side effects, events and errors**

- Updates the saved icon/color look for the library in app configuration.
- Events: [`library.listChanged`](../events/libraries.md#library-listchanged).
- Errors: `bad_request` (missing `library`), `unknown_library`, `not_allowed` (missing `library.manage` or library outside token scope).
- Related: [`library.list`](#library-list), [`library.place`](#library-place).

---

## library.stats {#library-stats}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/stats</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_stats</span>
</div>

Computes item counts and total byte sizes grouped by item kind, file extension, folder, tag or calendar month added, optionally filtered by a query expression. Only active (non-trashed) top-level items are counted; grouped member files are excluded.

When grouping by `folder`, each folder's count and bytes include its entire subfolder tree with each matching item counted once per folder. Because items can belong to multiple folders or tags, the sum of group counts may exceed `total.count`. Groups are ordered by descending count, except `month`, which is ordered chronologically (`YYYY-MM`, local time).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `by` | `string` | Yes | — | Grouping dimension: `kind`, `ext`, `folder`, `tag` or `month`. |
| `query` | `string` | No | `""` | Query language expression restricting which items are aggregated; empty includes all active items. See [Query language](../query/syntax.md). |
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `by` | `string` | Echoes the requested grouping dimension (`kind`, `ext`, `folder`, `tag` or `month`). |
| `total` | `{ count: number; bytes: number }` | Total matching top-level items and total bytes across the query result. |
| `groups` | `{ key: string; name: string; count: number; bytes: number }[]` | Breakdown buckets. `key` is the kind/ext/month string or stringified numeric folder/tag ID; `name` is the human label (for example folder name or tag name); `count` and `bytes` are the bucket totals. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/stats \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"by": "kind", "query": "rating >= 3"}'
    ```

=== "JavaScript"

    ```js
    const stats = await curio.library.stats({ by: 'kind', query: 'rating >= 3' });
    console.log(stats.total, stats.groups);
    ```

=== "MCP"

    ```json
    {
      "name": "library_stats",
      "arguments": {
        "by": "kind",
        "query": "rating >= 3"
      }
    }
    ```

**Example response**

```json
{
  "by": "kind",
  "total": {
    "count": 412,
    "bytes": 1984320110
  },
  "groups": [
    { "key": "image", "name": "image", "count": 350, "bytes": 912450800 },
    { "key": "video", "name": "video", "count": 42, "bytes": 1020310400 },
    { "key": "document", "name": "document", "count": 20, "bytes": 51558910 }
  ]
}
```

**Side effects, events and errors**

- Emits no public events.
- Errors: `bad_request` (invalid `by` value or invalid `query` syntax), `unknown_library`, `library_closed`, `library_offline`, `not_allowed` (missing `metadata.read`).
- Related: [`library.info`](#library-info), [`item.count`](./item-reads.md#item-count), [`query.facets`](./query.md#query-facets).

---

## library.switch {#library-switch}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/switch</span>
  <span class="curio-badge">Permission: library.manage</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_switch</span>
</div>

Switches an open Curio window to show a different library. If the targeted window already shows `to`, nothing changes (`switched: false`). If a different Curio window already has `to` open, Curio focuses that window instead of switching the targeted window (`switched: false`, returning the focused window's ID). Otherwise the targeted window closes its current library and loads `to` (waiting up to 60 seconds).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `to` | `string` | Yes | — | Destination library ID from [`library.list`](#library-list), folder path or unique name. |
| `window` | `string` | No | Most recently focused window | Routing parameter: window ID to switch. Not allowed for plugin callers. |
| `library` | `string` | No | Most recently focused window | Routing parameter: library whose window should be switched. Not allowed for plugin callers. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `switched` | `boolean` | `true` if the targeted window switched libraries; `false` if `to` was already open in the targeted window or another window was focused instead. |
| `window` | `string` | Window ID of the window now showing `to`. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/switch \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"to": "Client Shots"}'
    ```

=== "JavaScript"

    ```js
    const { switched, window: winId } = await curio.library.switch({ to: 'Client Shots' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_switch",
      "arguments": {
        "to": "Client Shots"
      }
    }
    ```

**Example response**

```json
{
  "switched": true,
  "window": "win-1a2b3c"
}
```

**Side effects, events and errors**

- Switches the active library in the targeted window, or focuses another window already holding `to`.
- Events: [`library.switched`](../events/libraries.md#library-switched) when a window switches, plus [`library.opened`](../events/libraries.md#library-opened) and [`library.closed`](../events/libraries.md#library-closed) as library processes change.
- Errors: `unknown_library` (unknown `to` or `library`), `library_offline` (`to` folder cannot be reached on disk), `no_window`, `not_found` (unknown `window` ID), `bad_request` (`window` and `library` disagree), `busy` (switch timed out after 60 seconds), `not_allowed` (missing `library.manage` or `to` outside token scope).
- Related: [`library.open`](#library-open), [`library.list`](#library-list), [`library.switched`](../events/libraries.md#library-switched).
