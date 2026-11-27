# item (Files)

Operations for reading original bytes, streaming files/previews in plugin pages, rotating or replacing files, uploading via one-use write URLs and managing thumbnails and color palettes.

## item.file {#item-file}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/file</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, Plugin (binary)</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Return an item's raw original file bytes. Over HTTP the response body is binary with the file's MIME `Content-Type`; `curio.js` resolves to a `Blob`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** Binary response (`Blob` in `curio.js`).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/file \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}' \
      --output download.png
    ```

=== "JavaScript"

    ```javascript
    const blob = await curio.item.file({ id: item.id })
    const bytes = new Uint8Array(await blob.arrayBuffer())
    ```

**Example response** Raw file bytes (`200 OK`, `Content-Type: image/png`).

**Events and errors** Read-only. Returns `404 not_found` if the item or disk file is missing/offline. For large files in plugin pages, prefer [`item.fileUrl`](#item-fileurl).

---

## item.fileUrl {#item-fileurl}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Obtain a scoped URL a plugin page can read via `fetch()`, `<video>`, `<audio>`, `pdf.js` or WebGL loaders with full HTTP `Range` support. For formats declared by the calling plugin, sibling files beside the primary file (such as `.mtl` materials, `.bin` buffers or textures) resolve relative to this URL.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |

**Returns** `{ url: string }`.

**Example request**

=== "Plugin page"

    ```javascript
    const curio = Curio.connect()
    const { url } = await curio.item.fileUrl({ id: item.id })
    const res = await fetch(url, { headers: { Range: 'bytes=0-65535' } })
    ```

**Example response**

```json
{ "url": "http://127.0.0.1:52190/plugin-file/abc123.../model.gltf" }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside plugin pages; `404 not_found` if missing/offline.

---

## item.previewUrl {#item-previewurl}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Obtain a scoped URL serving a scaled preview of any visual item (including camera RAW and PSD) at `size` pixels on its long edge, decoded efficiently and cached in memory.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `size` | `number` | Yes | — | Target long edge in pixels (rounded up to multiples of `256`, clamped `256`–`8192`). |

**Returns** `{ url: string }`.

**Example request**

=== "Plugin page"

    ```javascript
    const { url } = await curio.item.previewUrl({ id: item.id, size: 1024 })
    document.querySelector('img').src = url
    ```

**Example response**

```json
{ "url": "http://127.0.0.1:52190/plugin-preview/abc123...?size=1024" }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside plugin pages.

---

## item.thumbnail {#item-thumbnail}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/thumbnail</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_thumbnail</span>
</div>

Return an item's cached thumbnail image (`image/jpeg`, `image/png` or `image/webp`). Over HTTP returns raw bytes; in `curio.js` resolves to a `Blob`; over MCP returns a base64 `image` content block (up to 4 MiB).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** Binary image (`Blob` in `curio.js`, `image` content block in MCP).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/thumbnail \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012"}' \
      --output thumb.webp
    ```

=== "JavaScript"

    ```javascript
    const thumb = await curio.item.thumbnail({ id: item.id })
    img.src = URL.createObjectURL(thumb)
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_thumbnail","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response** Raw image bytes (`200 OK`, `Content-Type: image/webp`).

**Events and errors** Read-only. Returns `404 not_found` if no thumbnail exists yet.

---

## item.rotate {#item-rotate}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/rotate</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_rotate</span>
</div>

Set non-destructive display rotation in the database without modifying the original image file on disk. To permanently rotate the file itself on disk, use [`item.rotateFile`](#item-rotatefile).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `degrees` | `number` | Yes | — | Clockwise rotation angle: `0`, `90`, `180` or `270`. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/rotate \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"degrees":90}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.rotate({ ids: [item.id], degrees: 90 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_rotate","arguments":{"ids":["0194b7a2-..."],"degrees":90}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "metadata"`).

---

## item.rotateFile {#item-rotatefile}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/rotateFile</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_rotateFile</span>
</div>

