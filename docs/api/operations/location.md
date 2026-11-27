# location

Operations for indexing existing disk folders in place, controlling file access policies, rescanning, customizing sidebar appearance and relocating moved directories. All operations target a **library**.

Locations are watched disk directories indexed in place without copying files; for Curio's library folder hierarchy, see [`folder`](folder.md).

## location.list {#location-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_list</span>
</div>

Return all listed in-place locations in the library. `Location.path` is omitted unless the caller holds `files.read`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Location[]`](../types/library.md#location).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/list \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const locations = await curio.location.list()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"location_list","arguments":{}}}
    ```

**Example response**

```json
[
  { "id": 2, "name": "Stock Photos", "online": true, "watch": true, "access": "full", "count": 1520, "lastScan": 1763940100000, "icon": null, "color": null }
]
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## location.add {#location-add}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/add</span>
  <span class="curio-badge">Permission: files.outside</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_add</span>
</div>

Add an existing directory on disk as an in-place location and start scanning it. Passing an Eagle `.library` folder indexes it as an Eagle library (with its folders and tags); other options do not apply to Eagle libraries.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `path` | `string` | Yes | — | Absolute path to an existing directory on disk. |
| `label` | `string` | No | Folder basename | Sidebar display label. |
| `access` | `string` | No | `"full"` | `read_only`, `full` (edit files in place) or `organize` (edit, rename and move). |
| `watch` | `boolean` | No | `true` | Watch filesystem events automatically. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ success: true; id: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/add \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"path":"D:\\Photos\\2026","label":"Photos 2026","access":"read_only","watch":true}'
    ```

=== "JavaScript"

    ```javascript
    const { id } = await curio.location.add({ path: 'D:\\Photos\\2026', access: 'read_only' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"location_add","arguments":{"path":"D:\\Photos\\2026","access":"read_only"}}}
    ```

**Example response**

```json
{ "success": true, "id": 3 }
```

**Events and errors** Emits [`locations.changed`](../events/scans.md#locations-changed), [`scan.progress`](../events/scans.md#scan-progress), [`scan.done`](../events/scans.md#scan-done) and [`items.added`](../events/items.md#items-added). Returns `400 bad_request` if `path` is not an existing directory, lies inside a library store, `access` is invalid or `label` exceeds 100 characters; `403 not_allowed` without `files.outside`; `404 unknown_library`; or `409 library_closed` / `library_offline`.

---

## location.remove {#location-remove}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/remove</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_remove</span>
</div>

Stop indexing a location and remove its entries from the library database. Files on disk are untouched.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Location ID from `location.list`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/remove \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"id":3}'
    ```

=== "JavaScript"

    ```javascript
    await curio.location.remove({ id: 3 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"location_remove","arguments":{"id":3}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`locations.changed`](../events/scans.md#locations-changed) and [`items.removed`](../events/items.md#items-removed) (`purged: true`). Returns `403 not_allowed` without `organize`, `404 not_found` if no location has `id`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## location.rescan {#location-rescan}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/rescan</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_rescan</span>
</div>

Queue a full directory walk of a location to detect new, modified, moved or missing files.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Location ID. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ success: true; queued: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/rescan \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"id":2}'
    ```

=== "JavaScript"

    ```javascript
    await curio.location.rescan({ id: 2 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"location_rescan","arguments":{"id":2}}}
    ```

**Example response**

```json
{ "success": true, "queued": true }
```

**Events and errors** Emits [`scan.progress`](../events/scans.md#scan-progress) and [`scan.done`](../events/scans.md#scan-done). Returns `403 not_allowed` without `organize`, `404 not_found` if no location has `id`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## location.setAccess {#location-setaccess}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/setAccess</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_setAccess</span>
</div>

Change Curio's disk write policy for an indexed location.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Location ID. |
| `access` | `string` | Yes | — | `read_only`, `full` or `organize`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/setAccess \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"id":2,"access":"organize"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.location.setAccess({ id: 2, access: 'organize' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"location_setAccess","arguments":{"id":2,"access":"organize"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`locations.changed`](../events/scans.md#locations-changed). Returns `400 bad_request` for an invalid `access`, `403 not_allowed` without `organize`, `404 not_found` if no location has `id`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## location.setLook {#location-setlook}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/setLook</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_setLook</span>
</div>

Set or clear a location's sidebar icon and color swatch.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Location ID. |
| `icon` | `string` | No | — | Tabler icon name; `""` clears back to drive-type icon. |
| `color` | `string` | No | — | CSS color (`#rrggbb` or `var(--accent)`); `""` clears. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/setLook \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"id":2,"icon":"camera","color":"#4a90e2"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.location.setLook({ id: 2, icon: 'camera', color: '#4a90e2' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"location_setLook","arguments":{"id":2,"icon":"camera","color":"#4a90e2"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`locations.changed`](../events/scans.md#locations-changed). Returns `403 not_allowed` without `organize`, `404 not_found` if no location has `id`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## location.relocate {#location-relocate}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/location/relocate</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: location_relocate</span>
</div>

Update a location's root path after its directory was moved on disk (or mounted under a new drive letter) while preserving every item's UUID and metadata.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `number` | Yes | — | Location ID. |
| `path` | `string` | Yes | — | New absolute directory path. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/location/relocate \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"id":2,"path":"E:\\Photos\\2026"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.location.relocate({ id: 2, path: 'E:\\Photos\\2026' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"location_relocate","arguments":{"id":2,"path":"E:\\Photos\\2026"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`locations.changed`](../events/scans.md#locations-changed), then queues a rescan ([`scan.progress`](../events/scans.md#scan-progress) and [`scan.done`](../events/scans.md#scan-done)). Returns `400 bad_request` if `path` is not an existing directory, `403 not_allowed` without `organize`, `404 not_found` if no location has `id`, `404 unknown_library` or `409 library_closed` / `library_offline`.
