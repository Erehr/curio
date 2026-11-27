# field

Operations for defining custom metadata fields, querying field facets, inspecting extraction coverage and reader failures, checking where a field is referenced, cleaning up unindexed values and rescanning files. All operations target a **library**.

## field.list {#field-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/list</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_list</span>
</div>

Return every custom, built-in reader and plugin-declared metadata field defined in the library.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Field[]`](../types/organizing.md#field).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/list \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const fields = await curio.field.list()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"field_list","arguments":{}}}
    ```

**Example response**

```json
[
  {
    "id": 1,
    "key": "camera",
    "label": "Camera",
    "type": "text",
    "description": "",
    "defaultValue": null,
    "options": [],
    "validation": { "required": false },
    "format": "",
    "widget": "row",
    "owner": "builtin.format.image",
    "visible": true,
    "sortOrder": 0,
    "ownFilter": false,
    "filterKind": "",
    "indexed": true,
    "pair": null,
    "filterGroup": null,
    "readers": ["builtin.format.image/exif"],
    "needsScan": false
  }
]
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.get {#field-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/get</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_get</span>
</div>

Fetch a single metadata field definition by its key.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Field key (e.g. `"camera"`, `"steps"`). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Field`](../types/organizing.md#field).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/get \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"key":"camera"}'
    ```

=== "JavaScript"

    ```javascript
    const field = await curio.field.get({ key: 'camera' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"field_get","arguments":{"key":"camera"}}}
    ```

**Example response**

```json
{
  "id": 1,
  "key": "camera",
  "label": "Camera",
  "type": "text",
  "description": "",
  "defaultValue": null,
  "options": [],
  "validation": { "required": false },
  "format": "",
  "widget": "row",
  "owner": "builtin.format.image",
  "visible": true,
  "sortOrder": 0,
  "ownFilter": false,
  "filterKind": "",
  "indexed": true,
  "pair": null,
  "filterGroup": null,
  "readers": ["builtin.format.image/exif"],
  "needsScan": false
}
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 not_found` if no field has `key`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.define {#field-define}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/define</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_define</span>
</div>

Create a new custom field or update an existing field's settings. Omitted properties keep their current values. A field's `type` cannot be changed once items hold values for it.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Lowercase machine key used in queries (e.g. `client_project`). |
| `type` | `string` | For new fields | — | `text`, `integer`, `float`, `boolean`, `date`, `datetime`, `single_select`, `multi_select`, `json`, `url` or `file_ref`. |
| `label` | `string` | No | `key` | Display label. |
| `description` | `string` | No | `""` | Help text. |
| `defaultValue` | `string` | No | `null` | Default value. |
| `options` | `string[]` | No | `[]` | Choices for `single_select` or `multi_select`. |
| `validation` | `object` | No | `{}` | `{ min?, max?, maxLength?, pattern?, required? }`. |
| `format` | `string` | No | `""` | Display format hint. |
| `widget` | `string` | No | `"row"` | Editor widget style: `row`, `block` or `code` (`multi_select` renders as chips automatically). |
| `visible` | `boolean` | No | `true` | Show in Inspector and List column picker. |
| `sortOrder` | `number` | No | `0` | Display order. |
| `ownFilter` | `boolean` | No | `false` | Expose as a standalone Filter bar chip and Smart Folder rule. |
| `filterKind` | `string` | No | `""` | `list`, `range`, `text`, `has` or `""` (auto by type). |
| `indexed` | `boolean` | No | `true` | Enable/disable indexing and storage (switching a paired field switches its pair too). |
| `pair` | `string` | No | — | Key of the primary field this one pairs with (e.g. `lora_strength` → `lora`). |
| `filterGroup` | `string` | No | — | Shared numeric filter group name; empty string removes from group. |
| `needsScan` | `boolean` | No | `false` | Mark as switched on without having scanned existing files yet. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/define \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"key":"status_review","label":"Review Status","type":"single_select","options":["Draft","In Review","Approved"],"ownFilter":true}'
    ```

=== "JavaScript"

    ```javascript
    await curio.field.define({
      key: 'status_review',
      label: 'Review Status',
      type: 'single_select',
      options: ['Draft', 'In Review', 'Approved'],
      ownFilter: true,
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"field_define","arguments":{"key":"status_review","label":"Review Status","type":"single_select","options":["Draft","Approved"]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`fields.changed`](../events/items.md#fields-changed). Returns `400 bad_request` if `type` is missing on a new field, `type` is unknown, `validation` is invalid or changing a plugin-owned field's `type`; `403 not_allowed` without `organize`; `404 unknown_library`; or `409 library_closed` / `library_offline`.

---

## field.delete {#field-delete}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/delete</span>
  <span class="curio-badge">Permission: organize</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_delete</span>
</div>

Delete a user-defined custom field (`owner === "user"`) along with all of its stored values. Built-in and plugin-owned fields cannot be deleted (hide or turn off indexing via `field.define` instead).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Key of the user field to delete. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/delete \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"key":"status_review"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.field.delete({ key: 'status_review' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"field_delete","arguments":{"key":"status_review"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`fields.changed`](../events/items.md#fields-changed). Returns `400 bad_request` for built-in or plugin-owned fields, `403 not_allowed` without `organize`, `404 not_found` if no field has `key`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.facets {#field-facets}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/facets</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_facets</span>
</div>

Count distinct values of a field across items matching an optional query and scope. For built-in property histograms (rating, shape, kind, extension) see [`query.facets`](query.md#query-facets); for item counts and byte totals by kind, extension, folder, tag or month see [`library.stats`](libraries.md#library-stats).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Indexed field key. |
| `query` | `string` | No | `""` | Restrict counts to items matching this query. |
| `scope` | `string` | No | `"all"` | `all`, `uncategorized`, `untagged`, `recent`, `trash` or `missing`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`FieldFacets`](../types/organizing.md#fieldfacets).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/facets \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"key":"camera","query":"rating >= 4"}'
    ```

=== "JavaScript"

    ```javascript
    const facets = await curio.field.facets({ key: 'camera', query: 'rating >= 4' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"field_facets","arguments":{"key":"camera"}}}
    ```

**Example response**

```json
{ "values": [["Sony A7 IV", 34], ["Fujifilm X-T5", 19]], "set": 53, "total": 60 }
```

**Events and errors** Read-only. Returns `400 bad_request` on invalid query syntax or unknown `scope`, `403 not_allowed` without `metadata.read`, `404 not_found` if no field has `key`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.coverage {#field-coverage}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/coverage</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_coverage</span>
</div>

Report extraction coverage (`withValue`, `applies`, `waiting`, `failed`) for file-reading fields.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `keys` | `string[]` | No | All file-reading fields | Restrict report to specific field keys. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ fields: FieldCoverage[] }` ([`FieldCoverage`](../types/organizing.md#fieldcoverage)).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/coverage \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"keys":["camera","model_triangles"]}'
    ```

=== "JavaScript"

    ```javascript
    const { fields } = await curio.field.coverage({ keys: ['camera'] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"field_coverage","arguments":{}}}
    ```

**Example response**

```json
{ "fields": [{ "key": "camera", "withValue": 412, "applies": 500, "waiting": 0, "failed": 2 }] }
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.failures {#field-failures}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/failures</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_failures</span>
</div>

List files on which metadata readers failed, newest first.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `keys` | `string[]` | No | All | Filter by field keys. |
| `plugin` | `string` | No | All | Filter by plugin ID. |
| `limit` | `number` | No | `200` | Maximum entries (`1`–`2000`). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ failures: { id: string; name: string; reader: string; error: string; at: number }[] }`:

- `failures[].id` (`string`): Item UUID.
- `failures[].name` (`string`): Item display name.
- `failures[].reader` (`string`): Reader identifier (for example `"curio.models/models"`).
- `failures[].error` (`string`): Error message reported by the reader.
- `failures[].at` (`number`): Failure timestamp in milliseconds since the Unix epoch.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/failures \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"plugin":"curio.models","limit":20}'
    ```

=== "JavaScript"

    ```javascript
    const { failures } = await curio.field.failures({ plugin: 'curio.models', limit: 20 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"field_failures","arguments":{"limit":20}}}
    ```

**Example response**

```json
{
  "failures": [
    { "id": "0194b7a2-...", "name": "broken.gltf", "reader": "curio.models/models", "error": "Unexpected EOF", "at": 1763940500000 }
  ]
}
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.usedBy {#field-usedby}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/usedBy</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_usedBy</span>
</div>

Find every smart folder, plugin filter and plugin sort that references a field key (useful before disabling or deleting a field).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Field key to inspect. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ smartFolders: { id: number; name: string }[]; filters: { plugin: string; label: string }[]; sorts: { plugin: string; label: string }[] }`:

- `smartFolders` (`{ id: number; name: string }[]`): Smart folders whose query references `key`.
- `filters` (`{ plugin: string; label: string }[]`): Plugin filters (and panel filters, labeled as `"<panel> › <filter>"`) whose query references `key`.
- `sorts` (`{ plugin: string; label: string }[]`): Plugin sorts ordered by `key`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/usedBy \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"key":"steps"}'
    ```