Rotate the actual image file on disk clockwise in 90° quarter-turns (supports JPEG, PNG, BMP and still WebP). Rewriting the file resets any non-destructive display rotation previously set with [`item.rotate`](#item-rotate) back to `0`. Refused on `read_only` locations.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `turns` | `number` | No | `1` | Number of 90° clockwise turns. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/rotateFile \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-...","turns":1}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.rotateFile({ id: item.id, turns: 1 })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_rotateFile","arguments":{"id":"0194b7a2-...","turns":1}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "file"` and `"thumbnail"`). Returns `400 bad_request` for unsupported formats, `403 not_allowed` if lacking `files.write`, `404 not_found` if the item does not exist or `422 failed` on read-only locations.

---

## item.replaceFile {#item-replacefile}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/replaceFile</span>
  <span class="curio-badge">Permission: files.write (+ files.outside for external path)</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_replaceFile</span>
</div>

Replace an item's file on disk with another file of the same type while preserving its UUID, folders, tags, rating, notes and custom fields. The old file goes to the system trash / Recycle Bin.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `path` | `string` | One of `path` / `data` | — | Replacement file path on disk (requires `files.outside` unless created in plugin run folder). |
| `data` | `string` | One of `path` / `data` | — | Replacement file bytes as base64 or `data:` URI. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/replaceFile \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-...","path":"C:\\Temp\\upscaled.png"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.replaceFile({ id: item.id, data: base64Png })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_replaceFile","arguments":{"id":"0194b7a2-...","path":"C:\\Temp\\upscaled.png"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "file"`). Refused on `read_only` locations (`422 failed`).

---

## item.writeUrl {#item-writeurl}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Create a single-use loopback URL that a plugin page can `PUT` binary file bytes to, avoiding base64 encoding overhead.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Target item UUID or row ID. |
| `mode` | `string` | No | `"replace"` | `replace` (acts like `item.replaceFile`) or `beside` (acts like `item.add.beside`). |
| `name` | `string` | For `beside` | — | New filename with extension when `mode === "beside"`. |

**Returns** `{ url: string }`. The subsequent `PUT` returns `{ success: true; id: string }` (for `replace`) or [`Added`](../types/item.md#added) (for `beside`).

**Example request**

=== "Plugin page"

    ```javascript
    const { url } = await curio.item.writeUrl({ id: item.id, mode: 'beside', name: 'converted.webp' })
    const res = await fetch(url, { method: 'PUT', body: webpBlob })
    const added = await res.json()
    ```

**Example response**

```json
{ "url": "http://127.0.0.1:52190/plugin-write/xyz789..." }
```

**Events and errors** Once the `PUT` completes, emits [`items.changed`](../events/items.md#items-changed) (`what: "file"`, for `replace`) or [`items.added`](../events/items.md#items-added) (for `beside`). Returns `400 bad_request` for an unknown `mode` or a `name` without an extension, `403 not_allowed` outside plugin pages, `404 not_found` if the item does not exist or `422 failed` when replacing a file in a read-only location.

---

## item.setThumbnail {#item-setthumbnail}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/setThumbnail</span>
  <span class="curio-badge">Permission: files.write (+ files.outside for external path)</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_setThumbnail</span>
</div>

Assign a custom thumbnail cover to one or more items from a local image file or base64 image bytes (updates cached aspect ratio and palette automatically).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `path` | `string` | One of `path` / `data` | — | Image file path on disk (requires `files.outside` unless plugin run folder). |
| `data` | `string` | One of `path` / `data` | — | Image bytes as base64 or `data:` URI. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/setThumbnail \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."],"data":"data:image/png;base64,..."}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.setThumbnail({ ids: [item.id], data: coverDataUrl })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_setThumbnail","arguments":{"ids":["0194b7a2-..."],"path":"C:\\Temp\\cover.jpg"}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "thumbnail"`).

---

## item.refreshThumbnail {#item-refreshthumbnail}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/refreshThumbnail</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_refreshThumbnail</span>
</div>

Invalidate and queue regeneration of thumbnails for the given items (reverting custom covers back to generated thumbnails).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/refreshThumbnail \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.refreshThumbnail({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"item_refreshThumbnail","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "thumbnail"`) asynchronously as thumbnails complete.

---

## item.refreshPalette {#item-refreshpalette}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/refreshPalette</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_refreshPalette</span>
</div>

Re-extract dominant color palettes from items' current thumbnails for Color filters and Spectrum view. Extraction always runs asynchronously on a background thread, so the call returns immediately with `queued: true`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Target item IDs. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; queued?: true }` (always includes `queued: true` because palette extraction runs in the background).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/refreshPalette \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    await curio.item.refreshPalette({ ids: [item.id] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"item_refreshPalette","arguments":{"ids":["0194b7a2-..."]}}}
    ```

**Example response**

```json
{ "success": true, "queued": true }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (`what: "palette"`) asynchronously once extraction finishes. Returns `404 not_found` if any item ID does not exist.
