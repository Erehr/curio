# item (Read)

Operations for querying, counting, fetching, inspecting embedded EXIF/PNG metadata, reading unindexed fields on demand and finding visually similar items. All operations in this group target a **library** (pass optional `library` to route to a specific library).

## item.query {#item-query}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/query</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_query</span>
</div>

Search and paginate items in a library using the [Query language](../query/syntax.md).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | No | `""` | Query language expression (e.g. `tag = sky AND rating >= 4`). Empty matches every item in `scope`. |
| `scope` | `string` | No | `"all"` | One of `all`, `uncategorized`, `untagged`, `recent`, `trash`, `missing`. |
| `sort` | `string` | No | `"imported"` | One of `imported`, `modified`, `created`, `name`, `size`, `rating`, `resolution`, `duration` or `field:<key>`. |
| `descending` | `boolean` | No | `true` | `true` for newest/largest/highest first; `false` for ascending. |
| `offset` | `number` | No | `0` | Zero-based pagination offset. |
| `limit` | `number` | No | `50` | Maximum items to return (`1`–`1000`). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Page<Item>`](../types/item.md#page) (`{ total, offset, items }`). `Item.path` is included only with `files.read`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/query \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"query":"tag = sky AND rating >= 4","sort":"rating","descending":true,"limit":2}'
    ```

=== "JavaScript"

    ```javascript
    const page = await curio.item.query({
      query: 'tag = sky AND rating >= 4',
      sort: 'rating',
      descending: true,
      limit: 2,
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_query","arguments":{"query":"tag = sky AND rating >= 4","sort":"rating","descending":true,"limit":2}}}
    ```

**Example response**

```json
{
  "total": 42,
  "offset": 0,
  "items": [
    {
      "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
      "name": "Dawn clouds",
      "fileName": "dawn_clouds.png",
      "ext": "png",
      "kind": "image",
      "size": 2410290,
      "width": 2560,
      "height": 1440,
      "duration": null,
      "rating": 5,
      "favorite": false,
      "explicit": false,
      "annotation": "",
      "url": "",
      "tags": ["sky", "landscape"],
      "folders": [{ "id": 3, "name": "References" }],
      "status": "Available",
      "imported": 1763900000000,
      "modified": 1763890000000,
      "parts": 0
    }
  ]
}
```