=== "JavaScript"

    ```javascript
    const refs = await curio.field.usedBy({ key: 'steps' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"field_usedBy","arguments":{"key":"steps"}}}
    ```

**Example response**

```json
{
  "smartFolders": [{ "id": 5, "name": "Fast renders" }],
  "filters": [{ "plugin": "curio.ai-generated", "label": "≤ 12 steps" }],
  "sorts": [{ "plugin": "curio.ai-generated", "label": "Steps" }]
}
```

**Events and errors** Read-only. Returns `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.cleanup {#field-cleanup}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/cleanup</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_cleanup</span>
</div>

Delete stored values belonging to unindexed file-reading fields (excluding manual hand-edited overrides). Field definitions and smart-folder references stay intact.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `dryRun` | `boolean` | No | `false` | When `true`, only counts eligible values without deleting anything. |
| `keys` | `string[]` | No | All unindexed fields whose plugin is still present | Restrict cleanup to specific keys (required to clean fields whose plugin is gone). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ fields: { key: string; values: number; pluginGone: boolean }[]; removed: number }`:

- `fields[].key` (`string`): Eligible unindexed field key.
- `fields[].values` (`number`): Count of removable reader-extracted values found for that field.
- `fields[].pluginGone` (`boolean`): `true` when no installed reader fills the field anymore.
- `removed` (`number`): Total stored values deleted (`0` when `dryRun` is `true`).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/cleanup \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"dryRun":true}'
    ```

=== "JavaScript"

    ```javascript
    const preview = await curio.field.cleanup({ dryRun: true })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"field_cleanup","arguments":{"dryRun":true}}}
    ```

**Example response**

```json
{ "fields": [{ "key": "seed", "values": 140, "pluginGone": false }], "removed": 0 }
```

**Events and errors** Emits no library events. Returns `403 not_allowed` without `metadata.write`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## field.rescan {#field-rescan}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/field/rescan</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: field_rescan</span>
</div>

Re-run metadata readers as a background job for specific fields, a plugin's readers or specific items. Manual overrides are always preserved.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `keys` | `string[]` | No | — | Field keys to re-extract. |
| `plugin` | `string` | No | — | Plugin ID whose readers should run. |
| `items` | `(string \| number)[]` | No | All matching files | Restrict extraction to specific items. |
| `mode` | `string` | No | `"all"` | `missing` (only files with none of the readers' fields) or `all` (re-read and replace prior reader values). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ job: string | null; files: number }`: background job ID (`null` when `files === 0`) and count of queued files.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/field/rescan \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"plugin":"curio.ai-generated","mode":"missing"}'
    ```

=== "JavaScript"

    ```javascript
    const { job, files } = await curio.field.rescan({ plugin: 'curio.ai-generated', mode: 'missing' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"field_rescan","arguments":{"plugin":"curio.ai-generated","mode":"missing"}}}
    ```

**Example response**

```json
{ "job": "job-4a91f0", "files": 230 }
```

**Events and errors** Emits [`job.started`](../events/jobs.md#job-started), [`job.progress`](../events/jobs.md#job-progress), [`job.finished`](../events/jobs.md#job-finished) and [`items.changed`](../events/items.md#items-changed). Returns `400 bad_request` for an unknown `mode` or when no enabled reader matches `keys`/`plugin`, `403 not_allowed` without `metadata.write`, `404 not_found` for an unknown item ID, `404 unknown_library` or `409 library_closed` / `library_offline`.