**Events and errors** Read-only. Returns `400 bad_request` on invalid query syntax, unindexed field references or unknown `scope`/`sort`. See also [`item.count`](#item-count), [`item.getMany`](#item-getmany), [`query.validate`](query.md#query-validate).

---

## item.count {#item-count}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/count</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_count</span>
</div>

Count items matching a query without fetching item rows.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | No | `""` | Query language expression; empty counts all items in `scope`. |
| `scope` | `string` | No | `"all"` | `all`, `uncategorized`, `untagged`, `recent`, `trash` or `missing`. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ count: number }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/count \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"query":"ext = png AND width >= 1920"}'
    ```

=== "JavaScript"

    ```javascript
    const { count } = await curio.item.count({ query: 'ext = png AND width >= 1920' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_count","arguments":{"query":"ext = png AND width >= 1920"}}}
    ```

**Example response**

```json
{ "count": 128 }
```

**Events and errors** Read-only. Returns `400 bad_request` on invalid query syntax or unindexed field names.

---

## item.get {#item-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/get</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_get</span>
</div>

Fetch a single item along with all of its stored custom field values and (when grouped) its member file descriptors.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUIDv7 or integer row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`ItemWithFields`](../types/item.md#itemwithfields).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/get \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}'
    ```

=== "JavaScript"

    ```javascript
    const item = await curio.item.get({ id: '0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_get","arguments":{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}}}
    ```

**Example response**

```json
{
  "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
  "name": "Dawn clouds",
  "fileName": "dawn_clouds.png",
  "ext": "png",
  "kind": "image",
  "size": 2410290,
  "width": 2560,
  "height": 1440,
  "duration": null,
  "rating": 5,
  "favorite": false,
  "explicit": false,
  "annotation": "",
  "url": "",
  "tags": ["sky"],
  "folders": [{ "id": 3, "name": "References" }],
  "status": "Available",
  "imported": 1763900000000,
  "modified": 1763890000000,
  "parts": 0,
  "fields": { "camera": "Sony A7 IV", "iso": 100 }
}
```

**Events and errors** Read-only. Returns `404 not_found` if no item has that ID.

---

## item.getMany {#item-getmany}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/getMany</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_getMany</span>
</div>

Fetch multiple items with their custom fields and grouped members in one call, preserving input order.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item UUIDs or row IDs. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`ItemWithFields[]`](../types/item.md#itemwithfields).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/getMany \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012","0194b7a2-9d01-7b12-8a44-112233445566"]}'
    ```

=== "JavaScript"

    ```javascript
    const items = await curio.item.getMany({ ids: ['0194b7a2-...', '0194b7a3-...'] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_getMany","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
[
  {
    "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
    "name": "Dawn clouds",
    "fileName": "dawn_clouds.png",
    "ext": "png",
    "kind": "image",
    "size": 2410290,
    "width": 2560,
    "height": 1440,
    "duration": null,
    "rating": 5,
    "favorite": false,
    "explicit": false,
    "annotation": "",
    "url": "",
    "tags": ["sky"],
    "folders": [{ "id": 3, "name": "References" }],
    "status": "Available",
    "imported": 1763900000000,
    "modified": 1763890000000,
    "parts": 0,
    "fields": { "camera": "Sony A7 IV", "iso": 100 }
  }
]
```

**Events and errors** Read-only. Returns `404 not_found` if any requested ID does not exist.

---

## item.readFields {#item-readfields}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/readFields</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_readFields</span>
</div>

Run Curio's built-in file readers and plugins' declarative readers (`png_text`, `regex`, `json`) directly against an item's file on disk **without storing** the results in the library database. By default reads fields whose indexing switch is off in **Settings › Fields**. External plugin CLI programs are not invoked.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `keys` | `string[]` | No | All unindexed file-reading fields | Restrict reading to specific field keys. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ fields: Record<string, unknown> }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/readFields \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012","keys":["seed","model_vertices"]}'
    ```

=== "JavaScript"

    ```javascript
    const { fields } = await curio.item.readFields({ id: item.id, keys: ['seed'] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_readFields","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{ "fields": { "seed": "4192837461" } }
```

**Events and errors** Read-only; emits no events. Returns `404 not_found` if the item or its file on disk is missing/offline.

---

## item.exif {#item-exif}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/exif</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_exif</span>
</div>

Read raw EXIF tags directly from an image item on disk as a key-to-string map. Returns `{ "tags": {} }` when the file carries no EXIF block.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ tags: Record<string, string> }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/exif \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}'
    ```

=== "JavaScript"

    ```javascript
    const { tags } = await curio.item.exif({ id: item.id })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"item_exif","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{
  "tags": {
    "Make": "SONY",
    "Model": "ILCE-7M4",
    "FNumber": "f/2.8",
    "ExposureTime": "1/500 s",
    "PhotographicSensitivity": "100"
  }
}
```

**Events and errors** Read-only. Returns `404 not_found` if the item or file is missing/offline.

---

## item.pngText {#item-pngtext}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/pngText</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_pngText</span>
</div>

Read PNG `tEXt`, `zTXt` and `iTXt` metadata chunks by keyword (commonly used for AI generation prompts and ComfyUI workflows). Returns `{ "chunks": {} }` for non-PNG files or PNGs without text chunks.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ chunks: Record<string, string> }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/pngText \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}'
    ```

=== "JavaScript"

    ```javascript
    const { chunks } = await curio.item.pngText({ id: item.id })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"item_pngText","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{
  "chunks": {
    "parameters": "masterpiece, mountain lake at sunrise\nSteps: 20, Sampler: Euler a, CFG scale: 7"
  }
}
```

**Events and errors** Read-only. Returns `404 not_found` if the item or file is missing/offline.

---

## item.similar {#item-similar}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/similar</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_similar</span>
</div>

Find visually similar items ordered from highest to lowest similarity using Curio's stored perceptual thumbnail fingerprints.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Reference item UUID or row ID. |
| `threshold` | `number` | No | `0.8` | Minimum similarity score (`0.0`–`1.0`, fraction of agreeing fingerprint bits). |
| `limit` | `number` | No | `50` | Maximum matches to return (`1`–`1000`). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ id: string; similarity: number }[]`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/similar \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012","threshold":0.85,"limit":10}'
    ```

=== "JavaScript"

    ```javascript
    const matches = await curio.item.similar({ id: item.id, threshold: 0.85, limit: 10 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"item_similar","arguments":{"id":"0194b7a2-...","threshold":0.85,"limit":10}}}
    ```

**Example response**

```json
[
  { "id": "0194b7a9-1122-7c00-9abc-def012345678", "similarity": 0.96 },
  { "id": "0194b7aa-3344-7d11-8def-0123456789ab", "similarity": 0.89 }
]
```

**Events and errors** Read-only. Returns `404 not_found` if `id` does not exist; returns `[]` if the item has no thumbnail fingerprint yet. See also [`duplicates.find`](duplicates.md#duplicates-find).
